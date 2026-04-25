# Page Audit: Admin Marketplace Alerts

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/marketplace/alerts`
Template: `frontend/platform/admin/marketplace/alerts.html`
JavaScript: `frontend/platform/static/js/mp-alerts.js`, `frontend/platform/static/js/mp-toast.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/admin-marketplace.css`, `frontend/platform/static/css/bundle.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/marketplace.rs`

---

## Summary

`/admin/marketplace/alerts` is implemented as an admin marketplace page with KPI cards, a dynamic alerts table, and acknowledge/resolve actions backed by `/api/admin/marketplace/alerts`. The JavaScript parses, the backend compiles, unauthenticated page/API access returns `401`, and CSRF middleware rejects unauthenticated mutating POSTs without a token.

The page is not production-ready. If the alerts API fails, the page silently renders hardcoded mock fraud/security alerts and permits fake local state changes. Backend alert APIs use broad `AdminUser` access instead of marketplace compliance/manage permissions, list failures collapse to an empty array, alert status updates are not audited, and the frontend renders database alert messages with `innerHTML`.

---

## Tested Scope

- Static template review of `frontend/platform/admin/marketplace/alerts.html`
- JavaScript review of `frontend/platform/static/js/mp-alerts.js` and `frontend/platform/static/js/mp-toast.js`
- Backend route registration review in `backend/src/admin/mod.rs`
- Backend handler review in `backend/src/admin/marketplace.rs`
- Permission mapping review in `frontend/platform/static/js/admin-permission-guard.js`
- Schema review for `marketplace_alerts` and marketplace RBAC permissions
- Runtime unauthenticated curl checks against the local backend on `127.0.0.1:8888`

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/marketplace/alerts` | Registered clean admin marketplace route |
| URL alias | `/admin/marketplace/alerts.html` | Registered HTML route |
| Template | `frontend/platform/admin/marketplace/alerts.html` | KPI cards and alerts table shell |
| JS | `frontend/platform/static/js/mp-alerts.js` | Fetches alerts, renders rows, posts actions |
| JS | `frontend/platform/static/js/mp-toast.js` | Shared toast/button/modal helper |
| Backend page route | `GET /admin/marketplace/alerts` | `page_admin_generic` |
| Backend API route | `GET /api/admin/marketplace/alerts` | Lists up to 200 marketplace alerts |
| Backend API route | `POST /api/admin/marketplace/alerts/:alert_id` | Acknowledge, resolve, or mark false positive |
| Database table | `marketplace_alerts` | Alert type, severity, status, related asset/user/trade, metadata |
| Permissions | `marketplace.view`, `marketplace.manage`, `marketplace.compliance` | Seeded, but not enforced by these handlers |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `/admin/` | Navigate to admin dashboard | Link | Registered | Static verified |
| Marketplace breadcrumb | `/admin/marketplace/` | Navigate to marketplace admin overview | Link | Registered generic route | Static verified |
| Total Alerts KPI | `#kpi-total-alerts` | Show number of loaded alerts | `render()` | `GET /api/admin/marketplace/alerts` | Static verified |
| Critical KPI | `#kpi-critical` | Count critical severity alerts | `render()` | Same list API | Static verified |
| Unresolved KPI | `#kpi-unresolved` | Count non-resolved/non-false-positive alerts | `render()` | Same list API | Static verified |
| Alerts table | `#alerts-table` | Display alert rows | `render()` | Same list API | Static verified |
| Alerts body | `#alerts-body` | Dynamic row target | `tbody.innerHTML` | Same list API | XSS/error-state gaps |
| Alert ID cell | generated `<code>` | Display short alert ID | JS | API `id` | Static verified |
| Severity badge | `severityBadge()` | Show critical/warning/info badge | JS | API `severity` | Static verified |
| Status badge | `statusBadge()` | Show new/acknowledged/resolved/false positive | JS | API `status` | Static verified |
| Related users | generated `<code>` | Show related user ID | JS | API `user_id` | Static verified |
| Acknowledge button | `.btn-ack` | POST action `acknowledge` | `handleAlertAction()` | `POST /alerts/:id` | Authenticated mutation unverified |
| Resolve button | `.btn-resolve` | POST action `resolve` | `handleAlertAction()` | `POST /alerts/:id` | Authenticated mutation unverified |
| Toasts | `mpToast()` | Show action success/failure | Shared helper | N/A | Helper has raw `innerHTML` |
| Loading state | none visible in template | Show loading while API fetches | Missing | N/A | Missing |
| Error state | mock fallback | Show real API failure | Miswired | N/A | Broken behavior |
| Watchlist UI | page title mentions Watchlist | Manage watchlist entries | Missing | Backend watchlist API exists | Dead product surface |

---

## Frontend Findings

### P1 - API failures render fake fraud alerts and allow fake actions

Location:

- JS: `frontend/platform/static/js/mp-alerts.js:14`
- JS: `frontend/platform/static/js/mp-alerts.js:151`

Problem:

When `GET /api/admin/marketplace/alerts` fails, the page logs a warning, loads `MOCK_ALERTS`, and sets `usingMockData = true`. Action buttons then mutate local mock state through `mpButtonAction()` and show success to the admin.

Expected:

Admin compliance pages must fail closed with a visible retryable error state. They must never show demo fraud, settlement, negative-balance, KYC, or API-abuse data as if it were operational truth.

Evidence:

`loadAlerts()` catches any fetch error and assigns `alerts = [...MOCK_ALERTS]`. `handleAlertAction()` has a mock path that marks mock alerts as acknowledged/resolved.

Recommended fix:

Remove production mock fallback, render a clear API error/retry state, and keep action controls disabled until real API data loads.

### P1 - Alert rows render database-controlled text with innerHTML

Location:

- JS: `frontend/platform/static/js/mp-alerts.js:61`

Problem:

The table body is populated with a template string via `tbody.innerHTML`. Alert fields such as `alert_type`, `message`, `severity`, `status`, and user IDs are inserted without escaping.

Expected:

Alert rows should be built with DOM APIs and `textContent`, or all fields should be escaped before insertion. Fraud/compliance alerts can contain user- or system-derived text and should be treated as untrusted.

Evidence:

`desc = a.message`, `alertType = a.alert_type`, and related values are interpolated directly into table HTML.

Recommended fix:

Render rows with `document.createElement`, set text with `textContent`, and only use static trusted badge markup.

### P2 - Toast helper renders message with innerHTML

Location:

- JS: `frontend/platform/static/js/mp-toast.js:23`

Problem:

`mpToast()` interpolates `message` into `toast.innerHTML`. The current alerts page usually passes static strings or HTTP status text, but this shared helper is unsafe for any caller that passes server-derived text.

Expected:

The toast helper should build DOM nodes and use `textContent` for message content.

Evidence:

`toast.innerHTML = ... <span class="mp-toast-message">${message}</span>`.

Recommended fix:

Replace toast string templates with DOM construction.

### P2 - Missing loading, empty, and real error states

Location:

- Template: `frontend/platform/admin/marketplace/alerts.html:79`
- JS: `frontend/platform/static/js/mp-alerts.js:151`

Problem:

The template starts with an empty `<tbody>`, and the JS does not render a loading indicator, successful empty state, or real API error. API failures become fake data instead.

Expected:

Show loading during fetch, an empty state when the API returns `[]`, and a visible retryable error when the API fails.

Evidence:

`#alerts-body` is blank until `render()` runs; there is no error renderer.

Recommended fix:

Add explicit `renderLoading()`, `renderEmpty()`, and `renderError()` paths.

---

## Backend Findings

### P1 - Marketplace alert routes lack marketplace permission gates

Location:

- Backend page route: `backend/src/admin/pages.rs:140`
- Backend API routes: `backend/src/admin/marketplace.rs:1337`, `backend/src/admin/marketplace.rs:1366`

Problem:

The page and APIs require only `AdminUser`. The migration defines `marketplace.compliance` for alerts/OJK oversight and the sidebar hides `nav-mp-alerts` behind `marketplace.manage`, but the backend does not enforce either permission.

Expected:

`GET /admin/marketplace/alerts` and `GET /api/admin/marketplace/alerts` should require at least `marketplace.compliance` or a clearly documented marketplace alert permission. Mutating status actions should require `marketplace.compliance` or `marketplace.manage`.

Evidence:

No `require_permission()` call exists in the alert handlers. `page_admin_generic` only has a special permission branch for `admin/community/`.

Recommended fix:

Add server-side page/API permission checks and align the sidebar mapping with the backend decision.

### P1 - Alert action updates are not audited and do not check affected rows

Location:

- Backend: `backend/src/admin/marketplace.rs:1366`

Problem:

The action handler updates `marketplace_alerts` but does not write an `audit_logs` record. It also does not check `rows_affected()`, so a valid UUID that matches no alert still returns success.

Expected:

Alert status changes should be durably audit logged with actor, previous/new status, alert ID, and action. Missing alerts should return `404`.

Evidence:

The handler calls `.execute(db).await?` and immediately returns `{"status": new_status}` without inspecting the result.

Recommended fix:

Fetch the current alert first, update conditionally in a transaction, check affected rows, and insert an audit log before commit.

### P2 - Alert list query failures are hidden as an empty list

Location:

- Backend: `backend/src/admin/marketplace.rs:1353`

Problem:

`api_admin_marketplace_alerts()` uses `.unwrap_or_default()`, so database failures return `200 []`. Combined with the frontend mock fallback, operational failures are easy to miss.

Expected:

Database errors should return safe 5xx JSON and be visible in the frontend as a load failure.

Evidence:

The list handler does `.fetch_all(db).await.unwrap_or_default()`.

Recommended fix:

Map `sqlx::Error` into `ApiError::Database` or a client-safe internal error.

### P2 - Alert status transition semantics are weak

Location:

- Backend: `backend/src/admin/marketplace.rs:1388`

Problem:

`resolved_by` and `resolved_at` are set for every action, including `acknowledge`. The handler allows transitions from resolved/false_positive back to acknowledged/resolved without a state-machine rule.

Expected:

Acknowledgement should record acknowledgement metadata separately or leave resolution fields null. Resolved/false-positive terminal states should require explicit reopening or be immutable unless a documented override exists.

Evidence:

One SQL update writes `status`, `resolved_by`, and `resolved_at` for all actions.

Recommended fix:

Define alert transition rules and add columns or metadata for acknowledged-by/at if acknowledgement must be tracked separately.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/mp-alerts.js && node --check frontend/platform/static/js/mp-toast.js` | No syntax errors | Passed with no output | Pass |
| Rust compile | `cd backend && cargo check` | Backend compiles | Finished dev profile successfully | Pass |
| Unauthenticated page | `curl -i http://127.0.0.1:8888/admin/marketplace/alerts` | Auth required | `401 {"error":"Authentication required"}` | Pass |
| Unauthenticated API | `curl -i http://127.0.0.1:8888/api/admin/marketplace/alerts` | Auth required | `401 {"error":"Authentication required"}` | Pass |
| CSRF middleware smoke | POST alert action without token | `403` CSRF error before mutation | `403` JSON CSRF error | Pass |
| Authenticated page load | Log in as marketplace admin/compliance and open page | Real alerts or empty state render | Not run; no admin fixture session in this pass | Blocked |
| Alert action mutation | Acknowledge/resolve safe fixture alert | DB status changes, audit row, visible success | Not run; requires safe alert fixture/admin session | Blocked |
| Mobile/keyboard smoke | Use row actions and navigation by keyboard/mobile viewport | Accessible action controls and no overflow | Not run; static review only | Blocked |

---

## Security Findings

- P1: Backend does not enforce marketplace alert/compliance permissions.
- P1: API failures render mock fraud/security data as operational data.
- P1: Database alert text is inserted via `innerHTML`.
- P1: Alert status changes are not audit logged.
- P2: Shared toast helper is unsafe for server-derived messages.
- P2: Missing `rows_affected()` check can return success for nonexistent alerts.

---

## Database Findings

- `marketplace_alerts` exists with status/severity constraints and useful indexes.
- `resolved_by` and `resolved_at` exist, but there is no separate acknowledgement metadata.
- No audit-log write is performed for alert action changes.
- The list handler hides DB failures instead of returning errors.

---

## Missing Tests

- Add backend permission tests for alert list/action APIs covering unauthorized, generic admin, marketplace compliance, and marketplace manage roles.
- Add backend action tests for invalid UUID, nonexistent UUID, invalid action, valid acknowledge, valid resolve, false positive, terminal-state behavior, and audit-log persistence.
- Add frontend/E2E tests for successful empty state, API error state, no mock data fallback, XSS-safe alert message rendering, and action failure behavior.
- Add authenticated browser coverage for mobile and keyboard access to row action buttons.

---

## Recommended Fix Order

1. Remove mock fallback and add loading, empty, and visible API error states.
2. Enforce marketplace compliance/manage permissions on page and API routes.
3. Render alert rows and toast messages with DOM APIs and `textContent`.
4. Make alert action updates transactional, rows-affected aware, and audit logged.
5. Define status transition semantics for acknowledge/resolved/false-positive.
6. Add targeted backend and E2E regression tests.

---

## Final Status

`needs_recheck`

Reason: the page is implemented and anonymous/CSRF boundaries work, but fake fallback data, XSS-prone rendering, missing backend permission gates, hidden DB failures, missing audit logs, and incomplete state semantics require fixes and authenticated re-verification.
