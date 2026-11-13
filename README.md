# Chatty

Chatty is a cross-platform community and messaging app in development. The backend is Rust (Axum + SQLx), the web client is React + TypeScript, and PostgreSQL is the source of truth. Desktop and mobile clients are planned; this repository currently contains the API foundation and web preview.

## Current implementation

- PostgreSQL migrations for accounts, profiles, sessions and friend requests.
- Account registration with email/username validation and Argon2id password hashing.
- Login using email or username.
- Random bearer sessions; only SHA-256 token hashes are stored.
- Authenticated account lookup and logout/session revocation.
- Authenticated profile editing for display name and short bio.
- React registration/login screens and account settings.
- GitHub Actions checks for Rust formatting, Clippy, Rust tests and the web production build.

## Run locally

Requirements: Rust stable, Node.js 22+, npm and PostgreSQL.

1. Start PostgreSQL (a local option is provided in `infra/docker-compose.yml`).
2. Copy `apps/api/.env.example` to `.env` and set `DATABASE_URL`.
3. Start the API:

```sh
cargo run -p chatty-api
```

4. In another terminal, start the web client:

```sh
cd apps/web
npm install
npm run dev
```

The API defaults to `127.0.0.1:8080`. The web client uses that address unless `VITE_API_BASE_URL` is configured.

## API routes

| Method | Route | Authentication | Purpose |
|---|---|---|---|
| GET | `/health` | No | API health and database configuration |
| POST | `/auth/register` | No | Create account and session |
| POST | `/auth/login` | No | Sign in with email or username |
| GET | `/auth/me` | Bearer | Get the signed-in account |
| PATCH | `/auth/profile` | Bearer | Update display name and bio |
| POST | `/auth/logout` | Bearer | Revoke current session |

Registration expects `email`, `username` and `password`. Usernames are 3–24 characters and use letters, numbers or underscores. Passwords must be 10–128 characters. Profile display names are limited to 32 characters and bios to 280 characters.

## Important limitations

This is an early development build, not a production-ready chat service. Messages are still local demo data; friend requests, servers, channels, permissions and real-time messaging are not yet connected. Email verification, password recovery, session refresh/rotation, rate limiting, secure browser cookies, production CORS configuration, moderation and media handling remain future work. Voice/video, bots, native Tauri packaging and signed automatic updates remain on the roadmap.

The browser currently keeps the bearer token in memory, so a page reload signs the user out. Do not deploy the API publicly: the current permissive CORS configuration is intended only for local development. See [Security](docs/SECURITY.md), [Architecture](docs/ARCHITECTURE.md) and [Roadmap](docs/ROADMAP.md).
