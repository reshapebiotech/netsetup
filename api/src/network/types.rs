use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal_strength: u8, // 0-100
    pub security: String,    // e.g., "WPA2", "None"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedNetwork {
    pub id: String, // e.g. "managed-wifi-1"
    pub ssid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", content = "detail")]
pub enum ConnectionState {
    Testing,
    Connected,
    Failed(String), // Reason for failure
}
