use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::network::types::{
    AccessPointControllerStatus, AccessPointPreferences, AccessPointStatus, ConnectionState,
    Connectivity, EthernetNetwork, NetworkMacAddresses, NetworkSettings, SavedNetwork, WifiNetwork,
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
        .route("/saved/{id}", delete(forget_saved))
        .route("/wifi/test", post(test_wifi))
        .route("/ethernet/test", post(test_ethernet))
        .route("/tests/{test_id}/save", post(save_test))
        .route("/tests/{test_id}/discard", post(discard_test))
        .route("/wifi/connect", post(connect_wifi_compat))
        .route("/ethernet/connect", post(connect_ethernet_compat))
        .route("/status/{test_id}", get(status))
        .route("/connectivity", get(get_connectivity))
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

async fn forget_saved(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, NetworkError> {
    state.network.forget_network(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub test_id: Uuid,
}

async fn test_wifi(
    State(state): State<AppState>,
    Json(settings): Json<NetworkSettings>,
) -> Result<(StatusCode, Json<ConnectResponse>), NetworkError> {
    let test_id = state.network.start_wifi_test(settings).await?;
    Ok((StatusCode::ACCEPTED, Json(ConnectResponse { test_id })))
}

async fn test_ethernet(
    State(state): State<AppState>,
    Json(settings): Json<NetworkSettings>,
) -> Result<(StatusCode, Json<ConnectResponse>), NetworkError> {
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
    let test_id = state.network.start_wifi_test(settings).await?;
    Ok((StatusCode::ACCEPTED, Json(ConnectResponse { test_id })))
}

async fn connect_ethernet_compat(
    State(state): State<AppState>,
    Json(settings): Json<NetworkSettings>,
) -> Result<(StatusCode, Json<ConnectResponse>), NetworkError> {
    let test_id = state.network.start_ethernet_test(settings).await?;
    Ok((StatusCode::ACCEPTED, Json(ConnectResponse { test_id })))
}

async fn status(
    State(state): State<AppState>,
    Path(test_id): Path<Uuid>,
) -> Result<Json<ConnectionState>, NetworkError> {
    let current_status = state.network.connection_status(test_id).await?;
    Ok(Json(current_status))
}

async fn get_hotspot_status(
    State(state): State<AppState>,
) -> Result<Json<AccessPointStatus>, NetworkError> {
    let status = state.network.get_hotspot_status().await?;
    Ok(Json(status))
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
}
