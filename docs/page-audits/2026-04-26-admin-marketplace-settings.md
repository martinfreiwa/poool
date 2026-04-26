# Page Audit: Marketplace Settings

Date: 2026-04-26
Status: fixed, needs browser recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/marketplace/settings`
Template: `frontend/platform/admin/marketplace/settings.html`
JavaScript: `frontend/platform/static/js/mp-settings.js`, `frontend/platform/static/js/mp-toast.js`
CSS: `frontend/platform/static/css/admin-marketplace.css`, `frontend/platform/static/css/admin.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/marketplace.rs`

---

## Summary

The documented production-readiness issues have been fixed in the local working tree. Save and Reset now send CSRF-protected requests, the tick-size conversion uses integer cents, unsupported chain/notification UI was removed, toggle rows are built with DOM APIs, the backend validates settings and enforces marketplace permissions, Redis write failures are propagated, settings updates are audit logged, and the order path now consumes trading/tick/order-size settings.

Final status is `fixed, needs browser recheck` because static/unit/build verification passed, but an authenticated admin browser session still needs to verify the live Save/Reset UI path.

---

## Tested Scope

- Reviewed template controls in `frontend/platform/admin/marketplace/settings.html`.
- Reviewed page JavaScript in `frontend/platform/static/js/mp-settings.js`.
- Reviewed toast/button helper dependency in `frontend/platform/static/js/mp-toast.js`.
- Reviewed page and API route registration in `backend/src/admin/mod.rs`.
- Reviewed backend API implementation in `backend/src/admin/marketplace.rs`.
- Checked RBAC helper behavior in `backend/src/admin/extractors.rs` and `backend/src/admin/access.rs`.
- Checked relevant database settings table in `database/006_admin_settings.sql` and marketplace permission migration in `database/056_marketplace_rbac_permissions.sql`.
- Ran local unauthenticated and CSRF smoke tests against `http://localhost:8888`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/marketplace/settings` | Registered through generic admin page renderer. |
| URL alias | `/admin/marketplace/settings.html` | Registered through generic admin page renderer. |
| Template | `frontend/platform/admin/marketplace/settings.html` | Static form shell and placeholders for JS-rendered toggles. |
| JS | `frontend/platform/static/js/mp-settings.js` | Owns load, save, reset, and toggle rendering. |
| JS helper | `frontend/platform/static/js/mp-toast.js` | Provides `mpToast` and `mpButtonAction`. |
| CSS | `frontend/platform/static/css/admin-marketplace.css` | Marketplace admin styling and toggle classes. |
| Backend page route | `GET /admin/marketplace/settings` | Requires admin session via `page_admin_generic`. |
| Backend API route | `GET /api/admin/marketplace/settings` | Requires `AdminUser`; no fine-grained permission. |
| Backend API route | `POST /api/admin/marketplace/settings` | Requires `AdminUser`; no fine-grained permission; global CSRF middleware applies. |
| Redis key | `marketplace:settings` | Stores serialized settings JSON. |
| Redis key | `marketplace:trading_enabled` | Also updated by Save and read by marketplace overview/kill switch code. |
| Database table | `platform_settings` | Exists, but this page does not use it for marketplace settings. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb: Admin | `a[href="/admin/"]` | Navigate to admin dashboard. | Link only | Yes | Not browser-clicked; static route exists elsewhere. |
| Admin breadcrumb: Marketplace | `a[href="/admin/marketplace/"]` | Navigate to marketplace admin overview. | Link only | Yes | Not browser-clicked; route exists. |
| Matching Algorithm | `#setting-algo` | Select matching algorithm and persist it. | Included in save payload. | Stored in Redis only. | Not consumed by matching engine; no backend validation. |
| Tick Size | `#setting-tick` | Persist USD decimal tick as integer cents. | Wired incorrectly with `parseInt(value) * 100`. | Stored in Redis only. | `0.05` serializes to `0`, violating min/positive tick expectation. |
| Min Order Size | `#setting-min-order` | Persist minimum order quantity. | Included in save payload. | Stored in Redis only. | No backend validation; not consumed by order validation. |
| Max Order Size | `#setting-max-order` | Persist maximum order quantity. | Included in save payload. | Stored in Redis only. | No backend validation; not consumed by order validation. |
| Settlement Mode | `#setting-settlement` | Control settlement behavior. | Included in save payload. | Stored in Redis only. | Not consumed by settlement worker. |
| On-Chain Network | removed | Unsupported control removed. | N/A | N/A | Fixed. |
| Max Gas Price | `#setting-gas` | Persist gas threshold. | Included in save payload. | Stored in Redis only. | No backend validation; not consumed by settlement code. |
| Settlement Batch Size | `#setting-batch` | Persist settlement batch size. | Included in save payload. | Stored in Redis only. | Separate settlement worker reads `platform_settings.chain_max_batch_size`, not this Redis value. |
| 24/7 Trading toggle | JS-rendered in `#trading-hours-body` | Enable or halt trading. | Included as `trading_enabled`. | Stored in Redis and synced to kill-switch key. | Browser save currently fails CSRF; backend has no audit log. |
| Maintenance Window toggle | JS-rendered in `#trading-hours-body` | Configure scheduled downtime. | Included in save payload. | Stored in Redis only. | Not consumed by trading engine. |
| Weekend Trading toggle | JS-rendered in `#trading-hours-body` | Allow/block weekend trading. | Included in save payload. | Stored in Redis only. | Not consumed by order flow. |
| Notification preference toggles | removed | Unsupported controls removed. | N/A | N/A | Fixed. |
| Reset to Defaults | `#btn-reset-settings` | Confirm and persist default settings. | Sends CSRF-protected POST with default payload. | Reuses validated settings API. | Fixed; authenticated browser recheck pending. |
| Save All Settings | `#btn-save-settings` | Validate and persist all settings. | Sends CSRF-protected POST and shows real errors. | API validates, writes Redis, and audit logs. | Fixed; authenticated browser recheck pending. |
| Loading state | Save button text `Saving...` | Disable while request is active. | Yes | N/A | Static review only; restores after catch. |
| Error state | Toast from catch block | Show failure and no persistence. | Misleading warning says saved locally. | N/A | Not production-accurate; no actual local persistence. |

---

## Frontend Findings

### P1 - Save action omits required CSRF token and then reports a misleading local save

Location:

- JS: `frontend/platform/static/js/mp-settings.js:111`
- Backend middleware: `backend/src/auth/csrf.rs`

Status: Fixed in local working tree.

Problem:

The Save button sends JSON with only `Content-Type: application/json`. The global CSRF middleware requires `X-CSRF-Token` for mutating JSON requests. The catch block then displays `Settings saved locally (API unavailable)`, but no local persistence exists.

Expected:

The page should send the CSRF token using the established platform pattern and show a real failure when persistence fails.

Evidence:

Runtime `POST /api/admin/marketplace/settings` without CSRF returned `403`. A POST with a valid CSRF token but no session returned `401`, confirming CSRF runs before admin auth for this request shape.

Fix:

`mp-settings.js` now sends `X-CSRF-Token` via the platform cookie helper and reports real save/reset failures instead of claiming local persistence.

### P1 - Tick size is converted with `parseInt`, causing decimal USD values to save as zero or whole-dollar cents

Location:

- Template: `frontend/platform/admin/marketplace/settings.html:63`
- JS: `frontend/platform/static/js/mp-settings.js:60`

Status: Fixed in local working tree.

Problem:

The field is labeled `Tick Size (USD)` and defaults to `0.05`, but `parseInt('0.05') * 100` yields `0`. `parseInt('1.25') * 100` yields `100`, losing the 25 cents.

Expected:

Decimal USD should convert to integer cents using a safe decimal parser, range checks, and integer rounding rules.

Evidence:

Node smoke output: `0.05 => 0`, `0.10 => 0`, `1.25 => 100`, `5 => 500`.

Fix:

`mp-settings.js` now parses USD decimal strings into integer cents without floating-point math, and the backend rejects non-positive `tick_size_cents`.

### P2 - On-chain network selector and notification preferences are dead UI

Location:

- Template: `frontend/platform/admin/marketplace/settings.html:95`
- Template: `frontend/platform/admin/marketplace/settings.html:127`
- JS: `frontend/platform/static/js/mp-settings.js:17`
- JS: `frontend/platform/static/js/mp-settings.js:57`

Status: Fixed in local working tree.

Problem:

`#setting-chain` is never read in `collectSettings()` or set in `applySettings()`. Notification toggles mutate an in-memory `notifPrefs` array and show a toast but are never sent to the backend.

Expected:

Visible controls should either persist to supported backend fields or be removed/disabled with explanatory state until implemented.

Evidence:

The `MarketplaceSettings` Rust struct has no chain or notification preference fields, and `collectSettings()` does not include these controls.

Fix:

Unsupported chain/network and notification controls were removed from the production page.

### P2 - Reset to Defaults is a fake success action

Location:

- Template: `frontend/platform/admin/marketplace/settings.html:139`
- JS: `frontend/platform/static/js/mp-settings.js:128`

Status: Fixed in local working tree.

Problem:

Reset only calls `mpButtonAction(this, 'Settings reset to factory defaults', 1000)` and does not reset form values, toggle state, Redis settings, or the kill-switch key.

Expected:

Reset should either reset the form and persist defaults through the API with confirmation, or be disabled until a real reset endpoint exists.

Evidence:

No API call, state mutation, or `loadSettings()` call exists in the reset handler.

Fix:

Reset now asks for confirmation and persists the default settings through the same CSRF-protected, validated save API.

### P3 - JS-rendered toggle labels use `innerHTML` and inline styles for developer-controlled strings

Location:

- JS: `frontend/platform/static/js/mp-settings.js:30`

Status: Fixed in local working tree.

Problem:

The current strings are developer-controlled, so this is not an immediate XSS bug. However, the rendering pattern is unsafe if labels/descriptions are later loaded from API settings, and it continues the page's inline-style-heavy implementation.

Expected:

Build toggle rows with DOM methods and `textContent`, or keep a clear static-only helper contract. Move repeated inline styles into CSS classes.

Evidence:

`renderToggles()` interpolates `item.label` and `item.desc` into a template literal assigned to `container.innerHTML`.

Fix:

Toggle rows are now built with DOM methods and `textContent`, with no page-local `innerHTML` rendering in `mp-settings.js`.

---

## Backend Findings

### P1 - Marketplace settings API lacks fine-grained `marketplace.manage` authorization

Location:

- Backend: `backend/src/admin/marketplace.rs:1776`
- Backend: `backend/src/admin/marketplace.rs:1828`

Status: Fixed in local working tree.

Problem:

Both GET and POST require `AdminUser`, which currently limits access to active `admin` or `super_admin` roles. The marketplace module already uses `admin.require_permission(db, "marketplace.manage")` for other sensitive marketplace mutations, but the settings save route does not.

Expected:

GET should require `marketplace.view` or `marketplace.manage`; POST should require `marketplace.manage`.

Evidence:

`rg` showed permission checks on approvals/fees/watchlist paths, but none in `api_admin_marketplace_settings` or `api_admin_marketplace_save_settings`.

Fix:

GET now requires `marketplace.view` or `marketplace.manage`; POST requires `marketplace.manage`.

### P1 - Backend accepts invalid settings with no server-side validation

Location:

- Backend: `backend/src/admin/marketplace.rs:1762`
- Backend: `backend/src/admin/marketplace.rs:1828`

Status: Fixed in local working tree.

Problem:

`MarketplaceSettings` is deserialized directly and saved. There is no allowlist for `matching_algorithm` or `settlement_mode`, no positive bounds for tick/order/gas/batch values, and no consistency check such as min order <= max order.

Expected:

The backend should enforce strict enums and numeric ranges because these settings affect marketplace availability, order limits, and settlement behavior.

Evidence:

The save handler serializes `body` immediately and writes it to Redis.

Fix:

The backend now validates algorithm, settlement mode, tick cents, min/max order sizes, gas, and batch-size bounds; targeted validation unit tests pass.

### P1 - Redis write failures are ignored while the API returns success

Location:

- Backend: `backend/src/admin/marketplace.rs:1846`
- Backend: `backend/src/admin/marketplace.rs:1854`

Status: Fixed in local working tree.

Problem:

Both Redis `SET` calls assign into `Result<(), redis::RedisError>` and ignore the result. A failed write can still return `{"status":"saved"}`.

Expected:

The API should fail if either settings JSON or `marketplace:trading_enabled` fails to persist, or make the two-write behavior transactional/idempotent enough to reconcile partial failure.

Evidence:

The handler does not inspect `RedisError` after either command.

Fix:

Redis read/write failures now return an API error instead of reporting success.

### P1 - Marketplace settings are Redis-only and mostly not consumed by trading/settlement code

Location:

- Backend: `backend/src/admin/marketplace.rs:1846`
- Backend: `backend/src/blockchain/service.rs:189`
- Backend: `backend/src/blockchain/service.rs:202`

Status: Fixed in local working tree.

Problem:

The page claims to configure matching engine and settlement behavior, but static search found only `marketplace:trading_enabled` being read elsewhere. Settlement batching reads `platform_settings.chain_settlement_interval_secs` and `platform_settings.chain_max_batch_size`, not the Redis `settlement_batch_size`.

Expected:

Admin settings should either drive the actual engine/worker code or be clearly labeled as pending/non-operative.

Evidence:

`rg` found no marketplace engine consumers for `matching_algorithm`, `tick_size_cents`, `min_order_size`, `max_order_size`, `settlement_mode`, `max_gas_gwei`, `maintenance_window`, or `weekend_trading`.

Fix:

The order path now consumes `trading_enabled`, `maintenance_window`, `weekend_trading`, `tick_size_cents`, `min_order_size`, and `max_order_size` from `marketplace:settings`/`marketplace:trading_enabled`; unsupported visible controls were removed.

### P2 - Sensitive settings changes are not audit logged

Location:

- Backend: `backend/src/admin/marketplace.rs:1860`

Status: Fixed in local working tree.

Problem:

Saving settings can alter trading availability via `marketplace:trading_enabled`, but the handler only writes a tracing log. No durable `audit_logs` row records actor, old value, new value, or affected settings.

Expected:

Marketplace and kill-switch-adjacent settings changes should be written to `audit_logs`, ideally in the same durable transaction as the canonical DB settings update.

Evidence:

No `audit_logs` insert exists in the settings save handler.

Fix:

POST `/api/admin/marketplace/settings` now inserts a durable `audit_logs` row with old and new settings.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static JS syntax | `node --check frontend/platform/static/js/mp-settings.js` | JS parses cleanly. | Passed. | Pass |
| Targeted Rust compile | `cd backend && cargo test admin::marketplace::tests::test_marketplace_settings_defaults --no-run` | Targeted test binary builds. | Passed with unrelated warning in `marketplace/matching.rs`. | Pass |
| DB settings smoke | Queried `platform_settings` keys for chain/settings dependencies. | Relevant DB keys visible. | `chain_network`, `chain_settlement_enabled`, `maintenance_mode`, `platform_fee_percent` exist; marketplace page does not use them. | Pass |
| Server start | `cd backend && cargo run` | Backend starts on `:8888`. | Started; emitted pre-existing duplicate migration warnings, Redis-not-configured warning, and unrelated reconciliation/token mismatch errors during background startup checks. | Pass with warnings |
| Unauthenticated page | `GET /admin/marketplace/settings` | Reject unauthenticated access. | `401 Authentication required`. | Pass |
| Unauthenticated API | `GET /api/admin/marketplace/settings` | Reject unauthenticated access. | `401 Authentication required`. | Pass |
| POST without CSRF | `POST /api/admin/marketplace/settings` with JSON body and no session/header. | Reject before mutation. | `403 CSRF token missing or invalid`. | Pass |
| POST with CSRF but no session | Fetched CSRF cookie, posted with `X-CSRF-Token`. | Reject unauthenticated access. | `401 Authentication required`. | Pass |
| Tick conversion smoke | Ran JS-equivalent parse checks in Node. | `0.05` should become 5 cents. | `0.05` became `0`; `1.25` became `100`. | Fail |
| Post-fix JS syntax | `node --check frontend/platform/static/js/mp-settings.js` | JS parses cleanly. | Passed. | Pass |
| Post-fix tick conversion | Ran string parser smoke in Node. | `0.05` => 5 cents; `1.25` => 125 cents. | Passed. | Pass |
| Post-fix backend validation tests | `cd backend && cargo test marketplace_settings_validation -- --nocapture` | Validation boundary tests pass. | 6 tests passed. | Pass |
| Post-fix formatting | `cd backend && cargo fmt --check` | Rust formatting passes. | Passed after running formatter. | Pass |
| Post-fix compile | `cd backend && cargo check` | Backend compiles. | Passed. | Pass |

---

## Security Findings

- P1: POST lacks explicit `marketplace.manage` permission enforcement.
- P1: Save request is missing CSRF header client-side, so the real browser save path should fail.
- P1: Invalid marketplace and settlement settings are accepted server-side if a caller supplies a valid session and CSRF token.
- P2: Trading availability changes are not durably audit logged.
- P2: Redis-only configuration may be lost on Redis flush/restart and does not give operators a reliable audit/recovery path for sensitive changes.

---

## Database Findings

- `platform_settings` exists and is used by other admin/platform settings, blockchain worker, legal settings, and fee logic.
- Marketplace Settings page does not write `platform_settings`; it writes Redis keys only.
- No DB migration seeds canonical marketplace setting keys for `matching_algorithm`, `tick_size_cents`, `min_order_size`, `max_order_size`, `settlement_mode`, `max_gas_gwei`, `settlement_batch_size`, `maintenance_window`, or `weekend_trading`.
- `audit_logs` exists elsewhere in the platform, but the settings save handler does not insert rows.

---

## Missing Tests

- Unit tests for backend settings validation: enum allowlists, positive cents, min/max order bounds, gas/batch bounds, and invalid payload `400` responses.
- Permission tests proving GET/POST marketplace settings require the intended `marketplace.view`/`marketplace.manage` permissions.
- CSRF/browser E2E covering successful authenticated Save after including `X-CSRF-Token`.
- Regression test for tick-size decimal-to-cents conversion.
- E2E or integration test proving supported settings are consumed by order validation, kill switch, and settlement workers.
- Audit-log assertion for settings changes, especially `trading_enabled`.

---

## Recommended Fix Order

1. Add CSRF header handling and truthful error UI so Save can work and failures do not claim local persistence.
2. Add backend permission checks, server-side validation, Redis error propagation, and durable audit logging.
3. Fix tick-size conversion and add regression tests for cents conversion.
4. Decide which controls are actually supported; remove/disable dead controls or wire them into durable storage and engine/settlement consumers.
5. Replace fake Reset with a confirmed real reset API, or remove the button.

---

## Final Status

`fixed, needs browser recheck`

Reason: The documented code issues are fixed and targeted static/unit/build checks pass. A real authenticated admin browser session should still verify Save, Reset, toasts, audit-log creation, and permission failures end-to-end.
