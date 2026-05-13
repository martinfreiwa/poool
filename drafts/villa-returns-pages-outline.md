# Villa-Returns — Page-by-Page Outline

Per-surface breakdown of what gets built. Pulled from the implementation plan §2 + §2.5 + §4. Group A = extended (existing files), Group B = new admin pages, Group C = new developer pages.

**Totals: 8 extended + 4 new admin + 3 new developer = 15 surfaces.**

---

## GROUP A — EXTENDED (existing pages)

### A1. `frontend/platform/admin/asset-details.html`
**Audience:** Admin
**Today:** Single villa detail page — metadata, images, financial summary, milestones, documents.
**Add section:** `<section id="monthly-operations">` after the existing financial-summary block.

**Content of the new section:**
- 12-month strip of monthly status badges: `Draft / Submitted / Approved / Published / Superseded`
- Per row: month, distributable (USD + IDR toggle), occupancy %, net rental, **"Distribute payout"** button on `published` rows that have no distribution yet (W4), **"Top up $X"** button on superseded rows whose correction delta vs prior payout > 5 % (W15/Q11), action menu (`view`, `edit if draft`, `correct if published`)
- Overdue alert banner: villas with month >10 days past period end and no draft → red bar at top of section (W2)
- Top-right CTA: **"Enter month manually"** → opens `villa-operations-entry.html?asset_id=...&year=Y&month=M&mode=entry`
- Second CTA: **"View full history"** → links to `villa-history.html?asset_id=...`
- Add `<section id="valuations">` directly below: table of `villa_valuations` rows (date, value USD/IDR, method, appraiser, status, evidence-doc link), CTA "New valuation". **"CapEx since last valuation" tile** showing sum of approved CapEx events between last valuation and now (W9 input for next appraisal).
- Add `<section id="developer-access">` — list of Developer users authorised for this villa via `developer_asset_links`. Per row: user, effective_from, effective_until, "revoke" button. CTA "Add Developer" → modal with user autocomplete (W1).
- Add `<section id="config-summary">` (read-only mirror of the live config: tokenized %, reserve %, record day, mgmt fee bps, payout frequency, withholding bps, deduction policy doc link). Editing happens on `asset-tokenize.html` + `villa-deduction-policy.html`.

**API calls:**
- `GET /api/admin/villas/:id/operations?range=12m`
- `GET /api/admin/villas/:id/valuations`
- `GET /api/admin/villas/:id/config-summary`
- `GET /api/admin/villas/:id/developer-access`
- `POST /api/admin/villas/:id/developer-access` (add Developer)
- `DELETE /api/admin/villas/:id/developer-access/:user_id` (revoke)
- `POST /api/admin/villas/:id/operations/:log_id/distribute` (W4)
- `POST /api/admin/villas/:id/operations/:log_id/top-up` (Q11 manual top-up; creates `dividend_payouts.payout_type='bonus'`)

---

### A2. `frontend/platform/admin/asset-tokenize.html`
**Audience:** Admin
**Today:** Sets token count, token price, funding targets at onboarding.
**Add tabs / details blocks** (no new URL):

1. **Tokenization** (existing — leave as is): `tokens_total`, `tokens_owner_retained`, `tokens_payout_eligible` (new), `tokenized_pct_bps` (new), initial token price, min investment
2. **Payout config** (new): `payout_frequency`, `payout_currency`, `distribution_record_day`, `withholding_tax_bps`
3. **Fees & Reserves** (new): `poool_split_pct` (existing), `mgmt_fee_bps` (new contract-level), `reserve_pct_bps` (new)
4. **Forecast assumptions** (new): per-year forecast inputs from `villa_forecast_assumptions` — projected rent growth, occupancy, ADR, expense inflation, appreciation, exit assumption. "View Developer suggestions" panel pulling from `villa_forecast_suggestions`.
5. **Currency** (new): `native_currency_code` (default IDR), `allow_developer_submission` toggle

**API calls:**
- `PUT /api/admin/villas/:id/config`
- `GET /api/admin/villas/:id/forecast/:year`
- `POST /api/admin/villas/:id/forecast/:year`
- `GET /api/admin/villas/:id/forecast/:year/suggestions`

---

### A3. `frontend/platform/admin/approvals.html`
**Audience:** Admin
**Today:** Generic approvals queue (powered by `backend/src/admin/approvals.rs`).
**Add three new tabs alongside existing tabs:**

1. **Villa Operations** — list rows where `villa_operations_log.status = 'submitted'`. Columns: villa, period (YYYY-MM), submitter, submitted_at, supersedes (yes/no), correction_reason. Row actions: `Review` (opens B1 in `mode=review`), `Approve`, `Reject` (W6 — `PUT .../reject` with required reason textarea, sends row back to `draft` and notifies developer), `Edit and override` (W5 — opens B1 in `mode=override`).
2. **Villa Valuations** — same shape, for `villa_valuations.status = 'submitted'`.
3. **CapEx Events** — for `villa_capex_events` awaiting admin approval (developer-submitted).

**Enforcement:** approver ≠ submitter is enforced server-side. UI shows "You submitted this row — another admin must approve" disabled state on the Approve button.

**API calls:**
- `GET /api/admin/approvals/villa-operations`
- `GET /api/admin/approvals/villa-valuations`
- `GET /api/admin/approvals/villa-capex`
- `PUT /api/admin/villas/:id/operations/:log_id/approve`
- `PUT /api/admin/villas/:id/operations/:log_id/publish`

---

### A4. `frontend/platform/property.html`
**Audience:** Investor (logged-in)
**Today:** Property detail page — description, images, financials summary, buy button, documents.
**Add tab UI** above existing content. Tabs: `Overview` (existing) / `Performance` (new) / `Documents` (existing, reorganized) / `Reports` (existing or new).

**Performance tab content (PDF §6 + §8):**
- Two-card KPI row: **Annual Yield %**, **5-Year Total Return %** (with "projected" badge if forecast-derived)
- Two-line chart: **NAV Token Price** (one colour) vs **Market Token Price** (different colour) — never merged. Time-range buttons: 3M / 6M / 12M / All. Per PDF §8 hard rule.
- Monthly yield bar chart: 12 trailing months, hover shows distributable USD + IDR
- "Show in IDR / USD" toggle on the whole tab
- "As of" date picker (defaults to today) → time-travel to historical state per `villa_market_prices_daily` + `villa_operations_log` with `recorded_at ≤ as_of`
- **Banner when as_of ≠ NOW:** "Viewing as published at <date>. Some values may have been corrected since." with "View latest" reset button (W13)
- **Correction annotation:** months with a supersession get a small dot on the chart; hover tooltip shows original vs corrected values (W15)
- **Investor-position reference line** at `investments.purchased_at`, dashed style, so investor can see "performance since I bought in"
- Footer note: NAV formula explainer (§7), "data through {month}"

**Documents tab additions:**
- Group existing `asset_documents` by year/month using `villa_period_documents` link
- "Period proof" section: receipts, invoices, bank statements per published month, only visible if `is_investor_visible=true`

**API calls:**
- `GET /api/villas/:id/performance?as_of=&range=12m&display_currency=`
- `GET /api/villas/:id/history?metric=nav&series=as_corrected`
- `GET /api/villas/:id/history?metric=market`
- `GET /api/villas/:id/documents?group_by=period`

---

### A5. `frontend/platform/property-public.html`
**Audience:** Public (unauthenticated)
**Today:** Stripped-down property preview.
**Add:** read-only Annual Yield % and Projected Return % cards pulled from the new KPI layer. No history toggle (public can't time-travel). No NAV chart (gated behind login).

**API calls:**
- `GET /api/villas/:id/performance?range=12m` (public-safe subset)

---

### A6. `frontend/platform/poool_app_home.html`
**Audience:** Investor dashboard landing.
**Today:** Portfolio summary cards + orderbook + leaderboard.
**Add:**
- **"As of" date picker** in the page header — propagates to every KPI card on the page
- New KPI card: **Net Return per Investor (lifetime)** — sum across all positions, time-travel-aware
- New KPI card: **Portfolio Annual Yield %** — weighted by position size
- Existing portfolio card extended with NAV vs Market price for each position (mini sparkline)
- Currency toggle in the header (USD / IDR)

**API calls:**
- `GET /api/investors/me/portfolio?as_of=&display_currency=`

---

### A7. `frontend/platform/my-trading.html`
**Audience:** Investor positions + history.
**Today:** Active positions, historical trades, P&L.
**Add:**
- Per-position row: NAV Token Price + Market Token Price (two side-by-side columns, never one)
- Per-position **Annual Yield %** and **Net Return (since purchase)**
- "As of" date picker (same as A6)
- Currency toggle (USD / IDR)

**API calls:**
- Same `GET /api/investors/me/portfolio?...` as A6 — single endpoint serves both pages

---

### A8. `frontend/platform/transactions.html`
**Audience:** Investor wallet transactions.
**Today:** Lists buys, sells, payouts, fees.
**Add:**
- Filter dropdown: `type=distribution` to surface dividend payouts cleanly
- Each distribution row links to source: `dividend_payouts.id → villa_operations_log.id (period that funded it)`
- Show payout in payout-currency + USD equivalent (per `assets.payout_currency`)

**API calls:**
- Extend existing transactions endpoint with `?type=distribution` filter

---

## GROUP B — NEW ADMIN PAGES

### B1. `frontend/platform/admin/villa-operations-entry.html` (NEW)
**Route:** `/admin/villas/:asset_id/operations/:year/:month`
**Audience:** Admin
**Purpose:** Monthly operations entry form (PDF §2). Admin can enter or override any field.

**Three modes (URL param `mode=entry|review|override`):**
- `entry` — admin enters from scratch; all fields editable; saves to a new `draft` row
- `review` — read-only view of a submitted row; only Approve/Reject/Edit-and-override actions enabled
- `override` — pre-filled from supersedes row; admin edits fields; saving creates a new row with `supersedes_id` set, status=`draft`; original values shown as strikethrough next to each editable field for diff clarity

**Form sections** (vertical accordion):
1. **Rental** — `gross_rental_idr_cents`, `nights_available`, `nights_booked` (occupancy + ADR auto-computed and displayed live)
2. **Operating expenses** (one input per category): cleaning, maintenance, utilities, staff, pool/garden, pest, other
3. **Marketplace & adjustments**: `ota_fees_idr_cents`, `payment_fees_idr_cents`, `refunds_idr_cents`
4. **Management fee** — `mgmt_fee_idr_cents` (actual paid this month) vs `assets.mgmt_fee_bps × net` (computed reference shown beside)
5. **Computed preview** (read-only, live as user types): total OpEx, net rental, platform fee, reserve applied, withholding, **distributable** — all in IDR + USD frozen at submit
6. **Reserve override** (admin-only): `reserve_override_idr_cents` if applicable
7. **Proof documents** — upload zone for receipts, invoices, bank statement → `asset_documents` + `villa_period_documents` link
8. **Correction reason** — required in `override` mode, also required if this row supersedes another in `entry` mode
9. **Workflow actions** — `Save draft`, `Submit for approval`, `Approve` (disabled if approver=submitter — 4-eyes), `Publish` (disabled until approved), `Save override` (only in `override` mode — calls `.../override` endpoint, NOT a plain PUT on the original row)

**API calls:**
- `POST /api/admin/villas/:id/operations` (create draft)
- `PUT /api/admin/villas/:id/operations/:log_id` (edit draft)
- `PUT /api/admin/villas/:id/operations/:log_id/submit|approve|publish`
- `PUT /api/admin/villas/:id/operations/:log_id/override`
- `POST /api/admin/villas/:id/operations/:log_id/correct`

---

### B2. `frontend/platform/admin/villa-valuation.html` (NEW)
**Route:** `/admin/villas/:asset_id/valuations/new` (and `/:val_id/edit` for drafts)
**Audience:** Admin
**Purpose:** Annual valuation entry (PDF §3).

**Form fields:**
- `valuation_date`, `valuation_cents` (IDR), USD-derived shown live, `currency_code`
- `valuation_method` dropdown: `sales_comparison / income / cost / external_appraisal`
- `appraiser_name`, `appraiser_user_id` (autocomplete on existing users)
- `comparables` — repeatable rows of `{address, sale_price, sale_date, notes}` stored as JSONB array
- `notes` textarea
- `evidence_doc_id` — upload PDF → `asset_documents` then link
- Workflow buttons: `Save draft`, `Submit`, `Approve`, `Publish` (4-eyes)
- **Live preview panel:** NAV token price recomputed using this draft valuation per PDF §7 formula, shown to operator before publish to catch errors

**API calls:**
- `POST /api/admin/villas/:id/valuations`
- `PUT /api/admin/villas/:id/valuations/:val_id`
- `PUT /api/admin/villas/:id/valuations/:val_id/{submit,approve,publish}`

---

### B3. `frontend/platform/admin/villa-deduction-policy.html` (NEW)
**Route:** `/admin/villas/:asset_id/deduction-policy`
**Audience:** Admin
**Purpose:** Per-asset whitelist of permitted expense categories (PDF §4 "Permitted expense deductions defined contractually").

**Page content:**
- Checkbox grid of expense categories from `villa_expense_categories` lookup
- Per category: max % of gross rental allowed (optional cap), upload "evidence required" toggle
- "Effective from" date — earlier published rows are unaffected
- Audit-trail box showing prior policies + who changed them (from `audit_logs`)

**API calls:**
- `GET /api/admin/villas/:id/deduction-policy`
- `PUT /api/admin/villas/:id/deduction-policy`

---

### B4. `frontend/platform/admin/villa-history.html` (NEW)
**Route:** `/admin/villas/:asset_id/history?as_of=YYYY-MM-DD`
**Audience:** Admin
**Purpose:** Read-only forensic view of any past state.

**Sections:**
1. **Time-travel header:** date picker for `as_of`, "Jump to: 1 / 3 / 6 / 12 / 24 months ago" quick buttons
2. **Monthly grid:** all months from villa onboarding to `as_of`, colour-coded by status (published/superseded/never-published)
3. **Period drilldown** (clicking a month): full supersession chain rendered as a vertical timeline — each row shows `recorded_at`, who, status transitions, **field-level diff** vs prior version (red strikethrough → green new value). Pulls from `audit_logs` for who+when, from the log table for what.
4. **Snapshot view:** "Show how investor X saw this month on date Y" — useful for IR / regulatory queries
5. **Export:** CSV / JSON dump of the chain for legal

**API calls:**
- `GET /api/admin/villas/:id/operations/history?period=YYYY-MM`
- `GET /api/admin/villas/:id/operations?as_of=`
- `GET /api/admin/audit-logs?entity_type=villa_operations_log&entity_id=...`

---

## GROUP C — NEW DEVELOPER PAGES

### C1. `frontend/platform/developer/dashboard.html` (NEW)
**Route:** `/developer/dashboard`
**Audience:** Developer (management company login)
**Purpose:** Single landing page for assigned villas + status overview.

**Content:**
- Table of villas this developer is authorised to submit for (via `developer_asset_links` / extended `developer_projects`)
- Per villa columns: name, last published month, current month status badge (`not started / draft / submitted / approved / published / rejected`), days-since-last-submission, CTA per row
- **Rejected rows show the admin's rejection reason inline** (W6)
- **Overdue detection**: >`settings.villa_submission_grace_days` (default 10) past period end with no draft → red `Overdue` badge
- Top alert bar: "X villas overdue for {month}"
- Quick links: "Submit operations" / "Submit CapEx" / "Suggest forecast" / "Upload tax statement"
- Read-only widget showing global submission deadline (configurable in `settings`)

**API calls:**
- `GET /api/developer/dashboard`

---

### C2. `frontend/platform/developer/operations-submit.html` (NEW)
**Route:** `/developer/villas/:asset_id/operations/new?year=&month=` (and `/:log_id/edit` for drafts)
**Audience:** Developer
**Purpose:** Monthly operations entry **restricted to Dev-owned fields per §2.5**.

**Form sections** (same shape as B1 but field-level restricted):
1. **Editable (Dev fields):** gross rental IDR, nights available, nights booked, all expense categories, OTA fees, payment fees, refunds, mgmt fee (actual)
2. **Read-only (Admin/System fields):** computed preview pane shows what reserve %, platform fee, withholding will apply when admin publishes — pulled live from `GET /api/developer/villas/:id/asset-config` — but values are NOT user-editable
3. **Live computed-preview** (client-side JS — formulas duplicated in `developer-villa-operations-submit.js`, unit-tested; server is the authority on publish): total OpEx, net rental, platform fee, reserve applied, withholding, distributable, all in IDR + USD
4. **Document uploads:** receipts, invoices, bank statement → `asset_documents` + `villa_period_documents`; multiple files per period allowed
5. **Workflow actions** — `Save draft`, `Submit for approval`. **No** Approve / Publish / Override buttons (gated by role).
6. **Status banner:** "Submitted, awaiting Admin approval" / "Approved, awaiting publish" / "Published — to edit, request correction" / **"Rejected: <reason>"** (W6) / **"Deduction policy updated <date> — please re-check categories"** (W11)
7. **Correction request:** if a published row exists, button "Request correction" opens a textarea → creates a new draft row with `supersedes_id` populated, `correction_reason` required, status starts at `draft`. Admin must still approve.
8. **Policy-violation handling on submit:** if expense category violates `villa_deduction_policy`, server returns 400 with per-field detail; UI highlights offending fields and suggests moving cost to `expense_other_idr_cents` with a note.

**API calls:**
- `POST /api/developer/villas/:id/operations`
- `PUT  /api/developer/villas/:id/operations/:log_id`
- `PUT  /api/developer/villas/:id/operations/:log_id/submit`

---

### C3. `frontend/platform/developer/annual-data.html` (NEW)
**Route:** `/developer/villas/:asset_id/annual/:year`
**Audience:** Developer
**Purpose:** Annual data PDF §3 — Developer-owned subset.

**Sections:**
1. **CapEx events** — list of `villa_capex_events` for this year + form to submit a new event: `event_date`, `amount_idr_cents`, `category`, `description`, evidence doc upload. Status workflow: `draft → submitted` (admin approves).
2. **Annual tax statement** — upload zone → `asset_documents` typed `'tax_statement'`, year-tagged. Read-only after admin approval.
3. **Forecast suggestions** — form mirroring admin's forecast tab but saves to `villa_forecast_suggestions`. Admin merges/overrides in `asset-tokenize.html` Forecast tab.
4. **Annual rollup preview** (read-only) — system-computed totals from all published monthly logs for the year, displayed back to the developer for sanity-check.

**API calls:**
- `GET  /api/developer/villas/:id/capex?year=`
- `POST /api/developer/villas/:id/capex`
- `POST /api/developer/villas/:id/annual/:year/tax-statement`
- `GET  /api/developer/villas/:id/forecast/:year`
- `POST /api/developer/villas/:id/forecast/:year/suggest`

---

## Sequencing recommendation

Ship pages in this order to minimise half-states in production:

| Phase | Pages | Reason |
|---|---|---|
| **P2** | B1, B2, B3 (admin entry) + C1, C2, C3 (developer entry) + A3 (approvals tabs) | Write path online, both roles, with 4-eyes. No investor-visible changes yet. |
| **P3** | A1, A2 (asset-details + tokenize extensions) + B4 (history viewer) | Admin can read everything, time-travel, see corrections. |
| **P5** | A4, A5, A6, A7, A8 (investor surfaces) | Investor sees new KPIs only after KPI layer P4 is shadow-validated. |

Public/investor pages (A4–A8) deliberately ship last — admin and developer can stress-test the data pipeline against real numbers before any investor sees a changed figure.
