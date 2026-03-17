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
    Connected,
    Failed(String), // Reason for failure
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
