# Chatty

Independent cross-platform community and messaging platform with a Rust backend.

## Current status
Foundation milestone: Rust API health endpoint, React web client shell, PostgreSQL connection and initial identity/social migrations, documentation and CI. Registration, login, email verification, profile APIs, friend-request APIs and persistent messaging are not implemented yet. Voice/video, native packaging and production deployment remain planned.

## Stack
- API: Rust, Axum, Tokio, SQLx
- Data: PostgreSQL with versioned SQLx migrations
- Web: React, TypeScript, Vite
- Desktop/mobile: Tauri 2 roadmap
- Realtime: WebSockets; WebRTC for voice/video

## Run locally
Start PostgreSQL: `docker compose -f infra/docker-compose.yml up -d`

API: `cd apps/api && cp .env.example .env && cargo run` (GET http://127.0.0.1:8080/health). When `DATABASE_URL` is set, the API connects to PostgreSQL and applies pending migrations at startup. Without it, the API runs in non-persistent preview mode.

Web: `cd apps/web && npm install && npm run dev`

The initial migration creates accounts, profiles, sessions and friend-request tables. It does not expose account/authentication endpoints yet. See docs/ROADMAP.md, docs/ARCHITECTURE.md and docs/SECURITY.md. A feature is only marked complete when implemented and tested.
