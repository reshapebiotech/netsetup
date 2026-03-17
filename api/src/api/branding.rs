use axum::{Json, extract::State};

use crate::api::network::AppState;
use crate::branding::BrandingConfig;

pub async fn get_branding(State(state): State<AppState>) -> Json<BrandingConfig> {
    Json(state.branding.clone())
}

#[cfg(test)]
mod tests {
    use super::get_branding;
    use crate::api::network::AppState;
    use crate::branding::BrandingConfig;
    use crate::network::mock::MockNetworkController;
    use axum::{Router, routing::get};
    use axum_test::TestServer;
    use std::sync::Arc;

    #[tokio::test]
    async fn branding_endpoint_returns_state_values() {
        let state = AppState {
            network: Arc::new(MockNetworkController::new()),
            branding: BrandingConfig {
                product_name: "Reshape Netsetup".into(),
                ..BrandingConfig::default()
            },
        };
        let app = Router::new()
            .route("/branding", get(get_branding))
            .with_state(state);
        let server = TestServer::new(app);

        let response = server.get("/branding").await;
        response.assert_status_ok();
        let payload = response.json::<serde_json::Value>();
        assert_eq!(payload["product_name"], "Reshape Netsetup");
    }
}
