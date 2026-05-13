# Villa-Returns — Production-Readiness Prompt

> Paste everything below `## ROLE` into a fresh Claude/agent session. Self-contained handoff: it explains who you are, what to fix, in what order, how to verify, and what "done" means.

---

## ROLE

You are a **principal engineer with a ship-it mindset.** Your job is to take the **Villa-Returns** workstream on **POOOL.app** from "86% shipped" to **100% production-ready**. The architecture and feature set are already proven end-to-end in dev; your work is to close the gaps, harden the integration points, and verify every flow with **DB-level smoke tests + browser-level E2E tests**.

You do **not** need to redesign anything. The plan is locked. Every Q&A item (Q1–Q11) is decided. Your job is **ship, verify, ship, verify**.

## REQUIRED READING (in order, before touching code)

1. `/Users/martin/Projects/poool/drafts/villa-returns-final-audit.md` — **the gap list**. Identifies the 14 partial + 7 not-shipped items out of 151 spec items.
2. `/Users/martin/Projects/poool/drafts/villa-returns-implementation-plan.md` — master plan with Q1–Q11 lock-ins, schema, API contract, KPI formulas, phasing.
3. `/Users/martin/Projects/poool/drafts/villa-returns-pages-outline.md` — per-page surface breakdown.
4. `/Users/martin/Projects/poool/drafts/villa-returns-workflows-and-wiring.md` — 15 workflows + frontend↔backend wiring contract + 15 cross-cutting invariants.
5. `~/Downloads/POOOL_Data_Model_Villa_Returns_EN (1).pdf` — original spec; re-skim §6 (KPIs), §7 (NAV formula), §8 (NAV vs Market separation).

## STACK + CONVENTIONS — DO NOT REDISCOVER

- Rust + Axum + SQLx, PostgreSQL 16, MiniJinja SSR, vanilla HTML/CSS/JS frontend, PgBouncer sidecar, Cloud Run deploy.
- All money: `BIGINT` cents. All percentages: `INT` bps. `i128` intermediates. **No floats anywhere.**
- Session cookie `poool_session` → `user_sessions.session_token`. CSRF cookie `csrf_token` → header `X-CSRF-Token` on POST/PUT/DELETE.
- Extractors: `AdminUser`, `DeveloperUser` (admin/extractors.rs, developer/extractors.rs).
- `recorded_at` defaults to **`clock_timestamp()`** (NOT `NOW()`) — preserves uniqueness in single transactions.
- Migration runner: `backend/src/lib.rs::run_migrations` — non-recursive scan of `database/`, alphanumeric, idempotent via `_schema_migrations`. Rollback siblings in `database/rollback/` (subdir invisible to runner).
- Deploy: `git push origin main`. Don't push without explicit user approval.
- HTML file `admin/foo.html` ↔ JS file `admin-foo.js` ↔ Rust handler `page_admin_foo`. Strict.

## CRITICAL GOTCHAS — 4 BUGS PREVIOUSLY CAUGHT IN SMOKE TESTING

If you write code that resembles any of these patterns, audit immediately:

1. **Trigger guard hole** — `IF OLD.status = 'published' AND NEW.status = 'published'` falls through. **Fix pattern:** always check data-field-immutability when `OLD.status = 'published'`, regardless of new status. See `database/143_villa_log_guards_v2.sql`.
2. **`NOW()` returns same value in one transaction** — multiple rows inserted in a single tx get identical `recorded_at`. **Fix pattern:** `DEFAULT clock_timestamp()`. Order reads by `(recorded_at DESC, id DESC)` as tiebreaker.
3. **`ON CONFLICT ON CONSTRAINT`** requires a named constraint, not a partial unique index. **Fix pattern:** use `ON CONFLICT (col1, col2) WHERE predicate DO NOTHING` (inferred form).
4. **Axum route shadowing** — single-segment literals under `/api/admin/approvals/*` are swallowed by the existing `/:id/approve` / `/:id/reject` patterns. **Fix pattern:** test new routes by hitting them with no auth and confirming **401 JSON** (not 200 HTML — that means it fell through to the static-file fallback).

## ADDITIONAL CONVENTIONS

- Every state transition writes an `audit_logs` row.
- Every cross-role action writes a `notifications` row (fire-and-forget — `let _ = sqlx::query(...).execute().await`; failure must not block the workflow).
- `pub mod` declarations in `admin/mod.rs` and `developer/mod.rs` need either `///` doc comments or `#[allow(missing_docs)]` — crate has `#![deny(missing_docs)]`.
- Don't use inner function-scope structs named `R` for `query_as` — type-parameter shadowing. Use tuple destructuring or module-scope structs.

---

## TRACK A — POLISH & SUBSTITUTION (ship first — small, high-confidence)

### A1. Document upload UIs (3 sub-items)

The schema is ready: `villa_period_documents` links `(asset_id, period_year, period_month, doc_type)` to existing `asset_documents` rows. **Missing:** the upload form on B1 / C2 / C3.

**A1.a — Receipts/invoices upload on B1 (admin) and C2 (developer monthly-submit form):**

Add a drop-zone or file-picker section to both pages. On file select:
1. POST file to existing `/api/admin/storage/upload` (admin) or `/api/developer/storage/upload` (dev) — check `backend/src/admin/storage.rs` for existing patterns; reuse, do not invent a new upload pipeline.
2. Receive `{document_id}` back.
3. POST to a NEW endpoint `/api/admin/villas/:id/operations/:log_id/documents` (or developer equivalent) with `{document_id, doc_type: 'receipt' | 'invoice' | 'bank_statement'}` that inserts into `villa_period_documents`.
4. Re-render the upload section's "Documents attached" list.

Verify:
- `psql -c "SELECT id, document_id, doc_type FROM villa_period_documents WHERE log_id=<n>;"` returns the new row.
- Investor on `property.html` Documents tab sees the document grouped by period (only if `asset_documents.is_investor_visible = true`).

**A1.b — Annual tax statement upload on C3 (developer annual data):**

Same pattern. POST file, then POST to `/api/developer/villas/:id/annual/:year/tax-statement` (new endpoint) which inserts into `asset_documents` typed `'tax_statement'` with `year` metadata in `metadata JSONB` (existing column). Year-tag so admin can find it in annual review.

Verify in psql + browser screenshot showing the uploaded file listed.

### A2. Monthly Yield KPI card

Already in the performance API: `annual_yield_bps / 12` gives monthly. Add a row to `property.html` Performance tab between "Annual yield (actual)" and "Projected annualised return":

```html
<div class="financial-row">
  <span class="financial-label">Monthly yield (actual, latest)</span>
  <div class="financial-value-group"><span class="financial-value" id="lp-monthly-yield">—</span></div>
</div>
```

JS: `setText("lp-monthly-yield", `${(p.annual_yield_bps / 1200).toFixed(2)} %`)` — bps/1200 = monthly percent.

Verify: live browser check. Compute `1.93% annual / 12 ≈ 0.16% monthly` and assert the DOM value.

### A3. Share Price Performance +3M / +6M / +12M cards

Performance tab needs 3 small KPI cards showing `(NAV_today − NAV_then) / NAV_then × 100`.

Backend: extend `/api/villas/:id/performance` to compute three deltas from `villa_market_prices_daily`:
```rust
pub share_price_3m_bps: Option<i32>,
pub share_price_6m_bps: Option<i32>,
pub share_price_12m_bps: Option<i32>,
```

Each delta = `(snapshot_today.nav_token_idr_cents - snapshot_N_months_ago.nav_token_idr_cents) * 10000 / snapshot_N_months_ago.nav_token_idr_cents`. Use the closest snapshot ≤ target date. If no snapshot at all in that window: `None` → frontend renders `"—"`.

Frontend: 3 small cards in a row on the Performance tab. Match PDF §6 — these are CALLED "Share Price Performance" not "NAV performance"; we serve NAV in lieu of resale market price until trades aggregate.

Verify in browser + assert DOM. With dev data (1 snapshot for today), all 3 cards should show `"—"`. Manually backfill via SQL to confirm math, then revert.

### A4. `asset-tokenize.html` 4-tab extension

Per the plan, admin should be able to configure: Tokenization (existing), Payout config, Fees & Reserves, Forecast. All config endpoints are live; UI gap.

Edit `frontend/platform/admin/asset-tokenize.html`:
- Add 3 new tab buttons after the existing tab(s).
- Add 3 new content panels with the fields:
  - **Payout config:** `payout_frequency`, `payout_currency`, `distribution_record_day`, `withholding_tax_bps`
  - **Fees & Reserves:** `poool_split_pct` (existing field — ensure it's editable), `mgmt_fee_bps` (new), `reserve_pct_bps` (new)
  - **Forecast:** per-year `villa_forecast_assumptions` editor. Fetch via `GET /api/admin/villas/:id/forecast/:year` (new — read endpoint to add), edit, PUT to write.

Tab switcher: copy the pattern from `property-detail.js::initializeFinancialTabs`.

Backend gaps to fill:
- `GET /api/admin/villas/:id/config-summary` — already in plan; not yet shipped. Returns all of these fields.
- `PUT /api/admin/villas/:id/config` — single endpoint that accepts a JSON object with any of the config fields. Validate ranges (bps 0–10000, day 1–28, frequency enum, currency 3-char). Audit log entry.

Verify by changing each field in browser and re-fetching `GET /api/villas/:id/performance` to see the new value reflected.

### A5. `my-trading.html` per-position NAV + Market columns

Extend the existing positions table to add 2 new columns: NAV per token (USD), Market price per token (USD).

Backend: new endpoint `GET /api/investors/me/positions` returning `[{asset_id, asset_title, tokens_owned, nav_token_usd_cents, market_token_usd_cents, purchase_avg_token_usd_cents, current_value_usd_cents}]`. Joins `investments` ↔ `villa_market_prices_daily` (latest snapshot per asset).

Frontend: extend table headers, add cells, render values. USD/IDR toggle nice-to-have.

Verify: 1 row per position, NAV column populated for villas with `villa_market_prices_daily` snapshots, `—` otherwise.

### A6. W5 admin override workflow — browser test

Endpoint already shipped. Verify in browser:
1. Inject admin session.
2. Open a submitted operations row in B1 with `?mode=review`.
3. Click "Edit and override" → fields become editable.
4. Change a value → click "Save override".
5. Assert: new row inserted with `supersedes_id` pointing at the original; original stays `submitted`.
6. Approve + publish the new row with a different admin session (4-eyes).
7. Confirm `villa_operations_current` upserts to the new row's id.

If anything fails, fix and re-verify.

### A7. Feature-flag row seed

The plan calls for `settings.villa_returns.enabled` = `off | shadow | on` and `assets.villa_returns_pilot` boolean. Status: framework available, row not seeded.

Add a migration `database/146_villa_returns_feature_flag.sql`:

```sql
INSERT INTO platform_settings (key, value, value_type, description)
VALUES ('villa_returns.enabled', 'on', 'string', 'Villa-Returns rollout state: off|shadow|on')
ON CONFLICT (key) DO NOTHING;

ALTER TABLE assets ADD COLUMN IF NOT EXISTS villa_returns_pilot BOOLEAN NOT NULL DEFAULT TRUE;
```

(Default `TRUE` because dev is single-asset and we want it on; flip to `FALSE` per-asset before adding to a real cohort.)

Add a rollback sibling.

Verify: `psql -c "SELECT key, value FROM platform_settings WHERE key='villa_returns.enabled';"` returns `'on'`.

---

## TRACK B — OPERATIONAL AUTOMATION (after Track A)

### B1. Tokio background interval for daily NAV snapshot

Add a `tokio::spawn` task in `backend/src/lib.rs` after the existing background tasks (search for `community::background` or `event_indexer` patterns). The task:

```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86_400));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        if let Err(e) = admin::villa_nav_snapshot::run_snapshot_for_all_assets(&pool).await {
            tracing::error!("NAV snapshot job failed: {}", e);
        } else {
            tracing::info!("NAV snapshot job completed");
        }
    }
});
```

Gate on env var: `CHAIN_INDEXER_ENABLED`-style. New var: `VILLA_NAV_SNAPSHOT_ENABLED=true|false` (default `true` in dev, `false` until ready in prod). Read from `state.config` or env directly.

**Production gotcha:** don't fire on app start — the existing tokio interval pattern fires immediately on first `tick()`. Either skip the first tick or use `interval.tick().await` once before the loop to align with daily cycle.

Verify: start server in dev with env var set, watch logs for "NAV snapshot job completed" line within 10 minutes (override interval to 10 seconds for the dev test, then revert).

### B2. FX rate populator

`fx_rates_daily` table exists but is empty. Pick an approach:
1. **External API:** Hit `https://open.er-api.com/v6/latest/IDR` (or similar free source) nightly. Add `backend/src/jobs/fx_populator.rs`. Same tokio interval pattern.
2. **Manual seed only:** Add a `database/147_fx_rate_seed.sql` migration that inserts a placeholder current rate (e.g. `INSERT ... VALUES ('2026-05-13', 'IDR', 'USD', 645, 'manual')`). Operators manually update.

For first cut, ship **(2)** as a placeholder migration; deploy (1) as a follow-up that needs external HTTP egress allowed on Cloud Run.

Verify: `SELECT * FROM fx_rates_daily ORDER BY snapshot_date DESC LIMIT 1;` returns a row. The NAV snapshot job picks it up next run (FX rate flows into `villa_market_prices_daily.fx_rate_idr_to_usd_bps`).

### B3. Q7 backfill script

Write `scripts/backfill_villa_operations.rs` (Rust binary, registered in `Cargo.toml [[bin]]`). For each `(asset_id, period_month, period_year)` row in legacy `asset_financials`:

1. Skip if a `villa_operations_log` row already exists with same key + `status='published'` + `supersedes_id IS NULL` (idempotent).
2. INSERT a row with `status='published'`, `recorded_at = asset_financials.created_at`, `published_at = asset_financials.created_at`, `gross_rental_idr_cents = rental_income_cents` (assumed already in IDR cents — verify with a sample query first), `total_opex_idr_cents = expenses_cents`, `net_rental_income_idr_cents = net_income_cents`, expense breakdown columns set to 0, `correction_reason = 'legacy backfill — breakdown unavailable'`.
3. Print summary: `INSERT 0 N` per asset, `SKIP` for already-present rows.

Run via `cargo run --bin backfill_villa_operations -- --dry-run` first. Commit log output to a date-stamped file in `backups/`.

**Confirm with the user before running in prod.** This writes durable rows that survive forever in the append-only log.

### B4. Real `villa_returns.enabled` reading path in production code

Wire the feature flag at the investor-read gate:

Edit `backend/src/assets/villa_performance.rs::api_villa_performance` — if `settings.villa_returns.enabled` is `'off'`, fall back to the legacy `asset_financials` data instead of `villa_operations_current`. If `'shadow'`, read both, log divergence, return legacy. If `'on'`, return new (current behavior).

For dev: leave default `'on'`. Document the env semantics in the plan addendum.

---

## TRACK C — CUT-OVER INFRASTRUCTURE (after Track A + B verified)

**Do NOT proceed with Track C without the user's explicit sign-off per item.** These touch live data on the legacy `asset_financials` table.

### C1. Shadow-write to legacy `asset_financials`

When `villa_operations_log` row is published, also UPSERT to `asset_financials` for the same `(asset_id, period_year, period_month)`:

```rust
sqlx::query!(
    r#"
    INSERT INTO asset_financials (asset_id, period_year, period_month, rental_income_cents,
                                  expenses_cents, net_income_cents, occupancy_rate_bps, created_at)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
    ON CONFLICT (asset_id, period_year, period_month) DO UPDATE SET
        rental_income_cents = EXCLUDED.rental_income_cents,
        expenses_cents      = EXCLUDED.expenses_cents,
        net_income_cents    = EXCLUDED.net_income_cents,
        occupancy_rate_bps  = EXCLUDED.occupancy_rate_bps
    "#,
    asset_id, row.period_year, row.period_month, row.gross_rental_idr_cents,
    row.total_opex_idr_cents, row.net_rental_income_idr_cents,
    row.occupancy_bps, row.published_at
).execute(&mut *tx).await?;
```

In the same publish transaction. Gate on `settings.villa_returns.enabled` ≠ `'on'` (so shadow stops once full cut-over).

Verify: after publish, both tables have the same `(asset_id, period_year, period_month)` row.

### C2. P5 investor-pilot cut-over

For pilot assets (`assets.villa_returns_pilot = TRUE`):
- Investor reads use new layer.
- Other assets continue on legacy.
- Apply behind `settings.villa_returns.enabled = 'on'` AND per-asset flag.

Document procedure in `drafts/villa-returns-cutover-runbook.md`.

### C3. P6 read-path cut-over for all assets

When `settings.villa_returns.enabled = 'on'` AND all monitored metrics stable (no divergence between legacy and new for 14 days):
- Remove the legacy fallback path from `villa_performance.rs`.
- Add a deprecation warning to anything still reading `asset_financials`.

### C4. P7 multi-currency expansion

Schema already supports `payout_currency` per asset. Today only USD + IDR are handled in distribute/process. Extend to USDT, EUR, etc.:
- `assets.payout_currency` already accepts any 3-char code.
- Distribute endpoint: extend the match block to handle more currencies. Add FX-rate lookup for non-USD/IDR pairs.
- Wallet currencies: today the `wallets` table has `(user_id, wallet_type, currency)`; the distribute code upserts a `'cash'`/`'USD'` wallet. Extend to use the asset's payout_currency to find/create the right wallet.

Verify: change a test asset's `payout_currency` to `'EUR'`, distribute, check wallet credit went to EUR wallet not USD.

### C5. P8 drop legacy table

After 90 days of no reads on `asset_financials` (monitor via `pg_stat_user_tables`):
- `ALTER TABLE asset_financials RENAME TO asset_financials__deprecated_YYYYMMDD;`
- After another 30 days: `DROP TABLE asset_financials__deprecated_YYYYMMDD;`

Migration files for both. **Requires explicit user sign-off.**

### C6. Per-investor jurisdiction tax layer

Open scope. Q10 lock-in deferred this. Trigger only if/when a per-jurisdiction tax module is built — coordinate with the legal/compliance track.

---

## VERIFICATION STRATEGY

Every slice must verify at **3 levels** before being marked done:

### Level 1 — Compile + lint

```bash
cd /Users/martin/Projects/poool/backend && cargo check
```

Must finish with `Finished `dev` profile`. If errors, fix and recompile.

### Level 2 — DB-level smoke test (psql)

Write a single-transaction `BEGIN ... ROLLBACK` script under `/tmp/villa-returns-test-<slice>.sql` that exercises the trigger / constraint / state-machine invariants. Run via `psql -d poool -f /tmp/...`.

Pattern: see `/tmp/villa_ops_smoke_test.sql` (already exists from prior P2 verification) — 9 tests for trigger immutability, supersession chain, 4-eyes, time-travel `as_of` reads.

### Level 3 — Browser E2E via MCP preview

For every UI change, drive the end-to-end flow in the preview server.

Setup pattern (proven during P2):

```sql
-- Inject a session for admin@poool.app (bypasses login form):
INSERT INTO user_sessions (user_id, session_token, expires_at, is_2fa_verified)
SELECT id, 'smoke-<slice-name>', NOW() + INTERVAL '1 hour', TRUE
FROM users WHERE email='admin@poool.app';
```

```js
// In preview_eval, set the cookies before navigating:
document.cookie = 'poool_session=smoke-<slice-name>; path=/';
document.cookie = 'csrf_token=anything; path=/';
window.location.href = '/admin/...';
```

For developer-role testing, ensure the dev role + asset link exist:

```sql
INSERT INTO user_roles (user_id, role_id, is_active)
SELECT (SELECT id FROM users WHERE email='admin@poool.app'),
       (SELECT id FROM roles WHERE name='developer'), TRUE
ON CONFLICT DO NOTHING;
INSERT INTO developer_asset_links (developer_user_id, asset_id, granted_by)
SELECT id, '<test_asset_id>', id
FROM users WHERE email='admin@poool.app'
ON CONFLICT DO NOTHING;
```

**Cleanup after every test:** `DELETE FROM user_sessions WHERE session_token LIKE 'smoke-%';`

### E2E test definitions per Track-A slice

| Slice | E2E sequence (verify in browser) |
|---|---|
| A1.a receipts | Open B1 → upload a small txt file → assert document appears in linked-docs list → query `villa_period_documents` for the new row → screenshot |
| A1.b tax statement | Open C3 → upload → assert listed under "Annual tax statement" → query `asset_documents` for `type='tax_statement'` |
| A2 monthly yield | Hit `/api/villas/:id/performance` → assert `annual_yield_bps` matches `monthly_yield_bps × 12` (within 1 bps rounding) → DOM check on property.html |
| A3 share-price deltas | Inject backdated snapshot rows for 3, 6, 12 months ago → call performance API → assert 3 delta fields populated → DOM check |
| A4 tokenize tabs | Click each new tab → verify content panel shows → change a field → save → re-fetch and assert new value |
| A5 my-trading | Navigate to `/my-trading` → assert NAV column populated for villas with snapshots → assert `—` for villas without |
| A6 W5 override | Drive admin override flow → assert new log row inserted with supersedes_id → admin B (different user) approves → publishes → trigger fires → current view updated |
| A7 feature flag | `psql -c "SELECT value FROM platform_settings WHERE key='villa_returns.enabled';"` → `'on'` |

### E2E test definitions per Track-B slice

| Slice | E2E sequence |
|---|---|
| B1 snapshot cron | Set env var `VILLA_NAV_SNAPSHOT_ENABLED=true` with interval=10s override → restart → wait → check logs for "snapshot job completed" → query `villa_market_prices_daily` for new row |
| B2 FX populator | Run the migration / job → assert `fx_rates_daily` has at least one row dated today |
| B3 backfill | `cargo run --bin backfill_villa_operations -- --dry-run` → review output → manual approval → run for real → idempotency: re-run, expect 0 inserts |
| B4 feature-flag read path | `UPDATE platform_settings SET value='off' WHERE key='villa_returns.enabled'` → call performance API → assert returns legacy data → `UPDATE ... SET value='on'` → assert new data returned |

### E2E test definitions per Track-C slice

Each Track-C slice requires its own runbook entry in `drafts/villa-returns-cutover-runbook.md` and explicit user sign-off before execution.

---

## TESTING APPROACH OVERVIEW

**Pyramid:**

1. **Compile-level (fastest):** `cargo check` after every backend edit. Reject anything that doesn't compile.
2. **DB-level (fast):** psql smoke tests in transactions — exercise triggers, constraints, state machines. ~1s per run.
3. **API-level (medium):** `preview_eval` to call endpoints with injected session — verify JSON shapes, status codes, side effects.
4. **Browser-level (slowest, most valuable):** `preview_eval` to drive DOM — click, type, screenshot. Confirm end-to-end UI works.

**Existing infrastructure:**

- `backend/tests/` directory exists with integration tests. Add a `villa_returns_*` test file when the slice merits it (especially for state-machine logic).
- MCP `preview_*` tools handle browser automation: `preview_start`, `preview_eval`, `preview_screenshot`, `preview_logs`, `preview_console_logs`, `preview_stop`.
- Already-shipped smoke test reference: `/tmp/villa_ops_smoke_test.sql` (9 tests).

**Definition of done per slice:**

A slice is **DONE** only when:

1. ✅ Backend compiles clean (`cargo check`).
2. ✅ DB-level smoke test passes for any new trigger / constraint / state machine.
3. ✅ API-level test confirms endpoint returns correct shape + side effects.
4. ✅ Browser-level test confirms UI renders correctly + interactions work.
5. ✅ Screenshot captured for visual surfaces.
6. ✅ Test session cleaned up from `user_sessions`.
7. ✅ Cumulative status report posted (what shipped, what tests passed, what's next).

## ORDER OF OPERATIONS

Strict priority — ship oldest blocker first:

### Phase 1 — Track A (1-3 days)

A7 (feature-flag seed) → A6 (W5 verify) → A2 (monthly yield) → A3 (share-price deltas) → A1.a + A1.b (uploads) → A4 (tokenize tabs) → A5 (my-trading).

Reason: A7 is trivial seed. A6 verifies an existing endpoint with no new code. A2 + A3 are tiny KPI additions to an existing page. A1 is medium — needs file upload integration. A4 + A5 close last admin + investor surfaces.

### Phase 2 — Track B (1-2 days)

B2 (FX populator seed) → B1 (cron interval) → B4 (feature flag read path) → B3 (backfill — only if user approves).

Reason: B2 unblocks B1 (cron needs FX rates to be present). B4 is the gate before any cut-over. B3 is destructive-adjacent — needs explicit user sign-off.

### Phase 3 — Track C (each requires user sign-off)

C1 (shadow-write) → C2 (pilot cut-over) → C3 (full cut-over) → C5 (drop legacy after 90 days). C4 (multi-currency) is parallel. C6 (per-investor tax) is open scope.

Reason: each is destructive or operationally significant. User must approve before each.

## DEFINITION OF "PRODUCTION READY"

Villa-Returns is **production-ready** when:

1. ✅ Every Track-A slice shipped + verified at all 3 test levels.
2. ✅ Every Track-B slice shipped + verified, with B3 backfill execution approved + completed.
3. ✅ C1 shadow-write running stable for at least 14 days with zero divergence alerts.
4. ✅ C2 pilot cut-over approved + monitored for 30 days.
5. ✅ Final audit (replay `drafts/villa-returns-final-audit.md` methodology) shows 100% in every PDF section.
6. ✅ Documented runbook for: (a) submitting monthly data via API/UI, (b) approving + publishing via the queue, (c) distributing + processing payouts, (d) handling corrections + top-ups, (e) annual valuation cycle, (f) admin grants/revokes for developers.
7. ✅ Notifications wired on all 7 trigger points + verified to deliver to the right users.

## ANTI-GOALS

Do NOT:
- Redesign the schema. The 16 migrations are locked.
- Re-question the Q1–Q11 lock-ins. They're decided.
- Add new abstractions for the sake of cleanliness. Match shipped patterns.
- Bundle Track A slices together — ship one at a time, verify each.
- Push to production without explicit user approval. Per memory: deploy = `git push origin main`; user decides when.
- Skip the cleanup step (`DELETE FROM user_sessions ...`) after smoke tests.
- Add `f32`/`f64` anywhere. Stay integer-based.
- Skip writing `audit_logs` rows on state transitions.

## STYLE

- **Terse status updates.** One sentence between tool calls.
- **Match shipped patterns.** The 4 bugs in this prompt's "gotchas" section are real ones caught in prod-style testing. Don't invent new ones.
- **No TodoWrite.** Slices are atomic. Verify each, report, move on.
- **No bundling.** Each Track A slice ships independently — atomic backend + frontend + tests, then commit-able.

## START

1. Read the 4 planning docs + audit (in order listed above).
2. Re-skim PDF §6 + §7 + §8 for the data-model basics.
3. Pick **A7 (feature-flag seed)** as the first slice — smallest, validates your environment.
4. State which slice you're working on, in one sentence.
5. Ship it. Verify all 3 levels. Report.
6. Pick the next slice. Repeat until Phase 1 (Track A) is complete.
7. Then propose Phase 2 to user with a 1-paragraph plan summary; wait for green light.
8. Then propose Phase 3 per slice (each gated).

Do not redesign. Do not re-question lock-ins. Ship + verify + report + repeat.
