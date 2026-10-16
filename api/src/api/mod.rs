pub mod network;

use crate::api::network::AppState;
use crate::network::NetworkError;
use axum::{Json, Router, routing::get};
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
struct DeviceInfo {
    hostname: String,
}

async fn device_info() -> Result<Json<DeviceInfo>, NetworkError> {
    let output = tokio::time::timeout(
        Duration::from_secs(2),
        tokio::process::Command::new("hostname")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| NetworkError::Internal("Could not read device hostname".into()))?
    .map_err(|_| NetworkError::Internal("Could not read device hostname".into()))?;
    let hostname = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || hostname.is_empty() {
        return Err(NetworkError::Internal(
            "Could not read device hostname".into(),
        ));
    }
    Ok(Json(DeviceInfo { hostname }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/device", get(device_info))
        .nest("/network", network::router())
}
