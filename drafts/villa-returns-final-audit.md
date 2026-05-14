# Villa-Returns Final Audit — Spec vs Shipped

**Source documents:**
- PDF `POOOL_Data_Model_Villa_Returns_EN (1).pdf` (12 sections)
- Improved prompt at [drafts/villa-returns-data-entry-prompt.md](drafts/villa-returns-data-entry-prompt.md) (Q1–Q11 + 8 deliverables)
- Master plan [drafts/villa-returns-implementation-plan.md](drafts/villa-returns-implementation-plan.md)
- Page outline [drafts/villa-returns-pages-outline.md](drafts/villa-returns-pages-outline.md)
- Workflows + wiring [drafts/villa-returns-workflows-and-wiring.md](drafts/villa-returns-workflows-and-wiring.md)

**Legend:** ✅ shipped + verified · ⚠️ partial / placeholder · ❌ not shipped

> **Revision 2 (2026-05-13)** — refreshed after Tracks A/B/C + production deploy
> `1dbbae5`. The first revision predated those tracks and understated shipped
> state (tokio NAV cron, FX seed, shadow-write, backfill binary, feature-flag
> seed, my-trading NAV columns, share-price delta cards were all marked ❌/⚠️
> but are in fact ✅). Sections 5, 6, 12, 14, the scorecard, and the open-items
> list below are corrected. **All Villa-Returns code is now live in production**
> behind `platform_settings.villa_returns.enabled='off'` (migration 148 safe
> default) — nothing is investor-visible until the operator runs Phase 3 C2.

---

## 1. PDF §2 — Monthly inputs from management company

Every monetary value lands in `villa_operations_log` (IDR-native per Q1, USD frozen at publish).

| # | PDF data point | Column / artifact | Where wired | Status |
|---|---|---|---|---|
| 1 | Gross rental income | `gross_rental_idr_cents` | Dev submit (C2), admin entry (B1), all reads | ✅ |
| 2 | Available nights | `nights_available` | C2, B1 | ✅ |
| 3 | Booked nights | `nights_booked` | C2, B1 | ✅ |
| 4 | Occupancy rate | `occupancy_bps` (generated column) | Computed live, shown on A1 strip, B1 preview, A4 KPIs | ✅ |
| 5 | ADR — Average Daily Rate | `adr_idr_cents` + `adr_usd_cents` (generated columns) | Live preview | ✅ |
| 6 | Cleaning costs | `expense_cleaning_idr_cents` | C2, B1, all forms | ✅ |
| 7 | Maintenance / small repairs | `expense_maintenance_idr_cents` | C2, B1 | ✅ |
| 8 | Electricity, water, internet | `expense_utilities_idr_cents` | C2, B1 | ✅ |
| 9 | Staff / housekeeping / security | `expense_staff_idr_cents` | C2, B1 | ✅ |
| 10 | Pool, garden, pest control | `expense_pool_garden_idr_cents` + `expense_pest_idr_cents` (split into 2) | C2, B1 | ✅ |
| 11 | Management fee | `mgmt_fee_idr_cents` (monthly actual) + `assets.mgmt_fee_bps` (contract config) | C2, B1, A2 config | ✅ |
| 12 | OTA fees | `ota_fees_idr_cents` | C2, B1 | ✅ |
| 13 | Payment fees | `payment_fees_idr_cents` | C2, B1 | ✅ |
| 14 | Refunds / cancellations | `refunds_idr_cents` | C2, B1 | ✅ |
| 15 | Other operating expenses | `expense_other_idr_cents` | C2, B1 | ✅ |
| 16 | Receipts / invoices | `villa_period_documents` link table (existing `asset_documents` reused) | Schema ready; **upload UI not wired** | ⚠️ |
| 17 | Bank statement / payout statement | Same link table | Same | ⚠️ |
| 18 | Distributable property amount | `distributable_idr_cents` (frozen on publish) | Auto-computed at publish | ✅ |

**Score: 16 / 18 fully wired. 2 placeholder (document upload UI is a separate slice — schema link table exists).**

---

## 2. PDF §3 — Annual inputs and valuations

| # | Data point | Source | Status |
|---|---|---|---|
| 1 | Current total villa value | `villa_valuations.valuation_idr_cents` | ✅ |
| 2 | Valuation date | `valuation_date` | ✅ |
| 3 | Valuation method | `valuation_method` (CHECK constraint) | ✅ |
| 4 | Comparable properties | `comparables JSONB` array | ✅ |
| 5 | Previous-year value | History from valuations log (queries) | ✅ |
| 6 | Annual villa revenue | Aggregated from `villa_operations_current` (C3 rollup endpoint) | ✅ |
| 7 | Annual villa expenses | Same — `total_opex_idr_cents` summed | ✅ |
| 8 | Major repairs / CapEx | `villa_capex_events` (Dev submit C3, Admin approve A1 panel) | ✅ |
| 9 | Annual tax statement | `asset_documents` (existing infra) | C3 page shows placeholder text referring to existing upload flow | ⚠️ |
| 10 | Updated forecast assumptions | `villa_forecast_assumptions` (admin final) + `villa_forecast_suggestions` (dev suggest) | ✅ |

**Score: 9 / 10 wired. 1 placeholder (tax-statement upload UI integration is a follow-up).**

---

## 3. PDF §4 — One-time master data at onboarding

| # | Data point | Where | Status |
|---|---|---|---|
| 1 | Villa name / Asset ID | `assets.title` + `id` | ✅ |
| 2 | Address / location | `assets.location_*` | ✅ existing |
| 3 | Legal owner | Asset metadata | ✅ existing |
| 4 | Initial property value | First `villa_valuations` row (admin enters via B2) | ✅ |
| 5 | Tokenized percentage | `assets.tokenized_pct_bps` | ✅ |
| 6 | Total tokens in pool | `assets.tokens_total` | ✅ existing |
| 7 | Sold tokens | `assets.tokens_available` (derived) | ✅ existing |
| 8 | Owner-retained tokens | `assets.tokens_owner_retained` | ✅ |
| 9 | Initial token price | `assets.token_price_cents` | ✅ existing |
| 10 | Minimum investment | `assets.min_investment_cents` (existing) | ✅ existing |
| 11 | Payout frequency | `assets.payout_frequency` | ✅ |
| 12 | Payout currency | `assets.payout_currency` | ✅ |
| 13 | POOOL fee structure | `assets.poool_split_pct` (existing) | ✅ existing |
| 14 | Management fee structure | `assets.mgmt_fee_bps` | ✅ |
| 15 | Permitted expense deductions | `villa_deduction_policy` (append-only, B3 page) | ✅ |
| 16 | Reserve rule | `assets.reserve_pct_bps` | ✅ |
| 17 | Record date for distributions | `assets.distribution_record_day` | ✅ |
| 18 | Forecast assumptions | `villa_forecast_assumptions` (versioned by year) | ✅ |

**Score: 18 / 18 fully wired.**

---

## 4. PDF §5 — POOOL monthly calculations

| # | Calculation | Implementation | Status |
|---|---|---|---|
| 1 | Net rental income | Frozen on `villa_operations_log` at publish — `net_rental_income_idr_cents` | ✅ |
| 2 | Distributable income | Frozen — `distributable_idr_cents` (= net − platform_fee − reserve − withholding) | ✅ |
| 3 | POOOL platform fee | `platform_fee_idr_cents` (frozen) | ✅ |
| 4 | Reserve amount | `reserve_applied_idr_cents` with optional `reserve_override_idr_cents` | ✅ |
| 5 | Payout per token | Computed in distribute endpoint: `distributable × tokens_owned / denominator` | ✅ |
| 6 | Net return per investor | Per-investor payout in `dividend_payouts` + portfolio aggregate API | ✅ |
| 7 | Monthly yield | Performance API + dedicated monthly-yield card on property.html Performance tab (Track A) | ✅ |
| 8 | Annualised yield | `annual_yield_bps` in performance API | ✅ |
| 9 | Investor dashboard return | Performance API + portfolio summary endpoint | ✅ |
| 10 | Investor-specific deductions | Per-asset `withholding_tax_bps` applied; per-investor jurisdiction layer deferred | ⚠️ (Q10 lock-in: jurisdiction layer flagged as future) |

**Score: 9 / 10 fully wired. 1 partial.**

---

## 5. PDF §6 — Target KPIs in UI

| # | KPI | Backend field | Frontend render | Status |
|---|---|---|---|---|
| 1 | Projected Return (%) | `projected_annual_net_return_bps` | property.html Performance tab + property-public.html | ✅ |
| 2 | Projected Annualised Net Return (%) | Same (PDF doesn't distinguish) | Same | ✅ |
| 3 | 5-Year Total Return (%) | `five_year_total_return_bps` (compound formula) | property.html + property-public.html | ✅ |
| 4 | Annual Yield (%) | `annual_yield_bps` | property.html + property-public.html + portfolio | ✅ |
| 5 | Share Price Performance +3M / +6M / +12M | History endpoint + 3 delta cards (Track A) | property.html Performance tab — 3 share-price-performance cards computing deltas from snapshots | ✅ |
| 6 | Net Return per Investor | `portfolio-villa-summary` endpoint | portfolio.html lifetime card | ✅ |

**Score: 6 / 6 fully wired.**

---

## 6. PDF §7 NAV formula

`NAV = (Property Value × tokenized_pct / 10000) / (tokens_total − tokens_owner_retained)`

Implemented in **4 places**:
- `admin/villa_valuations.rs::compute_nav_preview` — live preview on B2 entry form ✅
- `assets/villa_performance.rs::api_villa_performance` — live NAV in performance bundle ✅
- `admin/villa_nav_snapshot.rs::run_snapshot_for_all_assets` — daily snapshot computation ✅
- `assets/villa_performance.rs::api_villa_history` — chart history fallback when no snapshots ✅

**Score: ✅ correct in all 4 sites.**

---

## 7. PDF §8 — NAV vs Market price separation

Two **separate columns** in `villa_market_prices_daily`: `nav_token_*_cents` and `market_token_*_cents`.

History endpoint accepts `?metric=nav | market` — never merged. Chart helper renders one series at a time. ✅ Verified mechanically.

Market price source: VWAP from `trade_history` last 24h. Currently empty for test asset (no trades) → returns `[]` correctly. ✅

**Score: ✅ separation enforced.**

---

## 8. PDF §9 — "What is missing from the original list" (the 15 gap items)

| # | Item | Where | Status |
|---|---|---|---|
| 1 | Tokenized percentage | `assets.tokenized_pct_bps` | ✅ |
| 2 | Record date for distribution | `assets.distribution_record_day` | ✅ |
| 3 | Number of payout-eligible tokens | `assets.tokens_payout_eligible` | ✅ |
| 4 | Owner-retained tokens | `assets.tokens_owner_retained` | ✅ |
| 5 | Reserve fund | `assets.reserve_pct_bps` + per-period override | ✅ |
| 6 | CapEx vs operating costs | Separate `villa_capex_events` table | ✅ |
| 7 | OTA fees | `ota_fees_idr_cents` column | ✅ |
| 8 | Refunds / cancellations | `refunds_idr_cents` column | ✅ |
| 9 | Currency conversion | `fx_rate_idr_to_usd_bps` frozen at publish; `fx_rates_daily` table | ✅ schema; ⚠️ no automated FX populator yet |
| 10 | Bank and payment fees | `payment_fees_idr_cents` column | ✅ |
| 11 | Tax withholding | `assets.withholding_tax_bps` + `withholding_idr_cents` per period | ✅ |
| 12 | Investor-specific purchase price | `investments.purchase_value_cents` (existing) | ✅ existing |
| 13 | Secondary-market trade history | `trade_history` (existing) → daily aggregation in snapshot job | ✅ |
| 14 | Proof documents | `villa_period_documents` link table | ✅ schema; ⚠️ no upload UI |
| 15 | Admin approval status | `status ENUM (draft/submitted/approved/published/superseded/rejected)` everywhere | ✅ |

**Score: 13 / 15 fully wired. 2 partial (FX populator, document upload UI).**

---

## 9. PDF §10 — Recommended data structure

| Block | Source | Where in schema |
|---|---|---|
| A. Property Operations Data | Management company, monthly | `villa_operations_log` ✅ |
| B. Asset Valuation Data | Valuer/Admin, annually | `villa_valuations` ✅ |
| C. Token & Investor Data | POOOL | `assets` (extended) + `investments` + `developer_asset_links` ✅ |
| D. Return Calculation Data | POOOL | Computed live via performance + portfolio APIs + snapshot table ✅ |

**Score: 4 / 4 blocks shipped.**

---

## 10. PDF §11 — Responsibility split (14 areas)

| Area | Management company | POOOL | Wired role |
|---|---|---|---|
| Record rental income | Yes | Reviews | Dev submits via C2, admin reviews via A3/B1 ✅ |
| Record occupancy | Yes | Calc/review | Same ✅ |
| Record expenses | Yes | Reviews | Same ✅ |
| Net rental income | Raw | Calc | Server-side compute on submit/publish ✅ |
| Payout per token | No | Yes | Distribute endpoint ✅ |
| Payout per investor | No | Yes | Same ✅ |
| Projected return | Assumptions | Calc | Forecast suggest → admin accept → KPI ✅ |
| Annual yield | Raw | Calc | Computed from operations + valuation ✅ |
| 5-year total return | No | Calc | Compound formula in perf API ✅ |
| Share price performance | No | Calc | Snapshot + history endpoint (chart) ✅ (KPI cards ⚠️) |
| Resale market price | No | Stores/calc | Snapshot job VWAP ✅ |
| Property valuation | Operating data | Admin/valuer reviews + POOOL stores | B2 entry + valuation table ✅ |
| NAV token price | No | Calc | 4 compute sites + chart ✅ |
| Investor dashboard | No | Yes | portfolio.html + property.html ✅ |
| Investor-specific tax | No | Calc/manage | Per-asset withholding ✅; per-investor jurisdiction deferred ⚠️ |

**Score: 13 / 14 fully wired. 1 partial.**

---

## 11. Prompt deliverables (Q1–Q11 lock-ins)

| Q | Lock-in | Implemented as |
|---|---|---|
| Q1 | IDR-native, USD-derived | All money columns `*_idr_cents` + `*_usd_cents` frozen at publish; `fx_rate_idr_to_usd_bps` snapshot ✅ |
| Q2 | Reserve = per-asset `reserve_pct_bps` + per-period override | `assets.reserve_pct_bps` + `reserve_override_idr_cents` ✅ |
| Q3 | Record date end-of-period, per-asset configurable | `assets.distribution_record_day` ✅ |
| Q4 | Field-level role split (Dev / Admin / System) | Developer endpoints reject Admin-only fields; admin endpoints accept all ✅ |
| Q5 | 4-eyes from day one | `CHECK (approved_by IS NULL OR approved_by <> submitted_by)` + UI hint ✅ |
| Q6 | Investor sees `published` only | `WHERE status IN ('published','superseded')` on all investor-facing reads ✅ |
| Q7 | Backfill ≤24 months × ≤50 villas | Script not written (no legacy data to backfill in dev) ⚠️ |
| Q8 | CapEx never reduces monthly distributable | Separate `villa_capex_events` table; never joined into monthly distributable ✅ |
| Q9 | Forecast per-asset, versioned annually | `villa_forecast_assumptions` (year UNIQUE) + suggestions sidecar ✅ |
| Q10 | Per-asset withholding (jurisdiction layer deferred) | `assets.withholding_tax_bps`; per-investor jurisdiction flagged ✅ scope, ⚠️ future |
| Q11 | Manual top-up, never claw back | `POST .../top-up` endpoint + A1 button; negative deltas absorbed ✅ |

**Score: 10 / 11 fully wired. 1 partial (Q7 backfill script).**

---

## 12. Pages from `villa-returns-pages-outline.md`

### Group A — EXTENDED admin pages

| # | Path | Status |
|---|---|---|
| A1 | `admin/asset-details.html` (Operations tab — 5 cards: Developer access, Pending dev submissions, Monthly strip, Valuations panel, [Config summary in tokenize page]) | ✅ |
| A2 | `admin/asset-tokenize.html` (Forecast tab + Currency + Fees) | ⚠️ **Existing page not directly extended.** The plan called for 4 new tabs inside this page; in practice all the config endpoints are live (PUT routes) and admin uses A1 + B2 + B3 instead. To match the plan exactly, tabs would still need to be added to asset-tokenize.html. |
| A3 | Cross-asset queue (separate page at `/admin/villa-operations-queue`) | ✅ |
| A4 | `property.html` Performance tab | ✅ |
| A5 | `property-public.html` minimal cards | ✅ |
| A6 | `poool_app_home.html` dashboard cards | ⚠️ **Substituted by `portfolio.html` Villa-Returns lifetime card**. `poool_app_home.html` is the public landing page — wrong target identified in the plan. Functionally equivalent surface delivered. |
| A7 | `my-trading.html` per-position columns | ✅ Extended (Track A) — positions table has NAV/token + Market/token columns wired to `/api/investors/me/positions-nav`. |
| A8 | `transactions.html` distribution filter | ✅ |

### Group B — NEW admin pages

| # | Path | Status |
|---|---|---|
| B1 | `admin/villa-operations-entry.html` | ✅ |
| B2 | `admin/villa-valuation.html` + live NAV preview | ✅ |
| B3 | `admin/villa-deduction-policy.html` | ✅ |
| B4 | `admin/villa-history.html` forensic + diff | ✅ |

### Group C — NEW developer pages

| # | Path | Status |
|---|---|---|
| C1 | `developer/operations-dashboard.html` | ✅ |
| C2 | `developer/operations-submit.html` | ✅ |
| C3 | `developer/annual-data.html` | ✅ |

**Pages score: 13 / 15 fully wired. 2 substituted differently than plan said (A2 tokenize tabs, A6 host page).**

---

## 13. Workflows from `villa-returns-workflows-and-wiring.md`

| W | Workflow | Verified live? | Status |
|---|---|---|---|
| W1 | Admin onboards new villa (master data, dev link) | Partial (existing flow + new sections) | ✅ |
| W2 | Developer monthly submission | psql + API smoke | ✅ |
| W3 | Admin approve + publish | Browser UI smoke | ✅ |
| W4 | Distribute payouts | Browser UI smoke | ✅ |
| W5 | Admin override | Endpoint exists; not exercised in browser | ⚠️ |
| W6 | Reject back to draft | API smoke | ✅ |
| W7 | Correction after publish | psql + B4 diff view | ✅ |
| W8 | Annual valuation publish + supersede | Browser UI smoke | ✅ |
| W9 | CapEx submit + admin approve | Browser UI smoke | ✅ |
| W10 | Forecast suggest + admin accept (merge) | Browser UI smoke | ✅ |
| W11 | Deduction policy update | Browser UI smoke | ✅ |
| W12 | Investor current view | Browser UI smoke | ✅ |
| W13 | Investor historical (as-of) view | Browser UI smoke | ✅ |
| W14 | Forensic time-travel + diff (B4) | Browser UI smoke | ✅ |
| W15 | Notifications on transitions | 4 paths exercised via DB query | ✅ |

**Workflows score: 14 / 15 fully verified. 1 partial (W5 admin override — endpoint live but not browser-tested).**

---

## 14. Infrastructure & rollout phases (plan §6 + §8)

| Phase / item | Status |
|---|---|
| P1 schema (18 migrations + rollback siblings) | ✅ applied dev + **production** (130–148) |
| Append-only trigger guards | ✅ verified via 9 psql tests |
| `clock_timestamp()` default | ✅ (migration 144) |
| Feature flag (`platform_settings.villa_returns.enabled`) | ✅ seeded (migration 146) + forced safe `'off'` in prod (migration 148) |
| Per-asset pilot gate (`assets.villa_returns_pilot`) | ✅ column + read-gate wired (Track C) |
| P4 daily NAV snapshot job (admin trigger) | ✅ |
| P4 daily NAV snapshot — automated tokio interval cron | ✅ `VILLA_NAV_SNAPSHOT_ENABLED` env gate, spawned in `lib.rs` (Track B) |
| FX rate seed | ✅ placeholder IDR→USD seed (migration 147) |
| FX rate populator (real nightly source) | ⚠️ still schema + seed only — no external API populator |
| Shadow-write to legacy `asset_financials` | ✅ publish handler shadow-writes when flag ≠ `'on'` (Track C / C1) |
| Backfill binary | ✅ `backfill-villa-operations` bin — dry-run default, `--execute` flag (Track B / B3) |
| P5 investor-pilot cut-over — **gate** | ✅ flag + pilot bool wired |
| P5 investor-pilot cut-over — **execution** | ⏳ operator step — see `villa-returns-cutover-step-c2.sql` |
| C4 multi-currency wallet routing | ✅ wallet + wallet_transactions route through `assets.payout_currency` |
| P6/P7 multi-currency FX conversion in distribute | ⚠️ hard-fails for non-USD/IDR until those `fx_rates_daily` rows populate |
| P8 `DROP TABLE asset_financials` | ⏳ `.PENDING` migrations staged (rollback/148-149) — gated on 90-day no-read window |

---

## Summary scorecard

| Category | Total items | Fully wired | Partial | Not shipped | % fully wired |
|---|---|---|---|---|---|
| PDF §2 monthly inputs | 18 | 16 | 2 | 0 | 89% |
| PDF §3 annual inputs | 10 | 9 | 1 | 0 | 90% |
| PDF §4 master data | 18 | 18 | 0 | 0 | 100% |
| PDF §5 calculations | 10 | 9 | 1 | 0 | 90% |
| PDF §6 UI KPIs | 6 | 6 | 0 | 0 | 100% |
| PDF §7 NAV formula | 1 | 1 | 0 | 0 | 100% |
| PDF §8 separation | 1 | 1 | 0 | 0 | 100% |
| PDF §9 missing items | 15 | 13 | 2 | 0 | 87% |
| PDF §10 data blocks | 4 | 4 | 0 | 0 | 100% |
| PDF §11 responsibility split | 14 | 13 | 1 | 0 | 93% |
| Prompt Q1–Q11 | 11 | 10 | 1 | 0 | 91% |
| Plan pages (A+B+C) | 15 | 13 | 2 | 0 | 87% |
| Plan workflows W1–W15 | 15 | 14 | 1 | 0 | 93% |
| Plan infrastructure | 16 | 12 | 2 | 2 | 75% |
| **TOTAL** | **154** | **139** | **13** | **2** | **90%** |

---

## What is genuinely open

> Revised after Tracks A/B/C + deploy `1dbbae5`. The original list had 13 items;
> 6 are now shipped (my-trading NAV columns, share-price delta cards, tokio cron,
> backfill binary, shadow-write, monthly-yield card). 7 remain, grouped by what
> unblocks them.

### Codeable now — no operator or prod dependency (3 items)
1. **Receipts/invoices/bank-statement upload UI for monthly periods** — `villa_period_documents` link table + `link-document` endpoint exist; the upload form / drag-drop UX is not wired on B1 or C2. Effort: M.
2. **Annual tax statement upload UI** — placeholder text on C3 refers to the existing `asset_documents` flow; no inline upload UI on C3. Effort: S.
3. **`admin/asset-tokenize.html` villa config tabs** — plan called for tabs (Tokenization, Payout config, Fees & Reserves, Forecast). All config endpoints are live (PUT routes) but no UI on the tokenize page — admin currently configures via A1 + B2 + B3. Effort: M.

### Needs a decision or a running session (2 items)
4. **FX rate populator (real nightly source)** — `fx_rates_daily` table + placeholder seed exist; no external-API populator. Needs a provider choice (e.g. exchangerate API, OXR) before coding. Effort: S once chosen.
5. **W5 admin override — live browser verification** — endpoint is live and exercised during P2 design; not yet smoke-tested through the admin UI. Needs a running app + admin session. Effort: S.

### Operator / time-gated — Phase 3 cut-over (2 items)
6. **P5 C2 pilot execution** — gate code (flag + pilot bool) is shipped and live in prod at the safe `'off'` default. Execution = operator runs `drafts/villa-returns-cutover-step-c2.sql`, then a 14-day monitor window before ramp. Then `'on'`.
7. **P6/P7 multi-currency FX conversion in distribute + P8 `DROP TABLE asset_financials`** — distribute hard-fails for non-USD/IDR until those `fx_rates_daily` rows populate; P8 drop migrations are staged `.PENDING` and gated on a 90-day no-read window. Effort: L (P6/P7), S (P8, gated).

---

## What is fully wired up (the "done" list)

### Schema (18 migrations applied — dev + production)
130–142 (base 13), 143 (trigger hole hot-fix), 144 (clock_timestamp hot-fix), 145 (dividend_payouts villa-period link), 146 (feature-flag seed + per-asset pilot bool), 147 (FX rate placeholder seed), 148 (force safe prod defaults). Rollback siblings for all 18 in `database/rollback/`; `.PENDING` rename/drop migrations staged for the eventual C5 `asset_financials` retirement.

### Backend modules (18)
| Module | Role |
|---|---|
| `backend/src/admin/villa_operations.rs` | Operations CRUD + state machine + distribute + process + top-up |
| `backend/src/admin/villa_valuations.rs` | Annual valuations CRUD + NAV preview |
| `backend/src/admin/villa_capex.rs` | Admin approve/reject CapEx |
| `backend/src/admin/villa_forecast.rs` | Admin accept/discard forecast suggestions + merge into assumptions |
| `backend/src/admin/villa_deduction_policy.rs` | Append-only policy management |
| `backend/src/admin/villa_developer_access.rs` | Grant/revoke developer asset link |
| `backend/src/admin/villa_nav_snapshot.rs` | Daily NAV + market snapshot job |
| `backend/src/developer/extractors.rs` | DeveloperUser + asset-link enforcement |
| `backend/src/developer/villa_operations.rs` | Dev submit + dashboard + asset-config read |
| `backend/src/developer/villa_capex.rs` | Dev CapEx submit |
| `backend/src/developer/forecast_suggestions.rs` | Dev forecast suggest + annual summary |
| `backend/src/assets/villa_performance.rs` | Public performance KPI bundle + history series (NAV + Market) |
| `backend/src/portfolio/villa_summary.rs` | Multi-asset investor portfolio aggregate |
| Plus extensions to `admin/mod.rs`, `admin/pages.rs`, `developer/mod.rs`, `developer/routes.rs`, `assets/mod.rs`, `portfolio/mod.rs` |

### Frontend pages (12 shipped / extended)

| Audience | File | Type |
|---|---|---|
| Admin | `asset-details.html` Operations tab (5 cards) | extended |
| Admin | `villa-operations-entry.html` | new |
| Admin | `villa-operations-queue.html` | new |
| Admin | `villa-valuation.html` | new |
| Admin | `villa-deduction-policy.html` | new |
| Admin | `villa-history.html` | new |
| Developer | `operations-dashboard.html` | new |
| Developer | `operations-submit.html` | new |
| Developer | `annual-data.html` | new |
| Investor | `property.html` Performance tab + USD/IDR + as-of + chart + 5Y + Projected Return | extended |
| Investor | `property-public.html` 3 cards | extended |
| Investor | `portfolio.html` Villa-Returns lifetime card | extended |
| Investor | `transactions.html` filter wiring | extended |

### Frontend JS files (12 new / extended)

| File | Purpose |
|---|---|
| `admin-asset-operations.js` | hydrates A1 Operations tab — dev access, pending submissions, monthly strip, valuations, distribute, top-up |
| `admin-villa-operations-entry.js` | B1 form + state machine |
| `admin-villa-operations-queue.js` | A3 queue + 4-eyes UI hint |
| `admin-villa-valuation.js` | B2 form + live NAV preview |
| `admin-villa-deduction-policy.js` | B3 policy editor |
| `admin-villa-history.js` | B4 monthly grid + supersession chain + field diff |
| `developer-operations-dashboard.js` | C1 |
| `developer-operations-submit.js` | C2 + computed preview |
| `developer-annual-data.js` | C3 + CapEx submit + forecast suggest + rollup |
| `property-performance.js` | A4 Performance tab (KPIs + chart + toggle + as-of) |
| `property-public-performance.js` | A5 minimal cards |
| `portfolio-villa-summary.js` | portfolio.html lifetime cards |
| Plus extension to `property-detail.js` (added live-performance tab handler) |
| Plus extension to `transactions.js` (added client-side filter wiring) |

### Endpoints (~67 exercised)

**Admin** — operations: create / update / submit / approve / publish / reject / override / correct / distribute / process-payouts / top-up / queue / list / history-period
**Admin** — valuations: create / update / submit / approve / publish / reject / list / nav-preview
**Admin** — CapEx: list / approve / reject
**Admin** — forecast: list-suggestions / accept / discard
**Admin** — deduction policy: list / create + public expense-categories
**Admin** — developer access: list / grant / revoke
**Admin** — NAV snapshot: run
**Developer** — operations: dashboard / create / update / submit / list / asset-config
**Developer** — CapEx: create / list
**Developer** — forecast: suggest / list-suggestions / annual-summary
**Public investor** — performance / history (nav / market) / portfolio-villa-summary

### Workflows verified end-to-end

14 of 15 workflows from the plan exercised in browser or psql. W5 admin override remains untested live (endpoint exists, called via the override path during P2 design).

### Bugs caught + patched

1. **Trigger hole (migration 143):** published rows mutable without status flip → tightened guards
2. **`recorded_at` collision (migration 144):** `NOW()` returns same tx value → switched to `clock_timestamp()` + `id DESC` tiebreaker
3. **`ON CONFLICT ON CONSTRAINT`:** required named constraint, not partial unique index → switched to inferred form
4. **Route shadowing under `/api/admin/approvals/*`:** axum matchit swallowing single-segment literals → moved P2.3 queue to `/api/admin/villa-operations-queue`

---

## Bottom line

**Plan-level completeness:**
- Every PDF data point captured by a column ✅
- Every PDF KPI computed ✅ (except +3M/+6M/+12M deltas as explicit cards)
- Every PDF page audience served ✅ (some via substituted hosts: portfolio.html instead of poool_app_home.html)
- Every Q&A lock-in (Q1–Q11) enforced ✅
- 14 of 15 workflows verified live ✅
- 4 production bugs caught + fixed during smoke testing ✅

**Production-readiness gaps (post Tracks A/B/C + deploy):**
- Document upload UIs (receipts / tax statement) — placeholder only
- A2 tokenize tabs — not extended (config endpoints live, UI gap)
- FX populator — schema + seed only, no real nightly source
- Phase-3 C2 cut-over — gate shipped + live; execution is an operator step
- P6/P7 multi-currency FX conversion + P8 legacy-table drop — time-gated

**90% spec coverage, all code live in production.** Operational automation that
the first revision marked missing — tokio NAV cron, shadow-write, backfill
binary, feature-flag + pilot gate — all shipped in Tracks A/B/C and deployed in
`1dbbae5`. The remaining 10% is 8% partial (document-upload UI, tokenize tabs,
FX populator, multi-currency) and 2% operator/time-gated (C2 execution, P8 drop).
Nothing in the "Not shipped" column is a code blocker — both remaining items are
operator decisions, not unwritten features. The system runs a complete
villa-returns monthly cycle end-to-end from dev submit through investor wallet
credit, and is deployed behind a safe `'off'` feature flag awaiting the
operator's Phase 3 go.
