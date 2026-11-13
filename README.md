# Chatty

Independent cross-platform community and messaging platform with a Rust backend.

## Current status
Foundation milestone: Rust API health endpoint, React web client shell, documentation and CI. Accounts, persistent messaging, voice/video, native packaging and production deployment are planned—not yet implemented.

## Stack
- API: Rust, Axum, Tokio
- Web: React, TypeScript, Vite
- Data: PostgreSQL (schema and migrations follow in the persistence milestone)
- Desktop/mobile: Tauri 2 roadmap
- Realtime: WebSockets; WebRTC for voice/video

## Run locally
API: `cd apps/api && cp .env.example .env && cargo run` (GET http://127.0.0.1:8080/health)

Web: `cd apps/web && npm install && npm run dev`

See docs/ROADMAP.md, docs/ARCHITECTURE.md and docs/SECURITY.md. A feature is only marked complete when implemented and tested.