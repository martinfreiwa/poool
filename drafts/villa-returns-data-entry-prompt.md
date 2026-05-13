# Improved Prompt — Villa-Returns Data-Entry Implementation Plan

> Paste the section below into a fresh Claude/agent session. It is self-contained: it tells the model who it is, what to read, what the codebase already has, what to deliver, and what questions to ask back.

---

## ROLE

You are a **principal SaaS architect** with deep experience in:
- Tokenized real-estate / fractional-ownership platforms (RWA tokenization, SPV mechanics, NAV vs market price)
- Multi-tenant admin tooling and back-office data-entry UX
- Bitemporal / append-only financial data modeling (period-of-record vs period-being-described, corrections without rewrites)
- Rust + Axum + PostgreSQL + SQLx stacks shipped to Google Cloud Run

You are advising on the **POOOL.app** platform. Output a **decision-ready implementation plan**, not generic SaaS advice.

## SOURCE DOCUMENT (must be read first)

`Downloads/POOOL_Data_Model_Villa_Returns_EN (1).pdf` — internal working document defining:
- Section 2: monthly inputs from management company (gross rent, available/booked nights, ADR, cleaning, maintenance, utilities, staff, pool, mgmt fee, OTA fees, payment fees, refunds, other OpEx, receipts, bank statement, distributable amount)
- Section 3: annual inputs (current valuation, valuation date/method, comparables, previous-year value, annual revenue/expenses, CapEx, tax statement, forecast assumptions)
- Section 4: one-time master data on onboarding (asset ID, address, legal owner, initial value, tokenized %, total tokens, sold tokens, owner-retained tokens, initial token price, min investment, payout frequency/currency, fee structures, permitted deductions, reserve rule, record date, forecast assumptions)
- Section 5: monthly POOOL calculations (net rental, distributable, platform fee, reserve, payout per token, net return per investor, monthly + annualised yield, investor dashboard return, investor-specific deductions)
- Section 6: target KPIs in UI (Projected Return %, Projected Annualised Net Return %, 5-Year Total Return %, Annual Yield %, Share Price Performance +3M/+6M/+12M, Net Return per Investor)
- Section 7: critical NAV formula — `NAV Token Price = (Property Value × tokenized %) / tokens in investor pool` — NOT `Property Value / total tokens`
- Section 8: NAV Token Price vs Resale Market Token Price must stay separate in UI
- Section 9–11: missing data points, recommended block structure (A. Property Ops, B. Asset Valuation, C. Token & Investor, D. Return Calc), responsibility split between management company / POOOL / Valuer

**Read the PDF end-to-end before outputting anything.** Quote section numbers when referencing requirements.

## CODEBASE GROUND TRUTH (do not invent — these are the actual files)

**Stack:** Rust + Axum + SQLx, PostgreSQL 16, MiniJinja SSR templates, vanilla HTML/CSS/JS frontend (no SPA framework), PgBouncer sidecar, Cloud Run deploy via `cloudbuild.yaml`. All money stored as `BIGINT` cents — **no floats anywhere**.

**Layers:**
- Backend: [backend/src/lib.rs](backend/src/lib.rs) (`build_platform_router`), [backend/src/main.rs](backend/src/main.rs)
- Admin routes: [backend/src/admin/](backend/src/admin/) — `assets.rs` (~70KB, asset CRUD + financials), `rewards.rs`, `mod.rs`
- Asset model: [backend/src/assets/models.rs:39](backend/src/assets/models.rs) — `Asset` struct
- Migrations: [database/](database/) — 154 numbered SQL files, consolidated in `full_migration.sql`
- Smart contracts: [contracts/src/](contracts/src/) — Solidity on Polygon Amoy
- Auth: session-cookie JWT (`poool_session`), `UserRole` enum (Admin / Developer / Investor) — RBAC at route level
- Audit: `audit_logs` table — `(actor_user_id, action, entity_type, entity_id, previous_state JSONB, new_state JSONB, created_at)`

**What already exists you can REUSE — do not rebuild:**

| Need | Already there |
|---|---|
| Asset/villa master record | `assets` table — title, slug, property_type, location, lease_term_years, land_size_sqm, building_size_sqm, bedrooms, bathrooms, total_value_cents, token_price_cents, tokens_total, tokens_available, annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps, operator_split_pct, poool_split_pct |
| Monthly financial bucket | `asset_financials` — `(asset_id, period_month, period_year)` UNIQUE, rental_income_cents, occupancy_rate_bps, expenses_cents, net_income_cents |
| Investor holdings | `investments` — user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, total_rental_cents, status, purchased_at |
| Payout history | `dividend_payouts` — investment_id, amount_cents, payout_type ('rental'/'exit'/'bonus'), status, paid_at |
| Documents | `asset_documents` — typed file pointers (prospectus, valuation reports, legal) |
| Milestones | `asset_milestones` |
| Admin UI shells | [frontend/platform/admin/assets.html](frontend/platform/admin/assets.html), [admin/asset-details.html](frontend/platform/admin/asset-details.html), [admin/asset-tokenize.html](frontend/platform/admin/asset-tokenize.html), [admin/dividends.html](frontend/platform/admin/dividends.html), [admin/settings.html](frontend/platform/admin/settings.html) |
| Investor UI | [frontend/platform/property.html](frontend/platform/property.html), [poool_app_home.html](frontend/platform/poool_app_home.html), [my-trading.html](frontend/platform/my-trading.html), [transactions.html](frontend/platform/transactions.html) |
| Audit infra | `audit_logs` (JSONB before/after) — already wired for admin actions |
| Trade history | `trade_history` — append-only, never UPDATEd |
| Snapshot pattern | `leaderboard_snapshots` — daily snapshot example |

**Schema gaps vs PDF requirements (THIS IS THE BUILD SURFACE):**

| PDF requirement | Current state | Action |
|---|---|---|
| Available / booked nights | missing | add columns or new table |
| ADR (avg daily rate) | missing (derive or store) | decide |
| Expense breakdown (cleaning / maintenance / utilities / staff / pool / pest / mgmt fee / OTA fees / payment fees / refunds / other OpEx) | lumped into `expenses_cents` | new `villa_expense_items` table OR widened columns |
| CapEx vs OpEx separation | none | new column or flag |
| Reserve fund | none | add `reserve_*_cents` |
| Tokenized % of villa | implicit in token math | store explicitly per PDF §7 |
| Payout-eligible token count vs total | not separated | add `tokens_payout_eligible` |
| Owner-retained tokens | not tracked separately | add |
| Annual valuation with appraiser, method, comparables | only single `total_value_cents` mutable field | new `villa_valuations` append-only table |
| Record date for distribution | not enforced | add to payout run |
| Multi-currency (IDR ↔ USD) | hardcoded cents, no ISO code | add `currency_code` + FX-rate table or fixed-USD policy |
| Receipts/invoices/bank statements proof | `asset_documents` exists but not linked to period | link by `(asset_id, period_month, period_year, doc_type)` |
| NAV token price calculated correctly per §7 formula | not implemented | calc + storage layer |
| Market token price (separate from NAV) | secondary-market trades exist but no aggregated price-history table | add `villa_market_prices_daily` snapshot |
| Time-travel "go back by month/year" | `asset_financials` is **mutable** — corrections overwrite history with no version trail | redesign as append-only OR add `villa_operations_corrections` table |
| Admin approval status before data goes live | no `status` column on `asset_financials` | add `status ENUM (draft, submitted, approved, published)` |

**Time-travel requirement (user explicitly named this):** Properties live on the platform for many years. Admin must be able to:
1. View any past month / year and see the data as it was recorded
2. Edit a past month (e.g. a correction arrives 6 months late) WITHOUT silently overwriting what investors already saw
3. Render charts and aggregates from history (monthly yield, 12M share-price performance, 5-year total return)
4. Show an audit trail of every correction (who, when, why, before → after)

This rules out plain `UPDATE` on `asset_financials`. Choose one and justify:
- **Bitemporal** (`valid_from`, `valid_to`, `recorded_at`) — strongest, most complex
- **SCD Type 2** with version number per `(asset_id, period_month, period_year)` row — pragmatic middle ground
- **Append-only operations log + materialized current-view table** — simplest read path

## DELIVERABLES (in this exact order)

### 1. Clarifying questions — ASK FIRST, DO NOT GUESS
Before writing the plan, output a numbered list of questions you need answered to make the plan executable. Cover at minimum:
- Currency policy: store FX-rate per period, or convert at ingest?
- Reserve fund: % of net rent, fixed amount, or admin-decision per period?
- Record date semantics: snapshot at end of period or configurable per asset?
- Who is "Developer panel" — asset developer (villa owner submitting their numbers) or platform engineer? PDF §2 implies management company submits — should they have a self-serve portal, or does admin enter on their behalf in MVP?
- Approval workflow: single admin approves, or 4-eyes (submitter + approver)?
- Investor visibility of mid-correction state: do investors see only `approved` data, or also `submitted/pending`?
- Backfill: how many months of historical data exists for current portfolio that must be imported?
- CapEx amortisation policy: spread over N months, or excluded from monthly yield calc entirely?
- Forecast assumptions (5-Year Total Return inputs): admin-set once, recalculated annually, or per-asset overrides?
- Tax withholding: jurisdiction-driven (investor) or per-asset?

Wait for answers OR mark each question with the **assumption you proceed with** if asked to continue immediately.

### 2. Page inventory — REUSE vs CREATE
For every UI surface listed in PDF §2/3/4/5/6, give a row:

| Surface | Audience | Existing page to extend | OR new page path | Reason |
|---|---|---|---|---|

Audiences: **Admin panel**, **Developer/Asset-owner portal** (clarify scope in Q1), **Investor dashboard**, **Public listing page**.

For each NEW page, propose URL slug, route file, template name, JSON API endpoint(s).

For each EXTENDED page, list the file path + the specific section (e.g. "add `<section id='monthly-operations'>` between line X and Y of `admin/asset-details.html`").

### 3. Data-model design (SQL)
For each gap row in the "Schema gaps" table above:
- Exact migration file name (`database/155_villa_operations_log.sql`, etc.)
- Full `CREATE TABLE` DDL with column types, NOT NULL, defaults, foreign keys, CHECK constraints
- Indexes (esp. `(asset_id, period_year DESC, period_month DESC)` for time-travel queries)
- Trigger / app-level rule that enforces append-only where required
- Migration order and dependencies

Decide and justify the time-travel pattern (bitemporal vs SCD2 vs append-only-log). Show example rows for "March data corrected in June" so the read path is concrete.

### 4. Backend API contract
For each new/changed endpoint:
- Method + path (`POST /api/admin/villas/:id/operations`, `GET /api/admin/villas/:id/operations?as_of=2026-03-31`, etc.)
- Request/response JSON shape (Rust struct names from existing codebase style)
- Authn role required
- Which Rust module file it lives in (extend `backend/src/admin/assets.rs` or new `backend/src/admin/villa_operations.rs`?)
- Calculation responsibility (which fields are stored vs computed on read)
- Transaction boundaries (which writes must be in one `BEGIN ... COMMIT`)

### 5. KPI calculation layer
Map every PDF §5 + §6 KPI to:
- Formula (exact, with cents/bps arithmetic — no floats)
- Inputs (which tables/columns)
- When it's computed (on write, on read, scheduled job)
- Where it's cached (column, materialized view, Redis, or recomputed)
- How "as-of date" parameter flows through (for time-travel)

### 6. Safe-rollout plan
The user's constraint: "implement without breaking it." Required deliverable:
- Feature flag mechanism (env var? `settings` table row? per-asset opt-in?)
- Migration sequence that is **forward-compatible with running prod** (no `DROP COLUMN`, no `NOT NULL` on existing tables without default, no destructive renames — every step deployable independently)
- Read path during transition: old `asset_financials` row vs new operations log — which wins? For how long?
- Shadow-write phase: write to both schemas, compare, then cut over
- Backfill plan with idempotent script
- Rollback procedure for each migration
- Test plan: integration tests in `backend/tests/` covering "correction in month N+3 does not change month N display for users who viewed it before correction" semantics
- Cloud Run / Cloud Build implications — note any required env-var changes or secret additions

### 7. History & chart consumption
The PDF KPIs (Share Price Performance +3M/+6M/+12M, 5-Year Total Return, monthly yield series) must be plottable. Specify:
- Query shape for "give me 60 monthly NAV prices for asset X as published at time T"
- Endpoint for chart data (`GET /api/assets/:id/history?metric=nav&from=...&to=...`)
- How corrections after the fact are surfaced in the chart (footnote? toggle "as-published" vs "as-corrected"?)

### 8. Phased delivery roadmap
Phase the plan into Cloud-Run-deployable increments. Each phase must be:
- Shippable on its own (no half-built state in prod)
- Reversible
- Demoable

Suggested phases — adjust if you have a better split:
1. Schema-only (new tables, no writes from app yet)
2. Admin write path (new tables only, no read switch)
3. Admin read path + asset-details UI integration
4. KPI calc layer + historical query API
5. Investor UI surfaces NAV per PDF §7 formula
6. Cut over `asset_financials` reads to new layer + deprecate legacy column
7. Multi-currency, CapEx amortisation, advanced KPIs

For each phase: files touched, migrations included, feature-flag state, success criteria, estimated effort (S/M/L).

## OUTPUT FORMAT

Use Markdown. Headings match section numbers above (1–8). Tables where the structure helps. Code blocks for SQL DDL and Rust struct sketches. Cite the PDF as "PDF §N" and the codebase as `path/file.rs:line`.

Do **not** write production code in this pass — this is the plan. Code comes after the plan is approved.

## STYLE

- Be direct. No "we should consider" hedging — recommend and justify.
- When two paths exist, name them, state the trade-off, pick one, and say why.
- Flag every assumption explicitly so it can be challenged.
- Where the PDF is ambiguous (e.g. "permitted expense deductions defined contractually"), surface it as a Q1 question, do not paper over it.
- Length is not a virtue. Density is. If a single table communicates it, use the table.
