use super::transaction::Commands;
use super::types::{Address, CurrentNetworkConfiguration, NetworkTransport};
use super::{NetworkError, Result};
use std::net::Ipv4Addr;

pub(super) async fn collect(
    commands: &impl Commands,
    profile: &str,
    interface: &str,
    transport: NetworkTransport,
) -> Result<CurrentNetworkConfiguration> {
    let method = commands
        .run(
            "nmcli",
            &["-g", "ipv4.method", "connection", "show", "uuid", profile].map(str::to_owned),
        )
        .await?;
    let ssid = if transport == NetworkTransport::Wifi {
        let value = commands
            .run(
                "nmcli",
                &[
                    "--escape",
                    "no",
                    "-g",
                    "802-11-wireless.ssid",
                    "connection",
                    "show",
                    "uuid",
                    profile,
                ]
                .map(str::to_owned),
            )
            .await?;
        Some(value.trim_end_matches('\n').to_owned()).filter(|s| !s.is_empty())
    } else {
        None
    };
    let raw = commands
        .run(
            "nmcli",
            &[
                "-t",
                "--escape",
                "no",
                "-f",
                "GENERAL.HWADDR,IP4.ADDRESS,IP4.GATEWAY,IP4.DNS",
                "device",
                "show",
                interface,
            ]
            .map(str::to_owned),
        )
        .await?;
    let mut configuration = CurrentNetworkConfiguration {
        transport,
        ssid,
        ipv4_method: method.trim().into(),
        addresses: Vec::new(),
        gateway: None,
        dns: Vec::new(),
        mac_address: None,
    };
    for line in raw.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.split('[').next().unwrap_or(key);
        let value = value.trim();
        if value.is_empty() || value == "--" {
            continue;
        }
        match key {
            "GENERAL.HWADDR" => configuration.mac_address = Some(value.into()),
            "IP4.ADDRESS" => {
                let invalid = || {
                    NetworkError::Internal(
                        "Could not read current IPv4 addresses from NetworkManager".into(),
                    )
                };
                let (address, prefix) = value.split_once('/').ok_or_else(invalid)?;
                let address: Ipv4Addr = address.parse().map_err(|_| invalid())?;
                let prefix: u8 = prefix.parse().map_err(|_| invalid())?;
                if prefix > 32 {
                    return Err(invalid());
                }
                configuration.addresses.push(Address {
                    address: address.to_string(),
                    prefix,
                });
            }
            "IP4.GATEWAY" => configuration.gateway = Some(value.into()),
            "IP4.DNS" => {
                if !configuration.dns.iter().any(|server| server == value) {
                    configuration.dns.push(value.into());
                }
            }
            _ => {}
        }
    }
    Ok(configuration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct DeviceCommands {
        raw: &'static str,
    }

    #[async_trait]
    impl Commands for DeviceCommands {
        async fn run(&self, program: &str, args: &[String]) -> Result<String> {
            assert_eq!(program, "nmcli");
            match args
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice()
            {
                ["-g", "ipv4.method", "connection", "show", "uuid", "profile"] => {
                    Ok("manual\n".into())
                }
                [
                    "--escape",
                    "no",
                    "-g",
                    "802-11-wireless.ssid",
                    "connection",
                    "show",
                    "uuid",
                    "profile",
                ] => Ok(" Lab:West \n".into()),
                [
                    "-t",
                    "--escape",
                    "no",
                    "-f",
                    "GENERAL.HWADDR,IP4.ADDRESS,IP4.GATEWAY,IP4.DNS",
                    "device",
                    "show",
                    "wlan0",
                ] => Ok(self.raw.into()),
                _ => panic!("Unexpected operation: {args:?}"),
            }
        }
    }

    #[tokio::test]
    async fn diagnostics_preserve_actual_addresses_dns_and_ssid_without_credentials() {
        let commands = DeviceCommands {
            raw: "GENERAL.HWADDR:02:AA:BB:CC:DD:EE\nIP4.ADDRESS[1]:192.168.11.0/23\nIP4.ADDRESS[2]:10.0.0.5/32\nIP4.GATEWAY:192.168.10.1\nIP4.DNS[1]:192.168.10.1\nIP4.DNS[2]:1.1.1.1\n802-11-wireless-security.psk:secret-sentinel",
        };
        let config = collect(&commands, "profile", "wlan0", NetworkTransport::Wifi)
            .await
            .unwrap();
        assert_eq!(config.ssid.as_deref(), Some(" Lab:West "));
        assert_eq!(config.ipv4_method, "manual");
        assert_eq!(
            config.addresses,
            vec![
                Address {
                    address: "192.168.11.0".into(),
                    prefix: 23
                },
                Address {
                    address: "10.0.0.5".into(),
                    prefix: 32
                }
            ]
        );
        assert_eq!(config.gateway.as_deref(), Some("192.168.10.1"));
        assert_eq!(config.dns, vec!["192.168.10.1", "1.1.1.1"]);
        assert_eq!(config.mac_address.as_deref(), Some("02:AA:BB:CC:DD:EE"));
        assert!(
            !serde_json::to_string(&config)
                .unwrap()
                .contains("secret-sentinel")
        );
    }

    #[tokio::test]
    async fn missing_runtime_ip_configuration_is_reported_as_absent() {
        let commands = DeviceCommands {
            raw: "GENERAL.HWADDR:--\nIP4.GATEWAY:--\nIP4.DNS[1]:",
        };
        let config = collect(&commands, "profile", "wlan0", NetworkTransport::Ethernet)
            .await
            .unwrap();
        assert!(config.ssid.is_none());
        assert!(config.addresses.is_empty());
        assert!(config.dns.is_empty());
        assert!(config.gateway.is_none());
        assert!(config.mac_address.is_none());
    }
}
