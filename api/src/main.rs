mod api;
mod branding;
pub mod dns;
mod network;

use axum::Router;
use axum::body::Body;
#[cfg(not(debug_assertions))]
use axum::http::HeaderValue;
#[cfg(not(debug_assertions))]
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, UdpSocket};
use tower::util::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::network::AppState;
use crate::branding::{branding_assets_dir, branding_config_path, load_branding_config};
use crate::dns::CaptivePortalDns;
use crate::network::NetworkController;

#[cfg(not(debug_assertions))]
#[derive(rust_embed::RustEmbed)]
#[folder = "../web/build"]
struct EmbeddedAssets;

fn static_dir_override() -> Option<String> {
    std::env::var("STATIC_DIR")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

async fn serve_from_dir(uri: Uri, static_dir: &str) -> Response {
    let index_file = format!("{}/index.html", static_dir);
    let service = ServeDir::new(static_dir).not_found_service(ServeFile::new(index_file));
    let request = Request::builder()
        .uri(uri)
        .body(Body::empty())
        .unwrap_or_else(|_| Request::new(Body::empty()));
    match service.oneshot(request).await {
        Ok(res) => res.map(Body::new),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to serve static content",
        )
            .into_response(),
    }
}

#[cfg(not(debug_assertions))]
fn embedded_asset_response(path: &str) -> Response {
    let requested = path.trim_start_matches('/');
    let requested = if requested.is_empty() {
        "index.html"
    } else {
        requested
    };

    let file = EmbeddedAssets::get(requested).or_else(|| EmbeddedAssets::get("index.html"));
    match file {
        Some(content) => {
            let served_path = if EmbeddedAssets::get(requested).is_some() {
                requested
            } else {
                "index.html"
            };
            let mime = mime_guess::from_path(served_path).first_or_octet_stream();
            let mut res = Response::new(Body::from(content.data.into_owned()));
            res.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_str(mime.essence_str())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            res
        }
        None => (StatusCode::NOT_FOUND, "Asset not found").into_response(),
    }
}

async fn static_fallback(uri: Uri) -> Response {
    if let Some(static_dir) = static_dir_override() {
        return serve_from_dir(uri, &static_dir).await;
    }

    #[cfg(debug_assertions)]
    {
        serve_from_dir(uri, "web/build").await
    }

    #[cfg(not(debug_assertions))]
    {
        embedded_asset_response(uri.path())
    }
}

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

    info!("Starting reshape-netsetup API...");

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

    let branding_config_path = branding_config_path();
    let branding_assets_dir = branding_assets_dir(&branding_config_path);

    let state = AppState {
        network: network_controller.clone(),
        branding: load_branding_config(),
    };

    if let Err(e) = network_controller.start_ap_supervisor().await {
        error!("Failed to start AP supervisor: {}", e);
    }

    if let Some(static_dir) = static_dir_override() {
        info!("Serving static files from {}", static_dir);
    } else if cfg!(debug_assertions) {
        info!("Serving static files from web/build (debug fallback)");
    } else {
        info!("Serving embedded static files from binary");
    }
    info!("Serving branding assets from {}", branding_assets_dir);

    // Build our application with routes
    let favicon_file = format!("{}/favicon.ico", branding_assets_dir);
    let app = Router::new()
        .nest("/api", api::router())
        .nest_service("/branding", ServeDir::new(branding_assets_dir))
        .route_service("/favicon.ico", ServeFile::new(favicon_file))
        .fallback(static_fallback)
        .with_state(state);

    // Run our app with hyper, listening globally on port 3000
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("API Server Listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
