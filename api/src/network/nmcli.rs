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

#[async_trait]
impl NetworkController for NmcliNetworkController {
    async fn scan_wifi(&self) -> NetResult<Vec<WifiNetwork>> {
        // Request a rescan first, don't strictly care if it fails (sometimes rate limited)
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
        let mut networks = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            // Since MAC addresses contain colons, they get split too.
            // Better to split from the right or just handle it knowing BSSID has 5 colons.
            // Format: BSSID (6 parts), SSID, SIGNAL, SECURITY
            if parts.len() >= 9 {
                // Parts: 0..=5 is BSSID
                // 6 is SSID
                // 7 is SIGNAL
                // 8 is SECURITY

                let ssid = parts[6].replace("\\:", ":");
                if ssid.is_empty() {
                    continue;
                }

                let signal = parts[7].parse::<u8>().unwrap_or(0);
                let security = parts[8..].join(":"); // in case security has colons

                networks.push(WifiNetwork {
                    ssid,
                    signal_strength: signal,
                    security,
                });
            }
        }

        // Deduplicate and sort by strength
        networks.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
        networks.dedup_by(|a, b| a.ssid == b.ssid);

        Ok(networks)
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
        let mut saved = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 && parts[2] == "802-11-wireless" {
                saved.push(SavedNetwork {
                    id: parts[0].to_string(),
                    ssid: parts[1].replace("\\:", ":"),
                });
            }
        }

        Ok(saved)
    }

    async fn connect(&self, ssid: &str, password: &str) -> NetResult<Uuid> {
        let test_id = Uuid::new_v4();

        let mut tests = self.active_tests.lock().await;
        tests.insert(test_id, ConnectionState::Testing);
        drop(tests);

        let active_tests = self.active_tests.clone();
        let ssid = ssid.to_string();
        let password = password.to_string();

        // Spawn nmcli in background so we can return the test_id immediately
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
