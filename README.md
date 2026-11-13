# Chatty

Independent cross-platform community and messaging platform with a Rust backend.

## Current status
The development branch includes a Rust/Axum API, responsive React web shell, PostgreSQL persistence foundation, and initial account registration/login endpoints. Registration, login, session lookup and logout require PostgreSQL. Messages and channels in the web client are still local preview data, not shared or persistent.

## Stack
- API: Rust, Axum, Tokio, SQLx
- Data: PostgreSQL with versioned SQLx migrations
- Passwords: Argon2id password hashing
- Web: React, TypeScript, Vite
- Desktop/mobile: Tauri 2 roadmap
- Realtime: WebSockets roadmap

## Run locally
1. Start PostgreSQL: `docker compose -f infra/docker-compose.yml up -d`
2. Configure the API: `cd apps/api && cp .env.example .env`
3. Start the API: `cargo run` (from `apps/api`; GET http://127.0.0.1:8080/health).
4. Start the web app: `cd apps/web && npm install && npm run dev`.

The API loads `.env`, connects to PostgreSQL and applies pending migrations at startup. The web client defaults to `http://127.0.0.1:8080`; set `VITE_API_BASE_URL` to override it.

## Account endpoints
- `POST /auth/register` — email, username and password (minimum 10 characters); creates account/profile/session.
- `POST /auth/login` — email or username and password.
- `GET /auth/me` — current account; requires `Authorization: Bearer <token>`.
- `POST /auth/logout` — revokes current session; requires the same bearer header.

Session tokens are returned once in the authentication response; only SHA-256 token hashes are stored. Passwords are hashed with Argon2id. The web preview keeps its session token in memory and signs out on reload. Email verification, password recovery, refresh-token rotation, rate limiting, production cookie configuration and persistent messaging are not implemented. The current permissive CORS configuration is development-only. Do not deploy this API publicly.

See docs/ROADMAP.md, docs/ARCHITECTURE.md and docs/SECURITY.md. A feature is only marked complete when implemented and tested.
