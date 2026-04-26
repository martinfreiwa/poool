# Page Audit: Admin Reports

Date: 2026-04-26
Status: completed
Auditor: ChatGPT/Codex
Page URL: `/admin/reports`
Template: `frontend/platform/admin/reports.html`
JavaScript: `frontend/platform/static/js/admin-reports.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/poool-dropdown.css`
Backend Routes: `GET /admin/reports`, `GET /admin/reports.html`, `GET /api/admin/reports/:report_type`

---

## Summary

`/admin/reports` is now fixed and verified. The page and report API enforce `reports.generate` plus category permissions for sensitive report types, validate date ranges before SQL, propagate query failures, audit successful exports, avoid fake PDF behavior, and show durable frontend status feedback. Targeted authenticated API and Playwright coverage passed.

---

## Tested Scope

- Reviewed `frontend/platform/admin/reports.html`.
- Reviewed `frontend/platform/static/js/admin-reports.js`.
- Reviewed admin route registration in `backend/src/admin/mod.rs`.
- Reviewed report API implementation in `backend/src/main.rs`.
- Reviewed tax report helper endpoints in `backend/src/admin/reports.rs`.
- Reviewed relevant schema docs and migrations for `wallet_transactions`, `users`, `kyc_records`, `assets`, `investments`, `orders`, `rewards_balances`, `referral_tracking`, `wallets`, `invoices`, `support_tickets`, `audit_logs`, and `tax_reports`.
- Added and ran targeted `/admin/reports` API and Playwright E2E coverage.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/reports` | Admin page route via generic admin page handler. |
| Alias | `/admin/reports.html` | Same page. |
| Template | `frontend/platform/admin/reports.html` | Page shell, date range, empty report grids, preview section. |
| JS | `frontend/platform/static/js/admin-reports.js` | Defines 16 report cards, presets, preview, checked CSV/JSON download helpers, and inline status states. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Sidebar maps `nav-reports` to `reports.generate`. |
| Backend page route | `GET /admin/reports` | Registered in `backend/src/admin/mod.rs`; admin page auth handled by generic admin page handler. |
| Backend API route | `GET /api/admin/reports/:report_type` | Registered in `backend/src/main.rs`; implemented in `api_admin_reports` with `AdminUser` and report/category permission checks. |
| Migration | `database/088_admin_report_permissions.sql` | Grants `reports.generate` to non-`all` report-capable roles. |
| Related API | `GET /api/admin/tax-reports`, `POST /api/admin/tax-reports/generate` | Separate tax report admin endpoints, not used by this page. |
| Database tables | `wallet_transactions`, `users`, `kyc_records`, `assets`, `investments`, `orders`, `rewards_balances`, `referral_tracking`, `wallets`, `invoices`, `support_tickets`, `audit_logs`, `tax_reports` | Used by report API branches. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `frontend/platform/admin/reports.html:32` | Navigate back to `/admin/`. | Link only | Yes | Not runtime-tested. |
| Date range label | `frontend/platform/admin/reports.html` | Label global report date range. | Yes | N/A | Verified in Playwright. |
| From date input | `#range-from` | Sets validated `from` query parameter. | Yes | Yes, parsed as `YYYY-MM-DD` before SQL. | Verified in Playwright/API test. |
| To date input | `#range-to` | Sets validated `to` query parameter. | Yes | Yes, parsed as `YYYY-MM-DD` before SQL. | Verified in Playwright/API test. |
| `30d` preset | inline `onclick="setPreset('30d')"` | Set last 30 days. | Yes | N/A | Static verified. |
| `90d` preset | inline `onclick="setPreset('90d')"` | Set last 90 days. | Yes | N/A | Static verified. |
| `YTD` preset | inline `onclick="setPreset('ytd')"` | Set year-to-date. | Yes | N/A | Static verified. |
| Financial grid | `#grid-financial` | Render 4 financial report cards. | Yes | Yes, with `treasury.read` where sensitive. | Verified by API/E2E. |
| Compliance grid | `#grid-compliance` | Render 4 compliance report cards. | Yes | Yes, with `kyc.read`/`audit.read` where sensitive. | Verified by static/API coverage. |
| Assets grid | `#grid-assets` | Render 3 asset/investment report cards. | Yes | Yes. | Verified by static/API coverage. |
| Operational grid | `#grid-operational` | Render 3 operational report cards. | Yes | Yes, support report requires `support.read`. | Verified by static/API coverage. |
| Tax grid | `#grid-tax` | Render Annual Investor P&L and Withholding Tax reports. | Yes | Yes, exported as CSV until PDF generation exists. | Verified in Playwright. |
| Download buttons | `button[id^="dl-btn-"]` | Download CSV/JSON only after `resp.ok` and valid `{ rows }`. | Yes | Yes | Verified in Playwright. |
| Preview buttons | generated secondary icon buttons | Preview first 5 rows with accessible labels. | Yes | Yes | Verified in Playwright. |
| Preview section | `#preview-section` | Show table after successful preview. | Yes | Yes | Verified in Playwright. |
| Preview close | inline close button | Hide preview section. | Yes | N/A | Static verified. |
| Preview table | `#preview-table`, `#preview-thead`, `#preview-tbody` | Render escaped result rows. | Yes | Yes | Static XSS handling looks safe for backend values. |
| Preview footer | `#preview-footer` | Describe visible rows. | Yes | N/A | Static verified. |

---

## Frontend Findings

### P1 - Download flow treats failed API responses as successful exports - Fixed

Location:

- JS: `frontend/platform/static/js/admin-reports.js:247`

Problem:

`downloadReport()` fetches the endpoint but does not reject non-2xx responses unless a fallback endpoint exists. For CSV and JSON reports, it immediately parses the response and downloads it. A 403, 400, 500, or backend error JSON can therefore become a downloaded "successful" report.

Expected:

Downloads should require `resp.ok`, verify the expected response shape, surface API errors visibly, and avoid creating files for failed exports.

Evidence:

`previewReport()` checks `resp.ok`, but `downloadReport()` does not. The unauthenticated/error path for `/api/admin/reports/:report_type` returns JSON with `error`, which the current download code can process.

Recommended fix:

Add a shared `fetchReport()` helper that validates `resp.ok`, content type, and `{ rows: [] }` shape before download or preview. Show an inline error state on the affected card.

Fix:

Implemented `fetchReport()` in `frontend/platform/static/js/admin-reports.js`; downloads now require `resp.ok` and a valid `rows` array, and failed responses update per-card and page-level status regions without creating a file.

### P1 - Annual Investor P&L is labeled PDF but backed by JSON - Fixed

Location:

- JS: `frontend/platform/static/js/admin-reports.js:154`
- JS: `frontend/platform/static/js/admin-reports.js:255`
- Backend: `backend/src/main.rs:1453`

Problem:

The Annual Investor P&L card advertises `format: "PDF"` and opens `/api/admin/reports/tax-pl` in a new tab. The backend branch returns a JSON report body, not a PDF file, signed URL, or redirect. The user-facing toast tells admins to check browser downloads even though no PDF is generated.

Expected:

Either generate/serve a real PDF with a PDF content type and safe authorization, or relabel the card as CSV/JSON and export the returned rows consistently.

Evidence:

The `tax-pl` branch returns `Json({"report_type": "tax-reporting", ...})`; there is no PDF response path in `api_admin_reports`.

Recommended fix:

Use existing `tax_reports.pdf_url` only through a controlled signed/download endpoint, or convert this card to CSV until PDF generation exists.

Fix:

Relabeled Annual Investor P&L as CSV and updated the description so the UI no longer promises PDF generation.

### P2 - Preview-only errors and empty states depend on toast availability - Fixed

Location:

- JS: `frontend/platform/static/js/admin-reports.js:294`
- JS: `frontend/platform/static/js/admin-reports.js:313`
- JS: `frontend/platform/static/js/admin-reports.js:360`

Problem:

Errors and empty states are only displayed through `window.showPooolToast`, and `showToast()` silently does nothing when that global is unavailable. The page has no persistent inline error region, no per-card failure message, and no `aria-live` status area for screen readers.

Expected:

The preview panel or report card should show durable loading, empty, and error states. Toasts can be secondary feedback, but not the only feedback channel.

Evidence:

`showToast()` has no fallback, and the HTML does not include a page-level status region.

Recommended fix:

Add an inline `#reports-status` live region and per-card state text. Keep disabled/loading state until the result is known, and restore the button after `finally`.

Fix:

Added `#reports-status` and per-card `report-status-*` regions. Download buttons are restored in `finally`, and loading/success/error/empty messages remain visible.

### P2 - Date inputs and icon-only preview controls need stronger accessible names - Fixed

Location:

- Template: `frontend/platform/admin/reports.html:60`
- Template: `frontend/platform/admin/reports.html:65`
- Template: `frontend/platform/admin/reports.html:67`
- JS: `frontend/platform/static/js/admin-reports.js:228`

Problem:

The date range label is not associated with either date input, the inputs use `title` instead of explicit labels, and generated preview buttons are icon-only with `title` but no durable text or `aria-label`.

Expected:

Each form control should have a programmatic label. Icon-only buttons should have `aria-label` values such as `Preview Monthly Financial Summary`.

Evidence:

No `for`, `aria-label`, or `aria-labelledby` is present for `#range-from` / `#range-to`, and the preview button text is only an SVG.

Recommended fix:

Add visually hidden labels or `aria-label` attributes to both date inputs and preview buttons.

Fix:

Added programmatic labels for `range-from` and `range-to`; generated preview buttons now include report-specific `aria-label` values.

### P3 - Page loads HTMX from a CDN even though no HTMX behavior is used - Fixed

Location:

- Template: `frontend/platform/admin/reports.html:12`

Problem:

The page loads `https://unpkg.com/htmx.org@1.9.10`, but the template has no `hx-*` attributes. This adds a third-party runtime dependency to an admin-only page for no visible function.

Expected:

Remove the CDN script or serve the required vendor script locally only when HTMX is actually used.

Evidence:

Static template review found no HTMX attributes on this page.

Recommended fix:

Delete the script tag for this page or replace it with the existing local/self-hosted dependency pattern if future HTMX functionality is added.

Fix:

Removed the unused HTMX CDN script from `frontend/platform/admin/reports.html`.

---

## Backend Findings

### P1 - Report API only checks broad admin status, not report/export permissions - Fixed

Location:

- Backend: `backend/src/main.rs:1196`
- Frontend permission map: `frontend/platform/static/js/admin-permission-guard.js:70`

Problem:

The sidebar exposes Reports behind `reports.generate`, but `GET /api/admin/reports/:report_type` only calls `auth::middleware::is_admin`. Any user treated as admin can export KYC, AML, wallet, order, audit, and tax data even if they do not have `reports.generate`, `audit.read`, `kyc.read`, `treasury.read`, or similar least-privilege permissions.

Expected:

The page route and report API should enforce server-side report permissions. Sensitive report categories should require category-specific permissions where appropriate.

Evidence:

The API does not use `AdminUser`, admin permission extractors, or a permission check; it only checks `is_admin`.

Recommended fix:

Move this API into the admin module and require `AdminUser` plus `require_admin_permission("reports.generate")`. Consider stricter permissions for KYC/AML/audit/wallet exports.

Fix:

`GET /api/admin/reports/:report_type` now uses `AdminUser`, requires `reports.generate`, and applies category permissions for financial/tax/order/wallet reports (`treasury.read`), KYC/AML (`kyc.read`), audit (`audit.read`), and support (`support.read`). The HTML page route also checks `reports.generate`.

### P1 - Backend silently converts report query failures into empty successful reports - Fixed

Location:

- Backend: `backend/src/main.rs:1228`
- Backend: `backend/src/main.rs:1263`
- Backend: `backend/src/main.rs:1296`
- Backend: `backend/src/main.rs:1378`
- Backend: `backend/src/main.rs:1804`

Problem:

Every report branch uses `.fetch_all(...).await.unwrap_or_default()`. SQL errors, bad date casts, missing columns, DB outages, and permission failures from the database layer are returned as `200 OK` with `rows: []`.

Expected:

Query failures should return a 4xx validation error for invalid dates or a 5xx operational error for database failures. The frontend should show the failure and avoid downloading misleading empty reports.

Evidence:

The API follows this pattern across all branches, including financial, KYC, asset, investment, order, reward, support, audit, and wallet transaction exports.

Recommended fix:

Parse date inputs before SQL, return `400` for invalid ranges, and propagate SQLx errors through `AppError`/admin `ApiError` with safe user messages and operator logs.

Fix:

Added date parsing/range validation before SQL and removed report-query `.unwrap_or_default()` calls. SQLx failures now become safe `500` JSON errors through `ApiError::Database`.

### P2 - Export actions are not audit logged - Fixed

Location:

- Backend: `backend/src/main.rs:1190`

Problem:

This page exports highly sensitive financial, KYC, AML, audit, order, wallet, tax, and support datasets. The report API does not record which admin exported which report, date range, IP, and row count.

Expected:

Sensitive exports should create immutable audit log events such as `report.exported` with report type, date range, actor, and row count metadata.

Evidence:

No `audit_logs` insert or shared audit helper call appears in `api_admin_reports`.

Recommended fix:

After successful report generation, insert an audit event for each download or server-side export request. If previews should be logged separately, use a lower-severity action such as `report.previewed`.

Fix:

Successful export requests now insert `report.exported` audit-log rows with actor, report type, date range, row count, user agent, and safe metadata. Preview requests use `mode=preview` and are not logged as exports.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static route mapping | Reviewed `backend/src/admin/mod.rs` and `backend/src/main.rs`. | Page and API routes exist. | `/admin/reports`, `/admin/reports.html`, and `/api/admin/reports/:report_type` are registered. | Pass |
| JS syntax | Ran `node --check frontend/platform/static/js/admin-reports.js`. | No syntax errors. | Command passed. | Pass |
| Rust compile | Ran `cargo check` in `backend/`. | Backend compiles. | Command passed. | Pass |
| Local backend health | Started `cargo run`, then checked `/health`. | Backend reachable. | Health returned 200 with DB ok and Redis not configured. | Pass |
| API permissions/date/audit | `python3 -m pytest tests/e2e/test_admin_reports_export.py -q`. | Permission denial, invalid dates, success, and audit log verified. | Targeted test passed. | Pass |
| Browser report workflow | `python3 -m pytest tests/e2e/test_admin_reports_export.py -q`. | Page load, labels, preview, CSV download, CSV P&L label, and API failure UI verified. | Targeted Playwright test passed. | Pass |

---

## Security Findings

- Fixed: Sensitive report exports now require `reports.generate` plus category permissions where appropriate.
- Fixed: Successful export requests now write `report.exported` audit-log rows.
- Fixed: Unused HTMX CDN dependency was removed.
- No direct user-data XSS was found in the preview table path; preview values remain escaped before table rendering.

---

## Database Findings

- Required source tables are present in schema/migrations for the reviewed report branches.
- Money fields are returned as integer cents from database-backed `BIGINT` or integer columns.
- Report export writes are limited to `audit_logs` entries for successful exports.
- Report query errors are now propagated instead of being swallowed as empty successful reports.
- `database/088_admin_report_permissions.sql` grants `reports.generate` to report-capable non-`all` roles.

---

## Missing Tests

- Added targeted API tests for report permission denial, category permission denial, invalid dates, successful financial export, and audit-log creation.
- Added targeted Playwright coverage for page load, labels, preview, CSV download, error UI, and CSV-only Annual Investor P&L behavior.
- Remaining useful follow-up: broader per-report fixture coverage for every report type and mobile viewport smoke.

---

## Recommended Fix Order

1. Broaden report-type fixtures to exercise every report branch with realistic data.
2. Add a mobile viewport smoke for the report grid and preview table.
3. Implement real PDF generation later if Annual Investor P&L needs tax-ready PDF delivery.

---

## Final Status

`completed`

Reason: The documented audit findings were fixed and targeted API/browser verification passed.
