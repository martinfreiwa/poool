# Page Audit: Community Users

Date: 2026-04-27; fixed 2026-04-28; verification coverage added 2026-04-28
Status: fixed, authenticated E2E coverage added
Auditor: ChatGPT/Codex
Page URL: `/admin/community/users`
Template: `frontend/platform/admin/community/users.html`
JavaScript: `frontend/platform/static/js/admin-community-users.js`; shared `frontend/platform/static/js/admin-permission-guard.js`, `frontend/platform/static/js/admin-theme.js`, `frontend/platform/static/js/user-data.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/community/routes.rs`

---

## Summary

The Community Users page has a registered admin page route, a real list API, and a real ban/unban API. The 2026-04-28 fix pass resolved the documented implementation issues: unsafe row rendering, missing API-level community permissions, missing route-level CSRF, non-transactional ban audit logging, weak moderation states, and unused CDN dependencies. A follow-up 2026-04-28 pass added authenticated API/browser/database E2E coverage for the documented verification gaps.

---

## Tested Scope

- Static review of `frontend/platform/admin/community/users.html`.
- Static review of page registration in `backend/src/admin/mod.rs`.
- Static review of generic admin page gating in `backend/src/admin/pages.rs`.
- Static review of `GET /api/admin/community/users` and `POST /api/admin/community/users/:id/ban` in `backend/src/community/routes.rs`.
- Static review of schema support in `database/community/005_community_profiles.sql`, `database/community/013_moderation.sql`, `database/community/014_shadowban.sql`, and `database/community/018_community_audit_log.sql`.
- Existing tracker evidence from 2026-04-25 unauthenticated smoke was considered.
- 2026-04-28 fix review covered `frontend/platform/static/js/admin-community-users.js`, the updated page template, and `backend/src/community/routes.rs`.
- `node --check frontend/platform/static/js/admin-community-users.js` passed.
- `python3 -m pytest tests/admin/test_admin_community_users_static.py -q` passed.
- `python3 -m py_compile tests/e2e/test_admin_community_users.py` passed.
- `cargo fmt` was run, and `cd backend && cargo fmt --check` passed.
- `CARGO_TARGET_DIR=/tmp/poool-community-users-check cargo check` passed.
- Local authenticated E2E execution requires the backend to be running on `localhost:8888`; the backend was not running at the start of the verification pass.
- `cd backend && cargo run` was attempted to start the backend, but local build output failed with `No space left on device` while writing Rust incremental artifacts.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/users` | Clean admin route registered. |
| URL alias | `/admin/community/users.html` | Sidebar route alias. |
| Template | `frontend/platform/admin/community/users.html` | Page shell, table, status region, and moderation dialog. |
| JS | `frontend/platform/static/js/admin-community-users.js` | Dedicated safe-rendering controller for list and ban/unban flow. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Global admin helper remains loaded; page controller also sends CSRF explicitly. |
| Backend page route | `GET /admin/community/users` | `page_admin_generic`; currently requires admin plus `community.view` or `community.manage` for community paths. |
| Backend API route | `GET /api/admin/community/users` | Lists community profiles joined to core user info. |
| Backend API route | `POST /api/admin/community/users/:id/ban` | Sets `is_community_banned` and `ban_reason`. |
| Database table | `community_profiles` | User moderation state, warning count, post count, and timestamps. |
| Database table | `community_audit_logs` | Intended audit log for ban/unban actions. |
| Core database | `users`, `user_profiles` via `user_bridge` | Source of display name and avatar URL. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Admin | `a[href="/admin/"]` | Navigate to admin dashboard. | Link only. | Yes. | Not browser-clicked. |
| Breadcrumb Community | `a[href="/admin/community/"]` | Navigate to community overview. | Link only. | Yes. | Not browser-clicked. |
| Refresh button | `#refresh-users-btn` | Reload table data with disabled pending state. | Yes. | Yes, list API exists. | Fixed; authenticated browser click still needs E2E. |
| Initial loading row | `#users-table` initial row | Show "Loading users..." before fetch returns. | Yes. | N/A. | Fixed. |
| Empty state row | empty list branch | Show no-user message. | Yes via DOM APIs. | Yes. | Fixed. |
| Error row | non-OK and catch branches | Show failed/network error. | Yes via DOM APIs and status region. | N/A. | Fixed. |
| User avatar | `buildAvatar()` | Show safe avatar image or initials. | Yes, with safe URL assignment and alt/fallback handling. | Yes. | Fixed. |
| User detail link | generated detail link | Navigate to detail page. | Link only; ID encoded. | Yes. | Fixed. |
| Post count | `${u.post_count}` | Show denormalized community post count. | Yes. | Yes. | Static verified. |
| Warning count badge | `${u.warning_count}` | Show warning count. | Yes. | Yes. | Static verified. |
| Status badge | `u.is_community_banned` branch | Show Active or Banned. | Yes. | Yes. | Static verified. |
| Joined Community date | `new Date(u.created_at).toLocaleDateString()` | Show profile creation date. | Yes. | Yes. | Static verified; invalid dates are not handled. |
| View button | row action link | Navigate to detail page. | Link only. | Yes. | Static verified. |
| Ban button | row action button | Open labelled reason dialog, POST ban with CSRF, reload list. | Yes. | Yes. | Fixed; authenticated DB/audit E2E remains. |
| Unban button | row action button | Open confirmation dialog, POST unban with CSRF, reload list. | Yes. | Yes. | Fixed; authenticated DB/audit E2E remains. |
| Moderation dialog | `#community-user-ban-dialog` | Collect required ban reason, show server errors, disable submit while pending. | Yes. | Yes. | Fixed; keyboard/mobile smoke remains. |

---

## Frontend Findings

### Fixed P1 - Users table injected unescaped profile data

Location:

- Template: `frontend/platform/admin/community/users.html:102-146`

Problem:

The table is assembled as one HTML string and assigned with `tbody.innerHTML`. It interpolates `u.avatar_url`, `u.display_name`, `u.user_id`, `post_count`, `warning_count`, and `created_at` directly into HTML attributes and text. Display names and avatar URLs come from user/profile data, so a malicious value can become admin-only DOM injection.

Expected:

Build rows with DOM APIs, assign text through `textContent`, validate image URLs before assigning `src`, add useful `alt` text or mark decorative initials appropriately, and URL-encode IDs in links.

Evidence:

Static review found the existing `PAGE-ISSUE-0141` condition at `frontend/platform/admin/community/users.html:114-146`; the page previously had four `innerHTML` assignments.

Recommended fix:

Fixed on 2026-04-28 by moving the controller to `frontend/platform/static/js/admin-community-users.js` and using DOM APIs, `textContent`, encoded links, and safe avatar URL assignment.

### Fixed P2 - Ban workflow used native dialogs and weak visible states

Location:

- Template: `frontend/platform/admin/community/users.html:49`
- Template: `frontend/platform/admin/community/users.html:156-179`

Problem:

Refresh and ban/unban actions do not disable controls while requests are pending, and the moderation flow uses `prompt()`, `confirm()`, and `alert()`. Server error details are hidden behind generic messages, and focus/status feedback is not accessible.

Expected:

Use an accessible confirmation modal or inline row action state with labelled reason input, required validation, disabled pending controls, visible success/error status, and server error text.

Evidence:

Static scan found one `prompt()`, one `confirm()`, and two `alert()` calls. The refresh button always remains clickable and calls `loadUsers()` without pending-state protection.

Recommended fix:

Fixed on 2026-04-28 with a labelled native dialog, visible live status regions, disabled pending controls, and surfaced server error text.

### Fixed P3 - Unused CDN dependencies increased page fragility

Location:

- Template: `frontend/platform/admin/community/users.html:11-12`

Problem:

The page loads external HTMX and Alpine scripts, but this page does not use `hx-*` attributes or Alpine directives. This adds third-party network dependency and CSP/supply-chain surface without page value.

Expected:

Remove unused CDN scripts or self-host required shared libraries consistently with the admin platform policy.

Evidence:

Static review found no `hx-`, `x-`, or Alpine component usage in the page body/script.

Recommended fix:

Fixed on 2026-04-28 by removing the unused HTMX and Alpine CDN script tags.

---

## Backend Findings

### Fixed P1 - Community users APIs lacked granular community permissions

Location:

- Backend: `backend/src/community/routes.rs:1548-1551`
- Backend: `backend/src/community/routes.rs:1609-1614`

Problem:

Both list and ban endpoints extract only `AdminUser`. `AdminUser` verifies broad `admin` / `super_admin` role membership, but it does not enforce `community.view` for listing or `community.manage` for ban/unban. This is inconsistent with newer community admin handlers that call `require_community_view_or_manage()` or `require_community_manage()`.

Expected:

`GET /api/admin/community/users` should call `require_community_view_or_manage(&state, &admin)`. `POST /api/admin/community/users/:id/ban` should call `require_community_manage(&state, &admin)`.

Evidence:

The helper functions exist at `backend/src/community/routes.rs:75-101` and are used by other community admin handlers, but not by these two endpoints. The page route itself is now gated by `page_admin_generic` at `backend/src/admin/pages.rs:189-194`, so the API contract is weaker than the page shell.

Recommended fix:

Fixed on 2026-04-28. `GET /api/admin/community/users` calls `require_community_view_or_manage`, and `POST /api/admin/community/users/:id/ban` calls `require_community_manage`.

### Fixed P1 - Ban/unban POST had no route-level CSRF validation

Location:

- Template: `frontend/platform/admin/community/users.html:166-171`
- Backend: `backend/src/community/routes.rs:1609-1640`

Problem:

The frontend POST does not set `X-CSRF-Token`, and the backend handler does not call `require_csrf_header`. The page may rely on a global fetch wrapper from `admin-permission-guard.js`, but a sensitive moderation endpoint should fail closed at the handler level.

Expected:

The handler should accept `headers: HeaderMap` and `jar: CookieJar`, call `require_csrf_header(&headers, &jar)?`, and tests should verify missing-token rejection.

Evidence:

`require_csrf_header` is defined at `backend/src/community/routes.rs:58-72`. The ban handler parameters do not include headers or cookie jar and do not call the helper.

Recommended fix:

Fixed on 2026-04-28. The handler calls `require_csrf_header(&headers, &jar)?`, and the page controller sends `X-CSRF-Token`.

### Fixed P1 - Ban update and audit log were not atomic

Location:

- Backend: `backend/src/community/routes.rs:1617-1638`
- Backend: `backend/src/community/audit.rs:10-41`

Problem:

The handler updates `community_profiles`, then calls the fire-and-forget audit logger. If the audit insert fails, the API still returns success after changing moderation state. The update also does not verify `rows_affected`, so banning a nonexistent community profile can return success and still attempt an audit row.

Expected:

Lock the target profile, update it and insert the audit log in one transaction, check exactly one affected row, and fail the request if audit persistence fails.

Evidence:

`community::audit::log` intentionally logs errors without failing callers. A stronger transactional moderation pattern already exists for report actions in `backend/src/community/service.rs:731-768`.

Recommended fix:

Fixed on 2026-04-28. The handler locks the target profile with `FOR UPDATE`, updates state, records previous/new state in `community_audit_logs`, and commits both in one transaction.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static route registration | Inspect `backend/src/admin/mod.rs` and `backend/src/admin/pages.rs`. | Clean and `.html` page routes exist and require admin/community page access. | Routes exist; generic community page gate checks `community.view` or `community.manage`. | Pass |
| Static API registration | Inspect `backend/src/community/routes.rs`. | List and ban APIs exist. | `GET /api/admin/community/users` and `POST /api/admin/community/users/:id/ban` are registered. | Pass |
| Inline JavaScript syntax | `node --check <(sed -n '86,180p' frontend/platform/admin/community/users.html)` | Valid JavaScript. | Passed. | Pass |
| Safe rendering review | Inspect table render code. | User data rendered through safe DOM APIs. | Dynamic rows use raw `innerHTML`. | Fail |
| Permission contract review | Compare page gate and API handlers. | Page/API enforce `community.view` / `community.manage`. | Page gate does; APIs only require broad admin. | Fail |
| CSRF contract review | Inspect ban POST and handler. | State-changing request is rejected without CSRF token. | Handler has no route-level CSRF validation. | Fail |
| Authenticated browser workflow | Load page with seeded admin, click refresh/view/ban/unban. | No console/network errors; states and DB/audit rows verified. | Not run; no authenticated fixture/session used in this documentation-only run. | Not run |
| Fixed JS syntax | `node --check frontend/platform/static/js/admin-community-users.js` | Valid JavaScript. | Passed. | Pass |
| Static regression tests | `python3 -m pytest tests/admin/test_admin_community_users_static.py -q` | Controller and backend contracts are statically enforced. | 2 passed. | Pass |
| Authenticated API permissions and CSRF coverage | `tests/e2e/test_admin_community_users.py::test_admin_community_users_api_permissions_csrf_and_audit` | List denial without community permission, ban denial for view-only admin, missing-CSRF rejection, missing-user/blank-reason failures, and transactional ban/unban audit persistence are covered. | Test added; local execution blocked because the backend was not running and `cargo run` hit local disk exhaustion. | Added, not executed locally |
| Authenticated browser rendering and dialog coverage | `tests/e2e/test_admin_community_users.py::test_admin_community_users_browser_rendering_detail_link_and_dialog` | Page load, refresh, safe malicious display/avatar rendering, detail link, keyboard dialog close, ban submit, DB state, and audit row are covered. | Test added; local execution blocked because the backend was not running and `cargo run` hit local disk exhaustion. | Added, not executed locally |
| E2E test syntax | `python3 -m py_compile tests/e2e/test_admin_community_users.py` | Test module parses. | Passed. | Pass |
| Rust format check | `cd backend && cargo fmt --check` | No formatting diffs. | Passed after `cargo fmt`. | Pass |
| Rust compile check | `CARGO_TARGET_DIR=/tmp/poool-community-users-check cargo check` | Backend compiles. | Passed. | Pass |

---

## Security Findings

- Fixed: Dynamic table rendering no longer injects profile-controlled values into admin DOM.
- Fixed: `GET /api/admin/community/users` enforces `community.view` or `community.manage`.
- Fixed: `POST /api/admin/community/users/:id/ban` enforces `community.manage`.
- Fixed: `POST /api/admin/community/users/:id/ban` enforces route-level CSRF.
- Fixed: Ban/unban moderation state and audit logging now commit atomically.
- Fixed: Ban reasons are server-validated for non-empty content and 1000-character maximum.

---

## Database Findings

- `community_profiles` supports the rendered fields and ban state.
- `community_audit_logs` exists with useful actor/target indexes.
- Fixed: Direct ban/unban uses a transaction, row lock, affected-row check, and transactional audit insert.
- The page displays denormalized `post_count` and `warning_count`; no runtime database consistency check was performed in this audit.

---

## Missing Tests

- Fixed: Added authenticated Playwright/API coverage for loading `/admin/community/users`, refreshing rows, opening a detail link, and checking for critical console errors.
- Fixed: Added authenticated API coverage for list denial without `community.view` / `community.manage`.
- Fixed: Added authenticated API coverage for ban denial without `community.manage`.
- Fixed: Added authenticated CSRF rejection coverage for ban/unban without `X-CSRF-Token`.
- Fixed: Added database-backed E2E coverage proving ban/unban changes profile state and writes durable `community_audit_logs` rows with previous/new state details.
- Fixed: Added browser XSS regression coverage with malicious display name and avatar URL values rendered as text or blocked from image `src`.
- Fixed: Added keyboard dialog coverage for focus and Escape close on the replacement moderation confirmation UI.

---

## Recommended Fix Order

1. Free local disk space for Rust build artifacts, then run `python3 -m pytest tests/e2e/test_admin_community_users.py -q` with the backend available on `localhost:8888`.
2. If the E2E run finds environment-specific fixture drift, update only the test fixture setup/cleanup needed for this page.

---

## Final Status

`fixed, authenticated E2E coverage added`

Reason: All documented implementation issues for this page are fixed in code, and the documented verification gaps now have targeted authenticated API/browser/database E2E coverage in `tests/e2e/test_admin_community_users.py`.
