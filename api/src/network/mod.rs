pub mod mock;
pub mod nmcli;
pub mod types;

#[cfg(test)]
pub mod integration_tests;

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::network::types::{
    ConnectionState, AccessPointStatus, Connectivity, EthernetNetwork, NetworkSettings, SavedNetwork, WifiNetwork,
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

    /// Applies wireless network settings and connects
    async fn connect_wifi(&self, settings: NetworkSettings) -> Result<Uuid>;

    /// Applies wired network settings and connects
    async fn connect_ethernet(&self, settings: NetworkSettings) -> Result<Uuid>;

    /// Polls the status of an ongoing connection test
    async fn connection_status(&self, test_id: Uuid) -> Result<ConnectionState>;

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
}
