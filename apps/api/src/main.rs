use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use futures_util::StreamExt;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{collections::HashMap, env, net::SocketAddr, sync::Arc, time::Instant};
use tokio::sync::{broadcast, Mutex};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: Option<PgPool>,
    events: broadcast::Sender<String>,
    ws_tickets: Arc<Mutex<HashMap<String, (Uuid, Instant)>>>,
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

#[derive(Deserialize)]
struct ProfileUpdateRequest {
    display_name: Option<String>,
    bio: Option<String>,
    avatar_url: Option<String>,
    banner_url: Option<String>,
    status_text: Option<String>,
}

#[derive(Serialize)]
struct ProfileResponse {
    display_name: Option<String>,
    bio: String,
    avatar_url: Option<String>,
    banner_url: Option<String>,
    status_text: Option<String>,
}

#[derive(Serialize)]
struct AccountResponse {
    id: Uuid,
    email: String,
    username: String,
    display_name: Option<String>,
    bio: String,
    avatar_url: Option<String>,
    banner_url: Option<String>,
    status_text: Option<String>,
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

fn valid_profile_url(value: &str) -> bool {
    value.is_empty()
        || (value.len() <= 2048
            && value.starts_with("https://")
            && !value.chars().any(char::is_whitespace))
}

fn valid_direct_message(content: &str) -> bool {
    let trimmed = content.trim();
    !trimmed.is_empty() && trimmed.chars().count() <= 4000
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
    PasswordHash::new(encoded).ok().is_some_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
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

async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
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
    let mut tx = db
        .begin()
        .await
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
    sqlx::query(
        "INSERT INTO sessions (account_id, refresh_token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(account.0)
    .bind(token_hash)
    .bind(expires_at)
    .execute(&mut *tx)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    tx.commit()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            expires_at,
            account: AccountResponse {
                id: account.0,
                email: account.1,
                username: account.2,
                display_name: None,
                bio: String::new(),
                avatar_url: None,
                banner_url: None,
                status_text: None,
            },
        }),
    ))
}

async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let login = input.login.trim().to_lowercase();
    let account = sqlx::query_as::<_, (Uuid, String, String, String, Option<String>, String, Option<String>, Option<String>, Option<String>)>(
        "SELECT a.id, a.email, a.username, a.password_hash, p.display_name, p.bio, p.avatar_url, p.banner_url, p.status_text FROM accounts a LEFT JOIN profiles p ON p.account_id = a.id WHERE a.email = $1 OR a.username = $1 ORDER BY CASE WHEN a.email = $1 THEN 0 ELSE 1 END LIMIT 1"
    )
    .bind(&login)
    .fetch_optional(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?
    .filter(|row| verify_password(&input.password, &row.3))
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "invalid_login"))?;

    let token = Uuid::new_v4().to_string() + &Uuid::new_v4().simple().to_string();
    let expires_at = Utc::now() + Duration::days(30);
    sqlx::query(
        "INSERT INTO sessions (account_id, refresh_token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(account.0)
    .bind(hash_token(&token))
    .bind(expires_at)
    .execute(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;

    Ok(Json(AuthResponse {
        token,
        expires_at,
        account: AccountResponse {
            id: account.0,
            email: account.1,
            username: account.2,
            display_name: account.4,
            bio: account.5,
            avatar_url: account.6,
            banner_url: account.7,
            status_text: account.8,
        },
    }))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AccountResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let token = bearer_token(&headers)
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "authentication_required"))?;
    let account = sqlx::query_as::<_, (Uuid, String, String, Option<String>, String, Option<String>, Option<String>, Option<String>)>(
        "SELECT a.id, a.email, a.username, p.display_name, p.bio, p.avatar_url, p.banner_url, p.status_text FROM sessions s JOIN accounts a ON a.id = s.account_id LEFT JOIN profiles p ON p.account_id = a.id WHERE s.refresh_token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > now()"
    )
    .bind(hash_token(token))
    .fetch_optional(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "invalid_session"))?;

    Ok(Json(AccountResponse {
        id: account.0,
        email: account.1,
        username: account.2,
        display_name: account.3,
        bio: account.4,
        avatar_url: account.5,
        banner_url: account.6,
        status_text: account.7,
    }))
}

async fn update_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<ProfileUpdateRequest>,
) -> Result<Json<ProfileResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let token = bearer_token(&headers)
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "authentication_required"))?;
    let display_name = input.display_name.map(|value| value.trim().to_owned());
    let bio = input.bio.map(|value| value.trim().to_owned());
    let avatar_url = input.avatar_url.map(|value| value.trim().to_owned());
    let banner_url = input.banner_url.map(|value| value.trim().to_owned());
    let status_text = input.status_text.map(|value| value.trim().to_owned());

    if display_name.is_none() && bio.is_none() && avatar_url.is_none()
        && banner_url.is_none() && status_text.is_none() {
        return Err(error(StatusCode::BAD_REQUEST, "profile_fields_required"));
    }
    if display_name
        .as_ref()
        .is_some_and(|value| value.chars().count() > 32)
    {
        return Err(error(StatusCode::BAD_REQUEST, "display_name_too_long"));
    }
    if bio
        .as_ref()
        .is_some_and(|value| value.chars().count() > 280)
    {
        return Err(error(StatusCode::BAD_REQUEST, "bio_too_long"));
    }
    if avatar_url.as_ref().is_some_and(|value| !valid_profile_url(value)) {
        return Err(error(StatusCode::BAD_REQUEST, "avatar_url_invalid"));
    }
    if banner_url.as_ref().is_some_and(|value| !valid_profile_url(value)) {
        return Err(error(StatusCode::BAD_REQUEST, "banner_url_invalid"));
    }
    if status_text.as_ref().is_some_and(|value| value.chars().count() > 80) {
        return Err(error(StatusCode::BAD_REQUEST, "status_text_too_long"));
    }

    let account_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT account_id FROM sessions WHERE refresh_token_hash = $1 AND revoked_at IS NULL AND expires_at > now()",
    )
    .bind(hash_token(token))
    .fetch_optional(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "invalid_session"))?;

    let profile = sqlx::query_as::<_, (Option<String>, String, Option<String>, Option<String>, Option<String>)>(
        "UPDATE profiles SET display_name = COALESCE($2, display_name), bio = COALESCE($3, bio), avatar_url = COALESCE($4, avatar_url), banner_url = COALESCE($5, banner_url), status_text = COALESCE($6, status_text), updated_at = now() WHERE account_id = $1 RETURNING display_name, bio, avatar_url, banner_url, status_text",
    )
    .bind(account_id)
    .bind(display_name)
    .bind(bio)
    .bind(avatar_url)
    .bind(banner_url)
    .bind(status_text)
    .fetch_one(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;

    Ok(Json(ProfileResponse {
        display_name: profile.0,
        bio: profile.1,
        avatar_url: profile.2,
        banner_url: profile.3,
        status_text: profile.4,
    }))
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let token = bearer_token(&headers)
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "authentication_required"))?;
    let result = sqlx::query(
        "UPDATE sessions SET revoked_at = now() WHERE refresh_token_hash = $1 AND revoked_at IS NULL",
    )
        .bind(hash_token(token))
        .execute(&db)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    if result.rows_affected() == 0 {
        return Err(error(StatusCode::UNAUTHORIZED, "invalid_session"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct FriendRequestCreate {
    username: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct FriendRequestView {
    id: Uuid,
    sender_id: Uuid,
    sender_username: String,
    recipient_id: Uuid,
    recipient_username: String,
    status: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, sqlx::FromRow)]
struct FriendView {
    id: Uuid,
    username: String,
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct DirectMessageCreate {
    content: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct DirectMessageView {
    id: Uuid,
    sender_id: Uuid,
    recipient_id: Uuid,
    content: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct DirectMessageEvent {
    event: &'static str,
    message: DirectMessageView,
}

async fn authenticated_account_id(
    db: &PgPool,
    headers: &HeaderMap,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    let token = bearer_token(headers)
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "authentication_required"))?;
    sqlx::query_scalar::<_, Uuid>(
        "SELECT account_id FROM sessions WHERE refresh_token_hash = $1 AND revoked_at IS NULL AND expires_at > now()",
    )
    .bind(hash_token(token))
    .fetch_optional(db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "invalid_session"))
}

async fn create_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<FriendRequestCreate>,
) -> Result<(StatusCode, Json<FriendRequestView>), (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let sender_id = authenticated_account_id(&db, &headers).await?;
    let username = normalize_username(&input.username);
    if !valid_username(&username) {
        return Err(error(StatusCode::BAD_REQUEST, "invalid_username"));
    }
    let recipient = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, username FROM accounts WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "user_not_found"))?;
    if recipient.0 == sender_id {
        return Err(error(StatusCode::BAD_REQUEST, "cannot_friend_yourself"));
    }

    let existing = sqlx::query_scalar::<_, String>(
        "SELECT status FROM friend_requests WHERE (sender_id = $1 AND recipient_id = $2) OR (sender_id = $2 AND recipient_id = $1) ORDER BY created_at DESC LIMIT 1",
    )
    .bind(sender_id)
    .bind(recipient.0)
    .fetch_optional(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    if existing.as_deref() == Some("accepted") {
        return Err(error(StatusCode::CONFLICT, "already_friends"));
    }
    if existing.as_deref() == Some("pending") {
        return Err(error(
            StatusCode::CONFLICT,
            "friend_request_already_pending",
        ));
    }

    let request = sqlx::query_as::<_, FriendRequestView>(
        "INSERT INTO friend_requests (sender_id, recipient_id) VALUES ($1, $2) RETURNING id, sender_id, (SELECT username FROM accounts WHERE id = sender_id) AS sender_username, recipient_id, (SELECT username FROM accounts WHERE id = recipient_id) AS recipient_username, status, created_at",
    )
    .bind(sender_id)
    .bind(recipient.0)
    .fetch_one(&db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_error) = &e {
            if db_error.code().as_deref() == Some("23505") {
                return error(StatusCode::CONFLICT, "friend_request_already_pending");
            }
        }
        error(StatusCode::INTERNAL_SERVER_ERROR, "database_error")
    })?;
    Ok((StatusCode::CREATED, Json(request)))
}

async fn list_friend_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<FriendRequestView>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let account_id = authenticated_account_id(&db, &headers).await?;
    let requests = sqlx::query_as::<_, FriendRequestView>(
        "SELECT fr.id, fr.sender_id, s.username AS sender_username, fr.recipient_id, r.username AS recipient_username, fr.status, fr.created_at FROM friend_requests fr JOIN accounts s ON s.id = fr.sender_id JOIN accounts r ON r.id = fr.recipient_id WHERE (fr.sender_id = $1 OR fr.recipient_id = $1) AND fr.status = 'pending' ORDER BY fr.created_at DESC",
    )
    .bind(account_id)
    .fetch_all(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    Ok(Json(requests))
}

async fn respond_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(request_id): Path<Uuid>,
    accept: bool,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let recipient_id = authenticated_account_id(&db, &headers).await?;
    let status = if accept { "accepted" } else { "declined" };
    let result = sqlx::query(
        "UPDATE friend_requests SET status = $1, updated_at = now() WHERE id = $2 AND recipient_id = $3 AND status = 'pending'",
    )
    .bind(status)
    .bind(request_id)
    .bind(recipient_id)
    .execute(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    if result.rows_affected() == 0 {
        return Err(error(StatusCode::NOT_FOUND, "pending_request_not_found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn accept_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    respond_friend_request(State(state), headers, Path(id), true).await
}

async fn decline_friend_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    respond_friend_request(State(state), headers, Path(id), false).await
}

async fn list_friends(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<FriendView>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let account_id = authenticated_account_id(&db, &headers).await?;
    let friends = sqlx::query_as::<_, FriendView>(
        "SELECT a.id, a.username, p.display_name FROM friend_requests fr JOIN accounts a ON a.id = CASE WHEN fr.sender_id = $1 THEN fr.recipient_id ELSE fr.sender_id END LEFT JOIN profiles p ON p.account_id = a.id WHERE (fr.sender_id = $1 OR fr.recipient_id = $1) AND fr.status = 'accepted' ORDER BY a.username",
    )
    .bind(account_id)
    .fetch_all(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    Ok(Json(friends))
}

async fn list_direct_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(peer_id): Path<Uuid>,
) -> Result<Json<Vec<DirectMessageView>>, (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let account_id = authenticated_account_id(&db, &headers).await?;
    let is_friend = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM friend_requests WHERE status = 'accepted' AND ((sender_id = $1 AND recipient_id = $2) OR (sender_id = $2 AND recipient_id = $1)))",
    )
    .bind(account_id)
    .bind(peer_id)
    .fetch_one(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    if !is_friend {
        return Err(error(
            StatusCode::FORBIDDEN,
            "direct_messages_require_friendship",
        ));
    }
    let messages = sqlx::query_as::<_, DirectMessageView>(
        "SELECT id, sender_id, recipient_id, content, created_at FROM direct_messages WHERE (sender_id = $1 AND recipient_id = $2) OR (sender_id = $2 AND recipient_id = $1) ORDER BY created_at DESC LIMIT 100",
    )
    .bind(account_id).bind(peer_id).fetch_all(&db).await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    Ok(Json(messages.into_iter().rev().collect()))
}

async fn send_direct_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(peer_id): Path<Uuid>,
    Json(input): Json<DirectMessageCreate>,
) -> Result<(StatusCode, Json<DirectMessageView>), (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .clone()
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let account_id = authenticated_account_id(&db, &headers).await?;
    if !valid_direct_message(&input.content) {
        return Err(error(StatusCode::BAD_REQUEST, "message_content_invalid"));
    }
    let content = input.content.trim().to_owned();
    if peer_id == account_id {
        return Err(error(StatusCode::BAD_REQUEST, "cannot_message_yourself"));
    }
    let is_friend = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM friend_requests WHERE status = 'accepted' AND ((sender_id = $1 AND recipient_id = $2) OR (sender_id = $2 AND recipient_id = $1)))",
    )
    .bind(account_id)
    .bind(peer_id)
    .fetch_one(&db)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    if !is_friend {
        return Err(error(
            StatusCode::FORBIDDEN,
            "direct_messages_require_friendship",
        ));
    }
    let message = sqlx::query_as::<_, DirectMessageView>(
        "INSERT INTO direct_messages (sender_id, recipient_id, content) VALUES ($1, $2, $3) RETURNING id, sender_id, recipient_id, content, created_at",
    )
    .bind(account_id).bind(peer_id).bind(content).fetch_one(&db).await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "database_error"))?;
    if let Ok(payload) = serde_json::to_string(&DirectMessageEvent {
        event: "message.created",
        message: DirectMessageView {
            id: message.id,
            sender_id: message.sender_id,
            recipient_id: message.recipient_id,
            content: message.content.clone(),
            created_at: message.created_at,
        },
    }) {
        let _ = state.events.send(payload);
    }
    Ok((StatusCode::CREATED, Json(message)))
}

#[derive(Serialize)]
struct WsTicketResponse {
    ticket: String,
}

#[derive(Deserialize)]
struct WsTicketQuery {
    ticket: String,
}

async fn create_ws_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<WsTicketResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db = state
        .db
        .as_ref()
        .ok_or_else(|| error(StatusCode::SERVICE_UNAVAILABLE, "database_unavailable"))?;
    let account_id = authenticated_account_id(db, &headers).await?;
    let ticket = Uuid::new_v4().to_string() + &Uuid::new_v4().simple().to_string();
    let mut tickets = state.ws_tickets.lock().await;
    tickets.retain(|_, (_, expires)| *expires > Instant::now());
    tickets.insert(
        ticket.clone(),
        (
            account_id,
            Instant::now() + std::time::Duration::from_secs(60),
        ),
    );
    Ok(Json(WsTicketResponse { ticket }))
}

async fn websocket_handler(
    State(state): State<AppState>,
    Query(query): Query<WsTicketQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let account_id = {
        let mut tickets = state.ws_tickets.lock().await;
        tickets.retain(|_, (_, expires)| *expires > Instant::now());
        tickets
            .remove(&query.ticket)
            .map(|(account_id, _)| account_id)
    }
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "invalid_or_expired_ws_ticket"))?;
    let receiver = state.events.subscribe();
    Ok(ws.on_upgrade(move |socket| websocket_connection(socket, receiver, account_id)))
}

async fn websocket_connection(
    mut socket: WebSocket,
    mut receiver: broadcast::Receiver<String>,
    account_id: Uuid,
) {
    loop {
        tokio::select! {
            event = receiver.recv() => match event {
                Ok(payload) => {
                    let visible = serde_json::from_str::<serde_json::Value>(&payload).ok()
                        .and_then(|value| {
                            let message = value.get("message")?;
                            let sender = message.get("sender_id")?.as_str()?;
                            let recipient = message.get("recipient_id")?.as_str()?;
                            Some(sender == account_id.to_string() || recipient == account_id.to_string())
                        }).unwrap_or(false);
                    if visible && socket.send(Message::Text(payload)).await.is_err() { break; }
                },
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            incoming = socket.next() => match incoming {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(_)) => break,
                _ => {}
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
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
    let (events, _) = broadcast::channel(512);
    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
        .route("/auth/profile", axum::routing::patch(update_profile))
        .route("/auth/logout", post(logout))
        .route("/friends", get(list_friends))
        .route(
            "/friends/requests",
            get(list_friend_requests).post(create_friend_request),
        )
        .route("/friends/requests/:id/accept", post(accept_friend_request))
        .route(
            "/friends/requests/:id/decline",
            post(decline_friend_request),
        )
        .route(
            "/dm/:peer_id/messages",
            get(list_direct_messages).post(send_direct_message),
        )
        .route("/auth/ws-ticket", post(create_ws_ticket))
        .route("/ws", get(websocket_handler))
        .with_state(AppState {
            db,
            events,
            ws_tickets: Arc::new(Mutex::new(HashMap::new())),
        })
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let host = env::var("CHATTY_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = env::var("CHATTY_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .expect("valid listen address");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind API listener");
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
