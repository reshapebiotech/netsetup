pub mod network;

use crate::api::network::AppState;
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new().nest("/network", network::router())
}
