# ── Stage 1: Prepare recipes ───────────────────────────────────
FROM rust:1-bookworm AS chef
# Install cargo-chef manually to ensure we use bookworm (glibc 2.36) to match the runtime
RUN cargo install cargo-chef --locked
WORKDIR /app/backend

# ── Stage 2: Plan dependencies ─────────────────────────────────
FROM chef AS planner
# Copy all workspace files
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/.sqlx* ./.sqlx/
COPY backend/src ./src
COPY backend/templates ./templates
# Compute logical dependency tree
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 3: Build & cache dependencies ────────────────────────
FROM chef AS builder
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app/backend
COPY --from=planner /app/backend/recipe.json recipe.json
# Build dependencies
ENV SQLX_OFFLINE=true
RUN cargo chef cook --release --locked --recipe-path recipe.json

# Now copy real source code
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/.sqlx* ./.sqlx/
COPY backend/templates ./templates
COPY backend/src ./src

# Build the real binary
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked

# Prepare frontend bundle in builder stage since it has bash
WORKDIR /app
# Cache-bust: ensure all static assets (images/subdirs) are freshly copied
ARG CACHEBUST=1
COPY frontend/platform/ /app/frontend/platform/
COPY frontend/www/ /app/frontend/www/
RUN bash /app/frontend/platform/static/css/build-bundle.sh

# ── IMPORTANT: Normalize file permissions ────────────────────────
# macOS may assign restrictive permissions (e.g. 750) to directories,
# which breaks file serving in the distroless container where the app
# runs as the non-root `poool` user. Force 755 on dirs and 644 on files.
RUN find /app/frontend -type d -exec chmod 755 {} + && \
    find /app/frontend -type f -exec chmod 644 {} +

# ── Stage 4: Runtime image with PgBouncer sidecar ───────────────
# Using debian-slim (instead of distroless) to support PgBouncer sidecar process.
# Security: runs as non-root 'poool' user, same as before.
FROM debian:bookworm-slim AS runtime

# Install PgBouncer + minimal runtime dependencies (libssl, ca-certs)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pgbouncer \
        ca-certificates \
        libssl3 && \
    rm -rf /var/lib/apt/lists/* && \
    # Create non-root user
    groupadd -g 1000 poool && \
    useradd -u 1000 -g poool -s /bin/sh -m poool

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/backend/target/release/poool-backend /app/poool-backend

# Copy frontend files (including generated bundle.css)
COPY --from=builder /app/frontend/platform/ /app/frontend/platform/
COPY --from=builder /app/frontend/www/ /app/frontend/www/

# Copy templates for runtime
COPY backend/templates /app/backend/templates

# Copy database migrations
COPY database/ /app/database/

# Copy PgBouncer entrypoint and config
COPY pgbouncer/entrypoint.sh /app/entrypoint.sh
COPY pgbouncer/pgbouncer.ini /app/pgbouncer/pgbouncer.ini
RUN chmod +x /app/entrypoint.sh

# Set working directory to /app/backend so relative paths like ../frontend work
WORKDIR /app/backend

# Use the non-root user
USER poool

# Environment variables
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV RUST_LOG=info
ENV APP_ENV=production
ENV POOOL_ENV=production
# PgBouncer enabled by default in production; set to "false" to disable
ENV PGBOUNCER_ENABLED=true

EXPOSE 8080

# Use ENTRYPOINT with the PgBouncer sidecar script
ENTRYPOINT ["/app/entrypoint.sh"]

