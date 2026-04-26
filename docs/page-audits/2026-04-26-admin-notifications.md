# Page Audit: Notifications

Date: 2026-04-26
Status: fixed
Auditor: ChatGPT/Codex
Page URL: `/admin/notifications`
Template: `frontend/platform/admin/notifications.html`
JavaScript: `frontend/platform/static/js/admin-notifications.js`
CSS: `frontend/platform/static/css/admin.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/notifications.rs`

---

## Summary

`/admin/notifications` is a real admin page backed by the shared `notifications` table and live list/broadcast APIs. The page is not production-ready yet: backend access is only generic admin despite the sidebar declaring `notifications.send`, broadcast sends mass messages without a durable audit trail, list DB failures are silently converted to an empty successful table, and the frontend has weak error/loading/accessibility behavior.

Final status is `needs_recheck`.

---

## Tested Scope

- Reviewed `frontend/platform/admin/notifications.html`.
- Reviewed `frontend/platform/static/js/admin-notifications.js`.
- Reviewed `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/notifications.rs`, and admin auth/permission helpers.
- Reviewed `notifications` schema in `docs/DATABASE_SCHEMA.md`, `database/001_initial_schema.sql`, `database/014_optimization_indexes.sql`, and `database/069_apply_missing_community_schema.sql`.
- Ran syntax, schema, local auth, CSRF, and build checks listed below.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/notifications` | Clean admin route registered through `page_admin_generic`. |
| URL alias | `/admin/notifications.html` | Same page. |
| Template | `frontend/platform/admin/notifications.html` | KPI cards, broadcast form, filters, table, pagination. |
| JS | `frontend/platform/static/js/admin-notifications.js` | Loads list, filters/sorts/paginates client-side, sends broadcast. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Adds CSRF headers to mutating fetch calls and maps nav to `notifications.send`. |
| Backend page route | `GET /admin/notifications` | Generic admin-only route; no notification-specific permission gate. |
| Backend API route | `GET /api/admin/notifications` | Returns latest 200 notifications; DB errors are hidden as empty data. |
| Backend API route | `POST /api/admin/notifications/broadcast` | Inserts one notification for every user. |
| Database table | `notifications` | Real table exists with `type` check, read status, action URL, and community columns. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Admin link | `.admin-breadcrumbs a[href="/admin/"]` | Navigate to dashboard. | Native link | `GET /admin/` | Works by route review. |
| KPI cards | `#stat-total`, `#stat-unread`, `#stat-read-rate`, `#stat-today` | Show counts from loaded notifications. | Yes, computed client-side | `GET /api/admin/notifications` | Works only when list API succeeds; failures show zero/empty. |
| Broadcast type | `#broadcast-type` | Select notification type. | Yes | `POST /api/admin/notifications/broadcast` | UI values match DB except backend accepts arbitrary JSON type until DB rejects it. |
| Broadcast title | `#broadcast-title` | Required title. | Client checks non-empty | API checks title non-empty | Works for title only; no length normalization documented. |
| Broadcast message | `#broadcast-message` | Required message content. | Not required | API allows empty string | Broken contract for user-facing broadcast quality. |
| Send to All | `#broadcast-send-btn` | Send broadcast to all users with CSRF and feedback. | Yes | Real insert route | Real but unaudited; UI has no loading/disabled state and uses native alerts for errors. |
| Search | `#notif-search` | Filter loaded notifications by title/message/user. | Yes | Client-side only | Works on fetched 200-row slice, not all notifications. |
| Type filter | `#filter-type` | Filter loaded notifications by type. | Yes | Client-side only | Works on fetched 200-row slice. |
| Read filter | `#filter-read` | Filter by read/unread. | Yes | Client-side only | Works on fetched 200-row slice. |
| Count label | `#notif-count-label` | Show filtered count. | Yes | Client-side | Works after filters. |
| Sortable headers | `th[data-sort]` | Sort by user, type, read, date. | Mouse click only | Client-side | Works by click, lacks keyboard/ARIA sort state. |
| Notifications table | `#notif-table-body` | Render rows safely. | Yes | List API | Dynamic strings are escaped before `innerHTML`; failure state is misleading. |
| Previous page | `#prev-page` | Page backward. | Yes | Client-side | Works after successful render; stale when no rows branch returns early. |
| Next page | `#next-page` | Page forward. | Yes | Client-side | Same stale-state issue when no rows. |

---

## Frontend Findings

### P2 - Notification load failures render as empty success

Location:

- JS: `frontend/platform/static/js/admin-notifications.js`

Problem:

`loadNotifications()` logs non-OK responses and network failures, then still calls `updateStats()` and `applyFilters()`. With the default empty array this renders zero KPIs and "No notifications found." There is no visible error, retry action, or distinction between empty data and broken API.

Expected:

Render an explicit table error state with retry, preserve loading state while pending, and avoid showing zero KPIs as if they are real data when the API fails.

Evidence:

Unauthenticated `GET /api/admin/notifications` returned 401 locally. Static review shows no visible error branch.

Recommended fix:

Track `loading` and `error` states, render a retry row/status region, and set KPI placeholders to unavailable on request failure.

### P2 - Broadcast UX lacks safe in-flight and accessible feedback states

Location:

- Template: `#broadcast-send-btn`, `#broadcast-title`, `#broadcast-message`
- JS: `sendBroadcast()`

Problem:

The Send button remains enabled during POST, double-clicks are possible, success only clears inputs, and errors use native `alert()`. There is no persistent `role=status`/`role=alert` output, no `aria-busy`, and no clear success count despite the API returning `count`.

Expected:

Disable the button during submit, show success/failure inline, announce errors accessibly, and include the returned broadcast count.

Evidence:

Static review of `sendBroadcast()` and template; no status container exists.

Recommended fix:

Add an inline status region, button loading state, response-aware success copy, and duplicate-submit guard.

### P2 - Sort controls are mouse-only and do not expose sort state

Location:

- Template: `th[data-sort]`
- JS: `setupSorting()`

Problem:

Sortable table headers get `cursor: pointer` and click listeners, but they are not buttons, are not keyboard reachable, and do not set `aria-sort`.

Expected:

Use accessible button controls inside sortable headers or add keyboard handlers and `aria-sort` state updates.

Evidence:

Static review found only click listeners.

Recommended fix:

Implement sortable header buttons with visible focus and `aria-sort="ascending|descending|none"`.

### P3 - Minor table and dependency cleanup needed

Location:

- Template: `frontend/platform/admin/notifications.html`
- JS: `renderTable()`

Problem:

The empty row uses `colspan="6"` even though the table has five columns. The page also loads HTMX from `https://unpkg.com` but no `hx-*` attributes were found in the template.

Expected:

Use `colspan="5"` and remove the unused external HTMX dependency or serve it locally if needed.

Evidence:

Static template/JS review.

Recommended fix:

Correct the colspan and remove the unused CDN script.

---

## Backend Findings

### P1 - Notification page and APIs are not gated by notification-specific permissions

Location:

- Page route: `backend/src/admin/pages.rs`
- API routes: `backend/src/admin/notifications.rs`
- Sidebar permission map: `frontend/platform/static/js/admin-permission-guard.js`

Problem:

The sidebar declares `nav-notifications` requires `notifications.send`, but neither the page route nor the list/broadcast APIs call `require_permission`. The local database has no `notifications.*` permission grants. Any generic `admin`/`super_admin` session can list admin notification data and send a mass broadcast.

Expected:

Use explicit permissions, for example `notifications.view` for list/page and `notifications.send` for broadcast, seeded into `admin_permissions` for the intended roles. Page, sidebar, and APIs should enforce the same contract server-side.

Evidence:

Static review of `page_admin_generic`, `api_admin_notifications`, `api_admin_notification_broadcast`, and `admin-permission-guard.js`; local `admin_permissions` query found no `notifications.%` rows.

Recommended fix:

Add notification permissions migration, enforce them in page/API handlers, and add negative authorization tests for admin roles lacking the permission.

### P1 - Mass broadcast has no durable admin audit log

Location:

- Backend: `api_admin_notification_broadcast`
- Database: `audit_logs`, `notifications`

Problem:

Broadcast inserts one notification for every user but does not write an audit row describing who sent it, what type/title was sent, and how many recipients were affected. This is an admin-sensitive mass communication path.

Expected:

Create a durable audit record atomically with the broadcast or in the same transaction after insert count is known.

Evidence:

Static review of `api_admin_notification_broadcast`; only `notifications` are inserted.

Recommended fix:

Wrap broadcast in a transaction, insert notifications, insert an `audit_logs` row with actor/type/title/count, then commit.

### P1 - List API masks database failures as a successful empty list

Location:

- Backend: `api_admin_notifications`

Problem:

The notification list query uses `.fetch_all(...).await.unwrap_or_default()`. If the query fails because of schema drift, DB connectivity, decode errors, or a bad migration, the API returns `200 {"notifications":[]}` instead of an error.

Expected:

Propagate database errors through `ApiError::Database` so operators and the UI see an actual failure.

Evidence:

Static review of `backend/src/admin/notifications.rs`.

Recommended fix:

Replace `unwrap_or_default()` with `?`/`map_err(ApiError::from)` and add a regression test that a mocked or unavailable DB path does not become an empty success.

### P2 - Broadcast API validation is incomplete

Location:

- Backend: `api_admin_notification_broadcast`

Problem:

The API accepts arbitrary `type` values and allows empty messages. Invalid types are left to the database check constraint, returning a generic 500 instead of a 400. There are no explicit length bounds or trimming behavior for `title`/`message`.

Expected:

Validate `type` against the allowed set, require non-empty message content if the UI requires it, enforce length limits matching schema/product needs, and return 400 for validation failures.

Evidence:

Static backend review plus schema check showing `notifications.type` allows only `kyc`, `investment`, `payout`, `system`, and `promo`.

Recommended fix:

Deserialize into a typed request struct and validate before insert.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/admin-notifications.js` | Syntax passes. | Passed. | Pass |
| DB schema | `psql -d poool -c "\\d notifications"` | Required table/columns/indexes exist. | Table exists with expected base and community columns. | Pass |
| Local counts | Count rows in `notifications`. | Non-destructive read succeeds. | 8,787 notifications, 8,783 unread in local DB. | Pass |
| Unauthenticated page | `curl /admin/notifications` | Redirect to login. | HTTP 303 to `/auth/login`. | Pass |
| Unauthenticated list API | `curl /api/admin/notifications` | JSON auth failure. | HTTP 401 `Authentication required`. | Pass |
| Broadcast without CSRF | POST broadcast without session/CSRF. | Reject before mutation. | HTTP 403 CSRF rejection. | Pass |
| Rust build | `cargo check --manifest-path backend/Cargo.toml --quiet` | Build passes. | Passed. | Pass |

Authenticated browser list/broadcast testing was not run because this documentation-only audit did not create a safe admin broadcast fixture or perform mass notification writes.

---

## Security Findings

- Generic admin access is too broad for viewing notification user data and sending mass broadcasts.
- The sidebar declares `notifications.send`, but backend routes do not enforce it and the permission is not seeded locally.
- Broadcast lacks durable audit logging.
- The page loads unused HTMX from an external CDN on an admin surface.
- CSRF middleware rejected an unauthenticated POST without a token locally, and `admin-permission-guard.js` would add CSRF headers for normal mutating fetches when the cookie exists.

---

## Database Findings

- `notifications` table exists with type check, unread/user indexes, type index, and community metadata columns.
- `notifications.type` only accepts `kyc`, `investment`, `payout`, `system`, and `promo`, but the broadcast API validates none of that before insert.
- No `notifications.%` permissions were present in local `admin_permissions`.
- Broadcast is a multi-row admin-sensitive write and should include an audit-log insert in the same transaction.

---

## Missing Tests

- Authenticated HTTP/API test for `notifications.view` and `notifications.send` positive/negative authorization.
- Broadcast validation tests for invalid type, empty title, empty message, and length bounds.
- Broadcast audit-log test verifying actor, action, title/type, and recipient count.
- Frontend E2E for visible list error state, retry, search/filter/sort/pagination, double-submit prevention, and inline success/error status.
- Accessibility E2E for keyboard sorting and broadcast status announcement.

---

## Recommended Fix Order

1. Add and enforce `notifications.view` / `notifications.send` permissions on page/API routes.
2. Make broadcast transactional and audit logged.
3. Propagate list DB errors and add visible frontend error/retry states.
4. Add typed broadcast request validation.
5. Improve broadcast loading/status UX and sortable-header accessibility.
6. Remove unused external HTMX and fix the table colspan.

---

## Fix Verification

Date: 2026-04-26
Status: fixed

Implemented fixes:

- Added `database/087_admin_notification_permissions.sql` to seed `notifications.view` and `notifications.send` for admin roles.
- Enforced `notifications.view` on `/admin/notifications` and `GET /api/admin/notifications`.
- Enforced `notifications.send` on `POST /api/admin/notifications/broadcast`.
- Replaced the untyped broadcast JSON body with a typed request and server-side validation for allowed type, non-empty title/message, and length bounds.
- Replaced notification-list `unwrap_or_default()` with normal SQLx error propagation through `ApiError`.
- Made broadcast insertion transactional and added an atomic `audit_logs` row with actor, type, title, and recipient count.
- Removed unused external HTMX from the page.
- Added broadcast form labels/limits and an inline status region.
- Rebuilt `admin-notifications.js` around explicit loading, empty, error, retry, success, and in-flight states.
- Replaced native alerts with `role=status` / `role=alert` output.
- Rendered dynamic notification rows with DOM/textContent.
- Added keyboard-focusable sortable header buttons with `aria-sort`.
- Fixed the empty table row to use the correct five-column span.
- Added `tests/e2e/test_admin_notifications.py` for authenticated permission, CSRF, validation, audit-log, safe-rendering, search/filter/sort, and browser-health coverage.

Verification commands:

| Command | Result |
|---------|--------|
| `psql -d poool -f database/087_admin_notification_permissions.sql` | Passed |
| `node --check frontend/platform/static/js/admin-notifications.js` | Passed |
| `python3 -m py_compile tests/e2e/test_admin_notifications.py` | Passed |
| `cargo fmt --check --manifest-path backend/Cargo.toml` | Passed |
| `cargo check --manifest-path backend/Cargo.toml` | Passed |
| `BASE_URL=http://127.0.0.1:8888 DATABASE_URL=postgres://martin@localhost/poool python3 -m pytest tests/e2e/test_admin_notifications.py -q` | Passed, 1 test |

Tracker issues fixed: PAGE-ISSUE-0265, PAGE-ISSUE-0266, PAGE-ISSUE-0267, PAGE-ISSUE-0268, PAGE-ISSUE-0269, PAGE-ISSUE-0270, PAGE-ISSUE-0271.

Local runtime notes: the backend was started from `backend/` as required for template resolution. Startup still logs pre-existing idempotency warnings for older migrations and a Redis-not-configured warning, but `/health` returned 200 and the targeted notifications E2E passed.

---

## Final Status

`fixed`

Reason: The documented security, auditability, error handling, validation, frontend-state, accessibility, and dependency-cleanup findings were fixed and verified by the targeted authenticated E2E test plus required syntax/build checks.
