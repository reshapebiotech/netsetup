use crate::network::types::{
    ConnectionState, AccessPointStatus, Connectivity, EthernetNetwork, NetworkSettings, SavedNetwork, WifiNetwork,
};
use crate::network::{NetworkController, NetworkError, Result as NetResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct MockNetworkController {
    active_tests: Arc<Mutex<HashMap<Uuid, ConnectionState>>>,
}

impl MockNetworkController {
    pub fn new() -> Self {
        Self {
            active_tests: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl NetworkController for MockNetworkController {
    async fn scan_wifi(&self) -> NetResult<Vec<WifiNetwork>> {
        Ok(vec![
            WifiNetwork {
                ssid: "Home_Network".into(),
                signal_strength: 85,
                security: "WPA2".into(),
            },
            WifiNetwork {
                ssid: "CoffeeShop_Guest".into(),
                signal_strength: 60,
                security: "None".into(),
            },
            WifiNetwork {
                ssid: "Neighbor_5G".into(),
                signal_strength: 30,
                security: "WPA3".into(),
            },
            WifiNetwork {
                ssid: "Enterprise_Corp".into(),
                signal_strength: 90,
                security: "802-1x".into(),
            },
        ])
    }

    async fn scan_ethernet(&self) -> NetResult<Vec<EthernetNetwork>> {
        Ok(vec![EthernetNetwork {
            interface: "eth0".into(),
            connected: true,
        }])
    }

    async fn get_saved_networks(&self) -> NetResult<Vec<SavedNetwork>> {
        Ok(vec![SavedNetwork {
            id: "managed-wifi-1".into(),
            ssid: "Home_Network".into(),
        }])
    }

    async fn connect_wifi(&self, settings: NetworkSettings) -> NetResult<Uuid> {
        let test_id = Uuid::new_v4();

        let mut tests = self.active_tests.lock().await;
        tests.insert(test_id, ConnectionState::Testing);
        drop(tests);

        let active_tests = self.active_tests.clone();
        
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let mut tests = active_tests.lock().await;
            
            // Mock logic: success if password is "password" or eap_password is "enterprise"
            let pwd = settings.password.unwrap_or_default();
            let eap_pwd = settings.eap_password.unwrap_or_default();

            if pwd == "password" || eap_pwd == "enterprise" || settings.ssid.unwrap_or_default() == "CoffeeShop_Guest" {
                tests.insert(test_id, ConnectionState::Connected);
            } else {
                tests.insert(
                    test_id,
                    ConnectionState::Failed("Incorrect password or network out of range".into()),
                );
            }
        });

        Ok(test_id)
    }

    async fn connect_ethernet(&self, _settings: NetworkSettings) -> NetResult<Uuid> {
        let test_id = Uuid::new_v4();
        let mut tests = self.active_tests.lock().await;
        tests.insert(test_id, ConnectionState::Testing);
        drop(tests);

        let active_tests = self.active_tests.clone();
        
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let mut tests = active_tests.lock().await;
            tests.insert(test_id, ConnectionState::Connected);
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
        tracing::info!("Mock forgetting network {}", id);
        Ok(())
    }

    async fn get_hotspot_status(&self) -> NetResult<AccessPointStatus> {
        Ok(AccessPointStatus { is_enabled: false })
    }

    async fn start_hotspot(&self, _ssid: &str, _password: &str) -> NetResult<()> {
        Ok(())
    }

    async fn stop_hotspot(&self, _ssid: &str) -> NetResult<()> {
        Ok(())
    }

    async fn get_connectivity(&self) -> NetResult<Connectivity> {
        Ok(Connectivity {
            is_connected: true,
            can_reach_host: true,
        })
    }
}
