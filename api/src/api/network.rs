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
    ConnectionState, AccessPointStatus, Connectivity, EthernetNetwork, NetworkSettings, SavedNetwork, WifiNetwork,
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
        .route("/wifi/connect", post(connect_wifi))
        .route("/ethernet/connect", post(connect_ethernet))
        .route("/status/{test_id}", get(status))
        .route("/connectivity", get(get_connectivity)).route("/access_point", get(get_hotspot_status))
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

async fn connect_wifi(
    State(state): State<AppState>,
    Json(settings): Json<NetworkSettings>,
) -> Result<(StatusCode, Json<ConnectResponse>), NetworkError> {
    let test_id = state.network.connect_wifi(settings).await?;
    Ok((StatusCode::ACCEPTED, Json(ConnectResponse { test_id })))
}

async fn connect_ethernet(
    State(state): State<AppState>,
    Json(settings): Json<NetworkSettings>,
) -> Result<(StatusCode, Json<ConnectResponse>), NetworkError> {
    let test_id = state.network.connect_ethernet(settings).await?;
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

#[cfg(test)]
mod tests {
    use crate::api::network::{router, AppState};
    use crate::network::mock::MockNetworkController;
    use crate::network::types::{IpMethod, NetworkSettings};
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
}
