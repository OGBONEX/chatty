# Security baseline

- Never commit credentials, signing keys, database URLs or production secrets.
- Hash passwords with Argon2id; never store plaintext passwords.
- Use HTTPS and secure HttpOnly SameSite cookies for browser sessions.
- Rotate refresh tokens and revoke sessions on logout, password reset and suspicious activity.
- Enforce authorization server-side for every server, channel, message and file operation.
- Apply account, IP and resource rate limits.
- Bound and validate all user input and uploads; scan uploaded files.
- Escape user content and never render arbitrary HTML.
- Use explicit production CORS origins and a restrictive Content Security Policy.
- Sign update manifests and binaries; verify before installation and preserve rollback.
- Provide account export/deletion and document retention before public launch.

The initial API has no account or messaging endpoints and is not production-ready.
