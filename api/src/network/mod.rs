pub mod mock;
pub mod types;

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::network::types::{ConnectionState, SavedNetwork, WifiNetwork};

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Failed to scan networks: {0}")]
    ScanFailed(String),
    #[error("Connection failed: {0}")]
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
    
    /// Returns up to 2 saved network profiles
    async fn get_saved_networks(&self) -> Result<Vec<SavedNetwork>>;
    
    /// Starts a connection attempt to a given SSID.
    /// Returns a UUID representing this connection test.
    async fn connect(&self, ssid: &str, password: &str) -> Result<Uuid>;
    
    /// Polls the status of an ongoing connection test
    async fn connection_status(&self, test_id: Uuid) -> Result<ConnectionState>;
    
    /// Forgets a saved network by its internal ID (e.g. "managed-wifi-1")
    async fn forget_network(&self, id: &str) -> Result<()>;
}
