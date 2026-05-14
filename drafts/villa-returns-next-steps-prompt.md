# Villa-Returns — Next-Steps Execution Prompt

> Paste everything below `## ROLE` into a fresh Claude Code / agent session.
> Self-contained: explains the workstream, the exact current state, the
> codebase facts you need, and the step-by-step work remaining.

---

## ROLE

You are a senior full-stack engineer continuing the **Villa-Returns** workstream
on POOOL.app — a tokenized Bali villa fractional-ownership platform. The data
model, backend, frontend, schema, and cut-over infrastructure are **already
built and deployed to production behind a feature flag that is OFF**. Your job
is to execute the remaining, well-scoped work: ship the last few commits, close
two decision-gated gaps, and run verification — **without** touching the
production cut-over, which is operator-driven.

You do **not** flip feature flags. You do **not** deploy or push without
explicit approval. You do **not** run the Phase 3 cut-over SQL.

## CONTEXT — read these first, in order

1. `drafts/villa-returns-final-audit.md` — **rev 2, accurate.** Spec vs shipped, 90% coverage, the open-items list.
2. `drafts/villa-returns-cutover-runbook.md` — operational runbook for the production cut-over (Phase 4 below — *not yours to execute*).
3. `drafts/villa-returns-implementation-plan.md` — original master plan + the Q1–Q11 lock-ins.
4. `drafts/villa-returns-cutover-step-c2.sql` — the operator's pilot cut-over script (*not yours to run*).

## STACK + CODEBASE FACTS

- Repo: `/Users/martin/Projects/poool`, default branch `main`.
- **Deploy is manual:** `gh workflow run deploy.yml -f environment=production`. Pushing to `main` does NOT auto-deploy. Cloud Run service `poool-backend`, region `europe-west1`.
- **CI** (`.github/workflows/ci.yml`, runs on push to main): `cargo fmt --all -- --check`, `cargo clippy -- -D warnings`, `cargo check --locked`, `cargo test --locked`, `cargo audit`, `python3 scripts/check_links_ci.py`, Docker build.
  - CI runs with `SQLX_OFFLINE=true` — if you add/change any compile-time `sqlx::query!`/`query_as!` macro you must regenerate `backend/.sqlx/` with `DATABASE_URL=postgres://martin@localhost/poool cargo sqlx prepare -- --tests` and commit it. (The Villa-Returns code uses **runtime** `sqlx::query(...)` — those need no cache.)
  - rustfmt is **1.95**; keep your local toolchain current (`rustup update stable`) or CI fmt will diverge.
- Stack: Rust + Axum + SQLx + PostgreSQL 16 + MiniJinja SSR + vanilla HTML/JS. Money = `BIGINT` cents, percentages = `INT` bps, `i128` intermediates, no floats.
- Auth: session cookie `poool_session`; CSRF via `X-CSRF-Token` header read from the `csrf_token` cookie. `AdminUser` + `DeveloperUser` extractors; `DeveloperUser::require_asset_link(&pool, asset_id)` enforces the `developer_asset_links` model.
- Migrations: `database/NNN_*.sql`, auto-run on container startup (`run_migrations` in `backend/src/lib.rs`), rollback siblings in `database/rollback/`. Highest applied is **148**. The migration runner does a non-recursive scan of `database/` — files in `database/rollback/` are invisible to it. Two `.PENDING` files (`database/rollback/148_rename_*`, `149_drop_*`) are C5 stagings — leave them.

## CURRENT STATE — verify before you start

Run `git log --oneline -8 origin/main` and `git status`. Expected:

- **Production is deployed at commit `1dbbae5`.**
- `origin/main` tip is `b3e56ad`. **Six commits are pushed but NOT yet deployed:**
  - `2dc1570` docs — C2 cut-over SQL helper *(no runtime effect)*
  - `bad0dd8` docs — audit refresh rev 2 *(no runtime effect)*
  - `5dbee19` feat — B1 admin period-document upload UI
  - `96aada9` feat — A2 Villa-Returns config card on asset-details Operations tab
  - `14c1e7d` feat — developer-accessible period-document upload backend endpoints
  - `b3e56ad` feat — C2 developer doc-upload UI
- **Production feature-flag state:** `platform_settings.villa_returns.enabled = 'off'`, every asset `villa_returns_pilot = FALSE` (migration 148 forced this safe default). Nothing Villa-Returns is investor-visible. **Do not change this** — the cut-over is the operator's call.
- The working tree has **pre-existing dirty files unrelated to Villa-Returns** — do NOT stage or commit them: `backend/src/{assets/models.rs,cart/routes.rs,payments/routes.rs,payments/service.rs,rewards/service.rs,settings/service.rs,storage/service.rs}`, `contracts/lib/*`, `database/*seed*.sql`, `database/00*.sql`, `database/full_migration.sql`, `database/publish_article*.sql`, `database/120_*.sql`, `frontend/platform/{40*,500,account-*,affiliate-*,checkout}.html`, `docs/IMPLEMENTATION_ROADMAP.md`, `uv.lock`, and the deleted `.claude/worktrees/*` entries. These predate this workstream.

## KEY FILES (Villa-Returns)

| Area | Files |
|---|---|
| Admin backend | `backend/src/admin/villa_operations.rs` (state machine, config CRUD, distribute, top-up, doc-link), `villa_valuations.rs`, `villa_capex.rs`, `villa_forecast.rs`, `villa_nav_snapshot.rs`, `villa_deduction_policy.rs`, `villa_developer_access.rs` |
| Developer backend | `backend/src/developer/villa_operations.rs` (restricted CRUD + **period-document upload+link endpoints**), `extractors.rs`, `villa_capex.rs`, `forecast_suggestions.rs` |
| Investor backend | `backend/src/assets/villa_performance.rs` (performance + history APIs, feature-flag + per-asset pilot gate at the top of `api_villa_performance`), `backend/src/portfolio/villa_summary.rs` |
| Ops automation | `backend/src/lib.rs` (tokio NAV-snapshot cron, `VILLA_NAV_SNAPSHOT_ENABLED` env gate), `backend/src/bin/backfill_villa_operations.rs` |
| Admin frontend | `frontend/platform/admin/villa-operations-entry.html` (B1 — has doc-upload), `villa-operations-queue.html`, `villa-valuation.html`, `villa-deduction-policy.html`, `villa-history.html`, `asset-details.html` (Operations tab — 5 cards incl. the config card) + matching `static/js/admin-villa-*.js`, `admin-asset-operations.js` |
| Developer frontend | `frontend/platform/developer/operations-dashboard.html`, `operations-submit.html` (C2 — has doc-upload), `annual-data.html` (C3 — tax-statement upload still a placeholder) + `static/js/developer-*.js` |
| Investor frontend | `frontend/platform/property.html` Performance tab, `property-public.html`, `portfolio.html`, `my-trading.html` + matching JS |

## KEY TABLES

`villa_operations_log` (append-only, state machine `draft→submitted→approved→published→superseded`, 4-eyes `CHECK (approved_by IS NULL OR approved_by <> submitted_by)`), `villa_operations_current`, `villa_valuations`, `villa_capex_events`, `villa_deduction_policy`, `villa_forecast_assumptions`, `villa_forecast_suggestions`, `villa_period_documents` (link table, **unique index on `(asset_id, period_year, period_month, document_id)` — `period_month` is mandatory in the key**), `villa_market_prices_daily`, `fx_rates_daily`, `developer_asset_links`. `assets` is extended with `tokenized_pct_bps`, `tokens_owner_retained`, `tokens_payout_eligible`, `reserve_pct_bps`, `mgmt_fee_bps`, `withholding_tax_bps`, `payout_frequency`, `payout_currency`, `distribution_record_day`, `allow_developer_submission`, `villa_returns_pilot`. `platform_settings` row `key='villa_returns.enabled'`, value ∈ `{off, shadow, on}`.

---

## THE WORK — execute in order

### PHASE 1 — Ship the 4 undeployed code commits

The four feature commits (`5dbee19`, `96aada9`, `14c1e7d`, `b3e56ad`) are on
`origin/main` but production is still at `1dbbae5`. They are flag-gated /
admin-and-developer-only, so deploying them changes nothing investor-facing.

1. Confirm CI is green on `b3e56ad`: `gh run list --workflow=ci.yml --limit=1`.
2. **Ask the user to confirm**, then: `gh workflow run deploy.yml -f environment=production`.
3. Watch with `gh run watch <id> --exit-status`. On green, verify `curl -s -o /dev/null -w "%{http_code}" https://platform.poool.app/health` returns `200`.
4. Migrations 130–148 are already applied in prod and unchanged — the deploy re-runs them as idempotent no-ops. Confirm the service booted (health 200 = migrations passed).

### PHASE 2 — Two decision-gated gaps

Both need **one decision from the user** before you write code. Ask using a
concise multiple-choice question, then implement the chosen option.

**2a. C3 — annual tax-statement upload** (`frontend/platform/developer/annual-data.html`
currently has placeholder text at the "Annual tax statement" section). The
blocker: `villa_period_documents` is keyed by `(asset_id, period_year,
period_month, document_id)` and `period_month` is mandatory — an *annual*
document has no month. Present these options:
  - **(a)** Make `period_month` nullable on `villa_period_documents` + adjust the unique index to treat NULL-month rows as annual. Migration + the existing developer endpoint generalised.
  - **(b)** Use a sentinel `period_month = 0` for annual docs. No schema change; a convention. Cheapest, slightly hacky.
  - **(c)** New `villa_annual_documents` table + a dedicated developer endpoint. Cleanest, most code.
  Once chosen: write the migration (if any) + rollback sibling, add/extend the developer endpoint mirroring the pattern in `backend/src/developer/villa_operations.rs::api_developer_villa_operations_upload_document` (multipart `file` + `doc_type`, `require_asset_link` guard, GCS upload via `crate::storage::service::upload_private` with `upload_local` fallback, insert `asset_documents` under the generic `'financial'` `document_type`), register the route in `backend/src/developer/mod.rs`, and wire the upload panel into `annual-data.html` + `static/js/developer-annual-data.js` (mirror the C2 panel in `operations-submit.html` / `developer-operations-submit.js`).

**2b. FX rate populator** — `fx_rates_daily` exists with only a placeholder
seed (migration 147). Present FX-provider options (e.g. exchangerate.host —
free/no-key; OpenExchangeRates — key; Wise API — key). Once chosen: add a
nightly `tokio` interval task in `backend/src/lib.rs` modelled exactly on the
existing NAV-snapshot cron (env-gated — add `VILLA_FX_POPULATOR_ENABLED`,
`MissedTickBehavior::Skip`, skip the initial tick), fetching the IDR→USD rate
(and any other pairs present in `assets.payout_currency`) and upserting into
`fx_rates_daily`. Keep all rate math in bps/integers.

### PHASE 3 — Verification (needs the local app + dev DB running)

The dev DB is `postgres://martin@localhost/poool`. Start the app with the
project's normal local command. Then:

5. **W5 admin override** — the only workflow from `drafts/villa-returns-workflows-and-wiring.md` never browser-verified. Exercise the override path on a published `villa_operations_log` row through the admin UI; confirm it creates a superseding row and writes `audit_logs`.
6. **Doc-upload smoke** — exercise the new B1 (`villa-operations-entry.html`) and C2 (`operations-submit.html`) period-document panels end-to-end: upload a small PDF, confirm the `villa_period_documents` row + `asset_documents` row are created and the download link resolves. If you built 2a, smoke C3 too.
7. If the `test-developer` skill is available, use it for the E2E run.

### PHASE 4 — Production cut-over — **DO NOT EXECUTE. Document only.**

This is the operator's job, multi-week, and gated on monitoring windows. It is
described fully in `drafts/villa-returns-cutover-runbook.md` and scripted in
`drafts/villa-returns-cutover-step-c2.sql`. Summary so a reader knows what's
pending: flip `enabled='shadow'` + one pilot asset's `villa_returns_pilot=TRUE`
→ admin publishes that villa's real months → monitor shadow-write divergence vs
legacy `asset_financials` for 14 days → expand to ~25% → expand to 100% +
`enabled='on'`. **If the user explicitly asks you to help with cut-over, walk
them through the runbook — do not run the SQL yourself.**

### PHASE 5 — Legacy retirement — gated, not now

After 90 consecutive days of zero reads on `asset_financials`: renumber and
apply `database/rollback/148_rename_asset_financials_deprecated.sql.PENDING`
(it will need a fresh migration number — 148 is taken; use the next free one).
30 days after the rename, with a verified GCS cold-storage backup, apply the
`149_drop` PENDING migration. Just leave the `.PENDING` files where they are.

### PHASE 6 — Optional: multi-currency FX conversion (no deadline)

`api_admin_villa_operations_distribute` in `backend/src/admin/villa_operations.rs`
hard-fails for `payout_currency` other than `USD`/`IDR`. When a EUR/USDT villa
onboards, extend the match block to look up the latest `fx_rates_daily` rate and
convert `distributable_idr_cents`. Depends on Phase 2b being done first.

---

## WORKFLOW RULES

- After **any backend change**: from `backend/`, run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo check --all-targets`, and `cargo test --lib` locally before committing. If you added compile-time sqlx macros, regenerate `.sqlx/`. To verify against exactly what CI sees, `git add` only your files then `git stash push --keep-index` the pre-existing dirty files, run the checks, `git stash pop`.
- Commit in small logical units. Standard Git messages with the trailer `Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>` via HEREDOC. **Stage explicit paths — never `git add -A` / `git add .`** (the working tree has unrelated dirty files).
- Frontend changes: `node --check <file>.js` and `python3 scripts/check_links_ci.py` before committing.
- After pushing, watch CI (`gh run watch`). Fix forward — never `--amend`, never `--no-verify`, never force-push.

## SAFETY RULES — do not violate

1. **Never `git push` or `gh workflow run deploy.yml` without an explicit "yes" from the user.** Stage + commit freely; stop and ask before push/deploy.
2. **Never change `platform_settings.villa_returns.enabled` or any asset's `villa_returns_pilot`.** The cut-over is operator-driven.
3. **Never run `drafts/villa-returns-cutover-step-c2.sql`** or any cut-over SQL against any database.
4. **Never stage the pre-existing dirty files** listed under "Current state" or the `.claude/worktrees/*` deletions.
5. Never commit `.env`, secrets, or `/tmp/*.sql`.
6. Never `--amend`, rebase, or force-push. New commits only.
7. The `.PENDING` migration files are intentional — do not rename or apply them.

## START

1. Read the 4 context docs.
2. `git log --oneline -8 origin/main` + `git status` — confirm the state matches "Current state" above.
3. Confirm Phase 1 CI is green, then **ask the user** to approve the deploy.
4. For Phase 2, **ask the user** the two decisions (2a, 2b) before writing code.
5. Work the phases in order. Terse status between steps. Stop and ask on any ambiguity.
