# POOOL Platform — Agent Context

> This file provides essential context for AI agents working on this codebase.
> Keep this file up to date as the project evolves.

---

## 🏗 Stack

| Layer | Technology |
|-------|-----------|
| **Backend** | Rust, Axum, SQLx, MiniJinja (SSR templates) |
| **Frontend (Platform)** | Vanilla HTML + CSS + JS — NO framework, NO bundler |
| **Frontend (Marketing)** | `frontend/www/` — separate, not touched often |
| **Database** | PostgreSQL 16 (`poool` db locally) |
| **Auth** | Session-based (HTTP-only cookie: `poool_session`) |
| **File Storage** | Google Cloud Storage (`GCS_BUCKET_NAME`) |
| **KYC Provider** | Didit.me (optional; falls back to manual review if not configured) |
| **Deployment** | Google Cloud Run via `Dockerfile` |
| **Error Monitoring** | Sentry (optional) |
| **Caching** | Redis (optional) |

---

## 🚀 Local Development

```bash
# Start backend (auto-reload on file save)
cd backend && cargo watch -x run

# Or one-shot
cd backend && cargo run

# Server: http://localhost:8888
# Port env var: PORT or SERVER_PORT (default: 8888)
```

```bash
# Connect to local DB
pgcli postgres://martin@localhost/poool
# Or: psql -d poool
```

```bash
# Kill port if stuck
lsof -i :8888 -t | xargs kill -9
```

---

## 💰 Critical Business Rules

- **All monetary values are `BIGINT` cents** — never floats. No exceptions.
- **All financial ops must be wrapped in a DB transaction** (ACID).
- **No client-side business logic** — all routing and financial verification happen in Rust.
- **Passwords**: Argon2id hashing.
- **Sessions**: HTTP-only cookies, no JWT.

---

## 🗂 Repository Structure

```
poool/
├── backend/
│   ├── src/
│   │   ├── main.rs          # Axum router — ALL routes registered here
│   │   ├── config.rs        # Env var config (Config::from_env())
│   │   ├── error.rs         # AppError — centralised error handling
│   │   ├── db.rs            # DB pool setup
│   │   ├── auth/            # Login, signup, sessions, OAuth
│   │   ├── admin/           # Admin dashboard APIs
│   │   ├── assets/          # Asset management
│   │   ├── cart/            # Cart & checkout
│   │   ├── developer/       # Developer dashboard
│   │   ├── kyc/             # KYC flow (Didit.me + manual)
│   │   ├── payments/        # Order approval, invoicing
│   │   ├── payment_methods/ # Bank accounts & cards
│   │   ├── portfolio/       # Investor portfolio
│   │   ├── rewards/         # Referrals, tiers, balances
│   │   ├── settings/        # User settings
│   │   ├── storage/         # GCS uploads (avatars, KYC docs)
│   │   ├── support/         # Support tickets
│   │   └── wallet/          # Wallet balance & transactions
│   └── .env                 # Local env (DATABASE_URL, etc.) — not committed
├── frontend/
│   └── platform/            # Investor & admin dashboard UI
│       ├── *.html           # Pages (served via MiniJinja SSR)
│       └── static/
│           ├── css/         # Vanilla CSS (fonts.css, global styles)
│           └── js/          # Page-specific JS files
├── database/
│   └── *.sql                # Migrations (applied in order)
├── docs/
│   ├── MASTERPLAN.md        # Architecture vision
│   ├── DATABASE_SCHEMA.md   # Full schema reference
│   └── KNOWLEDGE_HANDOFF.md # Historical ADRs & decisions
├── .agent/workflows/        # Agent workflow files (18 workflows)
├── BROKEN_LOGICS.md         # Known bugs & logic tracker — consolidated tracker
└── AGENTS.md                # This file
```

---

## 🔑 Environment Variables

| Key | Required | Description |
|-----|----------|-------------|
| `DATABASE_URL` | ✅ | `postgres://martin@localhost/poool` (local) |
| `SERVER_PORT` or `PORT` | ❌ | Default: `8888` |
| `BASE_URL` | ❌ | Default: `http://localhost:8888` |
| `GOOGLE_CLIENT_ID/SECRET` | ❌ | Google OAuth |
| `FACEBOOK_APP_ID/SECRET` | ❌ | Facebook OAuth |
| `DIDIT_API_KEY` | ❌ | KYC provider — omit to use manual review |
| `GCS_BUCKET_NAME` | ❌ | GCS bucket for uploads — omit to disable upload routes |
| `REDIS_URL` | ❌ | Caching — optional |
| `SENTRY_DSN` | ❌ | Error monitoring — optional |

---

## 📐 Code Conventions

### Rust Backend
- **Error handling**: Use `AppError` from `error.rs` — never `unwrap()` in production paths
- **Route registration**: ALL routes go in `src/main.rs` — one place
- **Module structure**: Each domain has `routes.rs`, `models.rs`, `service.rs`
- **SQL**: Use SQLx `query!` / `query_as!` macros (compile-time checked)
- **Format before commit**: `cargo fmt` + `cargo clippy` required

### Frontend (Platform)
- **No frameworks** — plain JS `fetch()`, no React/Vue/Alpine (some HTMX used)
- **Font**: TT Norms Pro (loaded via `frontend/platform/static/css/fonts.css`)
- **JS pattern**: Each page has its own `static/js/<page-name>.js`
- **CSS pattern**: Page-specific CSS in `static/css/<page-name>.css`
- **SSR**: Pages are rendered by MiniJinja templates on the Rust backend

---

## 🛠 Installed Tools

| Tool | Version | Use |
|------|---------|-----|
| `cargo-watch` | 8.5.3 | `cargo watch -x run` — auto-rebuild on save |
| `sqlx-cli` | 0.8.6 | DB migrations (`sqlx migrate run`) |
| `cargo-audit` | 0.22.1 | Security audit (`cargo audit`) |
| `pgcli` | 4.4.0 | Better PostgreSQL CLI |
| `ripgrep` (`rg`) | — | Fast file search (use for all code searches) |

---

## ⚠️ Known Issues

See `BROKEN_LOGICS.md` for the full tracker. Key outstanding items:
- Several admin system APIs not yet implemented (background jobs, sessions, webhooks)
- Admin approval "phantom approval" bug (marks approved but doesn't trigger business logic)
- Some report endpoints return all-time data regardless of date filters
- /checkout route missing (CRITICAL)

---

## 🧪 Testing

```bash
# Rust unit tests
cd backend && cargo test

# Python E2E tests
python3 -m pytest tests/

# Backend must be running on :8888 for E2E tests
```

---

## 📋 Workflows

18 agent workflows are defined in `.agent/workflows/`. Key ones:
- `/start-backend` — Start local dev server
- `/deploy` — Deploy to Cloud Run
- `/fix-all-issues` — Bug fixing workflow
- `/e2e-testing-master` — Full E2E test suite
