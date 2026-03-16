mod api;
pub mod dns;
mod network;

use axum::Router;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, UdpSocket};
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::network::AppState;
use crate::dns::CaptivePortalDns;
use crate::network::NetworkController;

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

    // Start DNS Server in background
    tokio::spawn(async move {
        let port: u16 = std::env::var("DNS_PORT")
            .unwrap_or_else(|_| "10053".to_string())
            .parse()
            .unwrap_or(10053);
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let handler = CaptivePortalDns::new(Ipv4Addr::new(10, 42, 0, 1));
        let mut server = hickory_server::ServerFuture::new(handler);

        match UdpSocket::bind(addr).await {
            Ok(udp_socket) => {
                server.register_socket(udp_socket);
                info!("DNS Captive Portal listening on UDP {}", addr);
            }
            Err(e) => {
                error!(
                    "Failed to bind DNS UDP socket on {}: {}. Are you running as root?",
                    addr, e
                );
                return;
            }
        }

        if let Err(e) = server.block_until_done().await {
            error!("DNS server error: {}", e);
        }
    });

    // Initialize network controller based on env var
    let network_controller: Arc<dyn NetworkController> =
        if std::env::var("MOCK_NETWORK").unwrap_or_default() == "1" {
            info!("Using Mock Network Controller");
            Arc::new(crate::network::mock::MockNetworkController::new())
        } else {
            info!("Using Nmcli Network Controller");
            Arc::new(crate::network::nmcli::NmcliNetworkController::new())
        };

    let state = AppState {
        network: network_controller,
    };

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "web/build".into());
    let index_file = format!("{}/index.html", static_dir);

    info!("Serving static files from {}", static_dir);
    let serve_dir = ServeDir::new(&static_dir).not_found_service(ServeFile::new(&index_file));

    // Build our application with routes
    let app = Router::new()
        .nest("/api/network", api::network::router())
        .fallback_service(serve_dir)
        .with_state(state);

    // Run our app with hyper, listening globally on port 3000
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("API Server Listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
