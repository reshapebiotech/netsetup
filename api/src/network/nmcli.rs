use crate::network::ap_supervisor::{
    ApSupervisorMachine, DEFAULT_BOOT_GRACE, SupervisorAction, SupervisorInput,
};
use crate::network::types::{
    AccessPointControllerStatus, AccessPointPreferences, AccessPointStatus, ConnectionState,
    Connectivity, EthernetNetwork, IpMethod, NetworkMacAddresses, NetworkSettings,
    NetworkTransport, SavedNetwork, WifiNetwork,
};
use crate::network::{NetworkController, NetworkError, Result as NetResult};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
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
    save_in_progress: bool,
    machine: ApSupervisorMachine,
}

const BOOT_DECISION_PATH: &str = "/run/reshape-netsetup/boot-decision";

const MANAGED_WIFI_PROFILE: &str = "reshape-managed-wifi";
const MANAGED_ETH_PROFILE: &str = "reshape-managed-ethernet";

async fn persist_boot_decision() {
    if let Err(e) = async {
        fs::create_dir_all("/run/reshape-netsetup").await?;
        fs::write(BOOT_DECISION_PATH, b"resolved\n").await
    }
    .await
    {
        warn!("Could not preserve AP boot decision across service restarts: {e}");
    }
}

pub struct NmcliNetworkController {
    active_tests: Arc<Mutex<HashMap<Uuid, TestRecord>>>,
    ap_state: Arc<Mutex<ApSupervisorState>>,
    supervisor_started: Arc<AtomicBool>,
    hotspot_name: String,
    hotspot_password: String,
    probe_host: String,
    mutation: Arc<Mutex<()>>,
    boot_grace: Duration,
}

impl NmcliNetworkController {
    async fn start_test(
        &self,
        settings: NetworkSettings,
        transport: NetworkTransport,
    ) -> NetResult<Uuid> {
        self.ensure_supervisor_started().await;
        let guard = self.mutation.clone().try_lock_owned().map_err(|_| {
            NetworkError::ConnectionFailed("Another network operation is in progress".into())
        })?;
        if transport == NetworkTransport::Wifi && settings.ssid.as_deref().is_none_or(str::is_empty)
        {
            return Err(NetworkError::ConnectionFailed(
                "A Wi-Fi SSID is required".into(),
            ));
        }
        let interface =
            super::transaction::select_interface(&super::transaction::SystemCommands, &transport)
                .await?;
        let test_id = Uuid::new_v4();
        self.active_tests.lock().await.insert(
            test_id,
            TestRecord {
                state: ConnectionState::Testing,
                settings: settings.clone(),
                transport: transport.clone(),
            },
        );
        let tests = self.active_tests.clone();
        tokio::spawn(async move {
            let _guard = guard;
            // Let the browser receive the test ID before its AP connection disappears.
            sleep(Duration::from_secs(1)).await;
            let result = super::transaction::run(
                &super::transaction::SystemCommands,
                &settings,
                &transport,
                &interface,
                None,
                None,
            )
            .await;
            if let Some(record) = tests.lock().await.get_mut(&test_id) {
                record.state = match result {
                    Ok(report) => ConnectionState::Connected(report),
                    Err(e) => ConnectionState::Failed(e.to_string()),
                };
            }
        });
        Ok(test_id)
    }

    pub fn new() -> Self {
        Self {
            active_tests: Arc::new(Mutex::new(HashMap::new())),
            ap_state: Arc::new(Mutex::new(ApSupervisorState {
                save_in_progress: false,
                machine: ApSupervisorMachine::new(),
            })),
            supervisor_started: Arc::new(AtomicBool::new(false)),
            hotspot_name: Self::resolve_hotspot_name(),
            hotspot_password: std::env::var("DEVICE_AP_PASSWORD")
                .unwrap_or_else(|_| "reshape123".to_string()),
            probe_host: Self::resolve_probe_host(),
            mutation: Arc::new(Mutex::new(())),
            boot_grace: std::env::var("AP_BOOT_GRACE_SECONDS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or(DEFAULT_BOOT_GRACE),
        }
    }

    fn resolve_hotspot_name() -> String {
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            let trimmed = hostname.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        if let Ok(output) = std::process::Command::new("hostname").output()
            && output.status.success()
        {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !value.is_empty() {
                return value;
            }
        }

        "reshape-device".to_string()
    }

    fn resolve_probe_host() -> String {
        let raw = std::env::var("CONNECTIVITY_PROBE_TARGET")
            .or_else(|_| std::env::var("CONNECTIVITY_PROBE_HOST"))
            .unwrap_or_else(|_| "1.1.1.1".to_string());
        probe_target_to_host(&raw).unwrap_or_else(|| "1.1.1.1".to_string())
    }

    async fn run_supervisor_tick(&self) -> bool {
        let Ok(_guard) = self.mutation.try_lock() else {
            return false;
        };
        {
            let state = self.ap_state.lock().await;
            if state.machine.resolved() {
                persist_boot_decision().await;
                return true;
            }
        }

        let test_in_progress = {
            let tests = self.active_tests.lock().await;
            tests
                .values()
                .any(|record| record.state == ConnectionState::Testing)
        };

        let save_in_progress = {
            let state = self.ap_state.lock().await;
            state.save_in_progress
        };

        let boot_grace_elapsed = read_device_uptime()
            .await
            .is_none_or(|elapsed| elapsed >= self.boot_grace);

        let current_enabled = match self.is_hotspot_enabled().await {
            Ok(v) => v,
            Err(e) => {
                warn!("AP supervisor could not determine hotspot state: {e}");
                false
            }
        };

        let mut uplink_healthy = false;

        if !(test_in_progress || save_in_progress) {
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
            uplink_healthy = connectivity.is_connected && connectivity.can_reach_host;
        }

        let decision = {
            let mut state = self.ap_state.lock().await;
            state.machine.step(SupervisorInput {
                test_in_progress,
                save_in_progress,
                uplink_healthy,
                hotspot_enabled: current_enabled,
                boot_grace_elapsed,
            })
        };

        if let Some(action) = decision.action {
            info!(
                ?action,
                test_in_progress,
                save_in_progress,
                uplink_healthy,
                hotspot_enabled = current_enabled,
                boot_grace_elapsed,
                persistent_enabled = decision.persistent_enabled,
                reason = %decision.last_reason,
                "AP supervisor issued action"
            );
            let result = match action {
                SupervisorAction::Enable => {
                    self.start_hotspot(&self.hotspot_name, &self.hotspot_password)
                        .await
                }
            };

            if let Err(e) = result {
                error!("AP supervisor action failed: {e}");
                let mut state = self.ap_state.lock().await;
                state.machine.note_action_error(action, &e.to_string());
            }
        }

        let resolved = self.ap_state.lock().await.machine.resolved();
        if resolved {
            persist_boot_decision().await;
        }
        resolved
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

        let boot_elapsed = read_device_uptime().await;
        {
            let mut state = self.ap_state.lock().await;
            state.machine = ApSupervisorMachine::for_boot_elapsed(boot_elapsed, self.boot_grace);
            if fs::try_exists(BOOT_DECISION_PATH).await.unwrap_or(false) {
                state
                    .machine
                    .resolve("AP boot decision already made for this boot");
            }
        }

        let cloned = Self {
            active_tests: self.active_tests.clone(),
            ap_state: self.ap_state.clone(),
            supervisor_started: self.supervisor_started.clone(),
            hotspot_name: self.hotspot_name.clone(),
            hotspot_password: self.hotspot_password.clone(),
            probe_host: self.probe_host.clone(),
            mutation: self.mutation.clone(),
            boot_grace: self.boot_grace,
        };

        tokio::spawn(async move {
            if cloned.run_supervisor_tick().await {
                return;
            }
            loop {
                sleep(Duration::from_secs(8)).await;
                if cloned.run_supervisor_tick().await {
                    return;
                }
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
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr_indicates_missing_connection(&stderr) {
                return Ok(false);
            }
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let active_connections = String::from_utf8_lossy(&output.stdout);
        Ok(hotspot_is_active_in_list(
            &active_connections,
            &self.hotspot_name,
        ))
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

    async fn get_connection_by_name(&self, name: &str) -> NetResult<Option<SavedNetwork>> {
        let output = Command::new("nmcli")
            .args([
                "-g",
                "connection.uuid,connection.id,connection.type",
                "connection",
                "show",
                name,
            ])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr_indicates_missing_connection(&stderr) {
                return Ok(None);
            }
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut lines = stdout
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty());

        let out_uuid = lines.next().unwrap_or_default();
        let name = lines.next().unwrap_or_default();
        let conn_type = lines.next().unwrap_or_default();

        if out_uuid.is_empty() || name.is_empty() {
            return Ok(None);
        }

        let transport = match conn_type.as_str() {
            "802-11-wireless" => NetworkTransport::Wifi,
            "802-3-ethernet" => NetworkTransport::Ethernet,
            _ => return Ok(None),
        };

        Ok(Some(SavedNetwork {
            id: out_uuid,
            transport,
            display_name: name,
            managed: true,
        }))
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
}

pub(super) fn parse_nmcli_terse_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next_c) = chars.peek()
                && (*next_c == ':' || *next_c == '\\')
            {
                current.push(*next_c);
                chars.next();
                continue;
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

fn stderr_indicates_missing_connection(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.contains("not found")
        || lower.contains("unknown")
        || lower.contains("no such connection profile")
        || lower.contains("no such connection")
}

async fn read_device_uptime() -> Option<Duration> {
    let uptime = fs::read_to_string("/proc/uptime").await.ok()?;
    let seconds = uptime.split_whitespace().next()?.parse::<f64>().ok()?;
    if seconds.is_finite() && seconds >= 0.0 {
        Some(Duration::from_secs_f64(seconds))
    } else {
        None
    }
}

#[cfg(test)]
fn hotspot_state_is_enabled(state_stdout: &str) -> bool {
    let normalized = state_stdout.trim().to_lowercase();
    normalized == "activated" || normalized.ends_with(": activated")
}

fn hotspot_is_active_in_list(active_stdout: &str, hotspot_name: &str) -> bool {
    active_stdout.lines().any(|line| {
        let fields = parse_nmcli_terse_line(line);
        fields.len() >= 2 && fields[0] == hotspot_name && fields[1] == "802-11-wireless"
    })
}

fn probe_target_to_host(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some((_, rest)) = trimmed.split_once("://") {
        let authority = rest
            .split('/')
            .next()
            .unwrap_or_default()
            .split('?')
            .next()
            .unwrap_or_default()
            .split('#')
            .next()
            .unwrap_or_default();
        let host = authority
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(':')
            .next()
            .unwrap_or_default()
            .trim();
        if host.is_empty() {
            None
        } else {
            Some(host.to_string())
        }
    } else {
        Some(trimmed.to_string())
    }
}

async fn is_physical_interface(device: &str) -> bool {
    let sysfs_device_path = format!("/sys/class/net/{device}/device");
    fs::metadata(sysfs_device_path).await.is_ok()
}

async fn read_interface_mac(device: &str) -> Option<String> {
    let path = format!("/sys/class/net/{device}/address");
    let raw = fs::read_to_string(path).await.ok()?;
    let mac = raw.trim().to_uppercase();
    if mac.is_empty() || mac == "00:00:00:00:00:00" {
        None
    } else {
        Some(mac)
    }
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

    networks.sort_by_key(|a| std::cmp::Reverse(a.signal_strength));
    let mut seen = HashSet::new();
    networks.retain(|n| seen.insert(n.ssid.clone()));
    networks
}

pub(super) fn build_nmcli_connection_args(
    settings: &NetworkSettings,
    is_wifi: bool,
    connection_name: &str,
) -> Vec<String> {
    let mut args = vec!["connection".to_string(), "add".to_string()];

    if is_wifi {
        args.push("type".to_string());
        args.push("wifi".to_string());
        if let Some(ssid) = &settings.ssid {
            args.push("con-name".to_string());
            args.push(connection_name.to_string());
            args.push("ssid".to_string());
            args.push(ssid.clone());
        }
    } else {
        args.push("type".to_string());
        args.push("ethernet".to_string());
        args.push("con-name".to_string());
        args.push(connection_name.to_string());
    }

    if let Some(eap_login) = &settings.eap_login {
        if is_wifi {
            args.push("wifi-sec.key-mgmt".to_string());
            args.push("wpa-eap".to_string());
        }

        args.push("802-1x.eap".to_string());
        args.push("peap".to_string());
        args.push("802-1x.identity".to_string());
        args.push(eap_login.clone());

        if let Some(eap_pwd) = &settings.eap_password {
            args.push("802-1x.password".to_string());
            args.push(eap_pwd.clone());
        }

        args.push("802-1x.phase2-auth".to_string());
        args.push("mschapv2".to_string());
    } else if let Some(pwd) = &settings.password
        && is_wifi
        && !pwd.is_empty()
    {
        args.push("wifi-sec.key-mgmt".to_string());
        args.push("wpa-psk".to_string());
        args.push("wifi-sec.psk".to_string());
        args.push(pwd.clone());
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

                if let Some(dns) = &ipv4.dns
                    && !dns.is_empty()
                {
                    args.push("ipv4.dns".to_string());
                    args.push(dns.join(","));
                }
            }
            IpMethod::Auto => {
                args.push("ipv4.method".to_string());
                args.push("auto".to_string());
                if let Some(dns) = &ipv4.dns
                    && !dns.is_empty()
                {
                    args.extend([
                        "ipv4.dns".into(),
                        dns.join(","),
                        "ipv4.ignore-auto-dns".into(),
                        "yes".into(),
                    ]);
                }
            }
        }
    }

    args
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
        let mut saved = Vec::new();

        if let Some(entry) = self.get_connection_by_name(MANAGED_WIFI_PROFILE).await? {
            saved.push(entry);
        }
        if let Some(entry) = self.get_connection_by_name(MANAGED_ETH_PROFILE).await? {
            saved.push(entry);
        }

        Ok(saved)
    }

    async fn start_wifi_test(&self, settings: NetworkSettings) -> NetResult<Uuid> {
        self.start_test(settings, NetworkTransport::Wifi).await
    }

    async fn start_ethernet_test(&self, settings: NetworkSettings) -> NetResult<Uuid> {
        self.start_test(settings, NetworkTransport::Ethernet).await
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
        let guard = self.mutation.clone().try_lock_owned().map_err(|_| {
            NetworkError::ConnectionFailed("Another network operation is in progress".into())
        })?;
        let mut tests = self.active_tests.lock().await;
        let record = tests
            .get_mut(&test_id)
            .ok_or_else(|| NetworkError::NotFound("Test ID not found".into()))?;
        if matches!(record.state, ConnectionState::Saved(_)) {
            return Ok(());
        }
        let ConnectionState::Connected(report) = &record.state else {
            return Err(NetworkError::ConnectionFailed(
                "Cannot save an unsuccessful test".into(),
            ));
        };
        let report = report.clone();
        let settings = record.settings.clone();
        let transport = record.transport.clone();
        let name = match transport {
            NetworkTransport::Wifi => MANAGED_WIFI_PROFILE,
            NetworkTransport::Ethernet => MANAGED_ETH_PROFILE,
        };
        let old_profile = self.get_connection_by_name(name).await?.map(|p| p.id);
        record.state = ConnectionState::Saving;
        self.ap_state.lock().await.save_in_progress = true;
        let tests = self.active_tests.clone();
        let ap_state = self.ap_state.clone();
        tokio::spawn(async move {
            let _guard = guard;
            // Let the accepted response reach the browser before switching its network.
            sleep(Duration::from_secs(1)).await;
            let result = super::transaction::run(
                &super::transaction::SystemCommands,
                &settings,
                &transport,
                &report.interface,
                Some(&report),
                old_profile.as_deref(),
            )
            .await;
            if let Some(record) = tests.lock().await.get_mut(&test_id) {
                record.state = match result {
                    Ok(report) => ConnectionState::Saved(report),
                    Err(e) => ConnectionState::Failed(e.to_string()),
                };
            }
            ap_state.lock().await.save_in_progress = false;
        });
        Ok(())
    }

    async fn discard_test(&self, test_id: Uuid) -> NetResult<()> {
        self.ensure_supervisor_started().await;
        let _guard = self.mutation.try_lock().map_err(|_| {
            NetworkError::ConnectionFailed(
                "Wait for the network operation to finish before discarding".into(),
            )
        })?;
        let mut tests = self.active_tests.lock().await;
        tests
            .remove(&test_id)
            .map(|_| ())
            .ok_or_else(|| NetworkError::NotFound("Test ID not found".into()))
    }

    async fn forget_network(&self, id: &str) -> NetResult<()> {
        let _guard = self.mutation.try_lock().map_err(|_| {
            NetworkError::ConnectionFailed("Another network operation is in progress".into())
        })?;
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

        info!("Forgot network with ID {id}");
        Ok(())
    }

    async fn reset_all_networks(&self) -> NetResult<()> {
        let _guard = self.mutation.try_lock().map_err(|_| {
            NetworkError::ConnectionFailed("Another network operation is in progress".into())
        })?;
        self.ensure_supervisor_started().await;

        {
            let mut state = self.ap_state.lock().await;
            state.save_in_progress = true;
        }

        let reset_result: NetResult<()> = async {
            for profile in [MANAGED_WIFI_PROFILE, MANAGED_ETH_PROFILE] {
                let output = Command::new("nmcli")
                    .args(["connection", "delete", profile])
                    .output()
                    .await
                    .map_err(|e| NetworkError::Internal(e.to_string()))?;

                if output.status.success() {
                    info!("Deleted managed network profile {profile}");
                    continue;
                }

                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if !stderr_indicates_missing_connection(&stderr) {
                    return Err(NetworkError::Internal(stderr));
                }
            }

            {
                let mut tests = self.active_tests.lock().await;
                tests.clear();
            }

            self.start_hotspot(&self.hotspot_name, &self.hotspot_password)
                .await
        }
        .await;

        {
            let mut state = self.ap_state.lock().await;
            state.save_in_progress = false;
        }

        reset_result
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
            "802-11-wireless.channel".to_string(),
            "6".to_string(),
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

    async fn get_ap_controller_status(&self) -> NetResult<AccessPointControllerStatus> {
        self.ensure_supervisor_started().await;
        let test_in_progress = {
            let tests = self.active_tests.lock().await;
            tests
                .values()
                .any(|record| record.state == ConnectionState::Testing)
        };
        let state = self.ap_state.lock().await.clone();
        let is_enabled = self.is_hotspot_enabled().await.unwrap_or(false);
        let automation_paused = test_in_progress || state.save_in_progress;
        let last_reason = if automation_paused {
            "AP automation paused during connection transition".to_string()
        } else {
            state.machine.last_reason().to_string()
        };
        Ok(AccessPointControllerStatus {
            is_enabled,
            persistent_enabled: state.machine.persistent_enabled(),
            automation_paused,
            last_reason,
        })
    }

    async fn get_ap_preferences(&self) -> NetResult<AccessPointPreferences> {
        self.ensure_supervisor_started().await;
        let state = self.ap_state.lock().await;
        Ok(AccessPointPreferences {
            persistent_enabled: state.machine.persistent_enabled(),
        })
    }

    async fn set_ap_preferences(&self, prefs: AccessPointPreferences) -> NetResult<()> {
        let _guard = self.mutation.try_lock().map_err(|_| {
            NetworkError::ConnectionFailed("Another network operation is in progress".into())
        })?;
        self.ensure_supervisor_started().await;

        {
            let mut state = self.ap_state.lock().await;
            state
                .machine
                .set_persistent_enabled(prefs.persistent_enabled);
        }

        if prefs.persistent_enabled {
            self.start_hotspot(&self.hotspot_name, &self.hotspot_password)
                .await
        } else {
            self.stop_hotspot(&self.hotspot_name).await
        }
    }

    async fn start_ap_supervisor(&self) -> NetResult<()> {
        self.ensure_supervisor_started().await;
        Ok(())
    }

    async fn get_network_mac_addresses(&self) -> NetResult<NetworkMacAddresses> {
        self.ensure_supervisor_started().await;
        let output = Command::new("nmcli")
            .args(["-t", "-f", "DEVICE,TYPE", "device", "status"])
            .output()
            .await
            .map_err(|e| NetworkError::Internal(e.to_string()))?;

        if !output.status.success() {
            return Err(NetworkError::Internal(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut wifi = Vec::new();
        let mut ethernet = Vec::new();

        for line in stdout.lines() {
            let fields = parse_nmcli_terse_line(line);
            if fields.len() < 2 {
                continue;
            }

            let device = fields[0].trim();
            let kind = fields[1].trim();
            if device.is_empty() || !is_physical_interface(device).await {
                continue;
            }

            let Some(mac) = read_interface_mac(device).await else {
                continue;
            };

            if kind == "wifi" {
                wifi.push(mac);
            } else if kind == "ethernet" {
                ethernet.push(mac);
            }
        }

        wifi.sort();
        wifi.dedup();
        ethernet.sort();
        ethernet.dedup();

        Ok(NetworkMacAddresses { wifi, ethernet })
    }
}

#[cfg(test)]
mod nmcli_unit_tests {
    use crate::network::types::NetworkSettings;

    use super::{
        build_nmcli_connection_args, hotspot_is_active_in_list, hotspot_state_is_enabled,
        probe_target_to_host, stderr_indicates_missing_connection,
    };

    #[test]
    fn hotspot_state_enabled_only_when_activated() {
        assert!(hotspot_state_is_enabled("activated"));
        assert!(hotspot_state_is_enabled("GENERAL.STATE: activated"));
        assert!(!hotspot_state_is_enabled("deactivated"));
        assert!(!hotspot_state_is_enabled("activating"));
        assert!(!hotspot_state_is_enabled(""));
    }

    #[test]
    fn missing_connection_errors_are_detected() {
        assert!(stderr_indicates_missing_connection(
            "Error: no such connection profile."
        ));
        assert!(stderr_indicates_missing_connection("unknown connection"));
        assert!(stderr_indicates_missing_connection("not found"));
        assert!(!stderr_indicates_missing_connection("permission denied"));
    }

    #[test]
    fn hotspot_active_list_detects_active_profile() {
        let stdout = "RIS0440:802-11-wireless\nWired connection 1:802-3-ethernet\n";
        assert!(hotspot_is_active_in_list(stdout, "RIS0440"));
    }

    #[test]
    fn hotspot_active_list_ignores_non_active_or_non_wifi_profiles() {
        let stdout = "RIS0440:802-3-ethernet\n";
        assert!(!hotspot_is_active_in_list(stdout, "RIS0440"));
    }

    #[test]
    fn probe_target_accepts_url_or_host() {
        assert_eq!(
            probe_target_to_host("https://ris-api.reshapebiotech.com"),
            Some("ris-api.reshapebiotech.com".to_string())
        );
        assert_eq!(
            probe_target_to_host("ris-api.reshapebiotech.com"),
            Some("ris-api.reshapebiotech.com".to_string())
        );
        assert_eq!(probe_target_to_host(""), None);
    }

    #[test]
    fn ethernet_enterprise_uses_peap_mschapv2() {
        let args = build_nmcli_connection_args(
            &NetworkSettings {
                ssid: None,
                password: None,
                id: None,
                eap_login: Some("user@example.com".into()),
                eap_password: Some("secret".into()),
                ipv4: None,
            },
            false,
            "reshape-managed-ethernet",
        );

        assert!(args.windows(2).any(|w| w == ["802-1x.eap", "peap"]));
        assert!(
            args.windows(2)
                .any(|w| w == ["802-1x.phase2-auth", "mschapv2"])
        );
        assert!(
            args.windows(2)
                .any(|w| w == ["802-1x.identity", "user@example.com"])
        );
        assert!(args.windows(2).any(|w| w == ["802-1x.password", "secret"]));
    }
}
