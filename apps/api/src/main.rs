use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, net::SocketAddr};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    db: Option<PgPool>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    database: &'static str,
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "chatty-api",
        version: env!("CARGO_PKG_VERSION"),
        database: if state.db.is_some() {
            "configured"
        } else {
            "not_configured"
        },
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let db = match env::var("DATABASE_URL") {
        Ok(database_url) => {
            let pool = PgPoolOptions::new()
                .max_connections(10)
                .connect(&database_url)
                .await
                .expect("connect to PostgreSQL");
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("apply Chatty database migrations");
            tracing::info!("PostgreSQL connected and migrations applied");
            Some(pool)
        }
        Err(_) => {
            tracing::warn!("DATABASE_URL is unset; running without persistence");
            None
        }
    };

    // Permissive CORS is for local development only; configure an origin allowlist before deployment.
    let app = Router::new()
        .route("/health", get(health))
        .with_state(AppState { db })
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
    async fn health_endpoint_reports_database_configuration() {
        let response = health(State(AppState { db: None })).await;
        assert_eq!(response.0.status, "ok");
        assert_eq!(response.0.service, "chatty-api");
        assert_eq!(response.0.database, "not_configured");
    }
}
