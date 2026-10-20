use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Serialize;
use std::net::Ipv4Addr;
use std::sync::Arc;
use uuid::Uuid;

use crate::network::types::{
    AccessPointControllerStatus, AccessPointPreferences, AccessPointStatus, Connectivity,
    CurrentConnectionCheck, EthernetNetwork, NetworkDraft, NetworkMacAddresses, NetworkSettings,
    SavedNetwork, SavedNetworkDetails, WifiNetwork,
};
use crate::network::{NetworkController, NetworkError};

#[derive(Clone)]
pub struct AppState {
    pub network: Arc<dyn NetworkController>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/wifi/scan", get(scan_wifi))
        .route("/ethernet/scan", get(scan_ethernet))
        .route("/saved", get(get_saved))
        .route("/saved/reset", post(reset_saved_networks))
        .route("/saved/{id}", get(get_saved_network).delete(forget_saved))
        .route("/wifi/test", post(test_wifi))
        .route("/ethernet/test", post(test_ethernet))
        .route("/tests", get(get_drafts))
        .route("/tests/{test_id}", get(get_draft))
        .route("/tests/{test_id}/save", post(save_test))
        .route("/tests/{test_id}/discard", post(discard_test))
        .route("/wifi/connect", post(connect_wifi_compat))
        .route("/ethernet/connect", post(connect_ethernet_compat))
        .route("/status/{test_id}", get(status))
        .route("/connectivity", get(get_connectivity))
        .route("/connectivity/check", get(check_current_connections))
        .route("/connectivity/groups", get(connectivity_groups))
        .route("/mac_addresses", get(get_mac_addresses))
        .route("/access_point", get(get_hotspot_status))
        .route("/access_point/controller", get(get_ap_controller_status))
        .route(
            "/access_point/preferences",
            get(get_ap_preferences).put(set_ap_preferences),
        )
}

// Map domain errors to HTTP errors
impl IntoResponse for NetworkError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            NetworkError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            NetworkError::ScanFailed(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            NetworkError::ConnectionFailed(msg) => (StatusCode::BAD_REQUEST, msg),
            NetworkError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

async fn scan_wifi(State(state): State<AppState>) -> Result<Json<Vec<WifiNetwork>>, NetworkError> {
    let networks = state.network.scan_wifi().await?;
    Ok(Json(networks))
}

async fn scan_ethernet(
    State(state): State<AppState>,
) -> Result<Json<Vec<EthernetNetwork>>, NetworkError> {
    let networks = state.network.scan_ethernet().await?;
    Ok(Json(networks))
}

async fn get_saved(State(state): State<AppState>) -> Result<Json<Vec<SavedNetwork>>, NetworkError> {
    let saved = state.network.get_saved_networks().await?;
    Ok(Json(saved))
}

async fn get_drafts(State(state): State<AppState>) -> Result<impl IntoResponse, NetworkError> {
    let drafts: Vec<NetworkDraft> = state.network.get_drafts().await?;
    Ok((
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Json(drafts),
    ))
}

async fn get_draft(
    State(state): State<AppState>,
    Path(test_id): Path<Uuid>,
) -> Result<impl IntoResponse, NetworkError> {
    let draft = state.network.get_draft(test_id).await?;
    Ok((
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Json(draft),
    ))
}

async fn get_saved_network(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SavedNetworkDetails>, NetworkError> {
    state.network.get_saved_network(&id).await.map(Json)
}

async fn forget_saved(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, NetworkError> {
    state.network.forget_network(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reset_saved_networks(State(state): State<AppState>) -> Result<StatusCode, NetworkError> {
    state.network.reset_all_networks().await?;
    Ok(StatusCode::ACCEPTED)
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub test_id: Uuid,
}

fn ipv4_mask(prefix: u8) -> u32 {
    u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0)
}

fn is_subnet_endpoint(address: u32, prefix: u8) -> bool {
    if prefix >= 31 {
        return false;
    }
    let host = address & !ipv4_mask(prefix);
    host == 0 || host == !ipv4_mask(prefix)
}

fn validate_network_settings(settings: &NetworkSettings) -> Result<(), NetworkError> {
    if let Some(ipv4) = &settings.ipv4 {
        if let Some(dns) = &ipv4.dns {
            for server in dns {
                if server.parse::<Ipv4Addr>().is_err() {
                    return Err(NetworkError::ConnectionFailed(format!(
                        "Invalid DNS server '{}': IPv4 only",
                        server
                    )));
                }
            }
        }

        if ipv4.method == crate::network::types::IpMethod::Manual {
            let addresses = ipv4.address_data.as_ref().ok_or_else(|| {
                NetworkError::ConnectionFailed("Manual IPv4 requires at least one address".into())
            })?;
            if addresses.is_empty() {
                return Err(NetworkError::ConnectionFailed(
                    "Manual IPv4 requires at least one address".into(),
                ));
            }

            let mut parsed_addresses = Vec::with_capacity(addresses.len());
            for addr in addresses {
                let address = addr.address.parse::<Ipv4Addr>().map_err(|_| {
                    NetworkError::ConnectionFailed(format!(
                        "Invalid manual IPv4 address '{}'",
                        addr.address
                    ))
                })?;
                if addr.prefix > 32 {
                    return Err(NetworkError::ConnectionFailed(format!(
                        "Invalid IPv4 prefix '{}': must be between 0 and 32",
                        addr.prefix
                    )));
                }
                let address = u32::from(address);
                if is_subnet_endpoint(address, addr.prefix) {
                    return Err(NetworkError::ConnectionFailed(
                        "Manual IPv4 address must not be the subnet's network or broadcast address"
                            .into(),
                    ));
                }
                parsed_addresses.push((address, addr.prefix));
            }

            let gateway = ipv4.gateway.as_ref().ok_or_else(|| {
                NetworkError::ConnectionFailed("Manual IPv4 gateway is required".into())
            })?;
            let gateway = u32::from(gateway.parse::<Ipv4Addr>().map_err(|_| {
                NetworkError::ConnectionFailed(format!("Invalid IPv4 gateway '{}'", gateway))
            })?);
            if parsed_addresses
                .iter()
                .any(|(address, _)| *address == gateway)
            {
                return Err(NetworkError::ConnectionFailed(
                    "Gateway must differ from the device IP address".into(),
                ));
            }
            let matching_subnets: Vec<_> = parsed_addresses
                .iter()
                .filter(|(address, prefix)| {
                    address & ipv4_mask(*prefix) == gateway & ipv4_mask(*prefix)
                })
                .collect();
            if matching_subnets.is_empty() {
                return Err(NetworkError::ConnectionFailed(
                    "Gateway must be in the same subnet as a manual IPv4 address".into(),
                ));
            }
            if matching_subnets
                .iter()
                .all(|(_, prefix)| is_subnet_endpoint(gateway, *prefix))
            {
                return Err(NetworkError::ConnectionFailed(
                    "Gateway must not be the subnet's network or broadcast address".into(),
                ));
            }
        }
    }

    Ok(())
}

async fn test_wifi(
    State(state): State<AppState>,
    Json(settings): Json<NetworkSettings>,
) -> Result<(StatusCode, Json<ConnectResponse>), NetworkError> {
    validate_network_settings(&settings)?;
    let test_id = state.network.start_wifi_test(settings).await?;
    Ok((StatusCode::ACCEPTED, Json(ConnectResponse { test_id })))
}

async fn test_ethernet(
    State(state): State<AppState>,
    Json(settings): Json<NetworkSettings>,
) -> Result<(StatusCode, Json<ConnectResponse>), NetworkError> {
    validate_network_settings(&settings)?;
    let test_id = state.network.start_ethernet_test(settings).await?;
    Ok((StatusCode::ACCEPTED, Json(ConnectResponse { test_id })))
}

async fn save_test(
    State(state): State<AppState>,
    Path(test_id): Path<Uuid>,
) -> Result<StatusCode, NetworkError> {
    state.network.save_test(test_id).await?;
    Ok(StatusCode::ACCEPTED)
}

async fn discard_test(
    State(state): State<AppState>,
    Path(test_id): Path<Uuid>,
) -> Result<StatusCode, NetworkError> {
    state.network.discard_test(test_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn connect_wifi_compat(
    State(state): State<AppState>,
    Json(settings): Json<NetworkSettings>,
) -> Result<(StatusCode, Json<ConnectResponse>), NetworkError> {
    validate_network_settings(&settings)?;
    let test_id = state.network.start_wifi_test(settings).await?;
    Ok((StatusCode::ACCEPTED, Json(ConnectResponse { test_id })))
}

async fn connect_ethernet_compat(
    State(state): State<AppState>,
    Json(settings): Json<NetworkSettings>,
) -> Result<(StatusCode, Json<ConnectResponse>), NetworkError> {
    validate_network_settings(&settings)?;
    let test_id = state.network.start_ethernet_test(settings).await?;
    Ok((StatusCode::ACCEPTED, Json(ConnectResponse { test_id })))
}

async fn status(
    State(state): State<AppState>,
    Path(test_id): Path<Uuid>,
) -> Result<impl IntoResponse, NetworkError> {
    let current_status = state.network.connection_status(test_id).await?;
    Ok((
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Json(current_status),
    ))
}

async fn get_hotspot_status(
    State(state): State<AppState>,
) -> Result<Json<AccessPointStatus>, NetworkError> {
    let status = state.network.get_hotspot_status().await?;
    Ok(Json(status))
}

async fn connectivity_groups(
    State(state): State<AppState>,
) -> Json<Vec<crate::config::EndpointGroup>> {
    Json(state.network.connectivity_groups())
}

async fn check_current_connections(
    State(state): State<AppState>,
) -> Result<Json<Vec<CurrentConnectionCheck>>, NetworkError> {
    state.network.check_current_connections().await.map(Json)
}

async fn get_connectivity(
    State(state): State<AppState>,
) -> Result<Json<Connectivity>, NetworkError> {
    let connectivity = state.network.get_connectivity().await?;
    Ok(Json(connectivity))
}

async fn get_mac_addresses(
    State(state): State<AppState>,
) -> Result<Json<NetworkMacAddresses>, NetworkError> {
    let macs = state.network.get_network_mac_addresses().await?;
    Ok(Json(macs))
}

async fn get_ap_controller_status(
    State(state): State<AppState>,
) -> Result<Json<AccessPointControllerStatus>, NetworkError> {
    let status = state.network.get_ap_controller_status().await?;
    Ok(Json(status))
}

async fn get_ap_preferences(
    State(state): State<AppState>,
) -> Result<Json<AccessPointPreferences>, NetworkError> {
    let prefs = state.network.get_ap_preferences().await?;
    Ok(Json(prefs))
}

async fn set_ap_preferences(
    State(state): State<AppState>,
    Json(prefs): Json<AccessPointPreferences>,
) -> Result<StatusCode, NetworkError> {
    state.network.set_ap_preferences(prefs).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use crate::api::network::{AppState, router};
    use crate::network::mock::MockNetworkController;
    use axum::http::StatusCode;
    use axum_test::TestServer;
    use serde_json::json;
    use std::sync::Arc;

    fn get_test_server() -> TestServer {
        let mock_controller = Arc::new(MockNetworkController::new());
        let state = AppState {
            network: mock_controller,
        };
        let app = router().with_state(state);
        TestServer::new(app)
    }

    #[tokio::test]
    async fn saved_network_edit_reuses_credentials_without_exposing_them() {
        let server = get_test_server();
        let details = server.get("/saved/managed-wifi-1").await;
        details.assert_status_ok();
        let json: serde_json::Value = details.json();
        assert!(json["settings"]["password"].is_null());
        assert_eq!(json["settings"]["ssid"], "Home_Network");
        let started = server.post("/wifi/test").json(&json!({"id":"managed-wifi-1", "ssid":"Home_Network", "ipv4":{"method":"auto","dns":["9.9.9.9"]}})).await;
        started.assert_status(axum::http::StatusCode::ACCEPTED);
        let id = started.json::<serde_json::Value>()["test_id"]
            .as_str()
            .unwrap()
            .to_string();
        tokio::time::sleep(std::time::Duration::from_millis(3100)).await;
        assert_eq!(
            server
                .get(&format!("/status/{id}"))
                .await
                .json::<serde_json::Value>()["status"],
            "Connected"
        );
        server
            .post(&format!("/tests/{id}/save"))
            .await
            .assert_status(axum::http::StatusCode::ACCEPTED);
        let saved = server
            .get(&format!("/saved/{id}"))
            .await
            .json::<serde_json::Value>();
        assert_eq!(saved["settings"]["ipv4"]["dns"][0], "9.9.9.9");
        assert!(saved["settings"]["password"].is_null());
    }

    async fn wait_for_test(server: &TestServer, id: &str) -> serde_json::Value {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let state = server
                    .get(&format!("/status/{id}"))
                    .await
                    .json::<serde_json::Value>();
                if state["status"] != "Testing" {
                    return state;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn recovers_tests_without_exposing_credentials_and_retries_with_retained_password() {
        let server = get_test_server();
        let started = server
            .post("/wifi/test")
            .json(&json!({
                "ssid": "Home_Network", "password": "password",
                "ipv4": { "method": "auto", "dns": ["9.9.9.9"] }
            }))
            .await;
        started.assert_status(StatusCode::ACCEPTED);
        let id = started.json::<serde_json::Value>()["test_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let response = server.get("/tests").await;
        response.assert_status_ok();
        response.assert_header("cache-control", "no-store");
        let drafts = response.json::<serde_json::Value>();
        assert_eq!(drafts[0]["test_id"], id);
        assert_eq!(drafts[0]["state"]["status"], "Testing");
        assert!(drafts[0].get("settings").is_none());
        let details = server.get(&format!("/tests/{id}")).await;
        details.assert_status_ok();
        details.assert_header("cache-control", "no-store");
        let details = details.json::<serde_json::Value>();
        assert!(details["settings"]["password"].is_null());
        assert!(details["settings"]["eap_password"].is_null());
        assert_eq!(details["settings"]["ipv4"]["dns"][0], "9.9.9.9");
        server
            .post(&format!("/tests/{id}/discard"))
            .await
            .assert_status(StatusCode::BAD_REQUEST);
        assert_eq!(wait_for_test(&server, &id).await["status"], "Connected");
        server
            .post("/wifi/test")
            .json(&json!({ "id": id, "ssid": "Different" }))
            .await
            .assert_status(StatusCode::BAD_REQUEST);
        let retried = server
            .post("/wifi/test")
            .json(&json!({ "id": id, "ssid": "Home_Network" }))
            .await;
        retried.assert_status(StatusCode::ACCEPTED);
        let retry_id = retried.json::<serde_json::Value>()["test_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let drafts = server.get("/tests").await.json::<serde_json::Value>();
        assert_eq!(drafts.as_array().unwrap().len(), 1);
        assert_eq!(drafts[0]["test_id"], retry_id);
        assert_eq!(
            wait_for_test(&server, &retry_id).await["status"],
            "Connected"
        );
        server
            .post(&format!("/tests/{retry_id}/save"))
            .await
            .assert_status(StatusCode::ACCEPTED);
        assert_eq!(
            server.get("/tests").await.json::<serde_json::Value>(),
            json!([])
        );
        server
            .get(&format!("/tests/{retry_id}"))
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn discarding_a_finished_draft_removes_recovery_and_save_access() {
        let server = get_test_server();
        let started = server
            .post("/wifi/test")
            .json(&json!({ "ssid": "Home_Network", "password": "incorrect" }))
            .await;
        let id = started.json::<serde_json::Value>()["test_id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(wait_for_test(&server, &id).await["status"], "Failed");
        assert_eq!(
            server.get("/tests").await.json::<serde_json::Value>()[0]["state"]["status"],
            "Failed"
        );
        server
            .post(&format!("/tests/{id}/discard"))
            .await
            .assert_status(StatusCode::NO_CONTENT);
        assert_eq!(
            server.get("/tests").await.json::<serde_json::Value>(),
            json!([])
        );
        server
            .get(&format!("/tests/{id}"))
            .await
            .assert_status(StatusCode::NOT_FOUND);
        server
            .post(&format!("/tests/{id}/save"))
            .await
            .assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn local_only_test_reports_checks_and_can_be_saved() {
        let server = get_test_server();
        let response = server
            .post("/wifi/test")
            .json(&json!({"ssid": "CoffeeShop_Guest"}))
            .await;
        response.assert_status(axum::http::StatusCode::ACCEPTED);
        let test_id = response.json::<serde_json::Value>()["test_id"]
            .as_str()
            .unwrap()
            .to_string();
        let status_url = format!("/status/{test_id}");
        let report = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let status = server.get(&status_url).await.json::<serde_json::Value>();
                if status["status"] != "Testing" {
                    break status;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(report["status"], "Connected");
        assert_eq!(report["detail"]["gateway_reachable"], true);
        assert_eq!(report["detail"]["internet_verified"], false);
        assert_eq!(report["detail"]["endpoint_groups"], json!([]));
        server
            .post(&format!("/tests/{test_id}/save"))
            .await
            .assert_status(axum::http::StatusCode::ACCEPTED);
        assert_eq!(
            server.get(&status_url).await.json::<serde_json::Value>()["status"],
            "Saved"
        );
    }

    #[tokio::test]
    async fn test_scan_wifi() {
        let server = get_test_server();
        let response = server.get("/wifi/scan").await;
        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_connect_success() {
        let server = get_test_server();
        let response = server
            .post("/wifi/connect")
            .json(&json!({
                "ssid": "Home_Network",
                "password": "password"
            }))
            .await;

        response.assert_status(axum::http::StatusCode::ACCEPTED);

        let json_body = response.json::<serde_json::Value>();
        assert!(json_body.get("test_id").is_some());
    }

    #[tokio::test]
    async fn test_ap_preferences_roundtrip() {
        let server = get_test_server();

        let initial = server.get("/access_point/preferences").await;
        initial.assert_status_ok();
        assert_eq!(
            initial.json::<serde_json::Value>(),
            json!({ "persistent_enabled": false })
        );

        server
            .put("/access_point/preferences")
            .json(&json!({ "persistent_enabled": true }))
            .await
            .assert_status(axum::http::StatusCode::NO_CONTENT);

        let updated = server.get("/access_point/preferences").await;
        updated.assert_status_ok();
        assert_eq!(
            updated.json::<serde_json::Value>(),
            json!({ "persistent_enabled": true })
        );
    }

    #[tokio::test]
    async fn test_rejects_invalid_dns_server() {
        let server = get_test_server();
        let response = server
            .post("/ethernet/test")
            .json(&json!({
                "ipv4": {
                    "method": "auto",
                    "dns": ["aaaa"]
                }
            }))
            .await;

        response.assert_status(axum::http::StatusCode::BAD_REQUEST);
        let body = response.json::<serde_json::Value>();
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .contains("Invalid DNS server")
        );
    }

    #[tokio::test]
    async fn test_rejects_invalid_manual_ipv4() {
        let server = get_test_server();
        let response = server
            .post("/ethernet/test")
            .json(&json!({
                "ipv4": {
                    "method": "manual",
                    "address_data": [{"address": "10.0.0.5", "prefix": 44}],
                    "gateway": "10.0.0.1"
                }
            }))
            .await;

        response.assert_status(axum::http::StatusCode::BAD_REQUEST);
        let body = response.json::<serde_json::Value>();
        assert!(
            body["error"]
                .as_str()
                .unwrap_or_default()
                .contains("Invalid IPv4 prefix")
        );
    }

    #[tokio::test]
    async fn manual_ipv4_combinations_are_validated_before_testing() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../../tests/ipv4-validation.json")).unwrap();
        let server = get_test_server();
        for case in cases {
            let payload = json!({
                "ssid": "LabWiFi",
                "password": "password",
                "ipv4": {
                    "method": "manual",
                    "address_data": case["address_data"],
                    "gateway": case["gateway"],
                },
            });
            let settings = serde_json::from_value(payload.clone()).unwrap();
            let result = super::validate_network_settings(&settings);
            if let Some(error) = case["error"].as_str() {
                assert!(result.is_err(), "{}", case["name"]);
                for endpoint in ["/wifi/test", "/ethernet/test"] {
                    let response = server.post(endpoint).json(&payload).await;
                    response.assert_status(axum::http::StatusCode::BAD_REQUEST);
                    let body = response.json::<serde_json::Value>();
                    assert!(
                        body["error"].as_str().unwrap().contains(error),
                        "{}: {body}",
                        case["name"]
                    );
                }
            } else {
                assert!(result.is_ok(), "{}: {result:?}", case["name"]);
            }
        }
    }

    #[tokio::test]
    async fn test_reset_saved_networks_clears_profiles() {
        let server = get_test_server();

        let initial = server.get("/saved").await;
        initial.assert_status_ok();
        assert_eq!(
            initial
                .json::<serde_json::Value>()
                .as_array()
                .map_or(0, |v| v.len()),
            1
        );

        server
            .post("/saved/reset")
            .await
            .assert_status(axum::http::StatusCode::ACCEPTED);

        let after = server.get("/saved").await;
        after.assert_status_ok();
        assert_eq!(after.json::<serde_json::Value>(), json!([]));
    }
}
