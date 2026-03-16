use crate::network::nm_dbus::{AccessPointProxy, DeviceProxy, NetworkManagerProxy, WirelessProxy};
use crate::network::types::{ConnectionState, SavedNetwork, WifiNetwork};
use crate::network::{NetworkController, NetworkError, Result as NetResult};
use async_trait::async_trait;
use uuid::Uuid;
use zbus::Connection;

pub struct DbusNetworkController {
    conn: Connection,
}

impl DbusNetworkController {
    pub async fn new() -> Result<Self, zbus::Error> {
        let conn = Connection::system().await?;
        Ok(Self { conn })
    }
}

#[async_trait]
impl NetworkController for DbusNetworkController {
    async fn scan_wifi(&self) -> NetResult<Vec<WifiNetwork>> {
        let nm = NetworkManagerProxy::new(&self.conn)
            .await
            .map_err(|e| NetworkError::ScanFailed(e.to_string()))?;

        let devices = nm
            .get_devices()
            .await
            .map_err(|e| NetworkError::ScanFailed(e.to_string()))?;

        let mut wifi_device_path = None;
        for path in devices {
            if let Ok(device) = DeviceProxy::builder(&self.conn)
                .path(&path)
                .unwrap()
                .build()
                .await
            {
                if let Ok(2) = device.device_type().await {
                    // 2 = NM_DEVICE_TYPE_WIFI
                    wifi_device_path = Some(path);
                    break;
                }
            }
        }

        let path = wifi_device_path
            .ok_or_else(|| NetworkError::ScanFailed("No Wi-Fi device found".into()))?;

        let wireless = WirelessProxy::builder(&self.conn)
            .path(&path)
            .unwrap()
            .build()
            .await
            .map_err(|e| NetworkError::ScanFailed(e.to_string()))?;

        // Optional: request_scan. It might throw if too soon, so we ignore errors.
        let _ = wireless
            .request_scan(std::collections::HashMap::new())
            .await;

        let mut networks = Vec::new();
        let aps = wireless
            .get_access_points()
            .await
            .map_err(|e| NetworkError::ScanFailed(e.to_string()))?;

        for ap_path in aps {
            if let Ok(ap) = AccessPointProxy::builder(&self.conn)
                .path(&ap_path)
                .unwrap()
                .build()
                .await
            {
                if let (Ok(ssid_bytes), Ok(strength), Ok(wpa), Ok(rsn)) = (
                    ap.ssid().await,
                    ap.strength().await,
                    ap.wpa_flags().await,
                    ap.rsn_flags().await,
                ) {
                    if ssid_bytes.is_empty() {
                        continue;
                    }
                    let ssid = String::from_utf8_lossy(&ssid_bytes).to_string();

                    let security = if rsn != 0 || wpa != 0 {
                        "WPA2".to_string() // Simplified, can be refined
                    } else {
                        "None".to_string()
                    };

                    networks.push(WifiNetwork {
                        ssid,
                        signal_strength: strength,
                        security,
                    });
                }
            }
        }

        // Deduplicate and sort by strength
        networks.sort_by(|a, b| b.signal_strength.cmp(&a.signal_strength));
        networks.dedup_by(|a, b| a.ssid == b.ssid);

        Ok(networks)
    }

    async fn get_saved_networks(&self) -> NetResult<Vec<SavedNetwork>> {
        Err(NetworkError::Internal("Not implemented".into()))
    }

    async fn connect(&self, _ssid: &str, _password: &str) -> NetResult<Uuid> {
        Err(NetworkError::ConnectionFailed("Not implemented".into()))
    }

    async fn connection_status(&self, _test_id: Uuid) -> NetResult<ConnectionState> {
        Err(NetworkError::Internal("Not implemented".into()))
    }

    async fn forget_network(&self, _id: &str) -> NetResult<()> {
        Err(NetworkError::Internal("Not implemented".into()))
    }
}
