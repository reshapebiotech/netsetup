use axum::{
    Router,
    extract::OriginalUri,
    http::{StatusCode, header},
    routing::get,
};

const PROBE_PATHS: &[&str] = &[
    "/hotspot-detect.html",
    "/library/test/success.html",
    "/generate_204",
    "/gen_204",
    "/connecttest.txt",
    "/ncsi.txt",
    "/redirect",
    "/canonical.html",
    "/success.txt",
    "/check_network_status.txt",
];

pub fn router<S: Clone + Send + Sync + 'static>(portal_url: String) -> Router<S> {
    let redirect = get(move |OriginalUri(uri): OriginalUri| {
        let portal_url = portal_url.clone();
        async move {
            tracing::info!(path = uri.path(), "Redirecting captive portal probe");
            (
                StatusCode::FOUND,
                [
                    (header::LOCATION, portal_url),
                    (header::CACHE_CONTROL, "no-store".to_owned()),
                ],
            )
        }
    });
    PROBE_PATHS.iter().fold(Router::new(), |router, path| {
        router.route(path, redirect.clone())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn operating_system_probes_redirect_to_the_setup_root() {
        let server = TestServer::new(router::<()>("http://10.42.0.1/".into()));
        for path in PROBE_PATHS {
            let response = server.get(path).await;
            response.assert_status(StatusCode::FOUND);
            response.assert_header(header::LOCATION, "http://10.42.0.1/");
            response.assert_header(header::CACHE_CONTROL, "no-store");
        }
        server
            .get("/setup/new/wifi")
            .await
            .assert_status_not_found();
        server.get("/_app/test.js").await.assert_status_not_found();
    }
}
