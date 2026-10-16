use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal_strength: u8, // 0-100
    pub security: String,    // e.g., "WPA2", "EAP", "None"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthernetNetwork {
    pub interface: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedNetwork {
    pub id: String,
    pub transport: NetworkTransport,
    pub display_name: String,
    pub managed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkTransport {
    Wifi,
    Ethernet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", content = "detail")]
pub enum ConnectionState {
    Testing,
    Connected(ConnectivityReport),
    Saving,
    Saved(ConnectivityReport),
    Failed(String), // Reason for failure
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectivityReport {
    pub interface: String,
    pub address: String,
    pub gateway_reachable: Option<bool>,
    pub internet_verified: bool,
    pub reshape_reachable: bool,
}

impl ConnectivityReport {
    pub fn preserves(&self, previous: &Self) -> bool {
        (!previous.internet_verified || self.internet_verified)
            && (!previous.reshape_reachable || self.reshape_reachable)
            && (previous.internet_verified
                || previous.reshape_reachable
                || self.internet_verified
                || self.reshape_reachable
                || previous.gateway_reachable != Some(true)
                || self.gateway_reachable == Some(true))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Address {
    pub address: String,
    pub prefix: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IpMethod {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IpConfig {
    pub method: IpMethod,
    pub address_data: Option<Vec<Address>>,
    pub gateway: Option<String>,
    pub dns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkSettings {
    pub ssid: Option<String>,
    pub password: Option<String>,
    pub id: Option<String>, // UUID of an existing connection
    pub eap_login: Option<String>,
    pub eap_password: Option<String>,
    pub ipv4: Option<IpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Connectivity {
    pub is_connected: bool,
    pub can_reach_host: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessPointStatus {
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessPointPreferences {
    pub persistent_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessPointControllerStatus {
    pub ssid: String,
    pub boot_grace_seconds: u64,
    pub is_enabled: bool,
    pub persistent_enabled: bool,
    pub automation_paused: bool,
    pub last_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkMacAddresses {
    pub wifi: Vec<String>,
    pub ethernet: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentConnectionCheck {
    pub name: String,
    pub interface: String,
    pub report: Option<ConnectivityReport>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedNetworkDetails {
    pub network: SavedNetwork,
    pub settings: NetworkSettings,
    pub security: String,
    pub edit_unavailable: Option<String>,
}
