# POOOL Platform — Agent Context

> This file provides essential context for AI agents working on this codebase.
> Keep this file up to date as the project evolves.

> **⚠️ BEFORE WRITING ANY CODE:**
> 1. Read [`docs/AGENT_DEVELOPMENT_PROMPT.md`](docs/AGENT_DEVELOPMENT_PROMPT.md) — mandatory zero-defect coding standards and security rules.
> 2. Read [`docs/DESIGN.md`](docs/DESIGN.md) — mandatory design system reference for ALL frontend/UI work. Contains colors, typography, spacing, component specs, and do's/don'ts.
> 3. Claim your task in [`docs/IMPLEMENTATION_ROADMAP.md`](docs/IMPLEMENTATION_ROADMAP.md) — 120+ tasks across 15 phases with multi-agent collaboration protocol.
> 4. Failure to follow the development directive will result in bugs with real financial consequences.

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

## 🚨 Production Architecture: PgBouncer + Cloud SQL (CRITICAL)

> **DO NOT modify `backend/src/db.rs` or `pgbouncer/entrypoint.sh` without reading this section.**
> Breaking the PgBouncer integration takes down ALL of production (login, marketplace, leaderboard, everything).

```
┌──────────────────────────────────────────────────────┐
│ Cloud Run Container                                   │
│                                                       │
│  ┌──────────────┐  TCP 127.0.0.1:6432  ┌──────────┐ │
│  │ POOOL Backend │ ─────────────────► │ PgBouncer │  │
│  │  (Rust/Axum)  │                    │ (sidecar) │  │
│  └──────────────┘                     └─────┬────┘  │
│                                              │       │
│                      Unix socket: /cloudsql/...      │
│                                              │       │
└──────────────────────────────────────────────┼───────┘
                                               │
                                         ┌─────▼─────┐
                                         │ Cloud SQL  │
                                         │ PostgreSQL │
                                         └───────────┘
```

### Rules — ALL mandatory:
1. **Backend → PgBouncer → Cloud SQL.** Backend must NEVER connect directly to the `/cloudsql/` Unix socket.
2. **`PGBOUNCER_ENABLED=true`** in Dockerfile tells `db.rs` to skip socket auto-detection.
3. **`pool_mode = session`** in PgBouncer config — mandatory for `sqlx` prepared statements to work without collision.
4. **`ignore_startup_parameters = extra_float_digits, options`** in PgBouncer config.
5. **`entrypoint.sh` rewrites `DATABASE_URL`** to `127.0.0.1:6432` before starting the backend.

### If you see these errors, here's what's broken:
| Error | Cause | Fix File |
|-------|-------|----------|
| `prepared statement "sqlx_s_N" already exists` | PgBouncer Session pool conflict | `db.rs` — ensure `statement_cache_capacity(0)` is set when Pgbouncer is enabled |
| `prepared statement "sqlx_s_N" already exists` | Backend bypassing PgBouncer | `db.rs` — ensure `PGBOUNCER_ENABLED` check exists |
| `unsupported startup parameter: extra_float_digits` | PgBouncer rejecting params | `entrypoint.sh` — add `ignore_startup_parameters` |
| `"trust" authentication failed` | Missing credentials upstream | `entrypoint.sh` — add `user=`/`password=` to `[databases]` |
| `GLIBC_X.XX not found` | Builder/runtime glibc mismatch | `Dockerfile` — pin builder to `rust:1-bookworm` |

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
│   ├── DESIGN.md                    # ⚠️ MANDATORY — Design system (colors, typography, components)
│   ├── IMPLEMENTATION_ROADMAP.md  # Active Multi-Agent Tracking & Implementation Path (120+ tasks)
│   ├── AGENT_DEVELOPMENT_PROMPT.md # ⚠️ MANDATORY — Zero-defect coding standards, read BEFORE coding
│   ├── MASTERPLAN.md              # Architecture vision (9,780 lines, Chapters 1-6)
│   ├── DATABASE_SCHEMA.md         # Full schema reference
│   └── KNOWLEDGE_HANDOFF.md       # Historical ADRs & decisions
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
- **Card-with-table standard**: Any new or refactored "card containing a table"
  MUST match the **`.developer-assets-performance-section`** reference pattern
  on the developer dashboard (Top Performing Assets). Spec: `docs/DESIGN.md` →
  *Tables → Card With Table — Reference Pattern (STANDARD)*. Brand-gradient
  top strip, icon tile + title + subtitle header, white `.table__header-row`,
  right-aligned numeric columns, brand-green hover wash on rows, grey-default
  icon buttons that color-only on hover. Do not invent new card-table styles.

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

See `docs/issue-tracking/BROKEN_LOGICS.md` for the full tracker.
- No critical (P0/P1) blockers at this time.
- 41 non-critical compilation warnings pending `cargo fix`.

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
