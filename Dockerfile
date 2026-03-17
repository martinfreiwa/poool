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
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app/backend
COPY --from=planner /app/backend/recipe.json recipe.json
# Build dependencies - this is the main caching Docker layer!
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

# ── Stage 2: Minimal runtime image ──────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary
COPY --from=builder /app/backend/target/release/poool-backend /app/poool-backend

# Copy frontend files
COPY frontend/platform/ /app/frontend/platform/
COPY frontend/www/ /app/frontend/www/

# Build CSS bundle (merges common CSS into a single file)
RUN bash /app/frontend/platform/static/css/build-bundle.sh

# Copy templates for runtime (if needed by minijinja loader) 
COPY backend/templates /app/backend/templates

# The backend serves static files from ../frontend/platform relative to CWD
RUN mkdir -p /app/backend
WORKDIR /app/backend

ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV RUST_LOG=info
ENV APP_ENV=production
ENV POOOL_ENV=production

EXPOSE 8080

CMD ["/app/poool-backend"]

