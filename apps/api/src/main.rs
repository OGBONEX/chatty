use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::{env, net::SocketAddr};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "chatty-api",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Permissive CORS is for local development only; configure an origin allowlist before deployment.
    let app = Router::new()
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let host = env::var("CHATTY_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = env::var("CHATTY_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let addr: SocketAddr = format!("{host}:{port}").parse().expect("valid listen address");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind API listener");
    tracing::info!(%addr, "Chatty API listening");
    axum::serve(listener, app).await.expect("serve API");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_endpoint_returns_service_identity() {
        let response = health().await;
        assert_eq!(response.0.status, "ok");
        assert_eq!(response.0.service, "chatty-api");
    }
}
