use super::drafts::{self, TestRecord};
use crate::network::types::{
    AccessPointControllerStatus, AccessPointPreferences, AccessPointStatus, ConnectionState,
    Connectivity, ConnectivityReport, CurrentConnectionCheck, EthernetNetwork, NetworkDraft,
    NetworkMacAddresses, NetworkSettings, NetworkTransport, SavedNetwork, SavedNetworkDetails,
    WifiNetwork,
};
use crate::network::{NetworkController, NetworkError, Result as NetResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct MockNetworkController {
    tests: Arc<Mutex<HashMap<Uuid, TestRecord>>>,
    saved: Arc<Mutex<Option<SavedNetwork>>>,
    saved_settings: Mutex<NetworkSettings>,
    endpoint_groups: Arc<Vec<crate::config::EndpointGroup>>,
    ap_prefs: Arc<Mutex<AccessPointPreferences>>,
}

impl MockNetworkController {
    #[cfg(test)]
    pub fn new() -> Self {
        Self::with_groups(vec![])
    }

    pub fn with_groups(groups: Vec<crate::config::EndpointGroup>) -> Self {
        Self {
            endpoint_groups: Arc::new(groups),
            tests: Arc::new(Mutex::new(HashMap::new())),
            saved: Arc::new(Mutex::new(Some(SavedNetwork {
                id: "managed-wifi-1".into(),
                transport: NetworkTransport::Wifi,
                display_name: "Home_Network".into(),
                managed: true,
            }))),
            saved_settings: Mutex::new(NetworkSettings {
                ssid: Some("Home_Network".into()),
                password: Some("password".into()),
                id: None,
                eap_login: None,
                eap_password: None,
                ipv4: Some(super::types::IpConfig {
                    method: super::types::IpMethod::Auto,
                    address_data: None,
                    gateway: None,
                    dns: None,
                }),
            }),
            ap_prefs: Arc::new(Mutex::new(AccessPointPreferences {
                persistent_enabled: false,
            })),
        }
    }

    async fn start_test(
        &self,
        mut settings: NetworkSettings,
        transport: NetworkTransport,
    ) -> NetResult<Uuid> {
        let source_draft = if let Some(id) = settings
            .id
            .as_deref()
            .and_then(|id| Uuid::parse_str(id).ok())
        {
            let mut tests = self.tests.lock().await;
            drafts::prune(&mut tests);
            tests
                .get(&id)
                .filter(|record| !matches!(record.state, ConnectionState::Saved(_)))
                .cloned()
        } else {
            None
        };
        if let Some(draft) = source_draft {
            draft.retain_credentials(&transport, &mut settings)?;
        }
        if let Some(id) = settings.id.clone() {
            let details = self.get_saved_network(&id).await?;
            if details.network.transport != transport
                || settings.ssid != details.settings.ssid
                || settings.eap_login != details.settings.eap_login
            {
                return Err(NetworkError::ConnectionFailed(
                    "The network or identity changed. Enter its password again.".into(),
                ));
            }
            let saved = self.saved_settings.lock().await;
            settings.password = saved.password.clone();
            settings.eap_password = saved.eap_password.clone();
            settings.id = None;
        }
        let test_id = Uuid::new_v4();
        {
            let mut tests = self.tests.lock().await;
            drafts::insert(
                &mut tests,
                test_id,
                TestRecord::new(settings.clone(), transport.clone()),
            );
        }

        let tests = self.tests.clone();
        let groups = self.endpoint_groups.clone();
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
                record.finish(ConnectionState::Connected(ConnectivityReport {
                    interface: if transport == NetworkTransport::Wifi {
                        "wlan0"
                    } else {
                        "eth0"
                    }
                    .into(),
                    address: "192.168.1.42".into(),
                    gateway_reachable: Some(true),
                    internet_verified: ssid != "CoffeeShop_Guest",
                    endpoint_groups: groups
                        .iter()
                        .map(|g| {
                            super::types::EndpointGroupResult::from_config(
                                g,
                                ssid != "CoffeeShop_Guest",
                            )
                        })
                        .collect(),
                }));
            } else {
                record.finish(ConnectionState::Failed(
                    "Incorrect password or network out of range".into(),
                ));
            }
        });

        Ok(test_id)
    }
}

#[async_trait]
impl NetworkController for MockNetworkController {
    async fn scan_wifi(&self) -> NetResult<Vec<WifiNetwork>> {
        Ok(vec![
            WifiNetwork::new(
                "Home_Network".into(),
                85,
                "WPA2".into(),
                vec!["02:00:00:00:01:01".into()],
            ),
            WifiNetwork::new(
                "CoffeeShop_Guest".into(),
                60,
                "Open".into(),
                vec!["02:00:00:00:02:01".into()],
            ),
            WifiNetwork::new(
                "Neighbor_5G".into(),
                30,
                "WPA3".into(),
                vec!["02:00:00:00:03:01".into()],
            ),
            WifiNetwork::new(
                "Enterprise_Corp".into(),
                90,
                "802.1X WPA2".into(),
                vec!["02:00:00:00:04:01".into()],
            ),
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

    async fn get_saved_network(&self, id: &str) -> NetResult<SavedNetworkDetails> {
        let network = self
            .saved
            .lock()
            .await
            .clone()
            .filter(|n| n.id == id)
            .ok_or_else(|| NetworkError::NotFound("Saved network not found".into()))?;
        let mut settings = self.saved_settings.lock().await.clone();
        let security = if settings.eap_login.is_some() {
            "enterprise"
        } else if settings.password.is_some() {
            "personal"
        } else {
            "open"
        };
        settings.password = None;
        settings.eap_password = None;
        Ok(SavedNetworkDetails {
            network,
            settings,
            security: security.into(),
            edit_unavailable: None,
        })
    }

    async fn start_wifi_test(&self, settings: NetworkSettings) -> NetResult<Uuid> {
        self.start_test(settings, NetworkTransport::Wifi).await
    }

    async fn start_ethernet_test(&self, settings: NetworkSettings) -> NetResult<Uuid> {
        self.start_test(settings, NetworkTransport::Ethernet).await
    }

    async fn get_drafts(&self) -> NetResult<Vec<NetworkDraft>> {
        Ok(drafts::list(&mut *self.tests.lock().await))
    }

    async fn get_draft(&self, test_id: Uuid) -> NetResult<SavedNetworkDetails> {
        let mut tests = self.tests.lock().await;
        drafts::prune(&mut tests);
        tests
            .get(&test_id)
            .ok_or_else(|| {
                NetworkError::NotFound(
                    "Draft expired or was discarded. Add a network to start again.".into(),
                )
            })?
            .details(test_id)
    }

    async fn connection_status(&self, test_id: Uuid) -> NetResult<ConnectionState> {
        let mut tests = self.tests.lock().await;
        drafts::prune(&mut tests);
        tests.get(&test_id).map(|r| r.state.clone()).ok_or_else(|| {
            NetworkError::NotFound(
                "This test expired or was discarded. Add a network to start again.".into(),
            )
        })
    }

    async fn save_test(&self, test_id: Uuid) -> NetResult<()> {
        let record = {
            let mut tests = self.tests.lock().await;
            drafts::prune(&mut tests);
            tests.get(&test_id).cloned().ok_or_else(|| {
                NetworkError::NotFound(
                    "This test expired or was discarded. Add a network to start again.".into(),
                )
            })?
        };

        let ConnectionState::Connected(report) = record.state else {
            return Err(NetworkError::ConnectionFailed(
                "Cannot save an unsuccessful test".into(),
            ));
        };

        *self.saved_settings.lock().await = record.settings.clone();
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

        if let Some(record) = self.tests.lock().await.get_mut(&test_id) {
            record.finish(ConnectionState::Saved(report));
        }
        Ok(())
    }

    async fn discard_test(&self, test_id: Uuid) -> NetResult<()> {
        let mut tests = self.tests.lock().await;
        drafts::prune(&mut tests);
        if tests.get(&test_id).is_some_and(|record| {
            matches!(
                record.state,
                ConnectionState::Testing | ConnectionState::Saving
            )
        }) {
            return Err(NetworkError::ConnectionFailed(
                "Wait for the network operation to finish before discarding".into(),
            ));
        }
        tests.remove(&test_id).map(|_| ()).ok_or_else(|| {
            NetworkError::NotFound(
                "This test expired or was discarded. Add a network to start again.".into(),
            )
        })
    }

    async fn forget_network(&self, _id: &str) -> NetResult<()> {
        let mut saved = self.saved.lock().await;
        *saved = None;
        Ok(())
    }

    async fn reset_all_networks(&self) -> NetResult<()> {
        {
            let mut saved = self.saved.lock().await;
            *saved = None;
        }
        {
            let mut tests = self.tests.lock().await;
            tests.clear();
        }
        Ok(())
    }

    async fn get_hotspot_status(&self) -> NetResult<AccessPointStatus> {
        Ok(AccessPointStatus { is_enabled: false })
    }

    async fn start_hotspot(&self, _ssid: &str, _password: &str) -> NetResult<()> {
        Ok(())
    }

    fn connectivity_groups(&self) -> Vec<crate::config::EndpointGroup> {
        self.endpoint_groups.as_ref().clone()
    }

    async fn check_current_connections(&self) -> NetResult<Vec<CurrentConnectionCheck>> {
        Ok(vec![CurrentConnectionCheck {
            name: "Home_Network".into(),
            interface: "wlan0".into(),
            configuration: Some(super::types::CurrentNetworkConfiguration {
                transport: NetworkTransport::Wifi,
                ssid: Some("Home_Network".into()),
                ipv4_method: "auto".into(),
                addresses: vec![super::types::Address {
                    address: "192.168.1.42".into(),
                    prefix: 24,
                }],
                gateway: Some("192.168.1.1".into()),
                dns: vec!["192.168.1.1".into(), "1.1.1.1".into()],
                mac_address: Some("02:00:00:00:00:01".into()),
            }),
            configuration_error: None,
            report: Some(ConnectivityReport {
                interface: "wlan0".into(),
                address: "192.168.1.42".into(),
                gateway_reachable: Some(true),
                internet_verified: true,
                endpoint_groups: self
                    .endpoint_groups
                    .iter()
                    .map(|g| super::types::EndpointGroupResult::from_config(g, true))
                    .collect(),
            }),
            error: None,
        }])
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
            ssid: "reshape-demo".into(),
            boot_grace_seconds: super::ap_supervisor::DEFAULT_BOOT_GRACE.as_secs(),
            automation_paused: false,
            is_enabled: prefs.persistent_enabled,
            persistent_enabled: prefs.persistent_enabled,
            last_reason: if prefs.persistent_enabled {
                "Persistent AP enabled by user".into()
            } else {
                "Internet arrived during boot; leaving setup AP unchanged".into()
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
