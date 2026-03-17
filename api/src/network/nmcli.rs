use crate::network::types::{
    AccessPointStatus, ConnectionState, Connectivity, EthernetNetwork, IpMethod, NetworkSettings, NetworkTransport,
    SavedNetwork, WifiNetwork,
};
use crate::network::{NetworkController, NetworkError, Result as NetResult};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

#[derive(Clone)]
struct TestRecord {
    state: ConnectionState,
    settings: NetworkSettings,
    transport: NetworkTransport,
}

pub struct NmcliNetworkController {
    active_tests: Arc<Mutex<HashMap<Uuid, TestRecord>>>,
}

impl NmcliNetworkController {
    pub fn new() -> Self {
        Self {
            active_tests: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Helper to parse nmcli -t output respecting escaped colons.
fn parse_nmcli_terse_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next_c) = chars.peek() {
                if *next_c == ':' || *next_c == '\\' {
                    current.push(*next_c);
                    chars.next(); // consume escaped char
                    continue;
                }
            }
            current.push(c);
        } else if c == ':' {
            fields.push(current);
            current = String::new();
        } else {
            current.push(c);
        }
    }
    fields.push(current);
    fields
}

pub fn parse_scan_output(stdout: &str) -> Vec<WifiNetwork> {
    let mut networks = Vec::new();

    for line in stdout.lines() {
        let fields = parse_nmcli_terse_line(line);
        if fields.len() >= 4 {
            let ssid = fields[1].clone();
            if ssid.is_empty() {
                continue;
            }

            let signal = fields[2].parse::<u8>().unwrap_or(0);
            let security = fields[3..].join(":");

            networks.push(WifiNetwork {
                ssid,
                signal_strength: signal,
                security,
            });
        }
    }

    networks.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
    let mut seen = HashSet::new();
    networks.retain(|n| seen.insert(n.ssid.clone()));

    networks
}

pub fn parse_saved_output(stdout: &str) -> Vec<SavedNetwork> {
    let mut saved = Vec::new();

    for line in stdout.lines() {
        let fields = parse_nmcli_terse_line(line);
        // We requested: UUID,NAME,TYPE
        if fields.len() >= 3 {
            let transport = match fields[2].as_str() {
                "802-11-wireless" => NetworkTransport::Wifi,
                "802-3-ethernet" => NetworkTransport::Ethernet,
                _ => continue,
            };
            saved.push(SavedNetwork {
                id: fields[0].clone(),
                transport,
                display_name: fields[1].clone(),
                managed: true,
            });
        }
    }

    saved
}

fn build_nmcli_connection_args(settings: &NetworkSettings, is_wifi: bool) -> Vec<String> {
    let mut args = vec!["connection".to_string(), "add".to_string()];
    
    if is_wifi {
        args.push("type".to_string());
        args.push("wifi".to_string());
        if let Some(ssid) = &settings.ssid {
            args.push("con-name".to_string());
            args.push(ssid.clone());
            args.push("ssid".to_string());
            args.push(ssid.clone());
        }
    } else {
        args.push("type".to_string());
        args.push("ethernet".to_string());
        args.push("con-name".to_string());
        args.push("Wired connection".to_string());
    }

    if let Some(eap_login) = &settings.eap_login {
        if is_wifi {
            args.push("wifi-sec.key-mgmt".to_string());
            args.push("wpa-eap".to_string());
        } else {
            args.push("802-1x.eap".to_string());
            args.push("peap".to_string());
        }
        args.push("802-1x.identity".to_string());
        args.push(eap_login.clone());
        
        if let Some(eap_pwd) = &settings.eap_password {
            args.push("802-1x.password".to_string());
            args.push(eap_pwd.clone());
        }
        
        // Typical defaults for PEAP
        if is_wifi {
            args.push("802-1x.eap".to_string());
            args.push("peap".to_string());
            args.push("802-1x.phase2-auth".to_string());
            args.push("mschapv2".to_string());
        }
    } else if let Some(pwd) = &settings.password {
        if is_wifi {
            args.push("wifi-sec.key-mgmt".to_string());
            args.push("wpa-psk".to_string());
            args.push("wifi-sec.psk".to_string());
            args.push(pwd.clone());
        }
    }

    if let Some(ipv4) = &settings.ipv4 {
        match ipv4.method {
            IpMethod::Manual => {
                args.push("ipv4.method".to_string());
                args.push("manual".to_string());
                
                if let Some(addrs) = &ipv4.address_data {
                    let addr_strs: Vec<String> = addrs.iter()
                        .map(|a| format!("{}/{}", a.address, a.prefix))
                        .collect();
                    if !addr_strs.is_empty() {
                        args.push("ipv4.addresses".to_string());
                        args.push(addr_strs.join(","));
                    }
                }
                
                if let Some(gw) = &ipv4.gateway {
                    args.push("ipv4.gateway".to_string());
                    args.push(gw.clone());
                }
                
                if let Some(dns) = &ipv4.dns {
                    if !dns.is_empty() {
                        args.push("ipv4.dns".to_string());
                        args.push(dns.join(","));
                    }
                }
            }
            IpMethod::Auto => {
                args.push("ipv4.method".to_string());
                args.push("auto".to_string());
            }
        }
    }

    args
}

async fn run_nmcli_test(settings: &NetworkSettings, transport: &NetworkTransport) -> NetResult<()> {
    match transport {
        NetworkTransport::Wifi => {
            if let Some(ssid) = &settings.ssid {
                let _ = Command::new("nmcli")
                    .args(["connection", "delete", ssid])
                    .output()
                    .await;
            }

            let args = build_nmcli_connection_args(settings, true);
            let add_out = Command::new("nmcli")
                .args(args)
                .output()
                .await
                .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;
            if !add_out.status.success() {
                return Err(NetworkError::ConnectionFailed(
                    String::from_utf8_lossy(&add_out.stderr).to_string(),
                ));
            }

            let ssid = settings.ssid.clone().unwrap_or_default();
            let up_out = Command::new("nmcli")
                .args(["connection", "up", &ssid])
                .output()
                .await
                .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;
            if !up_out.status.success() {
                return Err(NetworkError::ConnectionFailed(
                    String::from_utf8_lossy(&up_out.stderr).to_string(),
                ));
            }
        }
        NetworkTransport::Ethernet => {
            let _ = Command::new("nmcli")
                .args(["connection", "delete", "Wired connection"])
                .output()
                .await;

            let args = build_nmcli_connection_args(settings, false);
            let add_out = Command::new("nmcli")
                .args(args)
                .output()
                .await
                .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;
            if !add_out.status.success() {
                return Err(NetworkError::ConnectionFailed(
                    String::from_utf8_lossy(&add_out.stderr).to_string(),
                ));
            }

            let up_out = Command::new("nmcli")
                .args(["connection", "up", "Wired connection"])
                .output()
                .await
                .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;
            if !up_out.status.success() {
                return Err(NetworkError::ConnectionFailed(
                    String::from_utf8_lossy(&up_out.stderr).to_string(),
                ));
            }
        }
    }

    Ok(())
}

#[async_trait]
impl NetworkController for NmcliNetworkController {
    async fn scan_wifi(&self) -> NetResult<Vec<WifiNetwork>> {
        let _ = Command::new("nmcli")
            .args(&["device", "wifi", "rescan"])
            .output()
            .await;

        let output = Command::new("nmcli")
            .args(&[
                "-t",
                "-f",
                "BSSID,SSID,SIGNAL,SECURITY",
                "dev",
                "wifi",
                "list",
            ])
            .output()
            .await
            .map_err(|e| NetworkError::ScanFailed(e.to_string()))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(NetworkError::ScanFailed(err.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_scan_output(&stdout))
    }

    async fn scan_ethernet(&self) -> NetResult<Vec<EthernetNetwork>> {
        let output = Command::new("nmcli")
            .args(&["-t", "-f", "DEVICE,TYPE,STATE", "device", "status"])
            .output()
            .await
            .map_err(|e| NetworkError::ScanFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(NetworkError::ScanFailed("Failed to list devices".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut eth_devices = Vec::new();
        
        for line in stdout.lines() {
            let fields = parse_nmcli_terse_line(line);
            if fields.len() >= 3 && fields[1] == "ethernet" {
                eth_devices.push(EthernetNetwork {
                    interface: fields[0].clone(),
                    connected: fields[2] == "connected",
                });
            }
        }

        Ok(eth_devices)
    }

    async fn get_saved_networks(&self) -> NetResult<Vec<SavedNetwork>> {
        let output = Command::new("nmcli")
            .args(&["-t", "-f", "UUID,NAME,TYPE", "connection", "show"])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_saved_output(&stdout))
    }

    async fn start_wifi_test(&self, settings: NetworkSettings) -> NetResult<Uuid> {
        let test_id = Uuid::new_v4();
        {
            let mut tests = self.active_tests.lock().await;
            tests.insert(
                test_id,
                TestRecord {
                    state: ConnectionState::Testing,
                    settings: settings.clone(),
                    transport: NetworkTransport::Wifi,
                },
            );
        }

        let active_tests = self.active_tests.clone();

        tokio::spawn(async move {
            let state = match run_nmcli_test(&settings, &NetworkTransport::Wifi).await {
                Ok(()) => ConnectionState::Connected,
                Err(NetworkError::ConnectionFailed(msg)) => ConnectionState::Failed(msg),
                Err(e) => ConnectionState::Failed(e.to_string()),
            };
            let mut tests = active_tests.lock().await;
            if let Some(record) = tests.get_mut(&test_id) {
                record.state = state;
            }
        });

        Ok(test_id)
    }

    async fn start_ethernet_test(&self, settings: NetworkSettings) -> NetResult<Uuid> {
        let test_id = Uuid::new_v4();
        {
            let mut tests = self.active_tests.lock().await;
            tests.insert(
                test_id,
                TestRecord {
                    state: ConnectionState::Testing,
                    settings: settings.clone(),
                    transport: NetworkTransport::Ethernet,
                },
            );
        }

        let active_tests = self.active_tests.clone();

        tokio::spawn(async move {
            let state = match run_nmcli_test(&settings, &NetworkTransport::Ethernet).await {
                Ok(()) => ConnectionState::Connected,
                Err(NetworkError::ConnectionFailed(msg)) => ConnectionState::Failed(msg),
                Err(e) => ConnectionState::Failed(e.to_string()),
            };
            let mut tests = active_tests.lock().await;
            if let Some(record) = tests.get_mut(&test_id) {
                record.state = state;
            }
        });

        Ok(test_id)
    }

    async fn connection_status(&self, test_id: Uuid) -> NetResult<ConnectionState> {
        let tests = self.active_tests.lock().await;
        tests
            .get(&test_id)
            .map(|record| record.state.clone())
            .ok_or_else(|| NetworkError::NotFound("Test ID not found".into()))
    }

    async fn save_test(&self, test_id: Uuid) -> NetResult<()> {
        let record = {
            let tests = self.active_tests.lock().await;
            tests
                .get(&test_id)
                .cloned()
                .ok_or_else(|| NetworkError::NotFound("Test ID not found".into()))?
        };

        if record.state != ConnectionState::Connected {
            return Err(NetworkError::ConnectionFailed(
                "Cannot save an unsuccessful test".into(),
            ));
        }

        match record.transport {
            NetworkTransport::Wifi => {
                let Some(ssid) = &record.settings.ssid else {
                    return Err(NetworkError::ConnectionFailed("Missing SSID for Wi-Fi test".into()));
                };
                let output = Command::new("nmcli")
                    .args(["connection", "up", ssid])
                    .output()
                    .await
                    .map_err(|e| NetworkError::Internal(e.to_string()))?;
                if !output.status.success() {
                    return Err(NetworkError::ConnectionFailed(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }
            }
            NetworkTransport::Ethernet => {
                let output = Command::new("nmcli")
                    .args(["connection", "up", "Wired connection"])
                    .output()
                    .await
                    .map_err(|e| NetworkError::Internal(e.to_string()))?;
                if !output.status.success() {
                    return Err(NetworkError::ConnectionFailed(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    async fn discard_test(&self, test_id: Uuid) -> NetResult<()> {
        let mut tests = self.active_tests.lock().await;
        tests
            .remove(&test_id)
            .map(|_| ())
            .ok_or_else(|| NetworkError::NotFound("Test ID not found".into()))
    }

    async fn forget_network(&self, id: &str) -> NetResult<()> {
        let output = Command::new("nmcli")
            .args(&["connection", "delete", id])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        info!("Forgot network with ID {}", id);
        Ok(())
    }

    async fn get_connectivity(&self) -> NetResult<Connectivity> {
        let output = Command::new("nmcli")
            .args(&["networking", "connectivity", "check"])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        let is_connected = stdout == "full";
        // To accurately emulate can_reach_host from Python we could do a ping, 
        // but for now relying on NM's connectivity check is usually sufficient.
        Ok(Connectivity {
            is_connected,
            can_reach_host: is_connected, 
        })
    }

    async fn get_hotspot_status(&self) -> NetResult<AccessPointStatus> {
        let output_conn = Command::new("nmcli")
            .args(&["-t", "-f", "NAME,TYPE", "connection", "show", "--active"])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        let stdout_conn = String::from_utf8_lossy(&output_conn.stdout);
        let device_name = std::env::var("DEVICE_NAME").unwrap_or_else(|_| "Reshape Device".to_string());
        
        let mut is_enabled = false;
        for line in stdout_conn.lines() {
            let fields = parse_nmcli_terse_line(line);
            if fields.len() >= 2 && fields[0] == device_name {
                is_enabled = true;
                break;
            }
        }
        
        Ok(AccessPointStatus { is_enabled })
    }

    async fn start_hotspot(&self, ssid: &str, password: &str) -> NetResult<()> {
        let _ = Command::new("nmcli").args(&["connection", "delete", ssid]).output().await;

        let mut args = vec![
            "connection".to_string(), "add".to_string(),
            "type".to_string(), "wifi".to_string(),
            "ifname".to_string(), "wlan0".to_string(),
            "con-name".to_string(), ssid.to_string(),
            "autoconnect".to_string(), "no".to_string(),
            "ssid".to_string(), ssid.to_string(),
            "802-11-wireless.mode".to_string(), "ap".to_string(),
            "802-11-wireless.band".to_string(), "bg".to_string(),
            "ipv4.method".to_string(), "shared".to_string(),
        ];

        if !password.is_empty() {
            args.push("wifi-sec.key-mgmt".to_string());
            args.push("wpa-psk".to_string());
            args.push("wifi-sec.psk".to_string());
            args.push(password.to_string());
        }

        let output = Command::new("nmcli").args(&args).output().await;
        if output.map(|o| o.status.success()).unwrap_or(false) {
            let _ = Command::new("nmcli").args(&["connection", "up", ssid]).output().await;
        }
        
        info!("Started Hotspot {}", ssid);
        Ok(())
    }

    async fn stop_hotspot(&self, ssid: &str) -> NetResult<()> {
        let _ = Command::new("nmcli").args(&["connection", "down", ssid]).output().await;
        info!("Stopped Hotspot {}", ssid);
        Ok(())
    }
}
