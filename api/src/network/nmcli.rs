use crate::network::types::{
    AccessPointControllerStatus, AccessPointPreferences, AccessPointStatus, ConnectionState,
    Connectivity, EthernetNetwork, IpMethod, NetworkSettings, NetworkTransport, SavedNetwork,
    WifiNetwork,
};
use crate::network::{NetworkController, NetworkError, Result as NetResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::fs;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct TestRecord {
    state: ConnectionState,
    settings: NetworkSettings,
    transport: NetworkTransport,
}

#[derive(Clone)]
struct ApSupervisorState {
    persistent_enabled: bool,
    automation_paused: bool,
    save_in_progress: bool,
    last_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredManagedConnections {
    ids: Vec<String>,
}

pub struct NmcliNetworkController {
    active_tests: Arc<Mutex<HashMap<Uuid, TestRecord>>>,
    ap_state: Arc<Mutex<ApSupervisorState>>,
    supervisor_started: Arc<AtomicBool>,
    hotspot_name: String,
    hotspot_password: String,
    managed_connections_path: PathBuf,
    managed_connection_ids: Arc<Mutex<HashSet<String>>>,
    probe_host: String,
}

impl NmcliNetworkController {
    pub fn new() -> Self {
        let managed_connections_path = std::env::var("MANAGED_CONNECTIONS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/var/lib/machine-setup/managed-connections.json"));

        let managed_ids =
            Self::read_managed_connections_sync(&managed_connections_path).unwrap_or_default();

        Self {
            active_tests: Arc::new(Mutex::new(HashMap::new())),
            ap_state: Arc::new(Mutex::new(ApSupervisorState {
                persistent_enabled: false,
                automation_paused: false,
                save_in_progress: false,
                last_reason: "AP supervisor initializing".into(),
            })),
            supervisor_started: Arc::new(AtomicBool::new(false)),
            hotspot_name: Self::resolve_hotspot_name(),
            hotspot_password: std::env::var("DEVICE_AP_PASSWORD")
                .unwrap_or_else(|_| "reshape123".to_string()),
            managed_connections_path,
            managed_connection_ids: Arc::new(Mutex::new(managed_ids.into_iter().collect())),
            probe_host: std::env::var("CONNECTIVITY_PROBE_HOST")
                .unwrap_or_else(|_| "1.1.1.1".to_string()),
        }
    }

    fn resolve_hotspot_name() -> String {
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            let trimmed = hostname.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        if let Ok(output) = std::process::Command::new("hostname").output() {
            if output.status.success() {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !value.is_empty() {
                    return value;
                }
            }
        }

        "reshape-device".to_string()
    }

    fn read_managed_connections_sync(path: &Path) -> Option<Vec<String>> {
        let raw = std::fs::read_to_string(path).ok()?;
        let payload = serde_json::from_str::<StoredManagedConnections>(&raw).ok()?;
        Some(payload.ids)
    }

    async fn persist_managed_connections(&self) -> NetResult<()> {
        if let Some(parent) = self.managed_connections_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                NetworkError::Internal(format!("Failed to create managed state dir: {e}"))
            })?;
        }

        let ids = {
            let guard = self.managed_connection_ids.lock().await;
            let mut values: Vec<String> = guard.iter().cloned().collect();
            values.sort();
            values
        };

        let raw = serde_json::to_vec_pretty(&StoredManagedConnections { ids }).map_err(|e| {
            NetworkError::Internal(format!("Failed to encode managed connections: {e}"))
        })?;

        fs::write(&self.managed_connections_path, raw)
            .await
            .map_err(|e| {
                NetworkError::Internal(format!("Failed to persist managed connections: {e}"))
            })?;

        Ok(())
    }

    async fn run_supervisor_tick(&self) {
        let wifi_test_in_progress = {
            let tests = self.active_tests.lock().await;
            tests.values().any(|record| {
                record.transport == NetworkTransport::Wifi
                    && record.state == ConnectionState::Testing
            })
        };

        let (persistent_enabled, save_in_progress) = {
            let state = self.ap_state.lock().await;
            (state.persistent_enabled, state.save_in_progress)
        };

        if wifi_test_in_progress || save_in_progress {
            let mut state = self.ap_state.lock().await;
            state.automation_paused = true;
            state.last_reason = "AP automation paused during Wi-Fi transition".into();
            return;
        }

        let connectivity = match self.current_connectivity().await {
            Ok(value) => value,
            Err(e) => {
                warn!("Failed to fetch connectivity for AP supervisor: {e}");
                Connectivity {
                    is_connected: false,
                    can_reach_host: false,
                }
            }
        };

        let ethernet_connected = self.ethernet_connected().await.unwrap_or(false);
        let should_enable = if persistent_enabled && ethernet_connected {
            true
        } else {
            !connectivity.is_connected || !connectivity.can_reach_host
        };

        let reason = if persistent_enabled && ethernet_connected {
            "Persistent AP enabled in Ethernet mode"
        } else if should_enable {
            "No healthy uplink detected"
        } else {
            "Healthy uplink detected"
        };

        let current_enabled = self.is_hotspot_enabled().await.unwrap_or(false);
        if should_enable && !current_enabled {
            if let Err(e) = self
                .start_hotspot(&self.hotspot_name, &self.hotspot_password)
                .await
            {
                error!("Failed to enable setup AP: {e}");
            }
        }
        if !should_enable && current_enabled {
            if let Err(e) = self.stop_hotspot(&self.hotspot_name).await {
                error!("Failed to disable setup AP: {e}");
            }
        }

        let mut state = self.ap_state.lock().await;
        state.automation_paused = false;
        state.last_reason = reason.into();
    }

    async fn cleanup_hostname_ap_duplicates(&self) -> NetResult<()> {
        let output = Command::new("nmcli")
            .args(["-t", "-f", "UUID,NAME,TYPE", "connection", "show"])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut matching_wifi_ids = Vec::new();
        for line in stdout.lines() {
            let fields = parse_nmcli_terse_line(line);
            if fields.len() >= 3 && fields[1] == self.hotspot_name && fields[2] == "802-11-wireless"
            {
                matching_wifi_ids.push(fields[0].clone());
            }
        }

        if matching_wifi_ids.len() <= 1 {
            return Ok(());
        }

        let keep_id = matching_wifi_ids[0].clone();
        for id in matching_wifi_ids.iter().skip(1) {
            let out = Command::new("nmcli")
                .args(["connection", "delete", id])
                .output()
                .await
                .map_err(|e| NetworkError::Internal(e.to_string()))?;
            if !out.status.success() {
                warn!(
                    "Failed to delete duplicate hostname AP profile {}: {}",
                    id,
                    String::from_utf8_lossy(&out.stderr)
                );
            } else {
                info!("Deleted duplicate hostname AP profile {}", id);
            }
        }

        info!(
            "Kept single hostname AP profile {} ({})",
            self.hotspot_name, keep_id
        );
        Ok(())
    }

    async fn ensure_supervisor_started(&self) {
        if self
            .supervisor_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        if let Err(e) = self.cleanup_hostname_ap_duplicates().await {
            warn!("Failed AP duplicate cleanup at startup: {e}");
        }

        let cloned = Self {
            active_tests: self.active_tests.clone(),
            ap_state: self.ap_state.clone(),
            supervisor_started: self.supervisor_started.clone(),
            hotspot_name: self.hotspot_name.clone(),
            hotspot_password: self.hotspot_password.clone(),
            managed_connections_path: self.managed_connections_path.clone(),
            managed_connection_ids: self.managed_connection_ids.clone(),
            probe_host: self.probe_host.clone(),
        };

        tokio::spawn(async move {
            cloned.run_supervisor_tick().await;
            loop {
                sleep(Duration::from_secs(8)).await;
                cloned.run_supervisor_tick().await;
            }
        });
    }

    async fn is_hotspot_enabled(&self) -> NetResult<bool> {
        let output = Command::new("nmcli")
            .args(["-t", "-f", "NAME,TYPE", "connection", "show", "--active"])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().any(|line| {
            let fields = parse_nmcli_terse_line(line);
            fields.len() >= 2 && fields[0] == self.hotspot_name
        }))
    }

    async fn ethernet_connected(&self) -> NetResult<bool> {
        let output = Command::new("nmcli")
            .args(["-t", "-f", "DEVICE,TYPE,STATE", "device", "status"])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().any(|line| {
            let fields = parse_nmcli_terse_line(line);
            fields.len() >= 3 && fields[1] == "ethernet" && fields[2] == "connected"
        }))
    }

    async fn current_connectivity(&self) -> NetResult<Connectivity> {
        let output = Command::new("nmcli")
            .args(["networking", "connectivity", "check"])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        let connectivity_state = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let is_connected = connectivity_state == "full";

        let ping_ok = Command::new("ping")
            .args(["-c", "1", "-W", "1", &self.probe_host])
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);

        Ok(Connectivity {
            is_connected,
            can_reach_host: ping_ok,
        })
    }

    async fn get_connection_uuid_by_name(&self, name: &str) -> NetResult<String> {
        let output = Command::new("nmcli")
            .args(["-t", "-f", "UUID", "connection", "show", name])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let uuid = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();

        if uuid.is_empty() {
            return Err(NetworkError::NotFound(format!(
                "No connection UUID found for {name}"
            )));
        }

        Ok(uuid)
    }

    async fn get_connection_by_uuid(&self, uuid: &str) -> NetResult<Option<SavedNetwork>> {
        let output = Command::new("nmcli")
            .args(["-t", "-f", "UUID,NAME,TYPE", "connection", "show", uuid])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
            if stderr.contains("not found") || stderr.contains("unknown") {
                return Ok(None);
            }
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let parsed = parse_saved_output(&String::from_utf8_lossy(&output.stdout));
        Ok(parsed.into_iter().next())
    }
}

fn parse_nmcli_terse_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next_c) = chars.peek() {
                if *next_c == ':' || *next_c == '\\' {
                    current.push(*next_c);
                    chars.next();
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
                    let addr_strs: Vec<String> = addrs
                        .iter()
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
        self.ensure_supervisor_started().await;

        let _ = Command::new("nmcli")
            .args(["device", "wifi", "rescan"])
            .output()
            .await;

        let output = Command::new("nmcli")
            .args([
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
            return Err(NetworkError::ScanFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_scan_output(&stdout))
    }

    async fn scan_ethernet(&self) -> NetResult<Vec<EthernetNetwork>> {
        self.ensure_supervisor_started().await;

        let output = Command::new("nmcli")
            .args(["-t", "-f", "DEVICE,TYPE,STATE", "device", "status"])
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
        self.ensure_supervisor_started().await;

        let managed_ids = {
            let guard = self.managed_connection_ids.lock().await;
            guard.iter().cloned().collect::<Vec<_>>()
        };

        let mut stale = Vec::new();
        let mut saved = Vec::new();

        for id in managed_ids {
            match self.get_connection_by_uuid(&id).await {
                Ok(Some(entry)) => saved.push(entry),
                Ok(None) => stale.push(id),
                Err(e) => return Err(e),
            }
        }

        if !stale.is_empty() {
            {
                let mut guard = self.managed_connection_ids.lock().await;
                for id in stale {
                    guard.remove(&id);
                }
            }
            self.persist_managed_connections().await?;
        }

        Ok(saved)
    }

    async fn start_wifi_test(&self, settings: NetworkSettings) -> NetResult<Uuid> {
        self.ensure_supervisor_started().await;

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
        self.ensure_supervisor_started().await;

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
        self.ensure_supervisor_started().await;
        let tests = self.active_tests.lock().await;
        tests
            .get(&test_id)
            .map(|record| record.state.clone())
            .ok_or_else(|| NetworkError::NotFound("Test ID not found".into()))
    }

    async fn save_test(&self, test_id: Uuid) -> NetResult<()> {
        self.ensure_supervisor_started().await;

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

        if record.transport == NetworkTransport::Wifi {
            let mut state = self.ap_state.lock().await;
            state.save_in_progress = true;
        }

        let save_result = match record.transport {
            NetworkTransport::Wifi => {
                let Some(ssid) = &record.settings.ssid else {
                    return Err(NetworkError::ConnectionFailed(
                        "Missing SSID for Wi-Fi test".into(),
                    ));
                };
                let output = Command::new("nmcli")
                    .args(["connection", "up", ssid])
                    .output()
                    .await
                    .map_err(|e| NetworkError::Internal(e.to_string()))?;
                if !output.status.success() {
                    Err(NetworkError::ConnectionFailed(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ))
                } else {
                    Ok(ssid.clone())
                }
            }
            NetworkTransport::Ethernet => {
                let output = Command::new("nmcli")
                    .args(["connection", "up", "Wired connection"])
                    .output()
                    .await
                    .map_err(|e| NetworkError::Internal(e.to_string()))?;
                if !output.status.success() {
                    Err(NetworkError::ConnectionFailed(
                        String::from_utf8_lossy(&output.stderr).to_string(),
                    ))
                } else {
                    Ok("Wired connection".to_string())
                }
            }
        };

        if record.transport == NetworkTransport::Wifi {
            let mut state = self.ap_state.lock().await;
            state.save_in_progress = false;
        }

        let connection_name = save_result?;
        let uuid = self.get_connection_uuid_by_name(&connection_name).await?;
        {
            let mut guard = self.managed_connection_ids.lock().await;
            guard.insert(uuid);
        }
        self.persist_managed_connections().await?;
        Ok(())
    }

    async fn discard_test(&self, test_id: Uuid) -> NetResult<()> {
        self.ensure_supervisor_started().await;
        let mut tests = self.active_tests.lock().await;
        tests
            .remove(&test_id)
            .map(|_| ())
            .ok_or_else(|| NetworkError::NotFound("Test ID not found".into()))
    }

    async fn forget_network(&self, id: &str) -> NetResult<()> {
        self.ensure_supervisor_started().await;

        let output = Command::new("nmcli")
            .args(["connection", "delete", id])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        {
            let mut guard = self.managed_connection_ids.lock().await;
            guard.remove(id);
        }
        self.persist_managed_connections().await?;

        info!("Forgot network with ID {id}");
        Ok(())
    }

    async fn get_connectivity(&self) -> NetResult<Connectivity> {
        self.ensure_supervisor_started().await;
        self.current_connectivity().await
    }

    async fn get_hotspot_status(&self) -> NetResult<AccessPointStatus> {
        self.ensure_supervisor_started().await;
        Ok(AccessPointStatus {
            is_enabled: self.is_hotspot_enabled().await.unwrap_or(false),
        })
    }

    async fn start_hotspot(&self, ssid: &str, password: &str) -> NetResult<()> {
        let _ = Command::new("nmcli")
            .args(["connection", "delete", ssid])
            .output()
            .await;

        let mut args = vec![
            "connection".to_string(),
            "add".to_string(),
            "type".to_string(),
            "wifi".to_string(),
            "ifname".to_string(),
            "wlan0".to_string(),
            "con-name".to_string(),
            ssid.to_string(),
            "autoconnect".to_string(),
            "no".to_string(),
            "ssid".to_string(),
            ssid.to_string(),
            "802-11-wireless.mode".to_string(),
            "ap".to_string(),
            "802-11-wireless.band".to_string(),
            "bg".to_string(),
            "ipv4.method".to_string(),
            "shared".to_string(),
        ];

        if !password.is_empty() {
            args.push("wifi-sec.key-mgmt".to_string());
            args.push("wpa-psk".to_string());
            args.push("wifi-sec.psk".to_string());
            args.push(password.to_string());
        }

        let add_output = Command::new("nmcli")
            .args(args)
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;
        if !add_output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&add_output.stderr).to_string(),
            ));
        }

        let up_output = Command::new("nmcli")
            .args(["connection", "up", ssid])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;
        if !up_output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&up_output.stderr).to_string(),
            ));
        }

        info!("Started hotspot {ssid}");
        Ok(())
    }

    async fn stop_hotspot(&self, ssid: &str) -> NetResult<()> {
        let output = Command::new("nmcli")
            .args(["connection", "down", ssid])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        info!("Stopped hotspot {ssid}");
        Ok(())
    }

    async fn get_ap_controller_status(&self) -> NetResult<AccessPointControllerStatus> {
        self.ensure_supervisor_started().await;
        let state = self.ap_state.lock().await.clone();
        let is_enabled = self.is_hotspot_enabled().await.unwrap_or(false);
        Ok(AccessPointControllerStatus {
            is_enabled,
            persistent_enabled: state.persistent_enabled,
            automation_paused: state.automation_paused,
            last_reason: state.last_reason,
        })
    }

    async fn get_ap_preferences(&self) -> NetResult<AccessPointPreferences> {
        self.ensure_supervisor_started().await;
        let state = self.ap_state.lock().await;
        Ok(AccessPointPreferences {
            persistent_enabled: state.persistent_enabled,
        })
    }

    async fn set_ap_preferences(&self, prefs: AccessPointPreferences) -> NetResult<()> {
        self.ensure_supervisor_started().await;

        {
            let mut state = self.ap_state.lock().await;
            state.persistent_enabled = prefs.persistent_enabled;
            state.last_reason = if prefs.persistent_enabled {
                "Persistent AP enabled by user".into()
            } else {
                "Persistent AP disabled by user".into()
            };
        }

        self.run_supervisor_tick().await;
        Ok(())
    }

    async fn start_ap_supervisor(&self) -> NetResult<()> {
        self.ensure_supervisor_started().await;
        Ok(())
    }
}
