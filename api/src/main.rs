mod api;
mod network;

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::wifi::AppState;
use crate::network::mock::MockNetworkController;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting machine-setup API...");

    // Initialize our mocked network controller
    let network_controller = Arc::new(MockNetworkController::new());

    let state = AppState {
        network: network_controller,
    };

    // Build our application with routes
    let app = Router::new()
        .nest("/api/wifi", api::wifi::router())
        .with_state(state);

    // Run our app with hyper, listening globally on port 3000
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("Listening on {}", addr);
    
    axum::serve(listener, app).await.unwrap();
}
