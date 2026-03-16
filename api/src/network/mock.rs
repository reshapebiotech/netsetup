use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use uuid::Uuid;
use async_trait::async_trait;

use super::{NetworkController, NetworkError, Result};
use super::types::{ConnectionState, SavedNetwork, WifiNetwork};

#[derive(Default)]
struct MockState {
    saved_networks: VecDeque<SavedNetwork>,
    active_tests: HashMap<Uuid, ConnectionState>,
}

#[derive(Clone)]
pub struct MockNetworkController {
    state: Arc<RwLock<MockState>>,
}

impl MockNetworkController {
    pub fn new() -> Self {
        let mut state = MockState::default();
        state.saved_networks.push_back(SavedNetwork {
            id: "managed-wifi-1".to_string(),
            ssid: "Home_Network".to_string(),
        });
        
        Self {
            state: Arc::new(RwLock::new(state)),
        }
    }
}

#[async_trait]
impl NetworkController for MockNetworkController {
    async fn scan_wifi(&self) -> Result<Vec<WifiNetwork>> {
        // Simulate scanning delay
        sleep(Duration::from_millis(800)).await;
        
        Ok(vec![
            WifiNetwork {
                ssid: "Home_Network".to_string(),
                signal_strength: 85,
                security: "WPA2".to_string(),
            },
            WifiNetwork {
                ssid: "CoffeeShop_Guest".to_string(),
                signal_strength: 60,
                security: "None".to_string(),
            },
            WifiNetwork {
                ssid: "Neighbor_5G".to_string(),
                signal_strength: 30,
                security: "WPA3".to_string(),
            },
        ])
    }

    async fn get_saved_networks(&self) -> Result<Vec<SavedNetwork>> {
        let state = self.state.read().await;
        Ok(state.saved_networks.iter().cloned().collect())
    }

    async fn connect(&self, ssid: &str, password: &str) -> Result<Uuid> {
        let test_id = Uuid::new_v4();
        
        // Immediately record that a test is active
        {
            let mut state = self.state.write().await;
            state.active_tests.insert(test_id, ConnectionState::Testing);
        }

        // Spawn a background task to simulate the connection process
        let state_clone = self.state.clone();
        let ssid_clone = ssid.to_string();
        let password_clone = password.to_string();
        
        tokio::spawn(async move {
            // Simulate the "connectivity chasm"
            sleep(Duration::from_secs(5)).await;
            
            let final_state = if password_clone == "password" || password_clone.is_empty() {
                // Success: Add to saved networks (FIFO)
                let mut state = state_clone.write().await;
                
                if state.saved_networks.len() >= 2 {
                    state.saved_networks.pop_front();
                }
                
                // For mock, just use a random ID or next sequential
                let next_id = format!("managed-wifi-{}", Uuid::new_v4().simple());
                state.saved_networks.push_back(SavedNetwork {
                    id: next_id,
                    ssid: ssid_clone,
                });
                
                ConnectionState::Connected
            } else {
                // Failure
                ConnectionState::Failed("Incorrect password or network out of range".to_string())
            };
            
            // Update the test result
            let mut state = state_clone.write().await;
            state.active_tests.insert(test_id, final_state);
        });

        Ok(test_id)
    }

    async fn connection_status(&self, test_id: Uuid) -> Result<ConnectionState> {
        let state = self.state.read().await;
        state.active_tests.get(&test_id)
            .cloned()
            .ok_or_else(|| NetworkError::NotFound("Test ID not found".to_string()))
    }

    async fn forget_network(&self, id: &str) -> Result<()> {
        let mut state = self.state.write().await;
        state.saved_networks.retain(|net| net.id != id);
        Ok(())
    }
}
