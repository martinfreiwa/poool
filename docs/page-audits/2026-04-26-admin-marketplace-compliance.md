# Page Audit: Marketplace Compliance

Date: 2026-04-26
Status: fixed; authenticated E2E verified
Auditor: ChatGPT/Codex
Page URL: `/admin/marketplace/compliance`
Template: `frontend/platform/admin/marketplace/compliance.html`
JavaScript: `frontend/platform/static/js/mp-compliance.js`, `frontend/platform/static/js/mp-toast.js`
CSS: `frontend/platform/static/css/admin-marketplace.css`, `frontend/platform/static/css/admin.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/marketplace.rs`

---

## Summary

The Marketplace Compliance page renders a simple admin export UI for OJK quarterly metrics, AML Travel-Rule trade data, and tax/fiscal CSVs. Initial audit found that the backend implementation was not production-ready for regulatory use: authorization was inconsistent with the `marketplace.compliance` permission model, the Travel-Rule export query was incompatible with the actual `trade_history` schema and silently returned an empty CSV, the tax export was hardcoded placeholder data, and the OJK report could emit zeroed metrics when database reads failed.

Fix status: fixed in local working tree on 2026-04-26; authenticated HTTP/DB E2E recheck passed.

Final status: `fixed_e2e_verified`.

---

## Fix Applied

Date fixed: 2026-04-26

Files changed:

- `backend/src/admin/pages.rs`
- `backend/src/admin/mod.rs`
- `backend/src/admin/marketplace.rs`
- `frontend/platform/static/js/mp-compliance.js`
- `tests/e2e/test_admin_marketplace_compliance.py`

What changed:

- Added a dedicated Marketplace Compliance page gate based on `marketplace.compliance`, so the compliance role can access this report surface without broadening `AdminUser`.
- Updated all three compliance export APIs to require `marketplace.compliance`.
- Made the OJK export validate `YYYY-QN`, calculate quarter start/end dates server-side, filter trade metrics by period, count users as of period end, and propagate DB errors.
- Fixed Travel-Rule export to use `buyer_user_id` and `seller_user_id`, include buyer/seller emails and profile names, validate date ranges, filter by date, and propagate DB errors.
- Replaced the hardcoded tax placeholder CSV with real `tax_reports` rows filtered by fiscal year.
- Added CSV escaping for user-facing string fields.
- Reworked the frontend download helper to use `fetch()`, verify HTTP status and `text/csv`, download a blob only on success, and show real errors for 401/403/500/non-CSV responses.
- Added disabled/loading/`aria-busy` states and local AML date-range validation for export controls.
- Added an authenticated E2E fixture that seeds a compliance user, finance negative case, trade, and tax report, then verifies page/API access and CSV contents.

Verification:

- `node --check frontend/platform/static/js/mp-compliance.js`
- `python3 -m py_compile tests/e2e/test_admin_marketplace_compliance.py`
- `cargo fmt --check`
- `cargo check`
- `BASE_URL=http://localhost:8888 DATABASE_URL=postgres://martin@localhost/poool python3 -m pytest tests/e2e/test_admin_marketplace_compliance.py -q`

Runtime result: targeted authenticated HTTP/DB E2E passed (`1 passed`). Local backend startup still logs pre-existing migration/idempotency errors, but `/health` returned 200 and the compliance E2E passed against the running server.

---

## Tested Scope

- Reviewed `frontend/platform/admin/marketplace/compliance.html`.
- Reviewed `frontend/platform/static/js/mp-compliance.js` and shared `frontend/platform/static/js/mp-toast.js`.
- Reviewed page/API registration in `backend/src/admin/mod.rs`.
- Reviewed admin page permission logic in `backend/src/admin/pages.rs`.
- Reviewed compliance export handlers in `backend/src/admin/marketplace.rs`.
- Checked local PostgreSQL schema for `trade_history`, `wallet_transactions`, `dividend_payouts`, `marketplace_alerts`, `tax_reports`, and related report support.
- Ran JavaScript syntax checks for the page-specific and shared toast scripts.
- Attempted localhost page/API smoke checks; runtime browser testing was blocked because the backend was not running on port 8888.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/marketplace/compliance` | Dedicated `marketplace.compliance` page gate. |
| URL alias | `/admin/marketplace/compliance.html` | Dedicated `marketplace.compliance` page gate. |
| Template | `frontend/platform/admin/marketplace/compliance.html` | Three export cards. |
| JS | `frontend/platform/static/js/mp-compliance.js` | Builds CSV download URLs and clicks a temporary anchor. |
| Shared JS | `frontend/platform/static/js/mp-toast.js` | Shows text-node toasts; modal helper is not used by this page. |
| CSS | `frontend/platform/static/css/admin-marketplace.css` | Shared marketplace admin styling. |
| Backend page route | `GET /admin/marketplace/compliance` | `backend/src/admin/mod.rs`; `backend/src/admin/pages.rs`. |
| Backend API route | `GET /api/admin/marketplace/compliance/ojk-report` | CSV response. |
| Backend API route | `GET /api/admin/marketplace/compliance/travel-rule` | CSV response; fixed to use real trade schema and date filters. |
| Backend API route | `GET /api/admin/marketplace/compliance/tax-export` | CSV response; fixed to use `tax_reports`. |
| Database table | `trade_history` | Source for OJK volume and intended AML export. |
| Database table | `users` | Source for registered user count and intended trade identities. |
| Database table | `tax_reports` | Existing tax-report table, not used by this page's tax export. |
| Database table | `dividend_payouts` | Existing dividend payout data, not used by this page's tax export. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `a[href="/admin/"]`, `a[href="/admin/marketplace/"]` | Navigate to admin dashboard and marketplace admin overview. | Link navigation | Page routes exist elsewhere; not runtime tested. | Unverified. |
| OJK reporting period | `#ojk-quarter` | Choose quarter for the OJK CSV. | Yes; value is sent as `quarter`. | Partially; backend echoes quarter but does not filter by quarter. | Static mismatch found. |
| Download CSV | `#btn-export-ojk` | Download OJK quarterly CSV. | Yes; fetch/blob download with status handling. | Yes; quarter-scoped and permission-gated. | Authenticated E2E passed. |
| AML start date | `#aml-start` | Limit Travel-Rule export start date. | Yes; sends `from_date` and validates range. | Yes; backend parses and filters by `from_date`. | Authenticated E2E passed. |
| AML end date | `#aml-end` | Limit Travel-Rule export end date. | Yes; sends `to_date` and validates range. | Yes; backend parses and filters by `to_date`. | Authenticated E2E passed. |
| Export Complete Dataset | `#btn-export-aml` | Download AML Travel-Rule CSV. | Yes; fetch/blob download with status handling. | Yes; real buyer/seller schema and date filtering. | Authenticated E2E passed. |
| Fiscal year | `#tax-year` | Choose fiscal year for tax CSV. | Yes; value is sent as `year`. | Yes; filters `tax_reports.fiscal_year`. | Authenticated E2E passed. |
| Export Fiscal Data | `#btn-export-tax` | Download tax/fiscal CSV. | Yes; fetch/blob download with status handling. | Yes; real `tax_reports` data. | Authenticated E2E passed. |
| Toast/status feedback | `mpToast(...)`, `.mp-export-status` | Show export success/failure. | Yes; response-aware. | Yes; uses status and content type. | Static and E2E verification passed. |

---

## Frontend Findings

### P2 - Export buttons show success without verifying the HTTP response

Location:

- Template: `frontend/platform/admin/marketplace/compliance.html:70`, `frontend/platform/admin/marketplace/compliance.html:99`, `frontend/platform/admin/marketplace/compliance.html:123`
- JS: `frontend/platform/static/js/mp-compliance.js:7`

Problem:

`triggerDownload()` creates an anchor, clicks it, and immediately shows a success toast. Browser downloads triggered this way do not expose HTTP status, content type, auth redirects, or backend error bodies to the page, so users can see "Exporting..." even when the server returns a 401/403/500 or an HTML login page.

Expected:

Compliance exports should use `fetch()` or another backend-aware flow that validates response status and `text/csv`, disables the button during export, shows a real error state on failure, and only downloads a blob after a successful response.

Evidence:

`mp-compliance.js:7-16` has no response handling, loading state, disabled state, or retry/error path.

Recommended fix:

Replace anchor-click downloads with an async `downloadCsv(url, filename, button)` helper that checks `response.ok`, rejects non-CSV content, creates a blob URL on success, restores focus/state, and renders a visible error toast on failure.

### P3 - Date range and export controls lack validation and accessible progress states

Location:

- Template: `frontend/platform/admin/marketplace/compliance.html:88`
- JS: `frontend/platform/static/js/mp-compliance.js:31`

Problem:

The AML date fields allow an end date before the start date, empty open-ended exports, and repeated clicks. Export buttons do not expose `aria-busy`, disabled state, or persistent status text during long-running downloads.

Expected:

The frontend should block invalid date ranges, prevent double-clicks while a request is active, and expose progress/failure state to assistive technology.

Evidence:

`mp-compliance.js:33-42` forwards date values without comparison or control state changes.

Recommended fix:

Validate date ranges client-side for usability, while keeping server-side validation authoritative. Add disabled/loading/`aria-busy` states and a nearby `role="status"` region for export result messages.

---

## Backend Findings

### P1 - Compliance exports are not protected by the intended `marketplace.compliance` permission

Location:

- Page gate: `backend/src/admin/pages.rs:178`
- API handlers: `backend/src/admin/marketplace.rs:1888`, `backend/src/admin/marketplace.rs:1928`, `backend/src/admin/marketplace.rs:1986`
- RBAC seed: `database/056_marketplace_rbac_permissions.sql:25`

Problem:

The RBAC seed defines `marketplace.compliance` for the compliance role, and the sidebar hides the compliance page behind that permission, but the backend does not enforce the same contract. The page-level marketplace gate allows any user with `marketplace.view`, `marketplace.manage`, or `marketplace.compliance`, while the export APIs only require `AdminUser`. `AdminUser` itself only accepts `admin` and `super_admin`, so a dedicated `compliance` role granted `marketplace.compliance` is blocked, while broad admins can export sensitive compliance data without an explicit compliance check.

Expected:

The page and all three export endpoints should require `marketplace.compliance` or an explicit documented superset such as `marketplace.manage` plus audit/compliance approval. The extractor/permission system should make the seeded compliance role usable if that role is intended to operate these exports.

Evidence:

`admin-permission-guard.js` maps `nav-mp-compliance` to `marketplace.compliance`, but none of the three export handlers call `admin.require_permission(&state.db, "marketplace.compliance")`.

Recommended fix:

Add server-side `marketplace.compliance` checks to the page and APIs, decide whether `marketplace.manage` should also be allowed, and align `AdminUser` or a new extractor with non-superuser admin roles such as `compliance`, `finance`, and `support`.

### P1 - Travel-Rule export query is incompatible with the actual trade schema and silently returns an empty report

Location:

- Backend: `backend/src/admin/marketplace.rs:1935`
- Schema: `database/051_trade_history.sql`

Problem:

The Travel-Rule handler queries `t.buyer_id` and `t.seller_id`, but the actual `trade_history` table has `buyer_user_id` and `seller_user_id`. The handler then uses `.unwrap_or_default()`, so the query failure is hidden from the client and produces a valid-looking CSV header with no trade rows.

Expected:

The export should query the real buyer/seller columns, include the required Travel-Rule identity fields, and return a 500/operator-visible error if the report cannot be generated.

Evidence:

Running the handler SQL against local PostgreSQL failed with `ERROR: column t.buyer_id does not exist`. The schema check shows `buyer_user_id` and `seller_user_id` columns.

Recommended fix:

Change the joins to `t.buyer_user_id` and `t.seller_user_id`, use `DateTime<Utc>`-compatible decoding for `executed_at`, remove `.unwrap_or_default()`, and add a regression test that seeds a trade and verifies both buyer and seller appear in the CSV.

### P1 - Tax export returns hardcoded placeholder data and ignores the selected fiscal year

Location:

- Backend: `backend/src/admin/marketplace.rs:1986`
- Existing tax schema: `database/011_tax_reporting.sql`

Problem:

`/api/admin/marketplace/compliance/tax-export` returns `user_placeholder@poool.app,2025,0,0` for every request. It ignores the `year` query parameter, existing `tax_reports`, dividends, realized gains, and withholding tax fields.

Expected:

The export should read real fiscal-year data from `tax_reports` and/or the canonical financial ledger tables, filter by the requested year, and fail visibly if the data cannot be generated.

Evidence:

`api_admin_marketplace_compliance_tax()` does not read `query.year` or `state.db`; it returns a fixed string.

Recommended fix:

Wire this endpoint to the existing `tax_reports` table or consolidate it with the existing admin report path for tax P&L/withholding. Include fiscal year, user, dividends, realized gains, withholding, status, and generation metadata.

### P1 - OJK report is not actually quarter-scoped and masks database failures as zero metrics

Location:

- Backend: `backend/src/admin/marketplace.rs:1895`

Problem:

The OJK report accepts `quarter`, but the SQL sums all `trade_history` rows and counts all users without any reporting-period filter. Both queries also use `.unwrap_or(0)`, which turns database failures into a regulatory report showing zero values.

Expected:

The endpoint should validate the quarter format, derive exact start/end timestamps, filter trade volume and relevant incident/user metrics by period, and return an error if any required data source cannot be read.

Evidence:

The SQL in `api_admin_marketplace_compliance_ojk()` has no `WHERE executed_at >= ... AND executed_at < ...` clause and no error propagation.

Recommended fix:

Parse `YYYY-QN`, calculate the date range server-side, filter all period-sensitive metrics, replace `.unwrap_or(0)` with `?`/`ApiError`, and add tests for valid quarter, invalid quarter, empty period, and DB error behavior.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JavaScript syntax | `node --check frontend/platform/static/js/mp-compliance.js && node --check frontend/platform/static/js/mp-toast.js` | Both scripts parse. | Passed with no output. | Pass |
| Local page smoke | `curl -I --max-time 2 http://localhost:8888/admin/marketplace/compliance` | Auth redirect or page response. | Connection refused; backend not running. | Blocked |
| Local API smoke | `curl -I --max-time 2 http://localhost:8888/api/admin/marketplace/compliance/ojk-report` | Auth/API response. | Connection refused; backend not running. | Blocked |
| Travel-Rule SQL compatibility | Ran the handler's SQL shape against local PostgreSQL. | Query references existing schema columns. | Failed: `column t.buyer_id does not exist`. | Fail |
| Schema support check | Queried `information_schema` for `trade_history`, `wallet_transactions`, `dividend_payouts`, `marketplace_alerts`, `tax_reports` support. | Required reporting tables/columns are identifiable. | Tables exist; `trade_history` uses `buyer_user_id`/`seller_user_id`. | Pass with backend mismatch |
| Authenticated compliance export E2E | Seeded compliance and finance users, trade, and tax report; requested page and CSV endpoints. | Compliance role succeeds; finance role denied; CSVs contain real seeded data. | Passed. | Pass |

---

## Security Findings

- P1: Export APIs do not enforce `marketplace.compliance` and rely only on broad `AdminUser`.
- P1: The page permission model and seeded compliance role are inconsistent; a compliance-role user appears intended but cannot use `AdminUser`-guarded APIs.
- P2: CSV export URLs can be triggered repeatedly with open date ranges; rate limiting/abuse controls were not evident in the reviewed handlers.
- No client-side XSS issue was found in `mp-compliance.js`; it does not render backend data into HTML.
- No CSRF issue was found for the three visible exports because they are read-only `GET` downloads, though sensitive export access must be authorization-gated server-side.

---

## Database Findings

- `trade_history` exists, but the Travel-Rule endpoint references non-existent `buyer_id` and `seller_id` columns instead of `buyer_user_id` and `seller_user_id`.
- `tax_reports` exists with fiscal year, dividends, capital gains, and withholding fields, but the Marketplace Compliance tax export does not use it.
- `dividend_payouts` and `wallet_transactions` exist for financial reporting support, but the current page's tax export is not wired to real ledger data.
- OJK export uses integer cents for volume, which matches the money invariant, but lacks period filtering and error propagation.

---

## Missing Tests

- Added `tests/e2e/test_admin_marketplace_compliance.py` covering OJK quarter CSV, Travel-Rule rows/date filtering, tax export data, compliance-role access, and finance-role denial.
- Remaining optional coverage: browser-level visual/mobile smoke for the export status messages and blob download UX.

---

## Recommended Fix Order

1. Enforce and align `marketplace.compliance` authorization for the page and all three export APIs.
2. Fix the Travel-Rule SQL contract, date filtering, and DB error propagation.
3. Replace the placeholder tax export with real fiscal-year data from `tax_reports` or the canonical ledger source.
4. Make the OJK report quarter-scoped and fail closed on DB/report-generation errors.
5. Replace anchor-click downloads with status-aware fetch/blob downloads and add accessible loading/error states.

---

## Final Status

`fixed_e2e_verified`

Reason: The audit findings were fixed and the targeted authenticated HTTP/DB E2E passed. Optional browser visual/mobile recheck remains useful for polish, but the documented functional/security/export issues are covered.
