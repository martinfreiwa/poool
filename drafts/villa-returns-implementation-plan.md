# Villa-Returns Data-Entry — Implementation Plan

**Scope.** Implement PDF `POOOL_Data_Model_Villa_Returns_EN (1).pdf` §1–§12 against the live POOOL.app codebase. Decision-ready plan. No production code in this pass.

**Convention.** Cents = `BIGINT`, percentages = `bps` (basis points, INT, 10000 = 100.00 %), no floats. PDF cited as "PDF §N". Code cited as `path/file.rs:line`.

---

## 1. Clarifying Questions (with proceed-assumptions)

Each question carries the **assumption** the plan proceeds with if the question is not answered before build.

| # | Question | Proceed-assumption |
|---|---|---|
| Q1 | **Currency policy.** Villas earn in IDR but UI shows USD (PDF §9). Store native IDR per period + FX rate, or convert to USD cents at ingest? | **LOCKED: IDR-native, USD-derived.** All villa monetary inputs are stored as `*_idr_cents` (BIGINT). FX rate `fx_rate_idr_to_usd_bps` is frozen at publish into the log row. `*_usd_cents` columns are computed at write and frozen for historical fidelity. Investor UI defaults to USD with a "show native IDR" toggle. `assets.native_currency_code` defaults to `'IDR'`; non-IDR villas are an edge case, not the norm. |
| Q2 | **Reserve fund rule.** PDF §4 says "POOOL + contract". Configured per asset, % of net rent, or fixed per period? | **Per-asset config**: `assets.reserve_pct_bps` (default 500 = 5 %) applied to `net_rental_income_cents`. Per-period override allowed via `villa_operations_log.reserve_override_cents`. |
| Q3 | **Record date semantics** (PDF §4, §5). Snapshot end-of-period, or admin-set per payout? | **End-of-period, per-asset configurable.** `assets.distribution_record_day` (INT 1–28, default 1 = first of next month). Eligibility = investor held tokens at 23:59:59 UTC on record day. |
| Q4 | **Developer panel scope.** `backend/src/admin/developer_projects.rs` exists. Is the management company a "Developer" user who self-submits monthly numbers, or does admin always enter on their behalf? | **LOCKED: Field-level role split from day one.** The Developer user (= management company login) submits the fields PDF §2/§3/§11 assigns to "Management company". The Admin owns the fields PDF §3/§4 assigns to POOOL/Valuer (master data, tokenization, fees, reserves, record date, valuation, deduction policy, approvals). Admin can override **any** developer-entered field at any time (creates a correction row, supersedes developer entry, status returns to `submitted`/`approved`). System-computed fields are owned by neither role. Field-level permission matrix in §2.5 below. |
| Q5 | **Approval workflow.** Single admin, or 4-eyes (submitter ≠ approver)? | **4-eyes from day one.** `villa_operations_log.status` = `draft → submitted → approved → published`. Approver `user_id` must differ from submitter. Cheaper to start strict than to retrofit. |
| Q6 | **Investor visibility of pending/draft data.** | **Only `published` is investor-visible.** `submitted` and `approved` visible only to Admin/Developer roles. Investor UI never sees half-finished months. |
| Q7 | **Backfill volume.** How many months × villas of legacy data in `asset_financials` to import? | **Assumed ≤ 24 months × ≤ 50 villas = 1,200 rows.** One-shot idempotent migration script copies into new tables with `status = 'published'`, `recorded_at = created_at`, no corrections trail. Confirm exact count before Phase 1. |
| Q8 | **CapEx amortisation.** Spread over N months or excluded from monthly yield entirely? | **Excluded from monthly distributable income; tracked separately.** CapEx hits `villa_capex_events` table, surfaced in annual report and NAV (raises property value), never reduces a single month's payout. Mirrors REIT accounting. |
| Q9 | **Forecast assumptions** (PDF §3, §6). Per-asset, set annually? Admin override? | **Per-asset, versioned annually.** `villa_forecast_assumptions` row per asset per year. Used by Projected Return + 5-Year Total Return KPIs. Admin can override mid-year; old version retained. |
| Q10 | **Tax withholding** (PDF §5). Per-investor jurisdiction, or per-asset SPV-level? | **Both layers.** Per-asset `withholding_tax_bps` (SPV-level, default 0) applied before distribution. Per-investor jurisdiction layer deferred to a future tax-residency module — flag, do not block. |
| Q11 | **Re-payout on correction** (raised during workflow walkthrough). If a published month is corrected upward/downward after distributions were sent, do we auto-reconcile? | **Manual top-up only, never claw back.** UI surfaces the delta to admin on A1 asset-details with a "Top up $X" button → creates a `dividend_payouts.payout_type='bonus'` row linked to the correction via metadata. Negative deltas (over-paid in original) are absorbed by POOOL — investor-protection policy. Documented to investors. |

> The plan below proceeds against these assumptions. Any answer that changes Q1, Q4, Q5, or Q8 has meaningful downstream impact and should be confirmed before Phase 1 ships.

---

## 2. Page Inventory — Reuse vs Create

Audiences: **Admin** (POOOL ops), **Developer** (management company / asset owner, `UserRole::Developer`), **Investor**, **Public**.

| # | PDF surface | Audience | Decision | Target file | Reason |
|---|---|---|---|---|---|
| 1 | Monthly operations entry form (§2: gross rent, nights, ADR, all OpEx categories, OTA, refunds, mgmt fee, distributable) | Admin | **NEW page** | `frontend/platform/admin/villa-operations-entry.html` (route `/admin/villas/:id/operations/:year/:month`) | Form has ~25 fields + file uploads — too dense to bolt onto asset-details. New page lets us iterate UX without touching CRUD. |
| 2 | Monthly operations list / history per villa | Admin | **EXTEND** | `frontend/platform/admin/asset-details.html` — add `<section id="monthly-operations">` after the existing financials block | The existing page already shows villa metadata. Add a 12-month strip + "view history" deeplink. |
| 3 | Annual valuation entry (§3: value, date, method, comparables, appraiser, evidence docs) | Admin | **NEW page** | `frontend/platform/admin/villa-valuation.html` (route `/admin/villas/:id/valuations/new`) | Valuation is a discrete, document-heavy event — own page. Index of all valuations on `asset-details.html`. |
| 4 | One-time master data on onboarding (§4: tokenized %, reserve rule, record date, payout freq, fee structures, forecast assumptions, permitted deductions) | Admin | **EXTEND** | `frontend/platform/admin/asset-tokenize.html` — add tabs/sections for Reserve, Payout config, Forecast | Tokenization page is the natural home for token+payout config. Add `<details>` blocks for each section. |
| 5 | Permitted-deductions config (whitelist of expense categories per contract) | Admin | **NEW page** | `frontend/platform/admin/villa-deduction-policy.html` (route `/admin/villas/:id/deduction-policy`) | Contract-level config; separate page lets legal/admin review without scrolling tokenization screen. |
| 6 | Approval queue (pending `submitted` operations + valuations) | Admin | **EXTEND** | `frontend/platform/admin/approvals.html` (already exists per `backend/src/admin/approvals.rs`) — add tabs for "Villa Operations" and "Villa Valuations" | Approvals page already centralises 4-eyes review — add the two new entity types. |
| 7 | Developer self-submission form (P2, **not gated**) | Developer | **NEW page** | `frontend/platform/developer/operations-submit.html` (route `/developer/villas/:id/operations/new`) | Ships in P2 alongside admin form. Only the Dev-owned fields per §2.5 are editable; Admin-owned fields render read-only with the admin-set value. Status transitions `draft → submitted` only. |
| 8 | Developer dashboard (submission status, pending approvals, history) | Developer | **NEW page** | `frontend/platform/developer/dashboard.html` (route `/developer/dashboard`) | Lists all villas the developer is assigned to (via `developer_projects` link table), with per-month status badges and "submit March 2026" CTAs. |
| 8b | Developer annual data (CapEx events, tax statement upload, forecast suggestions) | Developer | **NEW page** | `frontend/platform/developer/annual-data.html` (route `/developer/villas/:id/annual/:year`) | Single page covering PDF §3 Dev-owned annual inputs. |
| 9 | Investor: monthly yield breakdown per villa | Investor | **EXTEND** | `frontend/platform/property.html` — add tab "Performance" alongside Description/Documents | Single source of truth per property already on this page. Tab UX = no new URL. |
| 10 | Investor: NAV vs Market price chart (PDF §8) | Investor | **EXTEND** | `frontend/platform/property.html` — within the Performance tab, two separate line series (NAV + Market), never overlaid as single line | PDF §8 mandates separation. Visual separation = legend with two colours + dual sub-headers. |
| 11 | Investor: portfolio-level KPIs (Annual Yield %, 5-Year Total Return %, Net Return per Investor) | Investor | **EXTEND** | `frontend/platform/poool_app_home.html` and `my-trading.html` — extend existing KPI cards, add an "as of" date selector | Dashboard already aggregates positions; add date picker → time-travel for free. |
| 12 | Investor: distribution history | Investor | **EXTEND** | `frontend/platform/transactions.html` — already shows wallet transactions; add filter `type=distribution` | Reuse existing transaction stream. |
| 13 | Investor: receipts / proof documents per period | Investor | **EXTEND** | `frontend/platform/property.html` Documents tab — group by year/month | Public-visibility flag already on `asset_documents.is_investor_visible`. |
| 14 | Public listing snapshot (current Annual Yield, Projected Return) | Public | **EXTEND** | `frontend/platform/property-public.html` — pull from the same KPI layer | No new page. |
| 15 | History viewer (any past month/year for a villa, with corrections diff) | Admin | **NEW page** | `frontend/platform/admin/villa-history.html` (route `/admin/villas/:id/history?as_of=YYYY-MM-DD`) | Distinct UX from entry form: read-only timeline + corrections diff view. |

**Total: 4 new pages, 8 extensions, 2 Developer pages (now P2, not P2-gated).** Backend routes follow the same split (§4 below).

### 2.5 Field-Level Permission Matrix (per Q4 lock-in)

Every input field on the villa-returns surface maps to exactly one of: **Dev** (developer / management can write, admin can override), **Admin** (admin-only write, developer read-only), **System** (computed, no user writes). Source citations refer to the PDF responsibility tables.

#### Monthly operations (PDF §2 + §11)

| Field | Role | Notes |
|---|---|---|
| `gross_rental_idr_cents` | Dev | PDF §2 row 1 "Gross rental income" — Management |
| `nights_available` | Dev | PDF §2 row 2 |
| `nights_booked` | Dev | PDF §2 row 3 |
| `occupancy_bps` | System | Generated column |
| `adr_idr_cents`, `adr_usd_cents` | System | Generated column |
| `expense_cleaning_idr_cents` | Dev | PDF §2 row 5 |
| `expense_maintenance_idr_cents` | Dev | PDF §2 row 6 |
| `expense_utilities_idr_cents` | Dev | PDF §2 row 7 |
| `expense_staff_idr_cents` | Dev | PDF §2 row 8 |
| `expense_pool_garden_idr_cents`, `expense_pest_idr_cents` | Dev | PDF §2 row 9 |
| `mgmt_fee_idr_cents` | Dev | PDF §2 row 10 — but `assets.mgmt_fee_bps` config is Admin |
| `ota_fees_idr_cents` | Dev | PDF §2 row 11 |
| `payment_fees_idr_cents` | Dev | PDF §2 row 12 |
| `refunds_idr_cents` | Dev | PDF §2 row 13 |
| `expense_other_idr_cents` | Dev | PDF §2 row 14 |
| Receipt / invoice docs | Dev | Upload to `asset_documents` + link in `villa_period_documents` |
| Bank statement / payout statement | Dev | Same |
| `fx_rate_idr_to_usd_bps` | System | Snapshot from `fx_rates_daily` on publish; admin can override per-period if challenged |
| All `*_usd_cents` derived | System | Frozen at publish |
| `total_opex_idr_cents`, `net_rental_income_idr_cents`, `distributable_idr_cents` | System | Derived at publish |
| `reserve_override_cents` | Admin | Override of per-asset `reserve_pct_bps` |
| `platform_fee_cents`, `reserve_applied_cents`, `withholding_cents` | System | Derived |
| `status` workflow transitions | Dev: `draft→submitted`. Admin: `submitted→approved→published`. Admin: any reverse / correction. | 4-eyes (Q5) means approver ≠ submitter |

#### Annual operations (PDF §3)

| Field | Role |
|---|---|
| Annual revenue / expenses (rollup) | System (sum of monthly published rows) |
| Major repairs / CapEx events | Dev submits, Admin approves (`villa_capex_events` table) |
| Annual tax statement upload | Dev (with Admin approval) |
| Updated forecast assumptions | Dev suggests, Admin finalises in `villa_forecast_assumptions` |
| Current property valuation | **Admin** (PDF §3 + §11) — Dev cannot edit |
| Valuation date, method, comparables, appraiser, evidence | **Admin** |

#### Onboarding master data (PDF §4)

All Admin-only writes. Dev sees as read-only.

| Field | Role |
|---|---|
| `assets.title`, `slug`, `address`, `location_*` | Admin |
| Legal owner | Admin (with Legal team) |
| `assets.tokenized_pct_bps` | Admin |
| `assets.tokens_total`, `tokens_payout_eligible`, `tokens_owner_retained` | Admin |
| `assets.token_price_cents` (initial), `min_investment_cents` | Admin |
| `assets.payout_frequency`, `payout_currency`, `distribution_record_day` | Admin |
| `assets.poool_split_pct` (POOOL fee), `mgmt_fee_bps` config | Admin |
| Permitted-deductions policy (`villa_deduction_policy`) | Admin |
| `assets.reserve_pct_bps` | Admin |
| `assets.withholding_tax_bps` | Admin |
| `villa_forecast_assumptions.*` final values | Admin (Dev can suggest in a `villa_forecast_suggestions` sidecar table) |

#### System-computed (PDF §5 + §6, no user writes)

All §5 calculations + KPI layer (§5 in this plan). NAV per PDF §7 formula. Market price from `trade_history`.

**Enforcement.** API handlers reject writes to fields outside the caller's role. App-level whitelist in `backend/src/admin/villa_operations.rs::FIELD_OWNERSHIP: &[(&'static str, Role)]`. Admin override route is a separate endpoint that emits an audit_log entry with `action = 'admin_override'` and the field path.

---

## 3. Data-Model Design

### 3.1 Time-Travel Pattern — Decision

**Chosen: Append-only operations log + materialised current-view (option 3).** Justification:

- **Bitemporal (option 1)** is correct but introduces `valid_from / valid_to / recorded_at` on every monthly row. Costs query complexity for marginal gain — the platform does not have arbitrary "valid period" semantics, only "as-recorded" vs "as-corrected".
- **SCD Type 2 (option 2)** versions full rows on every change. Acceptable, but the log model gives the same audit trail at lower write cost and matches the existing `audit_logs` immutability pattern (`database/001_initial_schema.sql:490`).
- **Append-only log (option 3)** mirrors `trade_history` and `audit_logs` already in the codebase — same team mental model, same backup/PITR story, simplest read path via a materialised view that picks the latest non-superseded entry per `(asset_id, period_year, period_month)`.

**Concrete shape:**

```
villa_operations_log              ← append-only, never UPDATEd, never DELETEd
  id, asset_id, period_year, period_month,
  ...all data columns...,
  status, supersedes_id (nullable, FK to villa_operations_log.id),
  submitted_by, submitted_at, approved_by, approved_at, published_at, recorded_at

villa_operations_current          ← materialised view (or trigger-maintained table)
  one row per (asset_id, period_year, period_month) = latest WHERE status='published'
```

**"March 2026 data corrected in June 2026" — concrete read path:**

```
id   asset  year/month  gross_rent  status      supersedes_id  recorded_at
A    villa1  2026-03    5,000,000   published   NULL           2026-04-05
B    villa1  2026-03    6,200,000   draft       A              2026-06-10 (correction drafted)
C    villa1  2026-03    6,200,000   submitted   A              2026-06-11
D    villa1  2026-03    6,200,000   approved    A              2026-06-12
E    villa1  2026-03    6,200,000   published   A              2026-06-13

Investor today (as_of=NOW) sees row E → corrected 6,200,000.
Investor with as_of=2026-05-01 sees row A → original 5,000,000.
Historical chart "as published" toggle uses recorded_at chain; "as of latest" uses E.
Audit trail = the chain A→B→C→D→E with supersedes_id.
```

### 3.2 Migration Files (in order)

Each migration is one file, idempotent (`CREATE TABLE IF NOT EXISTS`, `ADD COLUMN IF NOT EXISTS`), forward-compatible with running prod (no destructive ops). Highest existing migration = `129_disclosure_policy_version.sql`, so new files start at **130**.

| File | Purpose | Depends on |
|---|---|---|
| `database/130_asset_villa_ext.sql` | Add nullable columns to `assets`: `tokenized_pct_bps`, `tokens_payout_eligible`, `tokens_owner_retained`, `reserve_pct_bps`, `payout_frequency`, `payout_currency`, `distribution_record_day`, `withholding_tax_bps`, `allow_developer_submission`, `currency_code`. All nullable with defaults. | none |
| `database/131_villa_expense_categories.sql` | Lookup table for permitted expense categories (cleaning, maintenance, utilities, staff, pool, pest, ota, payment_fee, mgmt_fee, refund, other). | none |
| `database/132_villa_operations_log.sql` | Append-only monthly log table. | 130, 131 |
| `database/133_villa_operations_current.sql` | Materialised current-view (table maintained by trigger on log inserts of status='published'). | 132 |
| `database/134_villa_valuations.sql` | Append-only annual valuations. | 130 |
| `database/135_villa_forecast_assumptions.sql` | Versioned forecast inputs (one row per asset per year). | 130 |
| `database/136_villa_market_prices_daily.sql` | Daily NAV + market-price snapshot for chart query. | 130 |
| `database/137_villa_capex_events.sql` | CapEx separated from monthly OpEx. | 130 |
| `database/138_villa_deduction_policy.sql` | Per-asset whitelist of permitted expense categories (PDF §4: "permitted expense deductions defined contractually"). | 131 |
| `database/139_villa_period_documents.sql` | Link table joining `asset_documents` to a specific `(asset_id, period_year, period_month, doc_type)`. | 132 |
| `database/140_fx_rates_daily.sql` | Daily FX snapshot (USD ↔ IDR ↔ EUR), used to freeze a rate per published log row. | none |
| `database/141_villa_forecast_suggestions.sql` | Developer-submitted forecast suggestions awaiting admin finalisation (Q4 lock-in). | 135 |
| `database/142_developer_asset_links.sql` | Confirm/extend `developer_projects` so a Developer user can be linked to specific villas they're authorised to submit for. Adds row-level enforcement. | none |

**Append-only enforcement.** Two layers:
1. App-level: all writes funnelled through `backend/src/admin/villa_operations.rs::insert_log_row`. No raw UPDATE/DELETE path exposed.
2. DB-level: `REVOKE UPDATE, DELETE ON villa_operations_log FROM <app_role>` in the migration plus a trigger that raises `EXCEPTION 'append-only'` on UPDATE/DELETE. Mirrors `audit_logs` discipline.

### 3.3 DDL — Key Tables

```sql
-- 130_asset_villa_ext.sql (extends assets)
ALTER TABLE assets
  ADD COLUMN IF NOT EXISTS tokenized_pct_bps        INTEGER         -- 10000 = 100.00%
                                                    CHECK (tokenized_pct_bps BETWEEN 0 AND 10000),
  ADD COLUMN IF NOT EXISTS tokens_payout_eligible   INTEGER         CHECK (tokens_payout_eligible >= 0),
  ADD COLUMN IF NOT EXISTS tokens_owner_retained    INTEGER         CHECK (tokens_owner_retained  >= 0),
  ADD COLUMN IF NOT EXISTS reserve_pct_bps          INTEGER NOT NULL DEFAULT 500
                                                    CHECK (reserve_pct_bps BETWEEN 0 AND 10000),
  ADD COLUMN IF NOT EXISTS payout_frequency         VARCHAR(20) NOT NULL DEFAULT 'monthly'
                                                    CHECK (payout_frequency IN ('monthly','quarterly','annual')),
  ADD COLUMN IF NOT EXISTS payout_currency          CHAR(3) NOT NULL DEFAULT 'USD',  -- investor payout currency
  ADD COLUMN IF NOT EXISTS distribution_record_day  INTEGER NOT NULL DEFAULT 1
                                                    CHECK (distribution_record_day BETWEEN 1 AND 28),
  ADD COLUMN IF NOT EXISTS withholding_tax_bps      INTEGER NOT NULL DEFAULT 0
                                                    CHECK (withholding_tax_bps BETWEEN 0 AND 10000),
  ADD COLUMN IF NOT EXISTS allow_developer_submission BOOLEAN NOT NULL DEFAULT TRUE, -- Q4 lock-in: enabled by default
  ADD COLUMN IF NOT EXISTS native_currency_code     CHAR(3) NOT NULL DEFAULT 'IDR',  -- Q1 lock-in: IDR is the norm
  ADD COLUMN IF NOT EXISTS mgmt_fee_bps             INTEGER         CHECK (mgmt_fee_bps BETWEEN 0 AND 10000); -- contract config
```

```sql
-- 132_villa_operations_log.sql
CREATE TABLE villa_operations_log (
    id                          BIGSERIAL PRIMARY KEY,
    asset_id                    UUID NOT NULL REFERENCES assets(id),
    period_year                 INTEGER NOT NULL CHECK (period_year BETWEEN 2000 AND 2100),
    period_month                INTEGER NOT NULL CHECK (period_month BETWEEN 1 AND 12),

    -- Operational inputs (PDF §2) — IDR-native per Q1 lock-in
    gross_rental_idr_cents      BIGINT  NOT NULL DEFAULT 0 CHECK (gross_rental_idr_cents >= 0),
    currency_code               CHAR(3) NOT NULL DEFAULT 'IDR',
    fx_rate_idr_to_usd_bps      INTEGER NOT NULL DEFAULT 1,         -- 1 bps ≈ 0.0001 USD per IDR; frozen at publish
    gross_rental_usd_cents      BIGINT  NOT NULL DEFAULT 0,         -- derived at publish, frozen
    nights_available            INTEGER NOT NULL DEFAULT 0 CHECK (nights_available BETWEEN 0 AND 31),
    nights_booked               INTEGER NOT NULL DEFAULT 0 CHECK (nights_booked     BETWEEN 0 AND 31),
    occupancy_bps               INTEGER GENERATED ALWAYS AS
                                  (CASE WHEN nights_available > 0
                                        THEN (nights_booked * 10000) / nights_available
                                        ELSE 0 END) STORED,
    adr_idr_cents               BIGINT  GENERATED ALWAYS AS
                                  (CASE WHEN nights_booked > 0
                                        THEN gross_rental_idr_cents / nights_booked
                                        ELSE 0 END) STORED,
    adr_usd_cents               BIGINT  GENERATED ALWAYS AS
                                  (CASE WHEN nights_booked > 0
                                        THEN gross_rental_usd_cents / nights_booked
                                        ELSE 0 END) STORED,
    -- Cost categories (IDR-native, sum enforced in app)
    expense_cleaning_idr_cents      BIGINT  NOT NULL DEFAULT 0,
    expense_maintenance_idr_cents   BIGINT  NOT NULL DEFAULT 0,
    expense_utilities_idr_cents     BIGINT  NOT NULL DEFAULT 0,
    expense_staff_idr_cents         BIGINT  NOT NULL DEFAULT 0,
    expense_pool_garden_idr_cents   BIGINT  NOT NULL DEFAULT 0,
    expense_pest_idr_cents          BIGINT  NOT NULL DEFAULT 0,
    expense_other_idr_cents         BIGINT  NOT NULL DEFAULT 0,
    ota_fees_idr_cents              BIGINT  NOT NULL DEFAULT 0,
    payment_fees_idr_cents          BIGINT  NOT NULL DEFAULT 0,
    refunds_idr_cents               BIGINT  NOT NULL DEFAULT 0,
    mgmt_fee_idr_cents              BIGINT  NOT NULL DEFAULT 0,

    -- Derived totals (IDR + USD pair, both frozen at publish)
    total_opex_idr_cents            BIGINT  NOT NULL DEFAULT 0,
    total_opex_usd_cents            BIGINT  NOT NULL DEFAULT 0,
    net_rental_income_idr_cents     BIGINT  NOT NULL DEFAULT 0,
    net_rental_income_usd_cents     BIGINT  NOT NULL DEFAULT 0,
    reserve_override_idr_cents      BIGINT,                         -- NULL = use assets.reserve_pct_bps
    reserve_applied_idr_cents       BIGINT  NOT NULL DEFAULT 0,
    platform_fee_idr_cents          BIGINT  NOT NULL DEFAULT 0,
    withholding_idr_cents           BIGINT  NOT NULL DEFAULT 0,
    distributable_idr_cents         BIGINT  NOT NULL DEFAULT 0,
    distributable_usd_cents         BIGINT  NOT NULL DEFAULT 0,

    -- Workflow / versioning
    status                      VARCHAR(20) NOT NULL DEFAULT 'draft'
                                CHECK (status IN ('draft','submitted','approved','published','superseded')),
    supersedes_id               BIGINT REFERENCES villa_operations_log(id),
    correction_reason           TEXT,
    submitted_by                UUID REFERENCES users(id),
    submitted_at                TIMESTAMPTZ,
    approved_by                 UUID REFERENCES users(id),
    approved_at                 TIMESTAMPTZ,
    published_at                TIMESTAMPTZ,
    recorded_at                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT approver_differs CHECK (approved_by IS NULL OR approved_by <> submitted_by)
);
CREATE INDEX idx_vop_asset_period_recorded
    ON villa_operations_log(asset_id, period_year DESC, period_month DESC, recorded_at DESC);
CREATE INDEX idx_vop_status_pending
    ON villa_operations_log(status) WHERE status IN ('submitted','approved');
CREATE INDEX idx_vop_supersedes ON villa_operations_log(supersedes_id) WHERE supersedes_id IS NOT NULL;
```

```sql
-- 133_villa_operations_current.sql  (trigger-maintained current view)
CREATE TABLE villa_operations_current (
    asset_id                    UUID    NOT NULL REFERENCES assets(id),
    period_year                 INTEGER NOT NULL,
    period_month                INTEGER NOT NULL,
    log_id                      BIGINT  NOT NULL REFERENCES villa_operations_log(id),
    distributable_cents         BIGINT  NOT NULL,
    net_rental_income_cents     BIGINT  NOT NULL,
    occupancy_bps               INTEGER NOT NULL,
    published_at                TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (asset_id, period_year, period_month)
);
-- Trigger on villa_operations_log AFTER INSERT WHEN NEW.status='published'
-- UPSERTs into this table. Older row's status is flipped to 'superseded'
-- via the same trigger by id-chain (NEW.supersedes_id).
```

```sql
-- 134_villa_valuations.sql
CREATE TABLE villa_valuations (
    id                  BIGSERIAL PRIMARY KEY,
    asset_id            UUID NOT NULL REFERENCES assets(id),
    valuation_date      DATE NOT NULL,
    valuation_cents     BIGINT NOT NULL CHECK (valuation_cents > 0),
    currency_code       CHAR(3) NOT NULL DEFAULT 'USD',
    valuation_method    VARCHAR(50) NOT NULL,         -- 'sales_comparison','income','cost','external_appraisal'
    appraiser_name      VARCHAR(200),
    appraiser_user_id   UUID REFERENCES users(id),
    comparables         JSONB,                         -- array of comp evidence
    notes               TEXT,
    evidence_doc_id     UUID REFERENCES asset_documents(id),
    status              VARCHAR(20) NOT NULL DEFAULT 'draft'
                        CHECK (status IN ('draft','submitted','approved','published','superseded')),
    supersedes_id       BIGINT REFERENCES villa_valuations(id),
    submitted_by        UUID REFERENCES users(id),
    approved_by         UUID REFERENCES users(id),
    published_at        TIMESTAMPTZ,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT val_approver_differs CHECK (approved_by IS NULL OR approved_by <> submitted_by)
);
CREATE INDEX idx_villa_val_asset_date ON villa_valuations(asset_id, valuation_date DESC);
```

```sql
-- 136_villa_market_prices_daily.sql
CREATE TABLE villa_market_prices_daily (
    asset_id            UUID NOT NULL REFERENCES assets(id),
    snapshot_date       DATE NOT NULL,
    nav_token_cents     BIGINT NOT NULL,           -- (valuation × tokenized_pct) / tokens_in_pool  [PDF §7]
    market_token_cents  BIGINT,                    -- VWAP from trade_history, NULL if no trades
    trade_count         INTEGER NOT NULL DEFAULT 0,
    volume_tokens       BIGINT  NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (asset_id, snapshot_date)
);
CREATE INDEX idx_vmp_asset_date ON villa_market_prices_daily(asset_id, snapshot_date DESC);
```

CapEx, deduction-policy, period-documents, FX-rates, expense-categories follow the same pattern — DDL deferred to the migration files; structure dictated by the table name.

---

## 4. Backend API Contract

**Module placement decision.** `backend/src/admin/assets.rs` is already ~70KB. Add new file `backend/src/admin/villa_operations.rs` (operations log + current-view writes), `backend/src/admin/villa_valuations.rs` (valuation flow), and extend `backend/src/admin/approvals.rs` to surface the two new entity types. KPI compute layer in `backend/src/assets/kpi.rs` (new), shared by admin + public read paths.

**Route grouping (per Q4 lock-in):**
- `/api/developer/...` → `UserRole::Developer`, write only Dev-owned fields (§2.5), can drive `draft → submitted` only; never `approved` / `published`.
- `/api/admin/...` → `UserRole::Admin`, write any field (Dev or Admin), drive every workflow state including overrides.
- `/api/villas/:id/...` (public) → investor / unauthenticated, read-only, `published` rows only.

| Method | Path | Role | File | Purpose |
|---|---|---|---|---|
| POST | `/api/developer/villas/:asset_id/operations` | Developer (assigned to villa via `developer_projects`) | `developer/villa_operations.rs` (new) | Insert log row with Dev-owned fields only; status forced to `draft`; Admin-owned fields auto-filled from `assets` config |
| PUT  | `/api/developer/villas/:asset_id/operations/:log_id` | Developer | same | Edit own draft row; Admin-owned fields rejected |
| PUT  | `/api/developer/villas/:asset_id/operations/:log_id/submit` | Developer | same | `draft → submitted` |
| POST | `/api/developer/villas/:asset_id/capex` | Developer | same | Submit CapEx event for admin approval |
| POST | `/api/developer/villas/:asset_id/annual/:year/tax-statement` | Developer | same | Upload tax statement → `asset_documents` |
| POST | `/api/developer/villas/:asset_id/forecast/:year/suggest` | Developer | same | Insert into `villa_forecast_suggestions` for admin review |
| GET  | `/api/developer/dashboard` | Developer | same | All villas assigned + per-month status |
| GET  | `/api/developer/villas/:asset_id/operations?year=&month=&as_of=` | Developer | same | Read own villa's history |
| POST | `/api/admin/villas/:asset_id/operations` | Admin | `admin/villa_operations.rs` (new) | Insert log row, all fields; status=`draft` or `submitted` |
| PUT  | `/api/admin/villas/:asset_id/operations/:log_id` | Admin | same | Edit any field on a draft row |
| PUT  | `/api/admin/villas/:asset_id/operations/:log_id/override` | Admin | same | Override a Dev-submitted row: create correction row with `supersedes_id=:log_id`, emits `audit_logs.action='admin_override'`, `correction_reason` required |
| PUT  | `/api/admin/villas/:asset_id/operations/:log_id/approve` | Admin (≠ submitter) | same | `submitted → approved` |
| PUT  | `/api/admin/villas/:asset_id/operations/:log_id/publish` | Admin | same | `approved → published`, fires trigger into `_current` |
| POST | `/api/admin/villas/:asset_id/operations/:log_id/correct` | Admin | same | Correction row, status=`draft`, `correction_reason` required |
| GET  | `/api/admin/villas/:asset_id/operations?year=&month=&as_of=` | Admin | same | Read with time-travel |
| GET  | `/api/admin/villas/:asset_id/operations/history?period=YYYY-MM` | Admin | same | Full supersession chain |
| POST | `/api/admin/villas/:asset_id/valuations`                  | Admin | `admin/villa_valuations.rs` (new) | New valuation, status=`draft` |
| PUT  | `/api/admin/villas/:asset_id/valuations/:val_id/{submit,approve,publish}` | Admin (4-eyes) | same | Workflow |
| GET  | `/api/admin/villas/:asset_id/valuations?as_of=` | Admin | same | Read with time-travel |
| POST | `/api/admin/villas/:asset_id/forecast/:year` | Admin | same | Finalise `villa_forecast_assumptions` (may pull from Dev suggestion) |
| PUT  | `/api/admin/villas/:asset_id/deduction-policy` | Admin | same | Set permitted-deductions whitelist |
| PUT  | `/api/admin/villas/:asset_id/config` | Admin | same | Update tokenization / reserve / record-day / fees / withholding |
| GET  | `/api/admin/approvals/villa-operations` | Admin | `admin/approvals.rs` (extend) | Queue of `submitted` rows |
| GET  | `/api/admin/approvals/villa-valuations` | Admin | same | Same for valuations |
| GET  | `/api/admin/approvals/villa-capex` | Admin | same | CapEx awaiting approval |
| GET  | `/api/villas/:asset_id/performance?as_of=YYYY-MM-DD&range=12m&display_currency=USD\|IDR` | Investor / public | `assets/routes.rs` (extend) | Public KPI bundle: monthly yield series, NAV series, market series, Annual Yield. Default `display_currency=USD` |
| GET  | `/api/villas/:asset_id/history?metric=nav&from=&to=&series=as_published\|as_corrected&display_currency=` | Investor / public | same | Chart endpoint |
| GET  | `/api/investors/me/portfolio?as_of=YYYY-MM-DD&display_currency=` | Investor | `investments/routes.rs` (extend) | Net Return per Investor, time-travel-aware |

**Rust struct sketches** (codebase style: `_cents: i64`, `_bps: i32`, derive `Serialize, Deserialize`, `sqlx::FromRow`):

```rust
// backend/src/admin/villa_operations.rs
#[derive(Debug, Deserialize)]
pub struct VillaOperationsInput {
    pub period_year: i32,
    pub period_month: i32,
    pub currency_code: String,                   // ISO 4217
    pub fx_rate_to_usd_bps: i32,
    pub gross_rental_native_cents: i64,
    pub nights_available: i32,
    pub nights_booked: i32,
    pub expense_cleaning_cents: i64,
    pub expense_maintenance_cents: i64,
    pub expense_utilities_cents: i64,
    pub expense_staff_cents: i64,
    pub expense_pool_garden_cents: i64,
    pub expense_pest_cents: i64,
    pub expense_other_cents: i64,
    pub ota_fees_cents: i64,
    pub payment_fees_cents: i64,
    pub refunds_cents: i64,
    pub mgmt_fee_cents: i64,
    pub reserve_override_cents: Option<i64>,
    pub correction_reason: Option<String>,       // required if supersedes_id set
    pub supersedes_id: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct VillaOperationsRow { /* mirrors villa_operations_log */ }
```

**Transaction boundaries.** Each workflow transition is a single `BEGIN ... COMMIT`:
1. **submit/approve/publish**: log table update (status flip) + audit_logs insert + (on publish) `villa_operations_current` upsert + supersession of prior row. All in one tx.
2. **correct**: insert new log row (status=draft) + audit_logs insert. One tx.
3. **publish** also triggers FX rate freeze + recompute of `villa_market_prices_daily` for the affected month (re-runs NAV calc with new distributable). All in one tx.

**Calculation responsibility split.** Stored on write (frozen): `gross_rental_usd_cents`, `total_opex_cents`, `net_rental_income_cents`, `reserve_applied_cents`, `platform_fee_cents`, `distributable_cents`. Computed on read: KPI %ages, monthly yield, annualised yield, NAV token price (read live from `villa_market_prices_daily` or recomputed via `assets/kpi.rs`).

---

## 5. KPI Calculation Layer

All formulas use integer arithmetic (cents × bps / 10000). Multi-step calcs use `i128` intermediate to prevent overflow.

| PDF KPI | Formula | Inputs | When | Cache |
|---|---|---|---|---|
| Net Rental Income (§5) | `gross_rental_usd_cents − total_opex_cents` (total_opex = sum of 11 expense columns + ota + payment_fees − refunds, frozen) | `villa_operations_log` | On write | Column `net_rental_income_cents` |
| Platform Fee (§5) | `net_rental_income_cents * assets.poool_split_pct / 100` (existing column; `poool_split_pct` is whole percent — keep) | log + assets | On write | Column `platform_fee_cents` |
| Reserve Applied (§5) | `COALESCE(reserve_override_cents, net_rental_income_cents * assets.reserve_pct_bps / 10000)` | log + assets | On write | Column `reserve_applied_cents` |
| Withholding Tax (§5) | `(net_rental_income_cents − platform_fee_cents − reserve_applied_cents) * assets.withholding_tax_bps / 10000` | log + assets | On write | Folded into `distributable_cents` |
| Distributable Income (§5) | `net_rental_income_cents − platform_fee_cents − reserve_applied_cents − withholding_cents` | derived | On write | Column `distributable_cents` |
| Payout per Token (§5) | `villa_operations_current.distributable_cents * 10000 / assets.tokens_payout_eligible` (returns micro-cents per token, divide later) | current + assets | On read | None (cheap) |
| Net Return per Investor (§5, §6) | `Σ over each held period: payout_per_token * investments_at_record_date.tokens_owned` − investor-specific deductions | current + investments + dividend_payouts | On read, parameter `as_of` | Per-request, optional Redis (Phase 7) |
| Monthly Yield % (§5) | `distributable_cents * 10000 / latest_nav_total_cents` (basis: NAV — UI must label clearly per PDF §5 note) | current + valuations | On read | None |
| Annualised Yield % (§5, §6) | `(sum(distributable_cents over last 12 months) * 10000) / latest_nav_total_cents` if `as_of` ≥ 12 months of data; else extrapolate from available months with `is_extrapolated=true` flag | current (12 rows) + valuations | On read | Materialised in `villa_market_prices_daily.annual_yield_bps` (Phase 4) |
| Projected Return % (§6) | `villa_forecast_assumptions.projected_annual_net_yield_bps + projected_appreciation_bps` | forecast_assumptions | On read | Static per forecast version |
| Projected Annualised Net Return % (§6) | Same as above (PDF doesn't separate); UI label only | forecast | On read | Static |
| 5-Year Total Return % (§6) | Compound formula across 5 forecast years: `∏(1 + annual_yield_bps + appreciation_bps) − 1`, bps-arithmetic | forecast (5 yrs) | On read | Static, recomputed when forecast updated |
| Share Price Performance +3M / +6M / +12M (§6) | `(nav_today − nav_then) * 10000 / nav_then` and analogous for `market_token_cents` (two series, never merged — PDF §8) | `villa_market_prices_daily` | On read | None |
| NAV Token Price (§7) | `(latest_valuation_cents × tokenized_pct_bps / 10000) / tokens_in_investor_pool` where `tokens_in_investor_pool = tokens_total − tokens_owner_retained` | valuations + assets | On read, daily snapshot job | `villa_market_prices_daily.nav_token_cents` |
| Market Token Price (§8) | VWAP from `trade_history` over rolling 24h, snapshotted daily | trade_history | Nightly job | `villa_market_prices_daily.market_token_cents` |

**`as_of` parameter flow.** All read queries take `as_of: Option<DateTime<Utc>>`. Behaviour:

```
SELECT * FROM villa_operations_log
WHERE asset_id = $1
  AND (period_year, period_month) = ($2, $3)
  AND recorded_at <= $4         -- as_of
  AND status IN ('published','superseded')
ORDER BY recorded_at DESC
LIMIT 1;
```

This returns "what investors saw at `as_of`". For "current truth", use `villa_operations_current` directly. The endpoint exposes both via `?series=as_published` vs `?series=as_corrected`.

**Daily NAV snapshot job.** A scheduled task (new `backend/src/jobs/villa_nav_snapshot.rs`, cron via existing job runner if present, else a `tokio::spawn` interval task) populates `villa_market_prices_daily` once per day at 00:30 UTC for every active asset. Idempotent on `(asset_id, snapshot_date)`.

---

## 6. Safe-Rollout Plan

### 6.1 Feature flag mechanism

A row in the existing `settings` table (no new infra): `villa_returns.enabled` = `'off' | 'shadow' | 'on'`. Plus per-asset opt-in column `assets.villa_returns_pilot = BOOLEAN DEFAULT FALSE`. Code checks both gates before reading new tables on investor-facing paths.

### 6.2 Migration sequencing (forward-compatible)

- All new columns on existing tables are **nullable with defaults** or `DEFAULT 0` BIGINT → safe online ALTERs on PG 16.
- No `DROP COLUMN` until Phase 6.
- No `NOT NULL` retrofitted to existing tables until a backfill is verified and the column has been populated by app writes for ≥ 14 days.
- `asset_financials` is **not modified**; we read-shadow it.

### 6.3 Read path during transition

| Phase | `asset_financials` (legacy) | `villa_operations_current` (new) | Investor sees |
|---|---|---|---|
| 1 | live, mutable | empty, structures only | legacy |
| 2 | live, mutable | written-to via admin form | legacy |
| 3 | live, mutable | written-to via admin + read for admin UI | legacy (investor) + new (admin) |
| 4 | live, mutable | written-to, **shadow-read** by KPI layer (compute both, log diff to `villa_returns.shadow_diff` log table) | legacy + new compared offline |
| 5 | live, mutable | full read path for KPI layer; investor UI behind flag | new for pilot assets, legacy for others |
| 6 | read-only, gradually deprecated | authoritative | new |
| 7 | dropped after 90 days of zero reads | authoritative | new |

### 6.4 Shadow-write / shadow-read

Phase 2–4: every admin write to `villa_operations_log` (publish step) also UPDATEs the legacy `asset_financials` row for the same `(asset_id, period_year, period_month)`. Phase 4 adds shadow-read: KPI computed from both, divergence logged. Phase 5 cuts over reads. Phase 6 stops shadow-writes. Phase 7 drops legacy table.

### 6.5 Backfill plan

Script `scripts/backfill_villa_operations.rs` (Rust binary):
1. For each `(asset_id, period_month, period_year)` row in `asset_financials`, INSERT one row into `villa_operations_log` with status=`published`, `recorded_at = asset_financials.created_at`, `published_at = asset_financials.created_at`, `gross_rental_usd_cents = rental_income_cents`, `total_opex_cents = expenses_cents`, `net_rental_income_cents = net_income_cents`, expense category breakdown all zero (legacy data has no breakdown — flag with `correction_reason = 'legacy backfill, breakdown unavailable'`).
2. Idempotent on `(asset_id, period_year, period_month, status='published', supersedes_id IS NULL)` — re-runs are no-ops.
3. Run in a transaction per asset (not global) → partial failure does not poison the table.
4. Logs row count + diff to stdout for ops review.

### 6.6 Rollback per migration

Every migration `130–140` ships with a sibling `database/rollback/130_*.sql` containing the reverse DDL. Rollback never `DROP TABLE` if data already written — instead `RENAME TO villa_operations_log__rollback_YYYYMMDD` for forensic recovery, then re-deploy a hotfix.

### 6.7 Test plan

Integration tests in `backend/tests/villa_operations_http.rs`:

1. `test_publish_creates_current_row` — happy path.
2. `test_correction_preserves_original_for_earlier_as_of` — the load-bearing invariant. Publish March, set as_of=`April 1`, read = original. Correct March in June, set as_of=`April 1`, read = original still. Set as_of=`NOW`, read = corrected. **This test is the contract.**
3. `test_approver_must_differ_from_submitter` — 4-eyes enforcement.
4. `test_developer_cannot_approve_own_submission` — RBAC + 4-eyes.
5. `test_supersession_chain_is_complete` — 5-row chain from §3.1 example reads correctly via `?period=YYYY-MM` history endpoint.
6. `test_legacy_shadow_write_matches` (Phases 2–6 only) — every publish writes both tables, values match.
7. `test_nav_uses_pdf_section_7_formula` — assert NAV = `(valuation × tokenized_pct / 10000) / (tokens_total − tokens_owner_retained)`, NOT `valuation / tokens_total`.
8. `test_fx_rate_frozen_on_publish` — correcting an FX rate later does not change historical USD figures for prior published periods.

### 6.8 Cloud Run / Cloud Build

- No new secrets required for Phases 1–5.
- Phase 6 (cut-over) needs a Cloud Run revision tag for instant rollback (`--tag stable-pre-villa-returns`).
- Phase 7 (drop legacy) needs DB backup verified ≤ 24 h before deploy.
- `cloudbuild.yaml` unchanged — migrations run via the existing startup migrate step in `backend/src/lib.rs`.

---

## 7. History & Chart Consumption

### 7.1 Query shape — "60 monthly NAV prices for asset X as published at time T"

```sql
WITH months AS (
  SELECT generate_series(
    date_trunc('month', $2::date) - INTERVAL '59 months',
    date_trunc('month', $2::date),
    INTERVAL '1 month'
  ) AS m
),
latest_per_month AS (
  SELECT DISTINCT ON (asset_id, EXTRACT(YEAR FROM m), EXTRACT(MONTH FROM m))
         asset_id, m, nav_token_cents
  FROM villa_market_prices_daily
  WHERE asset_id = $1
    AND snapshot_date <= LEAST(m + INTERVAL '1 month' - INTERVAL '1 day', $3)
    -- $3 = as_of cap; defaults to NOW() for "as latest"
  ORDER BY asset_id, EXTRACT(YEAR FROM m), EXTRACT(MONTH FROM m), snapshot_date DESC
)
SELECT m AS month, nav_token_cents
FROM months
LEFT JOIN latest_per_month USING (m)
ORDER BY m;
```

### 7.2 Endpoint

```
GET /api/villas/:asset_id/history
    ?metric=nav | market | yield | distributable
    &from=YYYY-MM-DD
    &to=YYYY-MM-DD
    &series=as_published | as_corrected      (default: as_corrected)
    &as_of=YYYY-MM-DD                        (only for series=as_published)

Response: { metric, currency, points: [{date, value_cents, is_corrected: bool}] }
```

### 7.3 Corrections in charts

- **Default series: `as_corrected`** — the current truth. Points where the underlying row has `supersedes_id IS NOT NULL` are flagged `is_corrected: true` and rendered with a small dot marker. Hover tooltip: "Corrected June 2026 — see history".
- **Toggle `as_published`** — what investors saw at any given `as_of`. Useful for regulatory / investor-relations questions ("what did my dashboard say on April 5?").
- Frontend chart (existing chart-rendering in `frontend/platform/static/js`): one component, two series toggle, plus a corrections-annotation overlay.
- **PDF §8 mandate:** NAV and Market are always **two separate series with different colours**, never merged into a single line. Enforced in the frontend chart helper.

---

## 8. Phased Delivery Roadmap

Each phase is a Cloud-Run-deployable increment. `[FF]` = feature-flag state of `villa_returns.enabled`.

| Phase | Effort | `[FF]` | Files / migrations | Success criteria |
|---|---|---|---|---|
| **P1 — Schema-only** | M | `off` | Migrations 130–140 + rollback siblings; no app code changes | All tables/columns exist in prod, prod traffic unaffected, rollback dry-run passes |
| **P2 — Admin + Developer write paths** | L | `off` | `backend/src/admin/villa_operations.rs` + `villa_valuations.rs`; `backend/src/developer/villa_operations.rs` (new); admin pages (`villa-operations-entry.html`, `villa-valuation.html`); developer pages (`developer/operations-submit.html`, `developer/dashboard.html`, `developer/annual-data.html`); extend `approvals.rs` + `approvals.html`; field-ownership whitelist `FIELD_OWNERSHIP`; admin-override endpoint emits `audit_logs.action='admin_override'` | Developer can submit a monthly log restricted to Dev-owned fields; Admin can submit / approve / publish / override any field; 4-eyes enforced; field-level rejection on cross-role writes; correction chain works for both override and self-correct |
| **P3 — Admin read + asset-details integration** | M | `off` | `frontend/platform/admin/asset-details.html` extension; `villa-history.html` (new); `GET .../operations`, `.../history` endpoints | Admin sees the 12-month strip on asset-details; history viewer shows supersession chain |
| **P4 — KPI calc layer + historical query API** | L | `shadow` | `backend/src/assets/kpi.rs` (new); shadow-write to legacy `asset_financials`; daily NAV-snapshot job; `villa_returns.shadow_diff` log table | All KPIs from PDF §5/§6 computed; shadow-read diff < 0.5 % vs legacy on every published period for ≥ 14 days |
| **P5 — Investor UI surfaces (pilot)** | L | `shadow`, per-asset `villa_returns_pilot=true` for ≤ 3 villas | `frontend/platform/property.html` Performance tab; `poool_app_home.html` + `my-trading.html` KPI cards with as-of selector; `transactions.html` filter | Pilot investors see NAV per PDF §7 formula; NAV and Market shown as two separate series; corrections annotated; no regression on non-pilot assets |
| **P6 — Cut-over + deprecate legacy** | M | `on` | Investor UI reads new layer for all assets; remove shadow-write; `asset_financials` becomes read-only (REVOKE writes) | Zero reads from `asset_financials` for 30 days; KPI dashboards stable |
| **P7 — Multi-currency + CapEx + advanced KPIs** | L | `on` | `villa_capex_events`, `fx_rates_daily`, 5-Year Total Return UI, forecast-assumption editor | IDR-native villas display correctly; CapEx surfaced separately from monthly yield; 5-Year projection visible on property page |
| **P8 (cleanup) — Drop legacy** | S | `on` | `DROP TABLE asset_financials` after 90-day no-read window | Table dropped, build green, no app reference remains |

**Critical path:** P1 → P2 → P3 → P4 (≈ 6–8 weeks if one engineer; 3–4 weeks with two). P5 onward gated by pilot signal from real ops users.

**Demo-able moments:** End of P2 (admin entry form), end of P3 (history viewer), end of P5 (investor performance tab on pilot villa).

---

## Assumptions explicitly carried into the plan (all confirmed by user)

1. **Q1 lock-in:** Villa core operates in IDR. All monetary inputs stored as `*_idr_cents`. USD pair derived at publish and frozen. Investor UI defaults to USD with native-IDR toggle. `assets.native_currency_code` default = `'IDR'`.
2. **Q4 lock-in:** Field-level role split per §2.5. Developer submits PDF-§2/§3-Management fields. Admin owns master/tokenization/valuation/policy fields + can override any Developer field. System fields are computed, no user writes. `assets.allow_developer_submission` default `TRUE`; per-asset kill-switch.
3. **Q5 lock-in:** 4-eyes approval from day one. Approver ≠ submitter, enforced in CHECK constraint.
4. **Q6 lock-in:** Investor visibility = `published` only.
5. **Q8 lock-in:** CapEx in `villa_capex_events`, never reduces a monthly payout.
6. PDF §7 NAV formula is authoritative — `(valuation × tokenized_pct_bps / 10000) / (tokens_total − tokens_owner_retained)`, never `valuation / tokens_total`.
7. PDF §8: NAV and Market token price are always two separate UI series, never merged.
8. `asset_financials` is retired (not patched in place) — append-only log is incompatible with the current mutable schema.
9. FX rates frozen at publish — corrections never silently revalue historical USD.
10. `audit_logs` reused for action trail; `correction_reason` lives on the new log row; cross-ref via `audit_logs.entity_type='villa_operations_log'` and a new `action='admin_override'` for Q4 admin-override events.

Any of these challenged → re-open the affected section before Phase 1 ships.
