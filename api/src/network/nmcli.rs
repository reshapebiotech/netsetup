use crate::network::types::{ConnectionState, SavedNetwork, WifiNetwork};
use crate::network::{NetworkController, NetworkError, Result as NetResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{error, info};
use uuid::Uuid;

pub struct NmcliNetworkController {
    active_tests: Arc<Mutex<HashMap<Uuid, ConnectionState>>>,
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
        // We requested: BSSID,SSID,SIGNAL,SECURITY
        // Format: BSSID (often not escaped if MAC, but parse_nmcli_terse_line handles it?), 
        // Wait, nmcli escapes MAC colons! e.g., 00\:11\:22\:33\:44\:55
        // So a BSSID might actually be parsed as a single field if properly escaped by nmcli.
        // Let's check how nmcli formats BSSID in terse mode. Yes, it escapes them: `00\:11\:22\:33\:44\:55:SSID:SIGNAL:SECURITY`
        if fields.len() >= 4 {
            let ssid = fields[1].clone();
            if ssid.is_empty() {
                continue;
            }

            let signal = fields[2].parse::<u8>().unwrap_or(0);
            let security = fields[3..].join(":"); // Combine remainder just in case

            networks.push(WifiNetwork {
                ssid,
                signal_strength: signal,
                security,
            });
        }
    }

    networks.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
    networks.dedup_by(|a, b| a.ssid == b.ssid);

    networks
}

pub fn parse_saved_output(stdout: &str) -> Vec<SavedNetwork> {
    let mut saved = Vec::new();

    for line in stdout.lines() {
        let fields = parse_nmcli_terse_line(line);
        // We requested: UUID,NAME,TYPE
        if fields.len() >= 3 && fields[2] == "802-11-wireless" {
            saved.push(SavedNetwork {
                id: fields[0].clone(),
                ssid: fields[1].clone(),
            });
        }
    }

    saved
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

    async fn connect(&self, ssid: &str, password: &str) -> NetResult<Uuid> {
        let test_id = Uuid::new_v4();

        let mut tests = self.active_tests.lock().await;
        tests.insert(test_id, ConnectionState::Testing);
        drop(tests);

        let active_tests = self.active_tests.clone();
        let ssid = ssid.to_string();
        let password = password.to_string();

        tokio::spawn(async move {
            let mut cmd = Command::new("nmcli");
            cmd.args(&["dev", "wifi", "connect", &ssid]);
            if !password.is_empty() {
                cmd.args(&["password", &password]);
            }

            info!("Attempting to connect to {}", ssid);
            let output = cmd.output().await;

            let mut tests = active_tests.lock().await;
            match output {
                Ok(out) if out.status.success() => {
                    info!("Successfully connected to {}", ssid);
                    tests.insert(test_id, ConnectionState::Connected);
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    let out_str = String::from_utf8_lossy(&out.stdout).to_string();
                    error!("Failed to connect to {}: {} - {}", ssid, err, out_str);
                    tests.insert(
                        test_id,
                        ConnectionState::Failed(format!("{} {}", err, out_str).trim().to_string()),
                    );
                }
                Err(e) => {
                    error!("Failed to execute nmcli: {}", e);
                    tests.insert(test_id, ConnectionState::Failed(e.to_string()));
                }
            }
        });

        Ok(test_id)
    }

    async fn connection_status(&self, test_id: Uuid) -> NetResult<ConnectionState> {
        let tests = self.active_tests.lock().await;
        if let Some(state) = tests.get(&test_id) {
            Ok(state.clone())
        } else {
            Err(NetworkError::NotFound("Test ID not found".into()))
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_parse_nmcli_terse_line() {
        assert_eq!(
            parse_nmcli_terse_line(r"00\:11\:22\:33\:44\:55:My\:Network:100:WPA2"),
            vec!["00:11:22:33:44:55", "My:Network", "100", "WPA2"]
        );
        assert_eq!(
            parse_nmcli_terse_line(r"uuid-1234:Name with \: colon:802-11-wireless"),
            vec!["uuid-1234", "Name with : colon", "802-11-wireless"]
        );
    }

    proptest! {
        #[test]
        fn doesnt_panic_on_scan_output(s in "\\PC*") {
            let _ = parse_scan_output(&s);
        }

        #[test]
        fn doesnt_panic_on_saved_output(s in "\\PC*") {
            let _ = parse_saved_output(&s);
        }

        #[test]
        fn parses_valid_scan_output(
            bssid in "[0-9A-F]{2}(\\\\:[0-9A-F]{2}){5}",
            ssid in "[a-zA-Z0-9]+",
            signal in 0u8..=100,
            security in "(WPA2|WPA3|NONE)"
        ) {
            let input = format!("{}:{}:{}:{}", bssid, ssid, signal, security);
            let parsed = parse_scan_output(&input);
            
            // Note: because we deduplicate by SSID in parse_scan_output, and we only generated 1 network, 
            // the length should be 1.
            prop_assert_eq!(parsed.len(), 1);
            prop_assert_eq!(&parsed[0].ssid, &ssid);
            prop_assert_eq!(parsed[0].signal_strength, signal);
            prop_assert_eq!(&parsed[0].security, &security);
        }
    }
}
