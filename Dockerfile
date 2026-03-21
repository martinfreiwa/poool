# ── Stage 1: Prepare recipes ───────────────────────────────────
FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
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

# ── Stage 4: User setup ─────────────────────────────────────────
FROM debian:bookworm-slim AS user-setup
RUN groupadd -g 1000 poool && \
    useradd -u 1000 -g poool -s /bin/sh -m poool

# ── Stage 5: Distroless runtime image ───────────────────────────
FROM gcr.io/distroless/cc-debian12 AS runtime

# Copy user/group definitions
COPY --from=user-setup /etc/passwd /etc/passwd
COPY --from=user-setup /etc/group /etc/group

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

# Set working directory to /app/backend so relative paths like ../frontend work
# Note: WORKDIR in distroless will create the path if it doesn't exist
WORKDIR /app/backend

# Use the non-root user
USER poool

# Environment variables
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV RUST_LOG=info
ENV APP_ENV=production
ENV POOOL_ENV=production

EXPOSE 8080

# Use ENTRYPOINT for distroless
ENTRYPOINT ["/app/poool-backend"]
