# Page Audit: Admin Index

Date: 2026-04-25
Status: fixed_e2e_verified
Auditor: ChatGPT/Codex
Page URL: `/admin/`
Tracker ID: `admin.index`
Template: `frontend/platform/admin/index.html`
JavaScript: `frontend/platform/static/js/admin-dashboard.js`, `frontend/platform/static/js/admin-global-search.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`, `frontend/platform/static/css/poool-dropdown.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/dashboard.rs`, `backend/src/admin/system.rs`

---

## Summary

`/admin/` is registered to the admin dashboard and is protected by the `AdminUser` extractor in backend routing. The seven documented production-readiness findings have been fixed in code: dashboard data is rendered through DOM/text APIs, required stats reads now propagate DB errors, system health returns the fields the widget reads, dashboard failures show visible retry states, global search uses a minimal backend endpoint, the deposits label reflects the selected range, and unused third-party CDN scripts were removed.

Severity counts: P0 critical 0, P1 high 2, P2 medium 4, P3 low 1.

---

## Tested Scope

- Reviewed `frontend/platform/admin/index.html`.
- Reviewed `frontend/platform/static/js/admin-dashboard.js` and `frontend/platform/static/js/admin-global-search.js`.
- Reviewed `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/dashboard.rs`, `backend/src/admin/system.rs`, and `backend/src/admin/extractors.rs`.
- Checked database dependencies for `users`, `investments`, `wallet_transactions`, `kyc_records`, `assets`, `deposit_requests`, `support_tickets`, `rewards_balances`, `audit_logs`, and `orders`.
- Checked existing admin dashboard/security test references.
- Ran JS syntax checks and Rust compile checks.
- Ran unauthenticated local curl smoke against `localhost:8893`; admin page returned `401`.
- Ran `BASE_URL=http://localhost:8893 python3 tests/admin/test_admin_dashboard.py`; the existing admin dashboard HTTP/HTML suite passed with warnings.
- Ran `BASE_URL=http://localhost:8893 python3 -m pytest tests/e2e -m admin --maxfail=5 -q --base-url http://localhost:8893`; the existing Playwright admin subset passed.
- Ran authenticated `GET /api/admin/search?q=test` smoke against `localhost:8893`; the endpoint returned `200` with a capped `results` payload.
- Checked `/static/js/admin-dashboard.js` and `/static/js/admin-global-search.js` on localhost after the E2E run; both returned 200 and passed `node --check`.
- Added and ran targeted authenticated Playwright coverage for `/admin/` stats rendering, safe activity text rendering, system health contract fields, and server-side global search.
- Optional mobile layout smoke remains outside the fixed issue set.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/` | Registered to `page_admin_dashboard`; requires `AdminUser`. |
| URL alias | `/admin/index.html` | Same handler. |
| Template | `frontend/platform/admin/index.html` | KPI cards, global search, health dots, activity, quick actions, recent orders, pending deposits. |
| JS | `frontend/platform/static/js/admin-dashboard.js` | Loads stats and system health; renders dashboard cards/tables. |
| JS | `frontend/platform/static/js/admin-global-search.js` | Searches users/assets/orders/deposits from admin APIs. |
| Backend API | `GET /api/admin/stats/overview?range=...` | Dashboard KPI/activity/order/deposit data. |
| Backend API | `GET /api/admin/system` | System health data with dashboard-compatible service fields. |
| Database tables | `users`, `investments`, `wallet_transactions`, `kyc_records`, `assets`, `deposit_requests`, `support_tickets`, `rewards_balances`, `audit_logs`, `orders` | Read-only dashboard dependencies. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Audit Result |
|--------|---------------------|-------------------|-----------------|----------------|--------------|
| Breadcrumb Admin link | `a[href="/admin/"]` | Navigate to dashboard. | Native link | `GET /admin/` | Route exists by static review. |
| Global search | `#admin-global-search` | Search admin entities. | Yes | `GET /api/admin/search?q=...` | Fixed with minimal server-side search. |
| Date range selector | `#dashboard-range` | Reload stats by range. | Yes | `GET /api/admin/stats/overview?range=` | Wired by static review; runtime not verified. |
| Health dots | `#health-db`, `#health-psp`, `#health-kyc`, `#health-email` | Show service status. | Yes | `GET /api/admin/system` | Fixed; endpoint returns dashboard-compatible fields. |
| Notification button/badge | `.admin-notification-btn`, `#notification-count` | Navigate to notifications and show count. | Navigation only | `/admin/notifications.html` | Badge has no page-local loader. |
| KPI cards | `#kpi-*` | Render operational metrics. | Yes | Stats API | Fixed; required query failures propagate and visible errors render. |
| Activity feed | `#activity-feed` | Render audit events. | Yes, via DOM/text APIs | Stats API `recent_activity` | Fixed; malicious values render as text. |
| Recent orders table | `#recent-orders-table` | Render recent orders. | Yes, via DOM/text APIs | Stats API `recent_orders` | Fixed. |
| Pending deposits table | `#pending-deposits-table` | Render deposits with Review link. | Yes, via DOM/text APIs | Stats API `pending_deposits_list` | Fixed. |
| Quick actions | `.quick-action` links | Navigate to admin workflow pages. | Native links | Generic admin routes | Routes exist by static review. |

---

## Frontend Findings

### PAGE-ISSUE-0153 - High - Admin dashboard renders API data with `innerHTML`

Location:

- `frontend/platform/static/js/admin-dashboard.js`

Problem:

Recent activity, recent orders, and pending deposits interpolate API/database fields into HTML strings before assigning `innerHTML`. Fields include audit actions/entity IDs, order numbers, user emails, statuses, deposit providers, and deposit user emails.

Expected:

Render dynamic fields with DOM APIs and `textContent`, use class allowlists for statuses, and encode URL/query values.

Recommended fix:

Replace dynamic table/feed render blocks with DOM-building helpers. Keep `innerHTML` only for static developer-controlled empty/loading markup.

### PAGE-ISSUE-0155 - Medium - Dashboard load failures are invisible

Location:

- `frontend/platform/static/js/admin-dashboard.js`

Problem:

`loadDashboardStats()` logs non-OK responses and exceptions only to the console. KPI and table areas can remain as placeholders or loading rows with no visible retryable error feedback.

Expected:

Show explicit dashboard or per-section error states and announce the failure for assistive technology.

Recommended fix:

Add a `renderDashboardError()` path for KPI cards, activity, orders, and deposits, with a retry affordance.

### PAGE-ISSUE-0156 - Medium - System health UI and API response do not match

Location:

- `frontend/platform/static/js/admin-dashboard.js`
- `backend/src/admin/system.rs`

Problem:

The dashboard expects `psp_connected`, `kyc_provider`, and `email_configured`, but `/api/admin/system` returns `api_healthy`, `db_healthy`, `database`, `costs`, `environment`, and `recent_errors`.

Expected:

Use one documented response contract across endpoint, dashboard JS, and tests.

Recommended fix:

Add explicit service health/config fields to `/api/admin/system` or point the dashboard to a dedicated health endpoint.

### PAGE-ISSUE-0157 - Medium - Global search over-fetches admin datasets client-side

Location:

- `frontend/platform/static/js/admin-global-search.js`

Problem:

Typing a short query triggers full fetches from `/api/admin/users`, `/api/admin/assets`, `/api/admin/orders`, and `/api/admin/deposits`, then filters the results in the browser. This sends broader admin-only datasets than the dropdown needs.

Expected:

Use a dedicated server-side search endpoint with query, limit, permission scoping, and a minimal response shape.

Recommended fix:

Create `GET /api/admin/search?q=...&limit=...` and update the dropdown to consume only minimal result records.

### PAGE-ISSUE-0159 - Low - Admin dashboard loads unused CDN scripts

Location:

- `frontend/platform/admin/index.html`

Problem:

The dashboard loads HTMX from unpkg and Alpine from jsDelivr, but this template has no `hx-*` or `x-*` attributes. This increases admin-surface supply-chain exposure and can fail under stricter CSP/offline conditions.

Expected:

Remove unused scripts or self-host required dependencies only on pages that need them.

Recommended fix:

Remove the CDN script tags from this page unless this dashboard starts using those libraries.

---

## Backend Findings

### PAGE-ISSUE-0154 - High - Admin stats API masks DB failures as zero data

Location:

- `backend/src/admin/dashboard.rs`

Problem:

`GET /api/admin/stats/overview` uses `unwrap_or(0)`, `.ok().flatten().unwrap_or(0)`, and `unwrap_or_default()` across required dashboard queries. DB/schema failures can return `200 OK` with plausible zero/empty operational metrics.

Expected:

Required dashboard queries should propagate `ApiError::Database`; optional widgets should return explicit unavailable metadata.

Recommended fix:

Return `Result<Json<_>, ApiError>`, use `?` for required queries, and render visible unavailable/error states in the frontend.

### PAGE-ISSUE-0158 - Medium - System API hardcodes healthy status after fallback data

Location:

- `backend/src/admin/system.rs`

Problem:

`api_admin_system` falls back to `"unknown"`, `0`, or empty rows after failed system queries, but still returns `"api_healthy": true` and `"db_healthy": true`.

Expected:

Health output should distinguish successful probes from degraded fallback data.

Recommended fix:

Track query result status and return degraded/error health when fallbacks are used.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/admin-dashboard.js` | No syntax errors | Passed | Pass |
| JS syntax | `node --check frontend/platform/static/js/admin-global-search.js` | No syntax errors | Passed | Pass |
| Schema dependency check | Queried local Postgres catalog for key dashboard tables/columns | Required dependencies exist | Required checked dependencies found | Pass |
| Local page smoke | `curl` `/admin/` on `localhost:8893` | Backend responds with protected page behavior | Returned `401` unauthenticated | Pass |
| Admin dashboard HTTP/HTML E2E | `BASE_URL=http://localhost:8893 python3 tests/admin/test_admin_dashboard.py` | Admin dashboard suite passes with no failures | 308 passed, 0 failed, 87 warnings | Pass with warnings |
| Browser-level admin E2E subset | `BASE_URL=http://localhost:8893 python3 -m pytest tests/e2e -m admin --maxfail=5 -q --base-url http://localhost:8893` | Admin Playwright subset passes | 17 passed, 28 deselected in 23.68s; report at `tests/e2e/reports/report.html` | Pass |
| Admin search smoke | Authenticated `GET /api/admin/search?q=test` on `localhost:8893` | 200 minimal capped search payload | 200, `results` array with 10 records | Pass |
| Dashboard JS asset | `curl -i http://localhost:8893/static/js/admin-dashboard.js` | 200 JavaScript asset | `200 OK` | Pass |
| Global search JS asset | `curl -i http://localhost:8893/static/js/admin-global-search.js` | 200 JavaScript asset | `200 OK` | Pass |
| Dashboard-specific Playwright regression | Included in the marked admin subset via `tests/e2e/test_admin_dashboard_index.py` | Stats render, malicious activity text is not HTML, system health fields exist, and global search uses `/api/admin/search` without broad list fetches | Passed | Pass |
| Rust format/compile | `cargo fmt --check`; `cargo check` from `backend/` | No formatting or compile failures | Passed | Pass |
| Optional mobile smoke | Open `/admin/` on a small viewport after fixes | No layout overlap or mobile-only console issues | Not included in the targeted regression test | Not run |

---

## Security Findings

- P1 fixed: Dashboard activity/order/deposit rendering now uses DOM/text APIs for dynamic data.
- P2 fixed: Global search now uses a minimal server-side `/api/admin/search` contract.
- P3 fixed: Unused external HTMX/Alpine CDN scripts were removed from the admin page.
- Auth/CSRF: The page and read-only APIs are routed through admin authentication; no dashboard-local state-changing action was identified.

---

## Database Findings

- Required checked tables/columns for the dashboard stats route are present in the local schema.
- Monetary display values are sourced from integer cents fields.
- No financial mutations happen on this page.
- Fixed reliability issue: stats API propagates required DB failures and system health exposes degraded fallback state.

---

## Missing Tests

- Optional: add a DB-fault injection test proving `/api/admin/stats/overview` does not return all-zero success on required query failure.
- Optional: expand Playwright coverage to range changes, notification navigation, global search keyboard behavior, console capture, and mobile layout.
- Covered by new targeted E2E: malicious dashboard values render as text, `/api/admin/system` exposes the dashboard contract fields, and global search uses a minimal server-side endpoint.

---

## Recommended Fix Order

1. Fixed: Replace dashboard `innerHTML` data rendering with safe DOM/text rendering.
2. Fixed: Stop suppressing required stats/system API database errors.
3. Fixed: Align `/api/admin/system`, dashboard health JS, and tests to one contract.
4. Fixed: Add visible error/retry states for failed dashboard loads.
5. Fixed: Move global search to a dedicated minimal server-side search endpoint.
6. Fixed: Remove unused CDN scripts.
7. Fixed: Add targeted authenticated E2E coverage.

---

## Final Status

`fixed_needs_browser_recheck`

Reason: The code fixes compile and targeted static checks pass. Authenticated browser/mobile verification is still recommended after restarting the backend.

---

## Fix Summary

Fixed on 2026-04-25:

- Replaced dashboard activity/order/deposit `innerHTML` rendering with DOM/text rendering.
- Changed `/api/admin/stats/overview` to propagate required database errors instead of returning false zero/empty success responses.
- Added `unread_notifications` to the stats payload and wired `#notification-count`.
- Added system-health fields consumed by the dashboard: `psp_connected`, `kyc_provider`, and `email_configured`; health booleans now reflect degraded fallback reads.
- Added `GET /api/admin/search?q=...` for minimal capped global search results and changed browser search to call it instead of fetching broad admin list APIs.
- Updated the deposits KPI label/subtext so the selected range is visible.
- Removed unused external HTMX/Alpine CDN scripts from the dashboard page.

Verification run:

```bash
node --check frontend/platform/static/js/admin-dashboard.js
node --check frontend/platform/static/js/admin-global-search.js
cargo fmt --check
cargo check
python3 scripts/audit_page_review_tracker.py --write-md
```

Remaining recheck:

- Restart the backend and run an authenticated browser/mobile pass for dashboard load, range changes, health dots, notification badge, global search keyboard behavior, visible API failure states, and console/network errors.
