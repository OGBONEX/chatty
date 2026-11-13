# Deploying Chatty on Render

This repository includes a Dockerfile for the Rust/Axum API. Create the API service separately from the PostgreSQL database.

## API Web Service

1. In Render, open the Chatty project and choose **New → Web Service**.
2. Connect the GitHub repository `OGBONEX/chatty`.
3. Select branch `deploy/render-api` until its deployment pull request is merged.
4. Choose **Docker** as the runtime. Set the Docker build context to the repository root and use the root `Dockerfile`.
5. Set the health check path to `/health`.
6. Add these environment variables:
   - `DATABASE_URL`: the database's **Internal Database URL** (keep this secret).
   - `CHATTY_HOST`: `0.0.0.0`
   - `CHATTY_ALLOWED_ORIGINS`: the exact HTTPS origin of the deployed Chatty web client, e.g. `https://chatty-web.onrender.com` (no path and no trailing slash).
   - `RUST_LOG`: `info`
   
   Render supplies `PORT` automatically; the API reads it when `CHATTY_PORT` is not set.

The API applies SQL migrations automatically at startup. Check the service logs for the message that PostgreSQL connected and migrations were applied. Then open `https://YOUR-API-SERVICE.onrender.com/health`; the response should report `status: ok` and `database: configured`.

## Web client

Deploy `apps/web` as a Render Static Site after the API is live:
- Root directory: `apps/web`
- Build command: `npm install && npm run build`
- Publish directory: `dist`
- Build-time environment variable: `VITE_API_BASE_URL=https://YOUR-API-SERVICE.onrender.com`

Use the static site's exact HTTPS origin as `CHATTY_ALLOWED_ORIGINS` on the API service, then redeploy the API. Rebuild/redeploy the web client whenever its API URL changes.

## Important

- Never commit the database URL, passwords, or other secrets to GitHub.
- Use the database's **Internal Database URL** only for a service running in the same Render region.
- The API and web client are separate services. The local fallback URL is not suitable for a deployed phone or public website.
- This is a deployment foundation, not a security audit. Before opening registration to the public, add production-grade rate limiting, abuse protection, monitoring, and a reviewed CORS origin list.
