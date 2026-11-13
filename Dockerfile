# Build from the repository root so Cargo can resolve the workspace.
FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p chatty-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/chatty-api /usr/local/bin/chatty-api
ENV CHATTY_HOST=0.0.0.0
EXPOSE 8080
CMD ["chatty-api"]
