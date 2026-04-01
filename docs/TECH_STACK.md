# POOOL — Technology Stack Reference

> Last updated: 2026-03-28

---

## 1. High-Level Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                         Google Cloud Run                           │
│                                                                    │
│  ┌─────────────────┐   TCP :6432   ┌───────────┐   Unix socket   │
│  │  POOOL Backend   │ ───────────► │ PgBouncer  │ ─────────────►  │
│  │  (Rust / Axum)   │              │ (sidecar)  │                 │
│  └────────┬────────┘              └───────────┘                  │
│           │                                                       │
│           ├── serves HTML (MiniJinja SSR)                         │
│           ├── serves static CSS / JS / images                    │
│           ├── JSON API (fetch-based)                              │
│           ├── WebSocket (real-time trading)                       │
│           └── background Tokio tasks                              │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
            │                    │                    │
     ┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐
     │  Cloud SQL   │     │    Redis    │     │    GCS      │
     │ PostgreSQL   │     │  (optional) │     │  (storage)  │
     └─────────────┘     └─────────────┘     └─────────────┘
```

---

## 2. Runtime Versions (Local Development)

| Tool       | Version       | Notes                                  |
|------------|---------------|----------------------------------------|
| **Rust**   | 1.94.0        | Stable channel, Edition 2021           |
| **Cargo**  | 1.94.0        | Workspace root: `backend/`             |
| **Node.js**| 25.8.2        | Used only for marketing site tooling   |
| **npm**    | 11.11.1       | Marketing site only                    |
| **PostgreSQL** | 14.20 (Homebrew) | Local dev DB: `poool`            |
| **Redis**  | 8.4.0         | Optional — marketplace matching engine |

---

## 3. Backend — Rust

### 3.1 Framework & HTTP

| Crate              | Version | Purpose                                                  |
|--------------------|---------|----------------------------------------------------------|
| `axum`             | 0.7     | HTTP framework (routing, extractors, middleware)          |
| `axum-extra`       | 0.9     | Cookie support (`cookie` feature)                        |
| `tokio`            | 1.0     | Async runtime (`full` feature set)                       |
| `tower-http`       | 0.6     | Static file serving, CORS, tracing, compression          |
| `tower`            | 0.5     | Middleware (rate limiting, concurrency)                   |
| `axum_csrf`        | 0.11    | CSRF protection middleware                               |

### 3.2 Database

| Crate              | Version | Purpose                                                  |
|--------------------|---------|----------------------------------------------------------|
| `sqlx`             | 0.8     | Async PostgreSQL driver (compile-time checked queries)   |
| `rust_decimal`     | 1.33    | Decimal arithmetic (financial precision)                 |

**Features enabled on SQLx:** `runtime-tokio`, `tls-rustls`, `postgres`, `uuid`, `chrono`, `json`, `macros`, `rust_decimal`

### 3.3 Authentication & Security

| Crate              | Version | Purpose                                                  |
|--------------------|---------|----------------------------------------------------------|
| `argon2`           | 0.5     | Password hashing (Argon2id)                              |
| `totp-rs`          | 5.6     | Two-factor authentication (TOTP, QR codes)               |
| `oauth2`           | 5.0     | Google & Facebook OAuth2 flows                           |
| `hmac` / `sha2`    | 0.12 / 0.10 | HMAC webhook verification, token signing            |
| `ammonia`          | 4.1     | HTML sanitization (XSS prevention)                       |

### 3.4 Templating

| Crate              | Version | Purpose                                                  |
|--------------------|---------|----------------------------------------------------------|
| `minijinja`        | 2.0     | Server-side HTML rendering (Jinja2-compatible)           |

All HTML pages are rendered server-side via MiniJinja templates located in `backend/templates/`.

### 3.5 Serialization & Utilities

| Crate              | Version | Purpose                                                  |
|--------------------|---------|----------------------------------------------------------|
| `serde` / `serde_json` | 1.0 | JSON serialization/deserialization                      |
| `reqwest`          | 0.12    | HTTP client (KYC provider, OAuth, IPFS/Pinata)          |
| `uuid`             | 1.6     | UUID v4 generation                                       |
| `chrono`           | 0.4     | Date/time handling                                       |
| `regex`            | 1.12    | Pattern matching                                         |
| `url`              | 2.5     | URL parsing                                              |
| `base64`           | 0.22    | Base64 encoding/decoding                                 |
| `hex`              | 0.4     | Hex encoding (blockchain)                                |
| `anyhow`           | 1.0     | Error handling                                           |
| `dotenvy`          | 0.15    | `.env` file loading                                      |

### 3.6 Caching & Message Queue

| Crate              | Version | Purpose                                                  |
|--------------------|---------|----------------------------------------------------------|
| `redis`            | 0.24    | Redis client (orderbook, pub/sub, rate limiting)         |
| `deadpool-redis`   | 0.14    | Connection pooling for Redis                             |

### 3.7 Cloud Storage

| Crate              | Version | Purpose                                                  |
|--------------------|---------|----------------------------------------------------------|
| `google-cloud-storage` | 0.22 | GCS uploads (avatars, KYC docs, asset images)          |
| `google-cloud-auth`| 0.17    | GCP service account auth                                |
| `mime_guess`       | 2.0     | Content-type detection for uploads                       |

### 3.8 Monitoring & Observability

| Crate              | Version | Purpose                                                  |
|--------------------|---------|----------------------------------------------------------|
| `sentry`           | 0.34    | Error monitoring & performance tracking                  |
| `sentry-tracing`   | 0.34    | Sentry ↔ tracing integration                            |
| `tracing`          | 0.1     | Structured logging                                       |
| `tracing-subscriber` | 0.3  | Log subscriber with env-filter                           |

### 3.9 Backend Module Structure

The backend is organized into **24 domain modules** under `backend/src/`:

| Module           | Domain                                              |
|------------------|-----------------------------------------------------|
| `admin/`         | Platform admin dashboard, user management, reports  |
| `assets/`        | Asset management (listings, tokenization)           |
| `auth/`          | Login, signup, sessions, OAuth, rate limiting        |
| `blockchain/`    | On-chain settlement, event indexer, KYC whitelist   |
| `blog/`          | Blog articles, authors, categories                  |
| `cart/`           | Shopping cart logic                                  |
| `community/`     | Social feed, AMAs, circles, gamification            |
| `developer/`     | Developer portal (asset submissions)                |
| `dividends/`     | Dividend calculation, anti-sniping, payouts         |
| `email.rs`       | Email scheduling system                             |
| `ipfs/`          | Pinata-based IPFS metadata storage                  |
| `kyc/`           | KYC flow (Didit.me + manual review)                 |
| `leaderboard/`   | Investor leaderboard, scoring, rankings             |
| `legal/`         | Terms of service, privacy, legal consents           |
| `marketplace/`   | Secondary market: matching engine, settlement, P2P  |
| `payment_methods/`| Bank accounts & cards                               |
| `payments/`      | Order processing, invoicing, escrow                 |
| `portfolio/`     | Investor portfolio views                            |
| `rewards/`       | Referral system, tiers, reward balances             |
| `settings/`      | User settings management                            |
| `storage/`       | GCS file upload handlers                            |
| `support/`       | Support tickets, SLA monitoring                     |
| `wallet/`        | Wallet balance, transactions, transfers             |

**Codebase size:** ~155 Rust source files, ~59,500 lines of Rust code.

---

## 4. Frontend — Platform (Investor Dashboard)

### 4.1 Philosophy

> **Zero-framework, zero-bundler.** The platform frontend uses plain HTML, CSS, and JavaScript — no React, Vue, Angular, or build tools.

### 4.2 HTML

- **Rendering:** Server-side via **MiniJinja** (Jinja2-compatible templates)
- **Template location:** `backend/templates/`
- **HTML pages:** `frontend/platform/*.html` (~162 pages)
- **Served by Axum** directly as static files + SSR route handlers

### 4.3 CSS

| Aspect           | Details                                                       |
|------------------|---------------------------------------------------------------|
| **Methodology**  | Vanilla CSS with BEM-inspired naming conventions              |
| **Font**         | **TT Norms Pro** (400/500/700/800 weights, WOFF2/WOFF/TTF)  |
| **Design System**| Documented in `docs/DESIGN.md` — mandatory reference          |
| **Design Theme** | "Wealth Terminal — Holographic Edition" (dark mode, glassmorphism, gradients) |
| **File count**   | ~120 CSS files in `frontend/platform/static/css/`            |
| **Total size**   | ~1.9 MB                                                       |
| **Bundling**     | `build-bundle.sh` concatenates all CSS → `bundle.css` at deploy time |
| **Naming**       | Page-specific: `<page-name>.css` + mobile variant: `mobile-<page-name>.css` |
| **Components**   | Design system primitives via `ds-*.css` files (buttons, badges, cards, forms, modals, progress, tables, typography, utilities) |

### 4.4 JavaScript

| Aspect           | Details                                                       |
|------------------|---------------------------------------------------------------|
| **Approach**     | Vanilla JS — no frameworks, no bundler                        |
| **API calls**    | `fetch()` API for all HTTP requests                           |
| **HTMX**         | Used sparingly via `htmx-init.js` for dynamic partial updates |
| **WebSocket**    | Native `WebSocket` API for real-time marketplace trading      |
| **Charts**       | Custom SVG-based charts (portfolio, marketplace)              |
| **File count**   | ~129 JS files in `frontend/platform/static/js/`             |
| **Total size**   | ~2.0 MB                                                       |
| **Naming**       | Page-specific: `<page-name>.js` (e.g., `wallet.js`, `admin-dashboard.js`) |
| **Services**     | Shared: `csrf.js`, `currency-service.js`, `user-data.js`, `toast.js` |

### 4.5 Key Frontend Libraries / Patterns

| Library / Pattern      | Usage                                               |
|------------------------|-----------------------------------------------------|
| SVG icons              | Inline SVGs and custom icon CSS (`poool-icon-custom.css`) |
| CSRF tokens            | Managed via `csrf.js` — attached to all POST requests |
| Toast notifications    | `toast.js` / `mp-toast.js` — reusable notification system |
| Confirmation dialogs   | `poool-confirm.js` — modal confirmation component   |
| Dropdown component     | `poool-dropdown.js` — custom dropdown with search   |
| Cookie consent         | `cookie-consent.js` — GDPR-compliant cookie banner  |
| PDF export             | `pdf-export.css` — print/export styling              |

---

## 5. Frontend — Marketing Website

| Aspect           | Details                                                       |
|------------------|---------------------------------------------------------------|
| **Location**     | `frontend/www/`                                               |
| **Framework**    | Pre-built Angular SPA (compiled static assets)                |
| **Languages**    | English (`/en/`), Indonesian (`/id/`)                         |
| **Assets**       | PNG, SVG, WebP images; WebM videos; custom fonts              |
| **Hosting**      | Served by the same Axum backend under the `www.poool.app` host |
| **SEO**          | `robots.txt`, `sitemap.xml` included                          |

---

## 6. Database — PostgreSQL

### 6.1 Setup

| Aspect              | Details                                                  |
|----------------------|----------------------------------------------------------|
| **Engine**           | PostgreSQL 16 (Cloud SQL in production, 14.x locally)   |
| **Primary DB**       | `poool` — all platform data                              |
| **Community DB**     | `poool_community` — social features (separate schema)   |
| **Migrations**       | 72+ SQL files in `database/` — applied in alphanumeric order |
| **Migration tracker**| Custom `_schema_migrations` table (built into `main.rs`) |
| **Money format**     | **BIGINT cents** — never floats, no exceptions           |
| **Schema docs**      | `docs/DATABASE_SCHEMA.md`                                |

### 6.2 Connection Pooling

| Layer             | Tool         | Config                                      |
|-------------------|------------- |---------------------------------------------|
| Application       | SQLx PgPool  | `max_connections=30`                         |
| Sidecar (prod)    | PgBouncer    | Transaction pooling, `max_db_connections=30`, `max_client_conn=100` |
| Cloud             | Cloud SQL Proxy | Unix socket → Cloud SQL                  |

### 6.3 Key Tables

Core domain tables span: `users`, `assets`, `investments`, `orders`, `wallets`, `wallet_transactions`, `deposit_requests`, `withdrawal_requests`, `marketplace_orders`, `trade_history`, `dividends`, `referrals`, `support_tickets`, `blog_articles`, and many more.

---

## 7. Infrastructure & Deployment

### 7.1 Docker Build

```
Stage 1: rust:1-bookworm  → cargo-chef (dependency caching)
Stage 2: Planner          → cargo chef prepare
Stage 3: Builder          → cargo chef cook → cargo build --release
                             + CSS bundle + Foundry (cast binary)
Stage 4: debian:bookworm-slim → runtime with PgBouncer sidecar
```

- **Base image (build):** `rust:1-bookworm`
- **Base image (runtime):** `debian:bookworm-slim`
- **Runs as:** non-root user `poool` (uid 1000)
- **Port:** 8080 (production), 8888 (local dev)

### 7.2 Cloud Services

| Service                  | Provider     | Purpose                               |
|--------------------------|------------- |---------------------------------------|
| **Compute**              | Google Cloud Run | Containerized deployment            |
| **Database**             | Google Cloud SQL | Managed PostgreSQL 16               |
| **File Storage**         | Google Cloud Storage | Avatars, KYC docs, asset images |
| **Caching / Pub-Sub**   | Redis (Cloud Memorystore or self-hosted) | Marketplace orderbook, session rate limiting |
| **Error Monitoring**     | Sentry       | Error tracking, performance APM       |
| **KYC Provider**         | Didit.me     | Identity verification (fallback: manual review) |
| **Domain / DNS**         | Cloudflare (or GCP) | `poool.app`, `platform.poool.app`, `www.poool.app` |

### 7.3 Host-Based Routing

| Host                    | Serves                                         |
|-------------------------|-------------------------------------------------|
| `www.poool.app`         | Marketing website (Angular SPA)                 |
| `platform.poool.app`   | Investor dashboard + API                        |
| `poool.app` (bare)      | 301 → `www.poool.app`                          |
| `localhost:8888`        | Platform (dev default)                          |

---

## 8. Blockchain Integration

| Aspect             | Details                                                     |
|--------------------|-------------------------------------------------------------|
| **Network**        | Polygon (Amoy testnet / Mainnet)                            |
| **Token Standard** | ERC-1155 (`POOOLProperty1155`) — multi-asset tokenization   |
| **Factory**        | `AssetFactory` — deploys EIP-1167 minimal proxy clones      |
| **Tooling**        | Foundry (`cast` binary) for admin tokenization / emergency  |
| **On-chain tasks** | Settlement batching, event indexing, KYC whitelist sync     |
| **IPFS**           | Pinata — metadata and legal document pinning                |
| **Chain ID**       | 80002 (Amoy testnet)                                        |

---

## 9. Authentication & Sessions

| Mechanism            | Details                                                  |
|----------------------|----------------------------------------------------------|
| **Session storage**  | PostgreSQL (`user_sessions` table)                       |
| **Session token**    | HTTP-only cookie: `poool_session`                        |
| **Password hashing** | Argon2id                                                 |
| **2FA**              | TOTP (Time-based One-Time Password) with QR code setup  |
| **OAuth providers**  | Google, Facebook                                         |
| **CSRF protection**  | Double-submit cookie pattern (`axum_csrf`)               |
| **Rate limiting**    | Redis-backed (production) or in-memory (dev)             |
| **No JWT**           | By design — stateful sessions only                       |

---

## 10. Background Workers

The backend spawns multiple Tokio tasks on startup:

| Worker                           | Interval    | Purpose                                        |
|----------------------------------|-------------|-------------------------------------------------|
| Email scheduler                  | Continuous  | Queued email delivery                           |
| SLA breach monitor               | Continuous  | Support ticket SLA alerts                       |
| Rate limiter cleanup             | 10 min      | Purge stale rate limit entries                  |
| Expired order cleanup            | 15 min      | Reclaim tokens from expired primary orders      |
| Leaderboard score refresh        | 15 min      | Recalculate all investor scores & rankings      |
| Session/token housekeeping       | 6 hours     | Purge expired sessions & reset tokens           |
| Financial reconciliation         | 24 hours    | Cash/token balance invariant checks (P0)        |
| Marketplace matching engine      | Continuous  | Redis orderbook → match orders (requires Redis) |
| Marketplace settlement           | Continuous  | ACID settlement of matched trades               |
| Order expiry worker              | Hourly      | Expire stale marketplace orders                 |
| Redis sync worker                | 5 min       | Detect & fix Redis↔DB drift                     |
| Price snapshot worker            | 5 min       | Cache last-trade prices in Redis                |
| WebSocket pub/sub subscriber     | Continuous  | Cross-instance real-time message delivery       |
| Blockchain settlement            | Continuous  | Batch on-chain `settleBatch()` calls            |
| Blockchain event indexer         | Continuous  | Poll Polygon for ERC-1155 transfer events       |
| KYC → whitelist sync             | Continuous  | Auto-whitelist KYC-approved users on-chain      |
| Auto-refund worker               | Continuous  | Refund expired primary escrow offerings         |
| Community workers (7)            | Various     | Gamification, velocity, digests, GDPR cleanup   |

---

## 11. Development Tools

| Tool              | Version | Purpose                                           |
|-------------------|---------|---------------------------------------------------|
| `cargo-watch`     | 8.5.3   | Auto-rebuild on file save (`cargo watch -x run`)  |
| `sqlx-cli`        | 0.8.6   | Database migrations (`sqlx migrate run`)          |
| `cargo-audit`     | 0.22.1  | Security vulnerability audit                      |
| `cargo-chef`      | —       | Docker layer caching for Rust builds              |
| `pgcli`           | 4.4.0   | Enhanced PostgreSQL CLI client                    |
| `ripgrep` (`rg`)  | —       | Fast code search across the codebase              |
| `Foundry` (`cast`)| —       | Blockchain CLI tool (deployed in Docker)          |

---

## 12. Security Measures

| Layer              | Implementation                                           |
|--------------------|----------------------------------------------------------|
| Password storage   | Argon2id with automatic salt                             |
| Session security   | HTTP-only, secure cookies (no client-side access)        |
| CSRF               | Double-submit cookie pattern on all POST routes          |
| XSS prevention     | Ammonia HTML sanitization + MiniJinja auto-escaping      |
| SQL injection      | SQLx compile-time checked queries (parameterized)        |
| Rate limiting      | Per-IP rate limits on auth endpoints                     |
| Input validation   | Server-side validation on all user inputs                |
| File uploads       | 25 MB limit, content-type checking, GCS storage          |
| Security headers   | `X-Frame-Options`, `X-Content-Type-Options`, CSP headers |
| CORS               | Restricted origins in production, any in dev             |
| Secrets            | `.env` file (not committed), env vars in Cloud Run       |
| Monitoring         | Sentry with `[P0-FINANCIAL]` alerts for money issues     |

---

## 13. Codebase Size Summary

| Component                     | Count         | Size    |
|-------------------------------|---------------|---------|
| Rust source files             | ~155          | ~2.4 MB |
| Rust lines of code            | ~59,500       | —       |
| HTML pages (platform)         | ~162          | —       |
| CSS files                     | ~120          | ~1.9 MB |
| JavaScript files              | ~129          | ~2.0 MB |
| SQL migration files           | 72+           | ~660 KB |
| Database tables               | 60+           | —       |
| Backend domain modules        | 24            | —       |
| Background Tokio workers      | 18+           | —       |
| API routes                    | 100+          | —       |

---

## 14. Environment Quick Reference

### Local Development
```bash
# Start backend with auto-reload
cd backend && cargo watch -x run

# One-shot run
cd backend && cargo run

# Server: http://localhost:8888

# Connect to DB
pgcli postgres://martin@localhost/poool
```

### Production
```bash
# Deployed via Cloud Run
# Dockerfile handles multi-stage build + PgBouncer sidecar
# Port: 8080 (internal), exposed via Cloud Run HTTPS
```

### Required Environment Variables
| Variable           | Example                              |
|--------------------|---------------------------------------|
| `DATABASE_URL`     | `postgres://martin@localhost/poool`  |
| `SERVER_PORT`      | `8888`                                |
| `BASE_URL`         | `http://localhost:8888`               |
| `POOOL_ENV`        | `development` / `production`         |

### Optional Environment Variables
`GOOGLE_CLIENT_ID/SECRET`, `FACEBOOK_APP_ID/SECRET`, `DIDIT_API_KEY`, `GCS_BUCKET_NAME`, `REDIS_URL`, `SENTRY_DSN`, `CHAIN_*` (blockchain), `PINATA_*` (IPFS), `COMMUNITY_DATABASE_URL`

---

*This document is auto-maintained. Update when adding new dependencies, services, or infrastructure changes.*
