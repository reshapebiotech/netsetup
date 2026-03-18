mod ap_supervisor;
pub mod mock;
pub mod nmcli;
pub mod types;

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::network::types::{
    AccessPointControllerStatus, AccessPointPreferences, AccessPointStatus, ConnectionState,
    Connectivity, EthernetNetwork, NetworkMacAddresses, NetworkSettings, SavedNetwork, WifiNetwork,
};

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Failed to scan networks: {0}")]
    ScanFailed(String),
    #[error("Connection failed: {0}")]
    #[allow(dead_code)]
    ConnectionFailed(String),
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, NetworkError>;

#[async_trait]
pub trait NetworkController: Send + Sync {
    /// Scan for available Wi-Fi networks
    async fn scan_wifi(&self) -> Result<Vec<WifiNetwork>>;

    /// Scan for available Ethernet interfaces
    async fn scan_ethernet(&self) -> Result<Vec<EthernetNetwork>>;

    /// Returns up to 2 saved network profiles
    async fn get_saved_networks(&self) -> Result<Vec<SavedNetwork>>;

    /// Starts a Wi-Fi connection test without committing final managed config.
    async fn start_wifi_test(&self, settings: NetworkSettings) -> Result<Uuid>;

    /// Starts an Ethernet connection test without committing final managed config.
    async fn start_ethernet_test(&self, settings: NetworkSettings) -> Result<Uuid>;

    /// Polls the status of an ongoing connection test
    async fn connection_status(&self, test_id: Uuid) -> Result<ConnectionState>;

    /// Commits the settings from a successful test and replaces the existing managed profile.
    async fn save_test(&self, test_id: Uuid) -> Result<()>;

    /// Discards a test and rolls back temporary artifacts.
    async fn discard_test(&self, test_id: Uuid) -> Result<()>;

    /// Forgets a saved network by its internal ID (e.g. "managed-wifi-1" or UUID)
    async fn forget_network(&self, id: &str) -> Result<()>;

    /// Gets current internet connectivity status
    async fn get_connectivity(&self) -> Result<Connectivity>;

    /// Get the current status of the setup Hotspot
    async fn get_hotspot_status(&self) -> Result<AccessPointStatus>;

    /// Starts the local setup Hotspot
    async fn start_hotspot(&self, ssid: &str, password: &str) -> Result<()>;

    /// Stops the local setup Hotspot
    async fn stop_hotspot(&self, ssid: &str) -> Result<()>;

    /// Returns supervisor state for setup access point automation.
    async fn get_ap_controller_status(&self) -> Result<AccessPointControllerStatus>;

    /// Returns AP behavior preferences.
    async fn get_ap_preferences(&self) -> Result<AccessPointPreferences>;

    /// Updates AP behavior preferences.
    async fn set_ap_preferences(&self, prefs: AccessPointPreferences) -> Result<()>;

    /// Starts AP supervisor loop.
    async fn start_ap_supervisor(&self) -> Result<()>;

    /// Returns MAC addresses by network type.
    async fn get_network_mac_addresses(&self) -> Result<NetworkMacAddresses>;
}
