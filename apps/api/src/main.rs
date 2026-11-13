use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, net::SocketAddr};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

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

#[derive(Deserialize)]
struct RegisterRequest {
    email: String,
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    login: String,
    password: String,
}

#[derive(Serialize)]
struct AccountResponse {
    id: Uuid,
    email: String,
    username: String,
    display_name: Option<String>,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
    expires_at: chrono::DateTime<Utc>,
    account: AccountResponse,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

fn error(status: StatusCode, message: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (status, Json(ErrorResponse { error: message }))
}

fn normalize_email(value: &str) -> String {
    value.trim().to_lowercase()
}

fn normalize_username(value: &str) -> String {
    value.trim().to_lowercase()
}

fn valid_email(value: &str) -> bool {
    value.len() <= 254
        && value.matches('@').count() == 1
        && value.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && !local.starts_with('.')
                && !local.ends_with('.')
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !value.chars().any(char::is_whitespace)
        })
}

fn valid_username(value: &str) -> bool {
    (3..=24).contains(&value.len())
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded)
        .ok()
        .is_some_and(|hash| Argon2::default().verify_password(password.as_bytes(), &hash).is_ok())
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "chatty-api",
        version: env!("CARGO_PKG_VERSION"),
        database: if state.db.is_some() { "configured" } else { "not_configured" },
    })
}

async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let email = normalize_email(&input.email);
    let username = normalize_username(&input.username);

    if !valid_email(&email) {
        return Err(error(StatusCode::BAD_REQUEST, "invalid_email"));
    }
    if !valid_username(&username) {
        return Err(error(StatusCode::BAD_REQUEST, "invalid_username"));
    }
    if input.password.len() < 10 || input.password.len() > 128 {
        return Err(error(StatusCode::BAD_REQUEST, "password_length_invalid"));
    }

    let password_hash = hash_password(&input.password)
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "password_hash_failed"))?;
    let token = Uuid::new_v4().to_string() + &Uuid::new_v4().simple().to_string();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::days(30);
    let mut tx = db.begin().await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;

    let account = sqlx::query_as::<_, (Uuid, String, String)>(
        "INSERT INTO accounts (email, username, password_hash) VALUES ($1, $2, $3) RETURNING id, email, username"
    )
    .bind(&email)
    .bind(&username)
    .bind(password_hash)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_error) = &e {
            if db_error.code().as_deref() == Some("23505") {
                return error(StatusCode::CONFLICT, "email_or_username_taken");
            }
        }
        error(StatusCode::INTERNAL_SERVER_ERROR, "database_error")
    })?;

    sqlx::query("INSERT INTO profiles (account_id) VALUES ($1)")
        .bind(account.0)
        .execute(&mut *tx)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    sqlx::query("INSERT INTO sessions (account_id, refresh_token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(account.0)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    tx.commit().await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;

    Ok((StatusCode::CREATED, Json(AuthResponse {
        token,
        expires_at,
        account: AccountResponse { id: account.0, email: account.1, username: account.2, display_name: None },
    })))
}

async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let login = input.login.trim().to_lowercase();
    let account = sqlx::query_as::<_, (Uuid, String, String, String, Option<String>)>(
        "SELECT a.id, a.email, a.username, a.password_hash, p.display_name FROM accounts a LEFT JOIN profiles p ON p.account_id = a.id WHERE a.email = $1 OR a.username = $1 ORDER BY CASE WHEN a.email = $1 THEN 0 ELSE 1 END LIMIT 1"
    )
    .bind(&login)
    .fetch_optional(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?
    .filter(|row| verify_password(&input.password, &row.3))
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "invalid_login"))?;

    let token = Uuid::new_v4().to_string() + &Uuid::new_v4().simple().to_string();
    let expires_at = Utc::now() + Duration::days(30);
    sqlx::query("INSERT INTO sessions (account_id, refresh_token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(account.0)
        .bind(hash_token(&token))
        .bind(expires_at)
        .execute(&db)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;

    Ok(Json(AuthResponse {
        token,
        expires_at,
        account: AccountResponse { id: account.0, email: account.1, username: account.2, display_name: account.4 },
    }))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers.get(AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ")
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AccountResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let token = bearer_token(&headers).ok_or_else(|| error(StatusCode::UNAUTHORIZED, "authentication_required"))?;
    let account = sqlx::query_as::<_, (Uuid, String, String, Option<String>)>(
        "SELECT a.id, a.email, a.username, p.display_name FROM sessions s JOIN accounts a ON a.id = s.account_id LEFT JOIN profiles p ON p.account_id = a.id WHERE s.refresh_token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > now()"
    )
    .bind(hash_token(token))
    .fetch_optional(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "invalid_session"))?;

    Ok(Json(AccountResponse { id: account.0, email: account.1, username: account.2, display_name: account.3 }))
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let db = state.db.ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let token = bearer_token(&headers).ok_or_else(|| error(StatusCode::UNAUTHORIZED, "authentication_required"))?;
    let result = sqlx::query("UPDATE sessions SET revoked_at = now() WHERE refresh_token_hash = $1 AND revoked_at IS NULL")
        .bind(hash_token(token))
        .execute(&db)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    if result.rows_affected() == 0 {
        return Err(error(StatusCode::UNAUTHORIZED, "invalid_session"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let db = match env::var("DATABASE_URL") {
        Ok(database_url) => {
            let pool = PgPoolOptions::new()
                .max_connections(10)
                .connect(&database_url)
                .await
                .expect("connect to PostgreSQL");
            sqlx::migrate!("./migrations").run(&pool).await.expect("apply Chatty database migrations");
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
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
        .route("/auth/logout", post(logout))
        .with_state(AppState { db })
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let host = env::var("CHATTY_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = env::var("CHATTY_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8080);
    let addr: SocketAddr = format!("{host}:{port}").parse().expect("valid listen address");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind API listener");
    tracing::info!(%addr, "Chatty API listening");
    axum::serve(listener, app).await.expect("serve API");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_usernames_and_normalizes_email() {
        assert!(valid_username("chatty_user9"));
        assert!(!valid_username("ab"));
        assert!(!valid_username("bad name"));
        assert_eq!(normalize_email(" Test@Example.COM "), "test@example.com");
        assert!(valid_email("test@example.com"));
        assert!(!valid_email("not-an-email"));
    }

    #[test]
    fn password_hashes_are_verified_without_storing_plaintext() {
        let encoded = hash_password("a-long-test-password").unwrap();
        assert_ne!(encoded, "a-long-test-password");
        assert!(verify_password("a-long-test-password", &encoded));
        assert!(!verify_password("wrong-password", &encoded));
    }

    #[test]
    fn bearer_token_requires_expected_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer secret-token".parse().unwrap());
        assert_eq!(bearer_token(&headers), Some("secret-token"));
        headers.insert(AUTHORIZATION, "Basic secret-token".parse().unwrap());
        assert_eq!(bearer_token(&headers), None);
    }
}
