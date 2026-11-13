# Chatty architecture

## Clients
React + TypeScript web client. Tauri 2 is the planned desktop shell, followed by Android/iOS packaging after desktop and mobile UX validation.

## Backend boundaries
The Rust API is authoritative for identity, membership, permissions, messages and moderation. Planned modules: identity, servers, channels, messaging, realtime gateway, media, moderation, notifications and release metadata. Clients never enforce access control alone.

## Data and realtime
PostgreSQL is the source of truth. SQLx migrations define accounts, profiles, refresh-session records (token hashes only) and friend requests. The API applies pending migrations at startup when DATABASE_URL is configured. The current API supports account registration/login, session lookup/revocation and authenticated profile edits. WebSocket events will notify connected clients after successful transactions. Redis may be introduced for distributed presence, rate limits and fan-out. Voice/video uses WebRTC with an SFU for group calls.

## Security
Passwords use Argon2id and session tokens are stored only as SHA-256 hashes. The browser currently holds the bearer token in memory. Email verification, recovery, refresh-token rotation, rate limits, hardened cookies, strict CORS and production deployment controls are not yet implemented. The current permissive CORS layer is local-development-only and must be replaced with an origin allowlist before deployment.
