# Villa-Returns Continuation Prompt

> Paste everything below this line into a fresh Claude/agent session.
> It is self-contained: project context, what's shipped, what's left, gotchas, conventions, and verification pattern.

---

## ROLE

You are a principal SaaS engineer continuing a multi-phase build on the **POOOL.app** platform — a tokenized fractional-ownership platform for Bali villas. The workstream is **Villa-Returns**: a data pipeline for monthly operations data, annual valuations, payouts, and investor performance KPIs.

The plan was designed in three documents; read them first before touching code:

- `/Users/martin/Projects/poool/drafts/villa-returns-implementation-plan.md` — master plan: 11 clarifying questions (Q1–Q11) **all locked-in**, schema design, API contract, KPI formulas, 8-phase roadmap, 10 carried assumptions
- `/Users/martin/Projects/poool/drafts/villa-returns-pages-outline.md` — page-by-page surface breakdown (15 surfaces: 8 extended + 4 new admin + 3 new developer)
- `/Users/martin/Projects/poool/drafts/villa-returns-workflows-and-wiring.md` — 15 end-to-end workflows (W1–W15) + frontend↔backend wiring contract + 15 cross-cutting invariants

Source PDF the work is based on: `~/Downloads/POOOL_Data_Model_Villa_Returns_EN (1).pdf` — re-read sections §2 (monthly inputs), §3 (annual valuations), §5 (POOOL calculations), §6 (target KPIs), §7 (NAV formula — **load-bearing**), §8 (NAV vs Market must stay separate).

## STACK

- Rust + Axum + SQLx, PostgreSQL 16, MiniJinja SSR templates, vanilla HTML+CSS+JS frontend (no SPA framework), PgBouncer sidecar, Cloud Run deploy via `cloudbuild.yaml`.
- All money = `BIGINT` cents. All percentages = `INT` bps (10000 = 100.00%). **No floats anywhere.** Multi-step monetary math uses `i128` intermediates.
- Session cookie `poool_session` → `user_sessions.session_token`. CSRF cookie `csrf_token` → header `X-CSRF-Token` on mutating requests. Both enforced by middleware.
- Auth extractors: `AdminUser` (admin/extractors.rs), `DeveloperUser` (developer/extractors.rs — new for Villa-Returns).
- Migration runner: `backend/src/lib.rs::run_migrations` — non-recursive scan of `database/`, alphanumeric, idempotent via `_schema_migrations` table. Rollback siblings live in `database/rollback/` (invisible to runner — subdir not scanned).
- Deploy: **`git push origin main`** → Cloud Build → Cloud Run. No manual gcloud. (Recorded in user memory.)

## WHAT IS ALREADY SHIPPED + VERIFIED

### Migrations (15 forward + 7 rollbacks, applied in dev)

| File | What |
|---|---|
| `database/130_asset_villa_ext.sql` | 11 nullable/defaulted columns on `assets` (tokenized_pct_bps, payout config, reserve_pct, withholding, allow_developer_submission, native_currency_code='IDR', mgmt_fee_bps) |
| `database/131_villa_expense_categories.sql` | Lookup table + 11 seeded categories |
| `database/132_villa_operations_log.sql` | Append-only monthly log + guard trigger |
| `database/133_villa_operations_current.sql` | Trigger-maintained current view + supersession upsert |
| `database/134_villa_valuations.sql` | Append-only annual valuations + guard trigger |
| `database/135_villa_forecast_assumptions.sql` | Mutable per-(asset, year) forecasts |
| `database/136_villa_market_prices_daily.sql` | NAV + Market daily snapshots (table only — daily job not written yet, see P4 below) |
| `database/137_villa_capex_events.sql` | CapEx events separate from OpEx |
| `database/138_villa_deduction_policy.sql` | Per-asset allowed-categories whitelist |
| `database/139_villa_period_documents.sql` | Link table for proofs |
| `database/140_fx_rates_daily.sql` | Daily FX snapshots |
| `database/141_villa_forecast_suggestions.sql` | Developer forecast suggestions |
| `database/142_developer_asset_links.sql` | Developer-to-villa assignment |
| `database/143_villa_log_guards_v2.sql` | **Bug-fix hot-patch**: tighten guards so published rows are immutable even without status flip |
| `database/144_villa_recorded_at_clock.sql` | **Bug-fix hot-patch**: `clock_timestamp()` default so rows in one tx get distinct timestamps |
| `database/145_dividend_payouts_villa_period.sql` | Adds `period_year`, `period_month`, `source_villa_operations_log_id` + partial unique index for idempotent distribute |

### Backend modules (8 new, all live)

- `backend/src/developer/extractors.rs` — `DeveloperUser` extractor + `require_asset_link`
- `backend/src/admin/villa_operations.rs` — full operations CRUD + state machine (create/update/submit/approve/publish/reject/distribute/queue/list)
- `backend/src/admin/villa_valuations.rs` — full valuations CRUD + NAV preview endpoint + supersession on publish
- `backend/src/admin/villa_developer_access.rs` — grant/revoke developer access
- `backend/src/developer/villa_operations.rs` — dev-restricted CRUD + dashboard + asset-config read
- `backend/src/assets/villa_performance.rs` — public investor KPI endpoint (latest NAV, 12m distributable, annual yield)
- Existing modules extended: `backend/src/admin/mod.rs` (route wiring), `backend/src/admin/pages.rs` (page handlers), `backend/src/developer/mod.rs`, `backend/src/developer/routes.rs`

### Frontend pages (4 new admin + 2 new developer + 1 admin extension)

| Path | Status |
|---|---|
| `frontend/platform/admin/villa-operations-entry.html` + JS | Live |
| `frontend/platform/admin/villa-operations-queue.html` + JS | Live, verified end-to-end |
| `frontend/platform/admin/villa-valuation.html` + JS | Live, verified with NAV preview + supersession |
| `frontend/platform/developer/operations-dashboard.html` + JS | Live |
| `frontend/platform/developer/operations-submit.html` + JS | Live |
| `frontend/platform/admin/asset-details.html` Operations tab (extension) | Live, hydrated by `admin-asset-operations.js` |

### Verified workflows (live in browser smoke tests)

- W2 (developer submission path) — endpoints work; live UI test deferred until a Developer role user exists
- W3 (admin approve → publish) — full chain verified, trigger upserted `_current`, USD frozen
- W4 (distribute payouts) — `dividend_payouts` rows created pro-rata, idempotent
- W5 (admin override path) — endpoint exists, smoke deferred
- W7 (correction after publish, supersession chain) — verified via psql + Valuation supersession verified live via UI
- W8 (annual valuation publish + supersede prior) — verified live

### 4 bugs caught + patched during shipping (cumulative)

1. **Trigger hole** (migration 143): published rows were mutable if UPDATE had no status flip → tightened guard.
2. **`recorded_at` tx-collision** (migration 144): `NOW()` returns same value in one tx → switched default to `clock_timestamp()`, added `id DESC` tiebreaker to admin + developer list queries.
3. **`ON CONFLICT ON CONSTRAINT`** doesn't work with partial unique index — must use `ON CONFLICT (cols) WHERE predicate` (inferred form).
4. **Route shadowing** under `/api/admin/approvals/*` — `/:id/approve` and `/:id/reject` patterns interact with axum's matchit to swallow single-segment literals at that depth. Moved P2.3 queue endpoint to `/api/admin/villa-operations-queue`.

## WHAT IS LEFT — IN PRIORITY ORDER

**Track A — finish the admin/developer surface** (small slices, mostly UI):

1. **A1 Valuations panel** — extend `admin-asset-operations.js` to add a "Valuations" section beside Operations strip. List published valuations + "New valuation" CTA → navigates to `/admin/villas/:id/valuations/new`. ~30 lines.
2. **B4 admin history viewer** — new page `frontend/platform/admin/villa-history.html` showing per-period supersession chain with field-level diff. Backend: extend `villa_operations.rs` with `api_admin_villa_operations_history` (returns chain for a `(asset, year, month)`). Plan §3.1 has the example trace.
3. **B3 deduction policy admin page** — new page + admin endpoint to set `villa_deduction_policy` rows. Validates against `villa_expense_categories`.
4. **Top-up CTA** on A1 (Q11) — when superseded month's payout delta > 5%, surface "Top up $X" button → creates `dividend_payouts.payout_type='bonus'` row.
5. **C3 developer annual data** — new page `frontend/platform/developer/annual-data.html` for CapEx submit + tax statement upload + forecast suggestions. Backend: `backend/src/developer/villa_capex.rs` (new) + `backend/src/developer/forecast_suggestions.rs` (new).
6. **A3 approvals queue** — extend admin/villa-operations-queue.html (or add a tab) for villa_valuations + villa_capex queues.

**Track B — investor-facing P3 continuation** (the big-impact next slice):

7. **Performance tab on `property.html`** — endpoint `/api/villas/:id/performance` is **already live**; UI not yet wired. Add a third `financial-tab` "Live performance" to the Financials section (lines 286–328 in property.html). Hydrate via new JS file `static/js/property-performance.js` reading asset id from a data attribute injected by MiniJinja.
8. **NAV history + Market price chart** — PDF §8 hard rule: two separate series, never merged. Needs chart library or simple SVG (the codebase has Chart.js in places; check `static/js/admin-charts-loader.js`). For NAV chart, query valuations history + compute NAV at each publish date. For Market price, query `trade_history` aggregated daily.
9. **5-Year Total Return KPI** — needs forecast_assumptions data (table empty in dev). UI shows "Projected" badge if no data.
10. **As-of date picker** — every read endpoint already accepts `as_of`; just plumb through the UI.
11. **`poool_app_home.html` + `my-trading.html`** — extend with portfolio Annual Yield + lifetime Net Return cards. Backend endpoint `/api/investors/me/portfolio?as_of=` (new).
12. **`transactions.html`** — add `type=distribution` filter for dividend_payouts.
13. **Public listing `property-public.html`** — minimal Annual Yield + Projected Return cards (cached, no time-travel).

**Track C — infrastructure** (lower priority, plan §8 phases 4-8):

14. **P4 — KPI calc layer + daily NAV snapshot job** — populate `villa_market_prices_daily` via a tokio interval task. NAV recomputed when valuation publishes (immediate snapshot) + nightly at 00:30 UTC.
15. **P4 — Shadow-write to legacy `asset_financials`** — every publish writes to both schemas during transition. Plan §6.4.
16. **P5 — Investor UI cut-over** — read path switches to new layer for pilot assets behind `settings.villa_returns.enabled = 'on'` + `assets.villa_returns_pilot = TRUE`.
17. **Notifications integration (W15)** — fire `notifications` rows on submit (→ admin), approve (→ dev), publish material correction (→ investors).
18. **P6/P7/P8** — full cut-over, deprecate legacy, drop `asset_financials`. Not yet started.

## CRITICAL GOTCHAS — DO NOT REDISCOVER

- **PDF §7 NAV formula is `(valuation × tokenized_pct / 10000) / (tokens_total − tokens_owner_retained)`** — never `valuation / tokens_total`. The `compute_nav_preview` helper in `admin/villa_valuations.rs` is the canonical implementation.
- **PDF §8: NAV and Market token prices are two SEPARATE series in every UI chart** — there is no merged line. Enforce in any new chart code.
- **`recorded_at` defaults to `clock_timestamp()`** (not `NOW()`) — preserved by migration 144. If you add a new append-only table, copy the pattern: `recorded_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()`.
- **4-eyes**: `CHECK (approved_by IS NULL OR approved_by <> submitted_by)` is enforced at the DB level. UI must show "you submitted this row" disabled state on Approve to avoid guaranteed-409 clicks.
- **Append-only enforcement**: post-publish data mutation raises a trigger exception. Only `status='superseded'` flip is allowed, and even then only via the supersession flow. Don't bypass with raw SQL.
- **CSRF on every mutating fetch**: all new JS modules must include `X-CSRF-Token` header from the `csrf_token` cookie. The pattern is inlined per-file (`csrfHeaders()` helper) — match the existing files, do not add a shared module without checking.
- **JS file naming**: HTML `admin/foo-bar.html` ↔ JS `static/js/admin-foo-bar.js` ↔ Rust handler `page_admin_foo_bar`. Required for tooling.
- **No `#[derive(sqlx::FromRow)]` on inner function-scope structs named `R`** — type parameter `R` in `query_as` shadows. Use tuple destructuring or move the struct to module scope.
- **Public-doc lint**: every `pub mod foo` in `admin/mod.rs` and `developer/mod.rs` must either have a `///` doc comment or `#[allow(missing_docs)]`. The crate has `#![deny(missing_docs)]`.
- **Routing**: avoid path patterns that conflict with `/api/admin/approvals/:id/*` — single-segment literals at that depth get silently swallowed by axum's matchit. Test new routes by hitting them with no auth and confirming **401 JSON**, not 200 HTML.

## CODEBASE CONVENTIONS

- **Error type**: API handlers return `Result<Json<T>, ApiError>` where `ApiError` is defined in `backend/src/admin/extractors.rs`. Use `ApiError::BadRequest(msg)`, `ApiError::Conflict(msg)`, `ApiError::NotFound(msg)`, `ApiError::Forbidden(msg)`, `ApiError::Database(err)`. Internal errors logged + Sentry-captured automatically by `IntoResponse`.
- **Audit logs**: every state transition writes to `audit_logs` via `INSERT INTO audit_logs (actor_user_id, action, entity_type, entity_id, previous_state, new_state) VALUES (...)`. Match the existing pattern in `villa_operations.rs::write_audit`.
- **Page handler pattern**: Rust handler just calls `render_admin_template(&state, "admin/foo.html")` — no data context. JS reads URL params from `window.location.pathname` and hydrates via `fetch`. Initial-data SSR injection only when first-paint matters (B4 history viewer might need it).
- **Comparables/JSONB**: pass `Option<serde_json::Value>` from handler signature; let serde do the work.
- **Cents/bps**: `i64` for money, `i32` for bps. `i128` for intermediates. Never `f32`/`f64` anywhere monetary.

## VERIFICATION WORKFLOW (proven during P2 — keep using)

You have access to MCP preview tools (`preview_start`, `preview_screenshot`, `preview_eval`, `preview_logs`). Pattern that works:

1. `preview_start({name: "backend"})` — config in `.claude/launch.json`, port 8888. Wait for "Server listening" in logs (50-60s on first build, faster on hot restarts).
2. **Inject a session** to bypass the auth gate without a login form:
   ```sql
   INSERT INTO user_sessions (user_id, session_token, expires_at, is_2fa_verified)
   SELECT id, '<known-token>', NOW() + INTERVAL '1 hour', TRUE
   FROM users WHERE email = 'admin@poool.app';
   ```
3. Set cookies in the preview browser:
   ```js
   document.cookie = 'poool_session=<known-token>; path=/';
   document.cookie = 'csrf_token=<anything>; path=/';
   ```
4. Drive the UI via `preview_eval` (click buttons, set input values, navigate).
5. **Always cleanup**: `DELETE FROM user_sessions WHERE session_token='<known-token>';`

For trigger-layer invariants, use psql directly — see `/tmp/villa_ops_smoke_test.sql` for the pattern (9 tests in one transaction, ROLLBACK at end). Don't try to test triggers via the API alone.

## EXISTING TEST DATA YOU CAN USE

- Test asset: `fad407a3-b106-4903-bd4e-0772fc94c78e` ("Premium Bali Rice – Harvest Cycle Q2 2026") — has `tokenized_pct_bps=2000`, `tokens_total=1000`, `tokens_owner_retained=50`, 5 investments, 1 published valuation (id=2 with id=1 superseded), 1 published operations row (id=14 / 2026-04, distributable=4.085B IDR), 2 dividend_payouts rows.
- Admin users: `admin@poool.app` (id `c87443dc-b777-47b2-a1f0-e345c92e1b47`), `e2e-admin-c9f24e87@poool.app` (id `d212d51a-bb24-4ea8-8b83-4fb1f8198092`) — use these for 4-eyes testing.
- No Developer-role user exists yet. To smoke-test the developer flow, create one:
  ```sql
  INSERT INTO user_roles (user_id, role_id, is_active)
  SELECT (SELECT id FROM users WHERE email='admin@poool.app'),
         (SELECT id FROM roles WHERE name='developer'),
         TRUE
  ON CONFLICT DO NOTHING;
  -- Then link to the test asset:
  INSERT INTO developer_asset_links (developer_user_id, asset_id, granted_by)
  SELECT id, 'fad407a3-b106-4903-bd4e-0772fc94c78e', id
  FROM users WHERE email='admin@poool.app';
  ```

## STYLE

- **Caveman mode** (terse) is active in this project's hook. Status updates one sentence per moment. Plan/spec doc writing stays in normal prose. Code stays normal.
- **No TodoWrite** — the reminder fires but is ignored per instructions. Single coherent slice = no todo list needed.
- **Don't deploy** without the user's explicit say-so. Per their memory: `git push origin main` is the deploy command, but they decide when.
- **Spawn each slice as a coherent unit**: pick one item from the priority list above, ship it, verify via the workflow above, report. Don't try to bundle multiple Track-A items unless they're truly intertwined.
- **Match shipped patterns**. The 4 bugs above are all caught examples of where the existing pattern matters — don't invent new ones.

## START

Begin by:
1. Reading the three planning docs in `/Users/martin/Projects/poool/drafts/villa-returns-*.md` to load full context.
2. Picking the highest-priority remaining item that matches your effort budget (Track A items are S/M, Track B-7 is S, Track C is L).
3. Stating which item you're picking and why, in one sentence.
4. Building + smoke-verifying via the proven pattern.
5. Reporting cumulative state at the end of the slice, listing what's now shipped and what's next.

Do not redo planning; the plan is locked. Do not re-question Q1–Q11; the answers are confirmed. Ship.
