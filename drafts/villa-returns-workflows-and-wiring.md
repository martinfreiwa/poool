# Villa-Returns — Workflows & Frontend↔Backend Wiring

Companion to the implementation plan and the page outline. This doc:
1. Walks every user workflow end-to-end to verify the right surfaces, the right logic, and surface gaps.
2. Codifies the exact frontend↔backend wiring pattern the codebase uses, so every new page is built consistently.
3. Lists gap-fixes that need to be back-applied to the page outline.

---

## Part 1 — Frontend ↔ Backend Wiring Contract

Verified against existing code: `backend/src/admin/pages.rs:349` (`render_admin_template`), `frontend/platform/static/js/admin-asset-details.js:1`, `:681` (csrfHeaders).

**Every new page follows this five-file pattern:**

| Layer | File | Responsibility |
|---|---|---|
| HTTP route (HTML) | `backend/src/admin/pages.rs` (extend) or `backend/src/developer/pages.rs` (new module file) | `pub async fn page_admin_villa_operations_entry(...)` → calls `render_admin_template(state, "admin/villa-operations-entry.html")`. Auth-gated by `AdminUser` extractor. |
| HTTP route (JSON) | `backend/src/admin/villa_operations.rs` (new) or `backend/src/developer/villa_operations.rs` (new) | All `/api/admin/...` or `/api/developer/...` endpoints. Returns `Json<T>` where `T: Serialize`. |
| Router wiring | `backend/src/lib.rs` (`build_platform_router`) | Mount new routes under `/api/admin/villas/...`, `/api/developer/villas/...`, page routes under `/admin/villas/...`, `/developer/...`. |
| Template | `frontend/platform/admin/villa-operations-entry.html` (or `developer/...`) | MiniJinja `{% extends "admin/_base.html" %}` (or developer equivalent). Static skeleton — no data interpolated at render time except `csrf_token`, `current_user`, breadcrumbs. |
| Client JS | `frontend/platform/static/js/admin-villa-operations-entry.js` (or `developer-villa-operations-submit.js`) | All data fetching, form submission, validation, render. Loaded via `<script src="/static/js/admin-villa-operations-entry.js" defer>` at end of template. |

**Naming convention (enforced).** HTML file `admin/foo-bar.html` ↔ JS file `admin-foo-bar.js` ↔ Rust handler `page_admin_foo_bar`. Same for developer scope. Required so existing tooling and future devs can locate companions instantly.

### 1.1 Client request pattern

Every JS module reuses the same shared helpers (already in `frontend/platform/static/js/admin-asset-details.js:681` and similar):

```js
// Read (GET)
const resp = await fetch(`/api/admin/villas/${assetId}/operations?year=${year}&month=${month}`);
if (!resp.ok) throw new Error(await responseError(resp));
const data = await resp.json();

// Write (POST/PUT/DELETE) — MUST include CSRF header
const resp = await fetch(`/api/admin/villas/${assetId}/operations`, {
  method: 'POST',
  headers: csrfHeaders({ 'Content-Type': 'application/json' }),
  body: JSON.stringify(payload),
});

// csrfHeaders() pulls X-CSRF-Token from the csrf_token cookie (set by Rust on session creation)
```

**Hard requirement:** every mutating endpoint requires `X-CSRF-Token`. Server validates against the value bound to the session in `auth` middleware. Existing pattern — do not invent a new one.

**Auth:** session cookie `poool_session` is sent automatically (same-origin). Server extracts user via `AdminUser` / `DeveloperUser` extractor. New `DeveloperUser` extractor must enforce: role = Developer AND user is linked to the `asset_id` via `developer_asset_links` (new table from migration 142). Reject with 403 otherwise.

### 1.2 Server response shape (canonical)

Success: `{ ok: true, data: <T> }` or just `<T>` for resource reads (match neighbouring endpoints to stay consistent).

Error: HTTP 4xx/5xx with body `{ error: { code: 'string_slug', message: 'human readable', field?: 'gross_rental_idr_cents' } }`. JS `responseError(resp)` already understands this shape.

### 1.3 Initial SSR data (when JSON over-the-wire is wasteful)

For pages where the first render needs a non-trivial dataset (e.g. B4 history viewer with 60 months of NAV), the Rust page handler may inject a single JSON blob into the template:

```jinja
<script id="initial-data" type="application/json">{{ initial_data | tojson | safe }}</script>
```

The JS module reads `JSON.parse(document.getElementById('initial-data').textContent)` on init, then uses fetch for subsequent interactions. Used sparingly — only when first-paint matters or when the request would otherwise be a chatty waterfall.

### 1.4 Realtime / push

Out of scope for P1–P5. Notifications surface on next page load via the existing `notifications` table polled by `admin-bell.js`. Future: WebSocket channel for the approval queue if volume grows.

### 1.5 New backend module skeleton — concrete file layout

```
backend/src/admin/
  pages.rs                ← +12 lines: route handlers for B1/B2/B3/B4 templates
  villa_operations.rs     ← NEW: admin write/read JSON endpoints (P2)
  villa_valuations.rs     ← NEW: admin valuation JSON endpoints (P2)
  villa_capex.rs          ← NEW: admin approve CapEx endpoints (P2)
  approvals.rs            ← EXTEND: add villa_ops/valuations/capex queue endpoints
  assets.rs               ← EXTEND: add /config endpoint for the tokenize-page Q4 fields
  
backend/src/developer/
  pages.rs                ← NEW: route handlers for C1/C2/C3 templates
  villa_operations.rs     ← NEW: developer write JSON endpoints (P2)
  villa_capex.rs          ← NEW: developer CapEx submit endpoints
  forecast_suggestions.rs ← NEW: developer forecast suggest endpoints
  extractors.rs           ← NEW: DeveloperUser extractor + asset-link check

backend/src/assets/
  kpi.rs                  ← NEW: KPI calc layer (P4) — shared by admin/public reads
  history.rs              ← NEW: time-travel query layer
  
backend/src/jobs/
  villa_nav_snapshot.rs   ← NEW: daily NAV snapshot job (P4)
  villa_distribution.rs   ← NEW: per-period distribution trigger (P6)
```

`backend/src/lib.rs::build_platform_router()` adds the new routes. No new top-level module — `developer` and `admin` already exist.

---

## Part 2 — Workflow Walkthroughs

For each workflow: trigger → which page → user actions → frontend calls → backend transitions → side effects → which surface refreshes. Gaps surfaced inline.

### W1. Onboard new villa (admin only, PDF §4)

1. Admin → existing `admin/assets.html` → "New Asset" CTA → existing create flow.
2. **Gap-fix:** the new-asset form (modal or wizard) must collect at minimum: title, slug, `native_currency_code` (default `IDR`), property_type. Without `native_currency_code`, developer submissions fail validation downstream.
3. After creation → redirect to `admin/asset-details.html?id=<new>`.
4. Admin → A2 `admin/asset-tokenize.html` → fills tokenization, payout config, fees, reserve, forecast assumptions. PUT `/api/admin/villas/:id/config`.
5. Admin → B3 `admin/villa-deduction-policy.html` → picks permitted expense categories.
6. Admin → A1 `admin/asset-details.html` → "Management & Developer access" section (**NEW — add to A1**) → links a Developer user to this villa via `developer_asset_links`. POST `/api/admin/villas/:id/developer-access` with `{user_id, effective_from}`.
7. Admin → B2 `admin/villa-valuation.html` → enters initial valuation (PDF §4 "Initial property value"). Required before any NAV display.

**Backend transactions:**
- Create asset: existing flow, one tx.
- Link developer: one row insert in `developer_asset_links`, audit_log entry.
- Initial valuation: insert into `villa_valuations` with `status='published'` (admin can fast-path the first valuation — second-admin approval still required by 4-eyes).

**Refresh impact:** new villa shows up on developer's C1 dashboard immediately (provided developer linked in step 6).

**GAP IDENTIFIED:** A1 page outline did not list "Management & Developer access" section. **Back-applied below.**

---

### W2. Developer monthly submission (happy path)

1. **Trigger:** scheduled (we assume operations data for month M is ready around M+5 days). Developer either receives a notification ("March 2026 ready to submit") or self-navigates to `/developer/dashboard`.
2. Developer → C1 `developer/dashboard.html` loads → JS calls `GET /api/developer/dashboard` → returns list of assigned villas with per-month status. **Overdue** villas (>10 days past period end with no draft) highlighted.
3. Developer clicks "Submit March 2026" → navigates to `/developer/villas/:id/operations/new?year=2026&month=3`.
4. Page renders C2. JS init calls:
   - `GET /api/developer/villas/:id/asset-config` → returns the read-only admin fields (reserve %, platform fee %, mgmt fee bps, withholding bps, deduction policy whitelist) shown in the "Computed preview" pane.
   - `GET /api/developer/villas/:id/operations?year=2026&month=3` → returns existing draft if any (re-entering), otherwise null.
5. Developer fills the Dev-owned fields. As they type, JS optionally calls `POST /api/developer/villas/:id/operations/preview` (a non-persisting calc endpoint) to render the computed preview pane (net rental, reserve, fee, distributable) live. **Decision:** preview is client-computed in the first cut (formulas are simple integer arithmetic — duplicate the math in JS once). Server-side preview added only if business logic diverges.
6. Developer clicks "Save draft" → JS sends `POST /api/developer/villas/:id/operations` (or `PUT` if log_id exists). Backend creates row with `status='draft'`, `submitted_by=current_user`, returns log_id.
7. Developer attaches proofs → uploads to `asset_documents` via existing storage flow, then links via `POST /api/developer/villas/:id/operations/:log_id/documents`.
8. Developer clicks "Submit for approval" → JS sends `PUT /api/developer/villas/:id/operations/:log_id/submit`. Backend:
   - Validates row completeness (all Dev-owned required fields non-null).
   - **Validates expense categories against deduction policy.** Policy violations → 400 with `error.code='policy_violation'` and per-field detail. Developer fixes or moves cost to `expense_other_idr_cents` with a note (audit log captures the categorization).
   - Sets `status='submitted'`, `submitted_at=NOW()`.
   - Inserts `notifications` row for each Admin user: type='asset_submission', action_url to the approval page.
   - Inserts `audit_logs` row.
9. Developer dashboard refreshes → row now shows `Submitted, awaiting approval`.

**Refresh impact on other pages:**
- A3 admin/approvals: new row in "Villa Operations" tab.
- A1 admin/asset-details: month-strip badge flips to `Submitted`.

---

### W3. Admin approval — clean path (admin agrees with developer numbers)

1. Admin → A3 `admin/approvals.html` → Villa Operations tab → sees submitted row.
2. Click "Review" → opens B1 `admin/villa-operations-entry.html?asset_id=...&log_id=...&mode=review` in read-only mode (all fields disabled, "Approve" + "Reject" + "Edit and override" buttons enabled).
3. Admin clicks "Approve" → JS sends `PUT /api/admin/villas/:id/operations/:log_id/approve`. Backend:
   - Checks `approved_by != submitted_by` (4-eyes). If equal → 400.
   - Sets `status='approved'`, `approved_at=NOW()`, `approved_by=admin_id`.
   - Inserts `audit_logs` row.
4. Admin clicks "Publish" → `PUT /api/admin/villas/:id/operations/:log_id/publish`. Backend, single tx:
   - Sets `status='published'`, `published_at=NOW()`.
   - Freezes FX rate: if `fx_rate_idr_to_usd_bps` was the default placeholder, look it up from `fx_rates_daily` for the period's last day and write back; recompute the `*_usd_cents` derived columns; store frozen.
   - UPSERTs `villa_operations_current` for `(asset_id, period_year, period_month)`.
   - If a prior `published` row existed: flip prior row's `status` to `superseded` (via the new row's `supersedes_id`).
   - Recomputes and inserts `villa_market_prices_daily` row for today (NAV may have changed if this is the latest period).
   - Inserts `audit_logs` entry.
5. **Optional distribution trigger** (next workflow W4).

**Refresh impact:**
- C1 developer dashboard: status → `Published`.
- A1 admin asset-details: strip → `Published`.
- A4 investor property.html Performance tab: new month appears in chart on next load.
- A6/A7 investor dashboards: portfolio KPIs recompute on next load.

---

### W4. Distribution payout trigger (admin)

1. Admin → A1 `admin/asset-details.html` Monthly Operations section → on a `published` row, button **"Distribute payout"** appears.
2. **Gap-fix:** this button was not in the original A1 outline. **Back-applied below.**
3. Admin clicks → confirms modal "Distribute $X across N investors? Record date: YYYY-MM-DD". Backend `POST /api/admin/villas/:id/operations/:log_id/distribute`:
   - Reads `villa_operations_current.distributable_idr_cents` (or `_usd_cents` based on `assets.payout_currency`).
   - Queries `investments` for `tokens_owned` as of `record_date` (i.e. `purchased_at <= record_date AND status IN ('active','funded','rented')`).
   - For each eligible investor: insert `dividend_payouts` row with `amount_cents`, status='scheduled', wallet_tx_id NULL.
   - Idempotency: unique constraint on `(asset_id, period_year, period_month, user_id)` for `payout_type='rental'`. Re-runs raise duplicate-key, handled.
   - Insert `notifications` for each investor.
4. Existing dividend processor (cron or admin "Run scheduled" button on admin/dividends.html) advances payouts `scheduled → processing → paid` via wallet transactions.

**Refresh impact:**
- A8 transactions.html: new distribution rows visible per investor.
- A6 dashboard: portfolio "lifetime payouts" KPI bumps.

---

### W5. Admin overrides developer submission (admin disagrees)

1. From A3 approvals → admin clicks "Edit and override" on B1 review screen → fields become editable.
2. Admin edits values, fills `correction_reason` field (now required).
3. Click "Save override" → `PUT /api/admin/villas/:id/operations/:log_id/override`:
   - Backend creates a NEW row in `villa_operations_log` with `supersedes_id=<dev_row_id>`, `status='draft'`, `submitted_by=admin_id`, all fields from the override.
   - Original dev-submitted row stays `status='submitted'` (preserved for audit).
   - audit_logs row with `action='admin_override'`, before/after JSON.
4. Admin submits the new row → status `submitted`. **4-eyes: a second admin must approve.** If only one admin user exists, the workflow is blocked → admin must invite a second admin OR temporarily disable 4-eyes via `settings` row (operator decision, audited).
5. Approval flow same as W3.

**Gap-fix:** the original outline implied the admin can just "edit" the dev's row in place. That is wrong — would destroy audit. Chain is mandatory.

---

### W6. Rejection (admin sends back to developer)

1. From B1 review → admin clicks "Reject" → modal asks for reason.
2. `PUT /api/admin/villas/:id/operations/:log_id/reject` with `{reason}`. Backend:
   - Sets `status='draft'` (returns to developer for edit).
   - audit_logs row with `action='reject'`, metadata `{reason}`.
   - Notification to developer.
3. Developer sees on C1 dashboard → status `Draft (rejected: <reason>)`.
4. Developer reopens C2, edits, resubmits.

**Decision:** rejected rows do NOT supersede; they revert. Only published rows generate supersession chains.

---

### W7. Correction after publish (the load-bearing scenario)

**Case A: developer-initiated.**
1. Developer → C2 → loads existing published row → "Request correction" button.
2. Modal: reason textarea (required).
3. `POST /api/developer/villas/:id/operations/:log_id/correct`. Backend inserts new draft with `supersedes_id` set, `correction_reason`, fields copied from the published row pre-filled.
4. Developer edits fields → submits → admin approves → publishes.
5. On publish: prior published row flips to `superseded`; current view updates.

**Case B: admin-initiated.**
1. Admin → A1 → click published month → B1 in correction mode (`?supersedes_id=...`).
2. Edits → submit → second admin approves → publish.

**Investor visibility:**
- Default investor view (as_of=NOW) shows the corrected value.
- A4 chart marks the corrected month with an annotation dot.
- Investor with as_of=<before correction> still sees the original value.

**Re-payout on correction:** Q11 (new question, below).

---

### W8. Annual valuation (admin only)

1. Admin → A1 → Valuations section → "New valuation" → B2 form.
2. B2 init `GET /api/admin/villas/:id/valuations/draft-context` → returns: previous valuation, approved CapEx since previous valuation (helps decide new value), comparables template, tokenization snapshot for live NAV preview.
3. Admin fills fields. JS live-computes NAV preview from the draft `valuation_cents` + `tokenized_pct_bps` + `tokens_total - tokens_owner_retained`.
4. Admin attaches appraiser PDF → uploads to `asset_documents`, link via evidence_doc_id.
5. Submit → second admin approves → publish.
6. On publish: `villa_market_prices_daily` recomputes NAV for today + going forward. Historical NAV before this publish unchanged. audit_logs row.

**Refresh impact:**
- A4 NAV line jumps at the publish date (rendered as a discrete step in the chart, not interpolated).
- A1 valuations table updates.

---

### W9. CapEx event (developer submits, admin approves)

1. Developer → C3 `developer/annual-data.html` → "Submit CapEx" form → `POST /api/developer/villas/:id/capex` with `{event_date, amount_idr_cents, category, description, evidence_doc_id}`.
2. Status `submitted`. Notification to admin.
3. Admin → A3 → CapEx tab → reviews → approve / reject.
4. On approve: CapEx event lands in approved bucket. Does NOT reduce any monthly distributable (per Q8 lock-in).
5. Next time admin enters a new valuation on B2: "Approved CapEx since last valuation: $X" appears in the draft-context panel. Admin uses it as input but is not bound to it.

**Refresh impact:**
- A1 asset-details: "CapEx since last valuation" tile.
- Annual rollup on C3 / annual report: CapEx line item separated from OpEx.

---

### W10. Forecast assumption update

1. Developer → C3 → "Suggest forecast for 2027" → form with projected rent growth, occupancy, ADR, expense inflation, appreciation, exit yield.
2. `POST /api/developer/villas/:id/forecast/2027/suggest` → inserts row in `villa_forecast_suggestions`. Notification to admin.
3. Admin → A2 `asset-tokenize.html` Forecast tab → sees suggestion side panel → accepts whole / edits / discards.
4. Admin clicks "Publish forecast" → `POST /api/admin/villas/:id/forecast/2027` finalises `villa_forecast_assumptions` row for year 2027.
5. KPI layer recomputes Projected Return, 5-Year Total Return for this asset.

**Refresh impact:**
- A4 Performance tab projected-return card updates.
- A5 public listing updates (with whatever cache invalidation already exists).

---

### W11. Deduction policy update (admin only)

1. Admin → B3 `admin/villa-deduction-policy.html` → toggles allowed categories, sets per-category caps, "effective from" date.
2. `PUT /api/admin/villas/:id/deduction-policy` → upsert `villa_deduction_policy` row, audit_log entry.
3. Future developer submissions for periods >= effective date are validated against the new policy. Past published rows unaffected (already validated against then-current policy at their publish time).
4. If a draft exists when policy changes → developer sees a warning banner on next C2 load: "Policy changed on <date>, your draft may need revision".

---

### W12. Investor views current performance (no time-travel)

1. Investor → A4 `property.html` → Performance tab.
2. JS init:
   - `GET /api/villas/:id/performance?display_currency=USD&range=12m` → returns: Annual Yield, Projected Return, 5-Year Total Return, monthly distributable series (12 rows), per-investor net return (if logged in).
   - `GET /api/villas/:id/history?metric=nav&from=12m_ago&to=now` → NAV daily series.
   - `GET /api/villas/:id/history?metric=market&from=12m_ago&to=now` → Market price daily series.
3. Renders three chart panels per PDF §8 (NAV and Market never merged).
4. User toggles USD ↔ IDR → JS re-fetches with `?display_currency=IDR`.

**Caching:** `villa_market_prices_daily` is pre-aggregated, queries are cheap. No Redis layer in P5; add in P7 if needed.

---

### W13. Investor views historical state (time-travel)

1. Investor sets "As of" picker on A4 to `2026-03-15`.
2. JS re-issues every endpoint with `?as_of=2026-03-15`. Backend:
   - For operations: `SELECT ... FROM villa_operations_log WHERE asset_id=$1 AND period_year, period_month <= ($2,$3) AND recorded_at <= '2026-03-15' AND status IN ('published','superseded') ORDER BY recorded_at DESC LIMIT 1 per period`.
   - For NAV: snapshot row from `villa_market_prices_daily` for any date `<= 2026-03-15`.
   - For per-investor return: investor's tokens held AT 2026-03-15 (purchased_at <= as_of and exited_at IS NULL OR > as_of).
3. UI banner: "Viewing data as published at 2026-03-15. Some values may have been corrected since."
4. Investor sees the world as they would have on that date.

---

### W14. Audit / regulator query — "What did this investor see on April 5?"

1. Admin → B4 `admin/villa-history.html?asset_id=...&as_of=2026-04-05`.
2. Page renders SSR shell with breadcrumbs + as_of picker. JS init:
   - `GET /api/admin/villas/:id/operations?as_of=2026-04-05` (all months ever, as published at as_of).
   - `GET /api/admin/audit-logs?entity_type=villa_operations_log&entity_id=...` for the cross-cutting trail.
3. Renders monthly grid + per-month supersession chain.
4. "Export CSV" → `GET /api/admin/villas/:id/operations/export?as_of=2026-04-05&format=csv`.

---

### W15. Investor notified about a material correction

1. W7 (correction) publishes a corrected March row in June.
2. Backend, inside the publish tx, computes delta vs prior published distributable. If `abs(delta) / prior > 5%`:
   - For each investor with `dividend_payouts.payout_type='rental'` row for that period: insert `notifications` row "March 2026 figures were corrected. Updated distributable: $X (was $Y)."
3. Investor sees on next dashboard load.
4. A8 transactions: the original payout row remains; if Q11 says re-payout, a new `dividend_payouts` row may follow.

---

## Part 3 — New Open Question (raised by walkthrough)

| # | Question | Proceed-assumption |
|---|---|---|
| Q11 | **Re-payout on correction.** If a published month is corrected upward (or downward) after distributions were sent, what happens? Options: (a) no automatic re-payout, admins manually create top-up via `dividend_payouts` extras; (b) auto top-up positive delta, never claw back negative; (c) auto reconcile both ways. | **(a) Manual.** Auto re-payouts are operationally risky (wallet flows, tax events). UI surfaces the delta to admin on A1 with "Top up $X" suggestion button → creates a new `dividend_payouts` row with `payout_type='bonus'` and metadata linking to the correction. Negative deltas (over-paid in original) are written off — investor protection. Documented as policy. |

---

## Part 4 — Gap-Fixes Back-Applied to Page Outline

These are concrete edits to `villa-returns-pages-outline.md`. Listed here so they're easy to review.

### A1 (`admin/asset-details.html`) — additions discovered by W1, W4, W7

Add three sections to A1's content list:
1. **"Management & Developer access"** — list of Developer users authorised for this villa, add/remove via `developer_asset_links`. Effective-from date. (W1 gap)
2. **"Submission status alerts"** — surface overdue submissions for any month >10 days past period end with no draft. Mirror of C1 but admin sees ALL villas. (W2 polish)
3. **"Distribute payout" button** on each `published` monthly row in the operations strip. Opens confirmation modal → triggers W4. (W4 gap)
4. **"CapEx since last valuation" tile** in the valuations section — sum of approved CapEx events between last published valuation and now. (W9 gap)
5. **"Correction delta + Top-up CTA"** appears next to any superseded month with payouts already distributed and a delta > 5%. Surfaces Q11 manual top-up path. (W15/Q11 gap)

### A3 (`admin/approvals.html`) — additions discovered by W5, W6

1. "You submitted this row" disabled state on the Approve button when the current admin equals submitter (4-eyes UI hint).
2. **Reject** button on the review form → `PUT .../reject` with reason textarea (W6).

### B1 (`admin/villa-operations-entry.html`) — additions discovered by W5

1. Three explicit modes: `entry` (admin creates from scratch), `review` (read-only review of submitted dev row), `override` (admin creates a supersession of a submitted/published row). Mode set by URL param.
2. In `override` mode: `correction_reason` field is required, supersedes_id pre-filled, original values shown as strikethrough next to the editable field for diff clarity.
3. "Save override" button explicitly creates the chain (calls `.../override` endpoint, NOT a plain PUT on the original row).

### C1 (`developer/dashboard.html`) — additions discovered by W2, W6

1. Status badges include `Rejected` with the admin's rejection reason inline.
2. Overdue detection: `>10` days past period end with no draft → red badge. Configurable via `settings.villa_submission_grace_days`.

### C2 (`developer/operations-submit.html`) — additions discovered by W2, W11

1. Live computed-preview pane is **client-side** (formulas duplicated in JS — kept thin and unit-tested). Server endpoint exists as authority for publish.
2. Banner when deduction policy changed after the draft was started: "Policy updated <date>, please re-check categories."
3. Document upload zone tied to specific period via `villa_period_documents` link. Multiple files per period allowed.

### A4 (`property.html` Performance tab) — additions discovered by W13, W15

1. Banner when `as_of` ≠ NOW: "Viewing as published at <date>. Some values may have been corrected since." Toggle to "View latest" reverts.
2. Annotation dot on chart for months that have been corrected (with hover tooltip showing original vs current).
3. Investor's "position started" reference line on the chart at `investments.purchased_at`, dashed style.

### New page: Developer-asset-access management (lives in A1)

Already covered by the A1 gap-fix — not a separate page. Section inside A1 keeps the asset-centric mental model.

---

## Part 5 — Logic Sanity Checks (cross-cutting)

These are the invariants every page + endpoint must honour. Used as a checklist for code review and the test suite.

1. **No floats anywhere.** All monetary math in `i128` intermediate, stored as `i64` cents.
2. **NAV uses tokenized_pct (PDF §7).** Search every NAV computation site for `tokens_total` denominator without `tokenized_pct_bps` numerator — flag as bug.
3. **NAV and Market price never merged in any UI chart (PDF §8).** Chart helper has a hard guard.
4. **Append-only enforcement on `villa_operations_log` and `villa_valuations`.** DB-level trigger raises on UPDATE/DELETE. App layer never issues these statements. Migration includes `REVOKE UPDATE, DELETE`.
5. **4-eyes: approver ≠ submitter.** CHECK constraint at DB level. UI greys out Approve button if equal.
6. **FX rate frozen at publish.** Correcting an FX rate later never silently revalues historical USD. Test #8 in the plan covers this.
7. **`as_of` parameter end-to-end.** Every read endpoint that supports time-travel accepts `as_of`; every page that surfaces a time-travel control passes it. Audit on each page.
8. **Field-level role enforcement.** `FIELD_OWNERSHIP` whitelist in the Rust handler rejects writes outside caller's role. Test that Dev cannot write `reserve_pct_bps`. Test that Admin can write any.
9. **Developer asset-link enforcement.** `DeveloperUser` extractor checks `developer_asset_links` row exists and is not expired. Test for unauthorised cross-asset access.
10. **CSRF on every mutation.** Server validates `X-CSRF-Token` against session-bound csrf token. Existing middleware — no new code, but every new endpoint must use it.
11. **Notifications on every state transition.** Submit → admin; approve → dev; publish → investors (for material events); correction publish → impacted investors. Tested as a smoke test.
12. **Idempotency of distribution.** UNIQUE `(asset_id, period_year, period_month, user_id, payout_type='rental')`. Re-running cannot double-pay.
13. **Investor visibility filter.** Every public endpoint filters `status='published'` AND `recorded_at <= as_of`. Drafts/submitted/approved are never investor-visible.
14. **Deduction policy validation at submit, not publish.** Catches errors at the latest editable moment.
15. **Audit log written for every state transition.** Submit, approve, publish, reject, override, correct, distribute — all 7 produce an `audit_logs` row.

---

## Part 6 — Updated Sequencing (incorporating gaps)

No phases changed — every gap fits inside its existing phase:

- W1 onboarding gaps → P1 (schema) + P2 (write paths) + P3 (asset-details extensions)
- W4 distribution trigger → P6 (cut-over) — the button reads new tables; until then admins use legacy
- W7 correction → P2 (admin) + P2 (developer) — both ship together
- W15 notifications → P2 (basic) + P5 (investor-facing notifications when investor UI ships)
- Q11 manual top-up CTA → P3 (admin UI) since underlying `dividend_payouts.payout_type='bonus'` is already supported

---

## Part 7 — Single-Page Summary of Refresh Impact

For ops planning: when X happens, which pages change.

| Event | Pages that refresh / change content |
|---|---|
| Developer submits | C1 (dev), A1 (admin strip), A3 (approval queue) |
| Admin approves | A3, A1 |
| Admin publishes | A1, A4, A5, A6, A7, C1 (status) |
| Admin overrides | A1, A3, C1 (status updates), audit_log |
| Admin rejects | A3, C1 (back to draft) |
| Correction publishes | A1, A4 (annotation), A6 (KPI delta), notifications to investors, audit_log |
| Valuation publishes | A1, A4 NAV jump, A5 public listing card |
| CapEx approves | A1 capex tile, C3 dashboard |
| Forecast publishes | A2, A4 projected card, A5 public card |
| Deduction policy updates | B3, C2 (banner on draft load) |
| Distribution triggered | A8 transactions, A6 dashboard payout tile, notifications |
| Top-up triggered (Q11) | A1, A8, notifications |
