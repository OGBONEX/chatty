# Security baseline

- Passwords are hashed with Argon2id; plaintext passwords are never stored.
- Session bearer tokens are generated randomly and only SHA-256 hashes are stored in the database.
- Logout revokes the current session; expired and revoked sessions cannot access `/auth/me`.
- Never commit credentials, signing keys, database URLs or production secrets.
- Use HTTPS and secure HttpOnly SameSite cookies for browser sessions before production.
- Add refresh-token rotation, account recovery, email verification and rate limiting before public launch.
- Enforce authorization server-side for every server, channel, message and file operation.
- Bound and validate all user input and uploads; scan uploaded files.
- Escape user content and never render arbitrary HTML.
- Replace permissive development CORS with explicit production origins and a restrictive Content Security Policy.
- Sign update manifests and binaries; verify before installation and preserve rollback.
- Provide account export/deletion and document retention before public launch.

Registration/login endpoints are an initial development implementation, not a production-ready identity system. The browser client keeps its bearer token in memory only and loses the session on reload. Email verification, recovery, token rotation, rate limiting and hardened production cookie handling remain outstanding. Docker Compose credentials are local-development-only.
