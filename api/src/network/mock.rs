use crate::network::types::{
    AccessPointControllerStatus, AccessPointPreferences, AccessPointStatus, ConnectionState,
    Connectivity, EthernetNetwork, NetworkMacAddresses, NetworkSettings, NetworkTransport,
    SavedNetwork, WifiNetwork,
};
use crate::network::{NetworkController, NetworkError, Result as NetResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
struct MockTestRecord {
    state: ConnectionState,
    settings: NetworkSettings,
    transport: NetworkTransport,
}

pub struct MockNetworkController {
    tests: Arc<Mutex<HashMap<Uuid, MockTestRecord>>>,
    saved: Arc<Mutex<Option<SavedNetwork>>>,
    ap_prefs: Arc<Mutex<AccessPointPreferences>>,
}

impl MockNetworkController {
    pub fn new() -> Self {
        Self {
            tests: Arc::new(Mutex::new(HashMap::new())),
            saved: Arc::new(Mutex::new(Some(SavedNetwork {
                id: "managed-wifi-1".into(),
                transport: NetworkTransport::Wifi,
                display_name: "Home_Network".into(),
                managed: true,
            }))),
            ap_prefs: Arc::new(Mutex::new(AccessPointPreferences {
                persistent_enabled: false,
            })),
        }
    }

    async fn start_test(
        &self,
        settings: NetworkSettings,
        transport: NetworkTransport,
    ) -> NetResult<Uuid> {
        let test_id = Uuid::new_v4();
        {
            let mut tests = self.tests.lock().await;
            tests.insert(
                test_id,
                MockTestRecord {
                    state: ConnectionState::Testing,
                    settings: settings.clone(),
                    transport: transport.clone(),
                },
            );
        }

        let tests = self.tests.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let mut guard = tests.lock().await;
            let Some(record) = guard.get_mut(&test_id) else {
                return;
            };

            let pwd = settings.password.unwrap_or_default();
            let eap_pwd = settings.eap_password.unwrap_or_default();
            let ssid = settings.ssid.unwrap_or_default();

            let ok = match transport {
                NetworkTransport::Wifi => {
                    pwd == "password" || eap_pwd == "enterprise" || ssid == "CoffeeShop_Guest"
                }
                NetworkTransport::Ethernet => true,
            };

            if ok {
                record.state = ConnectionState::Connected;
            } else {
                record.state =
                    ConnectionState::Failed("Incorrect password or network out of range".into());
            }
        });

        Ok(test_id)
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
        let saved = self.saved.lock().await;
        Ok(saved.iter().cloned().collect())
    }

    async fn start_wifi_test(&self, settings: NetworkSettings) -> NetResult<Uuid> {
        self.start_test(settings, NetworkTransport::Wifi).await
    }

    async fn start_ethernet_test(&self, settings: NetworkSettings) -> NetResult<Uuid> {
        self.start_test(settings, NetworkTransport::Ethernet).await
    }

    async fn connection_status(&self, test_id: Uuid) -> NetResult<ConnectionState> {
        let tests = self.tests.lock().await;
        tests
            .get(&test_id)
            .map(|r| r.state.clone())
            .ok_or_else(|| NetworkError::NotFound("Test ID not found".into()))
    }

    async fn save_test(&self, test_id: Uuid) -> NetResult<()> {
        let record = {
            let tests = self.tests.lock().await;
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

        let display_name = match record.transport {
            NetworkTransport::Wifi => record.settings.ssid.unwrap_or_else(|| "Wi-Fi".into()),
            NetworkTransport::Ethernet => "Wired connection".into(),
        };

        let mut saved = self.saved.lock().await;
        *saved = Some(SavedNetwork {
            id: test_id.to_string(),
            transport: record.transport,
            display_name,
            managed: true,
        });

        Ok(())
    }

    async fn discard_test(&self, test_id: Uuid) -> NetResult<()> {
        let mut tests = self.tests.lock().await;
        tests
            .remove(&test_id)
            .map(|_| ())
            .ok_or_else(|| NetworkError::NotFound("Test ID not found".into()))
    }

    async fn forget_network(&self, _id: &str) -> NetResult<()> {
        let mut saved = self.saved.lock().await;
        *saved = None;
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

    async fn get_ap_controller_status(&self) -> NetResult<AccessPointControllerStatus> {
        let prefs = self.ap_prefs.lock().await.clone();
        Ok(AccessPointControllerStatus {
            is_enabled: prefs.persistent_enabled,
            persistent_enabled: prefs.persistent_enabled,
            automation_paused: false,
            last_reason: if prefs.persistent_enabled {
                "Persistent AP enabled".into()
            } else {
                "Uplink healthy".into()
            },
        })
    }

    async fn get_ap_preferences(&self) -> NetResult<AccessPointPreferences> {
        Ok(self.ap_prefs.lock().await.clone())
    }

    async fn set_ap_preferences(&self, prefs: AccessPointPreferences) -> NetResult<()> {
        let mut guard = self.ap_prefs.lock().await;
        *guard = prefs;
        Ok(())
    }

    async fn start_ap_supervisor(&self) -> NetResult<()> {
        Ok(())
    }

    async fn get_network_mac_addresses(&self) -> NetResult<NetworkMacAddresses> {
        Ok(NetworkMacAddresses {
            wifi: vec!["B8:27:EB:AA:BB:01".into()],
            ethernet: vec!["B8:27:EB:AA:BB:02".into()],
        })
    }
}
