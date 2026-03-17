pub mod branding;
pub mod network;

use axum::{Router, routing::get};

use crate::api::network::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/network", network::router())
        .route("/branding", get(branding::get_branding))
}
