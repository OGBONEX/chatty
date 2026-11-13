# Chatty architecture

## Clients
React + TypeScript web client. Tauri 2 is the planned desktop shell, followed by Android/iOS packaging after desktop and mobile UX validation.

## Backend boundaries
The Rust API is authoritative for identity, membership, permissions, messages and moderation. Planned modules: identity, servers, channels, messaging, realtime gateway, media, moderation, notifications and release metadata. Clients never enforce access control alone.

## Data and realtime
PostgreSQL is the source of truth. SQLx migrations currently define accounts, profiles, refresh-session records (token hashes only) and friend requests. The API applies pending migrations at startup when DATABASE_URL is configured. WebSocket events will notify connected clients after successful transactions. Redis may be introduced for distributed presence, rate limits and fan-out. Voice/video uses WebRTC with an SFU for group calls.

## Security
Use Argon2id password hashing, short-lived access tokens, rotating refresh sessions, email verification, optional TOTP/passkeys, strict authorization, rate limits, upload scanning and signed update manifests. The current permissive CORS layer is local-development-only and must be replaced with an origin allowlist before deployment. Authentication and account endpoints are not implemented yet.
