# Chatty roadmap

## Milestone 0 — Foundation
- [x] Rust workspace and Axum health endpoint
- [x] Responsive React web client shell with local demo messages and basic appearance/notification toggles
- [x] Product architecture, security and roadmap documentation
- [ ] Import final approved Chatty logo assets into all platform bundles

## Milestone 1 — Accounts and persistence
- [ ] PostgreSQL schema and SQLx migrations
- [ ] Registration, login, email verification and session management
- [ ] Argon2id password hashing, refresh-token rotation and account recovery
- [ ] Profiles, friends and privacy controls

## Milestone 2 — Servers and messaging
- [ ] Servers, memberships, roles, permissions and channels
- [ ] Persistent messages, edits, deletes, reactions, replies and attachments
- [ ] WebSocket gateway, presence, typing and unread counts
- [ ] Search, moderation logs and reporting

## Milestone 3 — Voice and community
- [ ] WebRTC voice rooms and group calls using an SFU
- [ ] Video and screen sharing
- [ ] Events, forums, polls, bots, webhooks and integrations
- [ ] Anti-spam, anti-raid and moderation automation

## Milestone 4 — Apps and releases
- [ ] Tauri desktop app for Windows, macOS and Linux
- [ ] Android and iOS builds and push notifications
- [ ] Signed desktop updates, staged rollout and rollback
- [ ] Web PWA and deployment pipeline

## Milestone 5 — Operations
- [ ] Admin dashboard, audit tooling and analytics
- [ ] Observability, backups, disaster recovery and load testing
- [ ] Security review and privacy/data export workflows

Only mark features complete after implementation and tests. Future roadmap items are not currently available.
