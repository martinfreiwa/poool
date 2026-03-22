# POOOL Platform — Agent Context

> This file provides essential context for AI agents working on this codebase.
> Keep this file up to date as the project evolves.

> **⚠️ BEFORE WRITING ANY CODE:**
> 1. Read [`docs/AGENT_DEVELOPMENT_PROMPT.md`](docs/AGENT_DEVELOPMENT_PROMPT.md) — mandatory zero-defect coding standards, self-healing patterns, and security rules.
> 2. Read [`docs/DESIGN.md`](docs/DESIGN.md) — mandatory design system reference for ALL frontend/UI work. Contains colors, typography, spacing, component specs, and do's/don'ts.
> 3. Claim your task in [`docs/IMPLEMENTATION_ROADMAP.md`](docs/IMPLEMENTATION_ROADMAP.md) — 120+ tasks across 15 phases with multi-agent collaboration protocol.
> 4. Failure to follow the development directive will result in bugs with real financial consequences.

> **🔧 SELF-HEALING PROTOCOL — MANDATORY:**
> If you encounter ANY bug, error, or broken logic while working on the codebase — **you MUST fix it immediately.**
> Do NOT ignore it. Do NOT just report it. Do NOT defer it to "a future task."
> This is a **financial platform handling real money.** A bug you walk past today is a bug that loses someone's investment tomorrow.
>
> ---
>
> **Severity Tiers — Determines How You Fix:**
>
> | Tier | What It Looks Like | Action |
> |------|-------------------|--------|
> | **P0 — CRITICAL** | Financial calculation wrong, money could be lost/duplicated, security hole, data corruption, broken transaction boundaries | **Stop your current task.** Fix this FIRST. Wrap in a DB transaction. Add a test. Verify with `cargo check` AND `cargo test`. Log in `BROKEN_LOGICS.md`. |
> | **P1 — HIGH** | Compilation error, route returns 500, wrong SQL column name, broken API contract, missing null/error check on user input | Fix immediately inline. Verify it compiles. Log in `BROKEN_LOGICS.md`. |
> | **P2 — MEDIUM** | UI glitch, wrong CSS class, misaligned layout, typo in user-facing text, missing loading state, console warning | Fix if it takes < 5 minutes. If longer, log it in `BROKEN_LOGICS.md` as unresolved and continue your task. |
>
> ---
>
> **Rules:**
>
> 1. **Fix on sight.** If you see broken code — fix it right then and there, even if it's outside your current task scope. The default is always to **fix it.**
>
> 2. **Financial code gets extra protection.** Any fix that touches monetary values (`BIGINT` cents), wallet balances, order processing, fee calculations, or payment flows **MUST:**
>    - Be wrapped in a SQL transaction (`BEGIN` / `COMMIT`)
>    - Never use floating-point math — cents only
>    - Include a before/after sanity check (e.g., balance shouldn't go negative)
>    - Be logged with `[P0-FINANCIAL]` tag in `BROKEN_LOGICS.md`
>    - If you are unsure about the business rule, **ask the user** — do not guess with money
>
> 3. **Verify the fix.** Every fix must be verified before moving on:
>    - **Rust backend:** Run `cargo check` (minimum) or `cargo test` (for P0)
>    - **Frontend:** Check the browser or confirm the CSS/JS change is syntactically correct
>    - **SQL migrations:** Confirm the migration applies without errors
>    - Never leave a fix unverified. An unverified fix is worse than no fix.
>
> 4. **Cascade check.** After fixing a bug, `grep` / `rg` the codebase for the same pattern. If the same mistake exists elsewhere, fix **every instance** in one pass. Common cascades:
>    - Wrong column name → search all queries using that table
>    - Missing error handling → search for other `unwrap()` calls in production paths
>    - Wrong CSS token → search for other hardcoded hex values that should use a variable
>    - Missing null check → search for similar unchecked `.get()` or `.parse()` calls
>
> 5. **Don't break other things.** Before fixing, read the surrounding code to understand the context. Check if other code depends on the current (broken) behavior. If the fix is risky:
>    - Make the smallest possible change
>    - If it touches a shared function, check all callers
>    - If it touches a DB schema, check all queries referencing that table
>
> 6. **Scope guardrail.** Self-healing is mandatory, but don't spiral. If you discover more than **3 unrelated P2 bugs** while working on a task, fix the first 3, log the rest in `BROKEN_LOGICS.md`, and return to your primary task. P0 and P1 bugs have no limit — always fix them all.
>
> 7. **Security bugs are always P0.** If you find any of these, treat as P0 regardless of context:
>    - SQL injection (raw string interpolation in queries)
>    - Missing authentication/authorization checks on routes
>    - Secrets or API keys hardcoded in source files
>    - User input directly rendered without sanitization (XSS)
>    - `unwrap()` on user-supplied data in production paths
>
> 8. **Log what you fixed.** After every fix, add a structured entry to `BROKEN_LOGICS.md`:
>    ```
>    ### [P0/P1/P2] — Short description
>    - **File:** `path/to/file.rs` (or `.html`, `.css`, `.js`)
>    - **What was wrong:** One sentence describing the bug
>    - **What I did:** One sentence describing the fix
>    - **Status:** ✅ Resolved
>    - **Date:** YYYY-MM-DD
>    ```
>    If the bug was already listed, update its status to ✅ Resolved with your fix description.

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
