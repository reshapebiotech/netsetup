use super::nmcli::{build_nmcli_connection_args, parse_nmcli_terse_line};
use super::types::{
    ConnectivityReport, CurrentConnectionCheck, EndpointGroupResult, NetworkSettings,
    NetworkTransport,
};
use super::{NetworkError, Result};
use crate::config::EndpointGroup;
use async_trait::async_trait;
use futures_util::future::join_all;
use hickory_server::proto::{
    op::{Message, MessageType, OpCode, Query, ResponseCode},
    rr::{Name, RData, RecordType},
};
use serde_json::Value;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::{net::UdpSocket, process::Command, time::timeout};
use tracing::warn;
use uuid::Uuid;

const NM: &str = "org.freedesktop.NetworkManager";
const NM_PATH: &str = "/org/freedesktop/NetworkManager";
const CHECKPOINT_SECONDS: &str = "120";
const RESTORE_FAILURE: &str = "Could not restore the previous connection. Reconnect to the device’s network when it becomes available, or reboot the device.";

fn failure(message: &str) -> NetworkError {
    NetworkError::ConnectionFailed(message.into())
}

fn activation_error(reason: &str, transport: &NetworkTransport) -> NetworkError {
    // NMDeviceStateReason values are stable across NetworkManager versions.
    match reason
        .split_whitespace()
        .next()
        .and_then(|code| code.parse::<u32>().ok())
    {
        Some(5 | 6 | 15..=17) => failure(
            "The network could not obtain an IP configuration. Check DHCP or the manual IP address, prefix, and gateway.",
        ),
        Some(7..=11) => failure(
            "Network authentication did not complete. Check the Wi-Fi password or enterprise credentials.",
        ),
        Some(53) => failure("The Wi-Fi network could not be found. Check the SSID and signal."),
        _ => failure(match transport {
            NetworkTransport::Wifi => {
                "Could not activate the Wi-Fi connection. Check the SSID, password, and signal."
            }
            NetworkTransport::Ethernet => {
                "Could not activate the Ethernet connection. Check the cable and authentication settings."
            }
        }),
    }
}

#[async_trait]
#[allow(
    clippy::double_must_use,
    reason = "async_trait marks its boxed futures must_use"
)]
pub(super) trait Commands: Send + Sync {
    async fn run(&self, program: &str, args: &[String]) -> Result<String>;
}

pub(super) struct SystemCommands;

#[async_trait]
impl Commands for SystemCommands {
    async fn run(&self, program: &str, args: &[String]) -> Result<String> {
        let output = timeout(
            Duration::from_secs(50),
            Command::new(program)
                .args(args)
                .env("LC_ALL", "C")
                .kill_on_drop(true)
                .output(),
        )
        .await
        .map_err(|_| NetworkError::ConnectionFailed(format!("{program} timed out")))?
        .map_err(|e| NetworkError::Internal(format!("Could not run {program}: {e}")))?;
        if !output.status.success() {
            // NetworkManager errors can echo configuration values, including credentials.
            return Err(NetworkError::ConnectionFailed(format!(
                "{program} could not complete the network operation"
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end_matches(['\r', '\n'])
            .to_string())
    }
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| (*s).to_string()).collect()
}

async fn nmcli(commands: &impl Commands, args: &[&str]) -> Result<String> {
    commands.run("nmcli", &strings(args)).await
}

async fn bus(commands: &impl Commands, method: &str, args: &[&str]) -> Result<Value> {
    let mut argv = strings(&[
        "--system",
        "--json=short",
        "--timeout=10",
        "call",
        NM,
        NM_PATH,
        NM,
        method,
    ]);
    argv.extend(strings(args));
    let raw = commands.run("busctl", &argv).await?;
    if method == "CheckpointDestroy" && raw.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&raw)
        .map_err(|e| NetworkError::Internal(format!("Invalid NetworkManager response: {e}")))
}

fn object_path(value: &Value) -> Result<String> {
    value["data"][0]
        .as_str()
        .filter(|s| s.starts_with("/org/freedesktop/NetworkManager/"))
        .map(str::to_string)
        .ok_or_else(|| NetworkError::Internal("Missing NetworkManager object path".into()))
}

async fn rollback(commands: &impl Commands, checkpoint: &str) -> Result<()> {
    let response = bus(commands, "CheckpointRollback", &["o", checkpoint])
        .await
        .map_err(|_| failure(RESTORE_FAILURE))?;
    let results = response["data"][0]
        .as_object()
        .ok_or_else(|| failure(RESTORE_FAILURE))?;
    if results.is_empty() || results.values().any(|v| v.as_u64() != Some(0)) {
        return Err(failure(RESTORE_FAILURE));
    }
    Ok(())
}

pub(super) async fn select_interface(
    commands: &impl Commands,
    transport: &NetworkTransport,
) -> Result<String> {
    let devices = nmcli(commands, &["-t", "-f", "DEVICE,TYPE", "device", "status"]).await?;
    let kind = match transport {
        NetworkTransport::Wifi => "wifi",
        NetworkTransport::Ethernet => "ethernet",
    };
    let interfaces: Vec<_> = devices
        .lines()
        .filter_map(|line| {
            let fields = parse_nmcli_terse_line(line);
            (fields.len() == 2 && fields[1] == kind).then(|| fields[0].clone())
        })
        .collect();
    if *transport == NetworkTransport::Wifi {
        let preferred = std::env::var("AP_IFACE").unwrap_or_else(|_| "wlan0".into());
        if interfaces.contains(&preferred) {
            return Ok(preferred);
        }
    }
    if interfaces.len() == 1 {
        return Ok(interfaces[0].clone());
    }
    Err(NetworkError::ConnectionFailed(format!(
        "Expected one {kind} interface; found {}",
        interfaces.len()
    )))
}

pub(super) async fn run(
    commands: &impl Commands,
    settings: &NetworkSettings,
    transport: &NetworkTransport,
    interface: &str,
    previous: Option<&ConnectivityReport>,
    old_profile: Option<&str>,
    groups: &[EndpointGroup],
) -> Result<ConnectivityReport> {
    let original = nmcli(
        commands,
        &["-g", "GENERAL.CON-UUID", "device", "show", interface],
    )
    .await?;
    let device = object_path(&bus(commands, "GetDeviceByIpIface", &["s", interface]).await?)?;
    // DELETE_NEW_CONNECTIONS ensures temporary profiles disappear on automatic rollback too.
    let checkpoint = bus(
            commands,
            "CheckpointCreate",
            &["aouu", "1", &device, CHECKPOINT_SECONDS, "2"],
        ).await.and_then(|response| object_path(&response))
        .map_err(|_| failure("Could not create a NetworkManager recovery checkpoint. No network changes were applied."))?;
    let profile = format!("reshape-pending-{}", Uuid::new_v4());
    let operation = timeout(Duration::from_secs(90), async {
        let mut args = build_nmcli_connection_args(settings, *transport == NetworkTransport::Wifi, &profile);
        args.extend(strings(&["ifname", interface, "connection.autoconnect", "no"]));
        commands.run("nmcli", &args).await
            .map_err(|_| failure("Could not prepare the network profile. Check the network and IP settings."))?;
        if nmcli(commands, &["--wait", "40", "connection", "up", "id", &profile, "ifname", interface]).await.is_err() {
            let reason = nmcli(commands, &["-g", "GENERAL.REASON", "device", "show", interface]).await.unwrap_or_default();
            return Err(activation_error(&reason, transport));
        }
        let report = probe(commands, interface, groups).await?;
        if let Some(previous) = previous {
            if !report.preserves(previous) {
                return Err(NetworkError::ConnectionFailed(
                    "The saved connection did not pass the checks it passed during testing; restoring the previous connection.".into(),
                ));
            }
            let managed = match transport {
                NetworkTransport::Wifi => "reshape-managed-wifi",
                NetworkTransport::Ethernet => "reshape-managed-ethernet",
            };
            nmcli(commands, &["connection", "modify", "id", &profile, "connection.id", managed, "connection.autoconnect", "yes", "connection.autoconnect-priority", "100"]).await
                .map_err(|_| failure("Could not save the verified network profile."))?;
            bus(commands, "CheckpointDestroy", &["o", &checkpoint]).await
                .map_err(|_| failure("Could not commit the verified network configuration."))?;
        }
        Ok(report)
    }).await.unwrap_or_else(|_| Err(NetworkError::ConnectionFailed("Network operation timed out; restoring the previous connection.".into())));

    if previous.is_none() || operation.is_err() {
        rollback(commands, &checkpoint).await?;
        timeout(Duration::from_secs(30), async {
            loop {
                let uuid = nmcli(commands, &["-g", "GENERAL.CON-UUID", "device", "show", interface]).await?;
                let state = nmcli(commands, &["-g", "GENERAL.STATE", "device", "show", interface]).await?;
                if uuid == original && (state.starts_with("100") || Uuid::parse_str(&original).is_err()) {
                    return Ok::<_, NetworkError>(());
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }).await.map_err(|_| failure("The previous connection is taking too long to return. Reconnect to the device’s network or reboot the device."))?
            .map_err(|_| failure(RESTORE_FAILURE))?;
    }
    let report = operation.map_err(|error| match error {
        NetworkError::ConnectionFailed(message) => failure(&format!(
            "{message} The previous connection has been restored."
        )),
        other => other,
    })?;
    if previous.is_some() {
        // Remove the superseded profile only after the replacement has been verified and committed.
        if let Some(uuid) = old_profile
            && let Err(e) = nmcli(commands, &["connection", "delete", "uuid", uuid]).await
        {
            warn!("Could not remove superseded profile {uuid}: {e}");
        }
    }
    Ok(report)
}

pub(super) async fn check_current_connections(
    commands: &impl Commands,
    groups: &[EndpointGroup],
) -> Result<Vec<CurrentConnectionCheck>> {
    let active = nmcli(
        commands,
        &[
            "-t",
            "-f",
            "UUID,TYPE,DEVICE,NAME",
            "connection",
            "show",
            "--active",
        ],
    )
    .await?;
    let mut checks = Vec::new();
    for line in active.lines() {
        let fields = parse_nmcli_terse_line(line);
        if fields.len() != 4 || fields[2].is_empty() || fields[2] == "--" {
            continue;
        }
        let transport = match fields[1].as_str() {
            "802-11-wireless" => {
                let mode = nmcli(
                    commands,
                    &[
                        "-g",
                        "802-11-wireless.mode",
                        "connection",
                        "show",
                        "uuid",
                        &fields[0],
                    ],
                )
                .await?;
                if mode.trim() == "ap" {
                    continue;
                }
                NetworkTransport::Wifi
            }
            "802-3-ethernet" => NetworkTransport::Ethernet,
            _ => continue,
        };
        checks.push(async move {
            let (configuration, result) = tokio::join!(
                timeout(
                    Duration::from_secs(10),
                    super::diagnostics::collect(commands, &fields[0], &fields[2], transport)
                ),
                timeout(Duration::from_secs(45), probe(commands, &fields[2], groups)),
            );
            let (configuration, configuration_error) = match configuration {
                Ok(Ok(configuration)) => (Some(configuration), None),
                Ok(Err(error)) => (None, Some(error.to_string())),
                Err(_) => (
                    None,
                    Some("Reading the current network configuration timed out.".into()),
                ),
            };
            let (report, error) = match result {
                Ok(Ok(report)) => (Some(report), None),
                Ok(Err(error)) => (None, Some(error.to_string())),
                Err(_) => (
                    None,
                    Some("The connectivity check timed out. Try again.".into()),
                ),
            };
            CurrentConnectionCheck {
                name: fields[3].clone(),
                interface: fields[2].clone(),
                report,
                error,
                configuration,
                configuration_error,
            }
        });
    }
    Ok(join_all(checks).await)
}

async fn probe(
    commands: &impl Commands,
    interface: &str,
    groups: &[EndpointGroup],
) -> Result<ConnectivityReport> {
    let addresses = nmcli(
        commands,
        &["-g", "IP4.ADDRESS", "device", "show", interface],
    )
    .await
    .map_err(|_| failure("Could not read the connection's IP address from NetworkManager."))?;
    let address = addresses
        .lines()
        .filter_map(|s| s.split('/').next()?.parse::<Ipv4Addr>().ok())
        .find(|ip| !ip.is_unspecified() && !ip.is_link_local() && !ip.is_loopback())
        .ok_or_else(|| {
            NetworkError::ConnectionFailed(
                "The network connected but did not provide a usable IPv4 address. Check DHCP or the manual IP settings.".into(),
            )
        })?;
    let gateway = nmcli(
        commands,
        &["-g", "IP4.GATEWAY", "device", "show", interface],
    )
    .await?;
    let dns = nmcli(commands, &["-g", "IP4.DNS", "device", "show", interface]).await?;
    let dns = dns_servers(&dns);
    let gateway_check = async {
        if let Ok(gateway) = gateway.parse::<Ipv4Addr>() {
            Some(
                commands
                    .run(
                        "ping",
                        &strings(&["-I", interface, "-c", "1", "-W", "2", &gateway.to_string()]),
                    )
                    .await
                    .is_ok(),
            )
        } else {
            None
        }
    };
    let (gateway_reachable, internet_verified, endpoint_groups) = tokio::join!(
        gateway_check,
        http_probe(
            commands,
            interface,
            address,
            &dns,
            "https://connectivitycheck.gstatic.com/generate_204",
            true
        ),
        probe_groups(commands, interface, address, &dns, groups),
    );
    Ok(ConnectivityReport {
        interface: interface.into(),
        address: address.to_string(),
        gateway_reachable,
        internet_verified,
        endpoint_groups,
    })
}

async fn probe_groups(
    commands: &impl Commands,
    interface: &str,
    source: Ipv4Addr,
    dns: &[Ipv4Addr],
    groups: &[EndpointGroup],
) -> Vec<EndpointGroupResult> {
    let jobs: Vec<_> = groups
        .iter()
        .enumerate()
        .flat_map(|(group_index, group)| {
            group
                .endpoints
                .iter()
                .enumerate()
                .map(move |(endpoint_index, endpoint)| (group_index, endpoint_index, endpoint))
        })
        .collect();
    let mut results = Vec::new();
    for batch in jobs.chunks(6) {
        let mut checks = Vec::new();
        for &(group_index, endpoint_index, endpoint) in batch {
            checks.push(async move {
                let reachable = timeout(
                    Duration::from_secs(10),
                    http_probe(commands, interface, source, dns, &endpoint.url, false),
                )
                .await
                .unwrap_or(false);
                (group_index, endpoint_index, reachable)
            });
        }
        results.extend(join_all(checks).await);
    }
    let mut groups: Vec<_> = groups
        .iter()
        .map(|group| EndpointGroupResult::from_config(group, false))
        .collect();
    for (group, endpoint, reachable) in results {
        groups[group].endpoints[endpoint].reachable = reachable;
    }
    groups
}

async fn http_probe(
    commands: &impl Commands,
    interface: &str,
    source: Ipv4Addr,
    dns: &[Ipv4Addr],
    url: &str,
    internet: bool,
) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let port = parsed.port_or_known_default().unwrap_or(443);
    let addresses = if let Ok(ip) = host.parse::<Ipv4Addr>() {
        vec![ip]
    } else {
        let Ok(addresses) = resolve(interface, source, dns, host).await else {
            return false;
        };
        addresses
    };
    for address in addresses {
        let mut args = strings(&[
            "--disable",
            "--globoff",
            "--proto",
            "=https",
            "--silent",
            "--show-error",
            "--ipv4",
            "--noproxy",
            "*",
            "--interface",
            &format!("if!{interface}"),
            "--resolve",
            &format!("{host}:{port}:{address}"),
            "--connect-timeout",
            "3",
            "--max-time",
            "6",
            "--max-filesize",
            "65536",
            "--output",
            "/dev/null",
            "--write-out",
            "%{http_code}:%{size_download}",
            parsed.as_str(),
        ]);
        if !internet {
            args.insert(1, "--head".into());
        }
        let result = commands.run("curl", &args).await;
        if let Ok(response) = result
            && response_verifies(&response, internet)
        {
            return true;
        }
    }
    false
}

fn dns_servers(value: &str) -> Vec<Ipv4Addr> {
    value
        .split(|c: char| c.is_whitespace() || c == '|')
        .filter_map(|entry| entry.parse().ok())
        .collect()
}

fn response_verifies(response: &str, internet: bool) -> bool {
    if internet {
        return response == "204:0";
    }
    // An authenticated TLS response, including 401/404, proves service reachability.
    response
        .split(':')
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        .is_some_and(|code| (200..500).contains(&code))
}

async fn resolve(
    interface: &str,
    source: Ipv4Addr,
    servers: &[Ipv4Addr],
    host: &str,
) -> Result<Vec<Ipv4Addr>> {
    timeout(Duration::from_secs(8), async {
        for server in servers {
            if let Ok(addresses) = dns_query(interface, source, *server, host).await
                && !addresses.is_empty()
            {
                return Ok(addresses);
            }
        }
        Err(NetworkError::ConnectionFailed(
            "DNS could not be verified on this connection".into(),
        ))
    })
    .await
    .unwrap_or_else(|_| Err(NetworkError::ConnectionFailed("DNS check timed out".into())))
}

async fn dns_query(
    interface: &str,
    source: Ipv4Addr,
    server: Ipv4Addr,
    host: &str,
) -> std::io::Result<Vec<Ipv4Addr>> {
    let socket = UdpSocket::bind((source, 0)).await?;
    #[cfg(target_os = "linux")]
    socket2::SockRef::from(&socket).bind_device(Some(interface.as_bytes()))?;
    #[cfg(not(target_os = "linux"))]
    let _ = interface;
    socket.connect(SocketAddr::from((server, 53))).await?;
    let id = Uuid::new_v4().as_u128() as u16;
    let name = Name::from_ascii(format!("{host}.")).map_err(std::io::Error::other)?;
    let mut message = Message::new(id, MessageType::Query, OpCode::Query);
    message.metadata.recursion_desired = true;
    message.add_query(Query::query(name, RecordType::A));
    let bytes = message.to_vec().map_err(std::io::Error::other)?;
    socket.send(&bytes).await?;
    let mut buffer = [0; 4096];
    let len = timeout(Duration::from_secs(2), socket.recv(&mut buffer)).await??;
    dns_answers(&buffer[..len], &message)
}

fn dns_answers(bytes: &[u8], query: &Message) -> std::io::Result<Vec<Ipv4Addr>> {
    let response = Message::from_vec(bytes).map_err(std::io::Error::other)?;
    if response.id != query.id
        || response.message_type != MessageType::Response
        || response.response_code != ResponseCode::NoError
        || response.truncation
        || response.queries != query.queries
    {
        return Err(std::io::Error::other("Invalid DNS response"));
    }
    Ok(response
        .answers
        .iter()
        .filter_map(|record| match &record.data {
            RData::A(address) => Some(address.0),
            _ => None,
        })
        .take(2)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    async fn run(
        commands: &impl Commands,
        settings: &NetworkSettings,
        transport: &NetworkTransport,
        interface: &str,
        previous: Option<&ConnectivityReport>,
        old_profile: Option<&str>,
    ) -> Result<ConnectivityReport> {
        super::run(
            commands,
            settings,
            transport,
            interface,
            previous,
            old_profile,
            &[],
        )
        .await
    }

    struct EndpointCommands;
    #[async_trait]
    impl Commands for EndpointCommands {
        async fn run(&self, program: &str, args: &[String]) -> Result<String> {
            assert_eq!(program, "curl");
            assert_eq!(args[0], "--disable");
            assert!(args.contains(&"--head".into()));
            assert!(args.contains(&"if!wlan0".into()));
            assert!(args.contains(&"192.0.2.1:8443:192.0.2.1".into()));
            assert!(!args.contains(&"--location".into()));
            assert!(!args.contains(&"--insecure".into()));
            let url = args.last().unwrap();
            Ok(if url.ends_with("/api") {
                "401:0"
            } else if url.ends_with("/images") {
                "403:0"
            } else {
                "503:0"
            }
            .into())
        }
    }
    fn endpoint_groups() -> Vec<EndpointGroup> {
        vec![EndpointGroup {
            name: "Core".into(),
            required: true,
            endpoints: ["api", "images", "unavailable"]
                .map(|name| crate::config::Endpoint {
                    name: name.into(),
                    url: format!("https://192.0.2.1:8443/{name}"),
                    description: String::new(),
                })
                .to_vec(),
        }]
    }
    #[tokio::test]
    async fn grouped_probes_accept_authentication_responses_and_preserve_configured_order() {
        let groups = probe_groups(
            &EndpointCommands,
            "wlan0",
            Ipv4Addr::new(192, 168, 1, 42),
            &[],
            &endpoint_groups(),
        )
        .await;
        assert_eq!(
            groups[0]
                .endpoints
                .iter()
                .map(|e| e.reachable)
                .collect::<Vec<_>>(),
            vec![true, true, false]
        );
        assert_eq!(groups[0].endpoints[1].name, "images");
    }
    #[test]
    fn save_preservation_detects_individual_endpoint_regressions() {
        let groups = endpoint_groups();
        let previous = ConnectivityReport {
            endpoint_groups: vec![EndpointGroupResult::from_config(&groups[0], true)],
            ..local_report()
        };
        let mut current = previous.clone();
        current.endpoint_groups[0].endpoints[1].reachable = false;
        assert!(!current.preserves(&previous));
        assert!(previous.preserves(&current));
    }
    #[tokio::test]
    async fn an_endpoint_regression_rolls_back_without_deleting_the_saved_profile() {
        let commands = FakeCommands::default();
        let groups = endpoint_groups();
        let previous = ConnectivityReport {
            endpoint_groups: vec![EndpointGroupResult::from_config(&groups[0], true)],
            ..local_report()
        };
        let error = super::run(
            &commands,
            &settings(),
            &NetworkTransport::Wifi,
            "wlan0",
            Some(&previous),
            Some("old-uuid"),
            &groups,
        )
        .await
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("checks it passed during testing")
        );
        let calls = commands.calls.lock().unwrap();
        assert!(calls.iter().any(|call| call.contains("CheckpointRollback")));
        assert!(!calls.iter().any(|call| call.contains("connection delete")));
    }

    #[derive(Default)]
    struct ReadOnlyCommands {
        fail_configuration: bool,
        fail_probe: bool,
    }
    #[async_trait]
    impl Commands for ReadOnlyCommands {
        async fn run(&self, program: &str, args: &[String]) -> Result<String> {
            let call = format!("{program} {}", args.join(" "));
            match call.as_str() {
                "nmcli -t -f UUID,TYPE,DEVICE,NAME connection show --active" => Ok("ap-id:802-11-wireless:wlan0:Device AP\neth-id:802-3-ethernet:eth0:Wired\nloopback:loopback:lo:lo".into()),
                "nmcli -g ipv4.method connection show uuid eth-id" => Ok("auto".into()),
                "nmcli -t --escape no -f GENERAL.HWADDR,IP4.ADDRESS,IP4.GATEWAY,IP4.DNS device show eth0" => {
                    if self.fail_configuration { return Err(failure("Configuration unavailable")); }
                    Ok("GENERAL.HWADDR:02:00:00:00:00:01\nIP4.ADDRESS[1]:192.168.1.42/24\nIP4.GATEWAY:192.168.1.1\nIP4.DNS[1]:192.168.1.1".into())
                },
                "nmcli -g 802-11-wireless.mode connection show uuid ap-id" => Ok("ap".into()),
                "nmcli -g IP4.ADDRESS device show eth0" => {
                    if self.fail_probe { return Err(failure("Probe unavailable")); }
                    Ok("192.168.1.42/24".into())
                },
                "nmcli -g IP4.GATEWAY device show eth0" => Ok("192.168.1.1".into()),
                "nmcli -g IP4.DNS device show eth0" => Ok(String::new()),
                "ping -I eth0 -c 1 -W 2 192.168.1.1" => Ok(String::new()),
                _ => panic!("Unexpected operation in read-only check: {call}"),
            }
        }
    }

    #[tokio::test]
    async fn current_checks_skip_access_points_and_never_modify_connections() {
        let results = check_current_connections(&ReadOnlyCommands::default(), &[])
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        let report = results[0].report.as_ref().unwrap();
        assert_eq!(report.interface, "eth0");
        assert_eq!(report.gateway_reachable, Some(true));
        assert!(!report.internet_verified);
    }

    #[tokio::test]
    async fn network_details_remain_available_when_connectivity_checks_fail() {
        let commands = ReadOnlyCommands {
            fail_probe: true,
            ..Default::default()
        };
        let results = check_current_connections(&commands, &[]).await.unwrap();
        assert!(results[0].report.is_none());
        assert!(results[0].error.is_some());
        let config = results[0].configuration.as_ref().unwrap();
        assert_eq!(config.ipv4_method, "auto");
        assert_eq!(config.addresses[0].prefix, 24);
        assert_eq!(config.gateway.as_deref(), Some("192.168.1.1"));
        assert!(results[0].configuration_error.is_none());
    }

    #[tokio::test]
    async fn configuration_failures_do_not_hide_connectivity_results() {
        let commands = ReadOnlyCommands {
            fail_configuration: true,
            ..Default::default()
        };
        let results = check_current_connections(&commands, &[]).await.unwrap();
        assert!(results[0].configuration.is_none());
        assert!(results[0].configuration_error.is_some());
        assert!(results[0].report.is_some());
        assert!(results[0].error.is_none());
    }

    #[derive(Default)]
    struct FakeCommands {
        calls: Mutex<Vec<String>>,
        fail_activation: bool,
        fail_checkpoint: bool,
        fail_commit: bool,
        fail_restore: bool,
        disconnected: bool,
        devices: &'static str,
        activation_reason: &'static str,
        missing_address: bool,
    }

    #[async_trait]
    impl Commands for FakeCommands {
        async fn run(&self, program: &str, args: &[String]) -> Result<String> {
            let call = format!("{program} {}", args.join(" "));
            self.calls.lock().unwrap().push(call.clone());
            if program == "busctl" {
                if call.contains("CheckpointCreate") && self.fail_checkpoint {
                    return Err(NetworkError::Internal("Checkpoints unavailable".into()));
                }
                if call.contains("CheckpointDestroy") && self.fail_commit {
                    return Err(NetworkError::Internal("Commit failed".into()));
                }
                if call.contains("CheckpointRollback") {
                    let code = if self.fail_restore { 1 } else { 0 };
                    return Ok(serde_json::json!({"type": "a{su}", "data": [{"/org/freedesktop/NetworkManager/Devices/1": code}]}).to_string());
                }
                if call.contains("CheckpointDestroy") {
                    return Ok(String::new());
                }
                return Ok(
                    serde_json::json!({"data": ["/org/freedesktop/NetworkManager/Checkpoint/1"]})
                        .to_string(),
                );
            }
            if call.contains("connection up") && self.fail_activation {
                return Err(NetworkError::ConnectionFailed(
                    "Incorrect credentials".into(),
                ));
            }
            if call.contains("DEVICE,TYPE") {
                return Ok(self.devices.into());
            }
            if call.contains("GENERAL.STATE") {
                return Ok(if self.disconnected {
                    "30 (disconnected)"
                } else {
                    "100 (connected)"
                }
                .into());
            }
            if call.contains("GENERAL.CON-UUID") {
                return Ok(if self.disconnected {
                    ""
                } else {
                    "11111111-1111-1111-1111-111111111111"
                }
                .into());
            }
            if call.contains("GENERAL.REASON") {
                return Ok(self.activation_reason.into());
            }
            if call.contains("IP4.ADDRESS") {
                return Ok(if self.missing_address {
                    ""
                } else {
                    "192.168.1.42/24"
                }
                .into());
            }
            if call.contains("IP4.GATEWAY") {
                return Ok("192.168.1.1".into());
            }
            // No DNS servers: remote checks remain unverified without making real requests.
            Ok(String::new())
        }
    }

    fn settings() -> NetworkSettings {
        NetworkSettings {
            ssid: Some("Lab".into()),
            password: Some("secret".into()),
            id: None,
            eap_login: None,
            eap_password: None,
            ipv4: None,
        }
    }

    fn local_report() -> ConnectivityReport {
        ConnectivityReport {
            interface: "wlan0".into(),
            address: "192.168.1.42".into(),
            gateway_reachable: Some(true),
            internet_verified: false,
            endpoint_groups: vec![],
        }
    }

    #[tokio::test]
    async fn activation_failures_explain_the_phase_and_confirm_restore() {
        for (reason, expected) in [
            (
                "7 (Secrets were required)",
                "authentication did not complete",
            ),
            (
                "15 (DHCP client failed to start)",
                "could not obtain an IP configuration",
            ),
            ("53 (SSID not found)", "Wi-Fi network could not be found"),
            ("0 (No reason)", "Could not activate the Wi-Fi connection"),
        ] {
            let commands = FakeCommands {
                fail_activation: true,
                activation_reason: reason,
                ..Default::default()
            };
            let error = run(
                &commands,
                &settings(),
                &NetworkTransport::Wifi,
                "wlan0",
                None,
                None,
            )
            .await
            .unwrap_err()
            .to_string();
            assert!(error.contains(expected), "{error}");
            assert!(error.contains("previous connection has been restored"));
            assert!(!error.contains("secret"));
        }
    }

    #[tokio::test]
    async fn missing_address_is_distinct_from_activation_failure() {
        let commands = FakeCommands {
            missing_address: true,
            ..Default::default()
        };
        let error = run(
            &commands,
            &settings(),
            &NetworkTransport::Wifi,
            "wlan0",
            None,
            None,
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(error.contains("did not provide a usable IPv4 address"));
        assert!(error.contains("previous connection has been restored"));
    }

    #[tokio::test]
    async fn failed_restore_does_not_claim_recovery_succeeded() {
        let commands = FakeCommands {
            fail_activation: true,
            fail_restore: true,
            ..Default::default()
        };
        let error = run(
            &commands,
            &settings(),
            &NetworkTransport::Wifi,
            "wlan0",
            None,
            None,
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(error.contains("Could not restore the previous connection"));
        assert!(!error.contains("has been restored"));
    }

    #[test]
    fn dns_servers_accept_networkmanager_list_output() {
        let expected = vec![Ipv4Addr::new(9, 9, 9, 9), Ipv4Addr::new(149, 112, 112, 112)];
        assert_eq!(dns_servers("9.9.9.9 | 149.112.112.112"), expected);
        assert_eq!(dns_servers("9.9.9.9\n149.112.112.112"), expected);
        assert!(dns_servers("--").is_empty());
    }

    #[test]
    fn probe_results_do_not_confuse_portals_or_server_failures_with_success() {
        assert!(response_verifies("204:0", true));
        for response in ["200:400", "302:0", "204:50", "500:0", "000:0", "invalid"] {
            assert!(!response_verifies(response, true));
        }
        for response in ["200:10", "401:0", "404:100"] {
            assert!(response_verifies(response, false));
        }
        assert!(!response_verifies("503:0", false));
    }

    #[test]
    fn dns_checks_reject_mismatched_or_incomplete_answers() {
        use hickory_server::proto::rr::{Record, rdata::A};
        let name = Name::from_ascii("ris-api.reshapebiotech.com.").unwrap();
        let mut query = Message::new(42, MessageType::Query, OpCode::Query);
        query.add_query(Query::query(name.clone(), RecordType::A));
        let mut response = query.clone();
        response.metadata.message_type = MessageType::Response;
        response.add_answer(Record::from_rdata(
            name.clone(),
            60,
            RData::A(A(Ipv4Addr::new(192, 0, 2, 1))),
        ));
        assert_eq!(
            dns_answers(&response.to_vec().unwrap(), &query).unwrap(),
            vec![Ipv4Addr::new(192, 0, 2, 1)]
        );
        response.metadata.id = 43;
        assert!(dns_answers(&response.to_vec().unwrap(), &query).is_err());
        response.metadata.id = 42;
        response.metadata.truncation = true;
        assert!(dns_answers(&response.to_vec().unwrap(), &query).is_err());
        response.metadata.truncation = false;
        response.metadata.response_code = ResponseCode::NXDomain;
        assert!(dns_answers(&response.to_vec().unwrap(), &query).is_err());
        response.metadata.response_code = ResponseCode::NoError;
        response.queries.clear();
        response.add_query(Query::query(
            Name::from_ascii("example.com").unwrap(),
            RecordType::A,
        ));
        assert!(dns_answers(&response.to_vec().unwrap(), &query).is_err());
    }

    #[tokio::test]
    async fn ethernet_test_can_restore_an_initially_disconnected_interface() {
        let commands = FakeCommands {
            disconnected: true,
            ..Default::default()
        };
        assert!(
            run(
                &commands,
                &settings(),
                &NetworkTransport::Ethernet,
                "eth0",
                None,
                None
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn ambiguous_ethernet_interfaces_are_rejected_before_changes() {
        let commands = FakeCommands {
            devices: "eth0:ethernet\neth1:ethernet\nwlan0:wifi",
            ..Default::default()
        };
        assert!(
            select_interface(&commands, &NetworkTransport::Ethernet)
                .await
                .is_err()
        );
        assert_eq!(commands.calls.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn successful_test_restores_ap_and_does_not_touch_saved_profiles() {
        let commands = FakeCommands::default();
        let report = run(
            &commands,
            &settings(),
            &NetworkTransport::Wifi,
            "wlan0",
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(report, local_report());
        let calls = commands.calls.lock().unwrap();
        let checkpoint = calls
            .iter()
            .position(|s| s.contains("CheckpointCreate"))
            .unwrap();
        let add = calls
            .iter()
            .position(|s| s.contains("connection add"))
            .unwrap();
        let rollback = calls
            .iter()
            .position(|s| s.contains("CheckpointRollback"))
            .unwrap();
        let restored = calls
            .iter()
            .position(|s| s.contains("GENERAL.STATE"))
            .unwrap();
        assert!(checkpoint < add && add < rollback && rollback < restored);
        assert!(calls[checkpoint].ends_with("120 2"));
        assert!(calls[add].contains("connection.autoconnect no"));
        assert!(
            !calls
                .iter()
                .any(|s| s.contains("connection delete") || s.contains("CheckpointDestroy"))
        );
        assert!(calls.iter().any(|s| s.starts_with("ping -I wlan0")));
    }

    #[tokio::test]
    async fn failed_activation_rolls_back() {
        let commands = FakeCommands {
            fail_activation: true,
            ..Default::default()
        };
        assert!(
            run(
                &commands,
                &settings(),
                &NetworkTransport::Wifi,
                "wlan0",
                None,
                None
            )
            .await
            .is_err()
        );
        assert!(
            commands
                .calls
                .lock()
                .unwrap()
                .iter()
                .any(|s| s.contains("CheckpointRollback"))
        );
    }

    #[tokio::test]
    async fn missing_checkpoint_support_prevents_network_changes() {
        let commands = FakeCommands {
            fail_checkpoint: true,
            ..Default::default()
        };
        assert!(
            run(
                &commands,
                &settings(),
                &NetworkTransport::Wifi,
                "wlan0",
                None,
                None
            )
            .await
            .is_err()
        );
        assert!(
            !commands
                .calls
                .lock()
                .unwrap()
                .iter()
                .any(|s| s.contains("connection add"))
        );
    }

    #[tokio::test]
    async fn failed_restore_never_publishes_success() {
        let commands = FakeCommands {
            fail_restore: true,
            ..Default::default()
        };
        assert!(
            run(
                &commands,
                &settings(),
                &NetworkTransport::Wifi,
                "wlan0",
                None,
                None
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn save_anyway_commits_local_connectivity_before_removing_old_profile() {
        let commands = FakeCommands::default();
        run(
            &commands,
            &settings(),
            &NetworkTransport::Wifi,
            "wlan0",
            Some(&local_report()),
            Some("old-uuid"),
        )
        .await
        .unwrap();
        let calls = commands.calls.lock().unwrap();
        let commit = calls
            .iter()
            .position(|s| s.contains("CheckpointDestroy"))
            .unwrap();
        let delete = calls
            .iter()
            .position(|s| s.contains("connection delete uuid old-uuid"))
            .unwrap();
        assert!(commit < delete);
        assert!(!calls.iter().any(|s| s.contains("CheckpointRollback")));
    }

    #[tokio::test]
    async fn save_regression_rolls_back_and_preserves_old_profile() {
        let commands = FakeCommands::default();
        let previous = ConnectivityReport {
            internet_verified: true,
            ..local_report()
        };
        assert!(
            run(
                &commands,
                &settings(),
                &NetworkTransport::Wifi,
                "wlan0",
                Some(&previous),
                Some("old-uuid")
            )
            .await
            .is_err()
        );
        let calls = commands.calls.lock().unwrap();
        assert!(calls.iter().any(|s| s.contains("CheckpointRollback")));
        assert!(
            !calls
                .iter()
                .any(|s| s.contains("connection delete") || s.contains("CheckpointDestroy"))
        );
    }

    #[tokio::test]
    async fn failed_commit_preserves_old_profile_and_rolls_back() {
        let commands = FakeCommands {
            fail_commit: true,
            ..Default::default()
        };
        assert!(
            run(
                &commands,
                &settings(),
                &NetworkTransport::Wifi,
                "wlan0",
                Some(&local_report()),
                Some("old-uuid")
            )
            .await
            .is_err()
        );
        let calls = commands.calls.lock().unwrap();
        assert!(calls.iter().any(|s| s.contains("CheckpointRollback")));
        assert!(!calls.iter().any(|s| s.contains("connection delete")));
    }

    #[test]
    fn blocked_gateway_probe_does_not_override_working_internet() {
        let previous = ConnectivityReport {
            internet_verified: true,
            ..local_report()
        };
        let current = ConnectivityReport {
            gateway_reachable: Some(false),
            ..previous.clone()
        };
        assert!(current.preserves(&previous));
        assert!(current.preserves(&local_report()));
        assert!(!current.preserves(&ConnectivityReport {
            endpoint_groups: vec![EndpointGroupResult {
                name: "Core".into(),
                required: true,
                endpoints: vec![super::super::types::EndpointResult {
                    name: "API".into(),
                    url: "https://example.com".into(),
                    description: String::new(),
                    reachable: true
                }]
            }],
            ..previous
        }));
    }
}
