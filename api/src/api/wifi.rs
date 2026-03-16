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
