use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::network::types::{ConnectionState, SavedNetwork, WifiNetwork};
use crate::network::{NetworkController, NetworkError};

#[derive(Clone)]
pub struct AppState {
    pub network: Arc<dyn NetworkController>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/scan", get(scan))
        .route("/saved", get(get_saved))
        .route("/saved/{id}", delete(forget_saved))
        .route("/connect", post(connect))
        .route("/status/{test_id}", get(status))
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

async fn scan(State(state): State<AppState>) -> Result<Json<Vec<WifiNetwork>>, NetworkError> {
    let networks = state.network.scan_wifi().await?;
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

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub ssid: String,
    pub password: Option<String>,
}

#[derive(Serialize)]
pub struct ConnectResponse {
    pub test_id: Uuid,
}

async fn connect(
    State(state): State<AppState>,
    Json(payload): Json<ConnectRequest>,
) -> Result<(StatusCode, Json<ConnectResponse>), NetworkError> {
    let pwd = payload.password.unwrap_or_default();
    let test_id = state.network.connect(&payload.ssid, &pwd).await?;

    Ok((StatusCode::ACCEPTED, Json(ConnectResponse { test_id })))
}

async fn status(
    State(state): State<AppState>,
    Path(test_id): Path<Uuid>,
) -> Result<Json<ConnectionState>, NetworkError> {
    let current_status = state.network.connection_status(test_id).await?;
    Ok(Json(current_status))
}
#[cfg(test)]
mod tests {
    use crate::api::wifi::{router, AppState};
    use crate::network::mock::MockNetworkController;
    use axum_test::TestServer;
    use std::sync::Arc;
    use serde_json::json;

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
        let response = server.get("/scan").await;
        
        response.assert_status_ok();
        response.assert_json(&json!([
            {
                "ssid": "Home_Network",
                "signal_strength": 85,
                "security": "WPA2"
            },
            {
                "ssid": "CoffeeShop_Guest",
                "signal_strength": 60,
                "security": "None"
            },
            {
                "ssid": "Neighbor_5G",
                "signal_strength": 30,
                "security": "WPA3"
            }
        ]));
    }

    #[tokio::test]
    async fn test_get_saved_networks() {
        let server = get_test_server();
        let response = server.get("/saved").await;
        
        response.assert_status_ok();
        response.assert_json(&json!([
            {
                "id": "managed-wifi-1",
                "ssid": "Home_Network"
            }
        ]));
    }

    #[tokio::test]
    async fn test_connect_success() {
        let server = get_test_server();
        let response = server.post("/connect")
            .json(&json!({
                "ssid": "Home_Network",
                "password": "password"
            }))
            .await;
            
        response.assert_status(axum::http::StatusCode::ACCEPTED); // ACCEPTED
        
        let json_body = response.json::<serde_json::Value>();
        assert!(json_body.get("test_id").is_some());
        
        let test_id = json_body.get("test_id").unwrap().as_str().unwrap();
        
        // Let's immediately check status, should be "Testing" or "Connected" depending on timing
        // The mock sleeps for 3 seconds, so it will be "Testing"
        let status_res = server.get(&format!("/status/{}", test_id)).await;
        status_res.assert_status_ok();
        status_res.assert_json(&json!({
            "status": "Testing"
        }));
    }

    #[tokio::test]
    async fn test_connect_failure() {
        let server = get_test_server();
        let response = server.post("/connect")
            .json(&json!({
                "ssid": "Home_Network",
                "password": "wrongpassword"
            }))
            .await;
            
        response.assert_status(axum::http::StatusCode::ACCEPTED);
        
        let json_body = response.json::<serde_json::Value>();
        let test_id = json_body.get("test_id").unwrap().as_str().unwrap();
        
        // Wait for mock's 3-second sleep to finish so we see the failure
        tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
        
        let status_res = server.get(&format!("/status/{}", test_id)).await;
        status_res.assert_status_ok();
        status_res.assert_json(&json!({
            "status": "Failed",
            "detail": "Incorrect password or network out of range"
        }));
    }
}
