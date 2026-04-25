# Page Audit: Admin Community Leaderboard

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/community/leaderboard`
Template: `frontend/platform/admin/community/leaderboard.html`
JavaScript: `frontend/platform/static/js/admin-community-leaderboard.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/community/routes.rs`, `backend/src/community/xp.rs`

---

## Summary

The Admin Community Leaderboard page is route-registered, renders through the generic protected admin page handler, and its read-only leaderboard API is implemented against `community_profiles`.

Fix update: the documented issues were fixed locally after the audit. The read API now enforces community permissions; XP adjustment now requires `community.manage`, validates target users and payloads, writes ledger/profile/level/audit changes in one transaction, and blocks negative XP totals. The page now uses local vanilla JavaScript with visible load/mutation errors and accessible modal behavior.

Final status remains `needs_recheck` until authenticated browser/API fixture testing verifies read, grant, revoke, audit-log, CSRF, keyboard, and mobile behavior.

---

## Tested Scope

- Static template and JavaScript review of `frontend/platform/admin/community/leaderboard.html` and `frontend/platform/static/js/admin-community-leaderboard.js`.
- Static backend route review of `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/community/routes.rs`, and `backend/src/community/xp.rs`.
- Database migration review for `community_profiles`, `xp_ledger`, `xp_levels`, `circles`, and community admin permissions.
- Existing test search for admin community leaderboard and XP adjustment coverage.
- Runtime unauthenticated HTTP smoke against local `:8888` for page and API auth behavior.
- Inline JavaScript syntax check by extracting the original Alpine controller and running `node --check`.
- Post-fix checks: `node --check frontend/platform/static/js/admin-community-leaderboard.js`, `cargo fmt --check`, `cargo check`, CDN/Alpine/alert scan, tracker regeneration, and scoped `git diff --check`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/leaderboard` | Registered to `page_admin_generic`. |
| URL alias | `/admin/community/leaderboard.html` | Registered to `page_admin_generic`. |
| Template | `frontend/platform/admin/community/leaderboard.html` | Contains page structure and modal markup. |
| JS | `frontend/platform/static/js/admin-community-leaderboard.js` | Local vanilla controller for loading, modal behavior, and XP submit. |
| Component | `frontend/platform/admin/components/sidebar.html` | Included by the template. |
| Shared JS | `frontend/platform/static/js/admin-theme.js` | Loaded globally. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Adds CSRF headers to mutating fetch calls and hides sidebar links by permission. |
| Backend page route | `GET /admin/community/leaderboard` | Generic admin renderer with community.view/community.manage check for community pages. |
| Backend API route | `GET /api/admin/community/leaderboard?limit=100` | Returns leaderboard rows from `community_profiles`; now enforces community view/manage permission. |
| Backend API route | `POST /api/admin/community/users/:id/xp` | Applies validated transactional manual XP adjustment. |
| Database table | `community_profiles` | Source of `xp_total`, `level`, `level_name`, `circle_id`, and `login_streak`; updated by XP adjustment. |
| Database table | `xp_ledger` | Append-only XP entries; admin adjustment writes here. |
| Database table | `xp_levels` | Reference table for level recalculation. |
| Database table | `community_audit_logs` | Admin XP adjustment now writes a required audit row inside the transaction. |
| Database migration | `database/084_community_xp_nonnegative.sql` | Adds non-negative XP constraint for the root migration path. |
| Database migration | `database/community/026_community_xp_nonnegative.sql` | Adds non-negative XP constraint for the community migration path. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb: Admin | `a[href="/admin/"]` | Navigate to admin dashboard. | Link only. | `GET /admin/` exists. | Static verified. |
| Admin breadcrumb: Community | `a[href="/admin/community/"]` | Navigate to community overview. | Link only. | `GET /admin/community/` exists. | Static verified. |
| Refresh button | `#refresh-leaderboard-btn` | Reload leaderboard rows. | Yes. | `GET /api/admin/community/leaderboard` exists and is permission-gated. | Syntax verified; authenticated behavior unverified. |
| Adjust XP button | `#open-xp-modal-btn` | Open empty XP adjustment modal. | Yes. | POST API exists and is permission-gated. | Static verified. |
| Leaderboard loading state | `#leaderboard-loading` | Show loading message while fetch is in flight. | Yes. | Not backend-dependent. | Static verified. |
| Leaderboard error state | `#leaderboard-error` | Show retryable API/network failures. | Yes. | Depends on API failure shape. | Static verified. |
| Leaderboard table | `#leaderboard-table`, `#leaderboard-body` | Render rank, user ID, level, XP total, streak, action button. | Yes. | GET API returns matching fields. | Authenticated runtime unverified. |
| User detail link | Generated by local JS | Navigate to user detail page for the row. | Yes. | Page route exists. | Static verified. |
| Manage XP row button | Generated by local JS | Open modal prefilled with row user ID. | Yes. | POST API exists and is permission-gated. | Static verified. |
| Empty state | `#leaderboard-empty` | Show only after successful empty response. | Yes. | Depends on GET API response. | Static verified. |
| Modal overlay | `#xp-modal-overlay` | Display XP adjustment modal. | Yes. | Not backend-dependent. | Static verified. |
| Modal close button | `#close-xp-modal-btn` | Close modal. | Yes. | Not backend-dependent. | Accessible label and focus behavior added. |
| User ID input | `#xp-user-id` | Capture target UUID. | Yes, validates UUID shape. | Backend verifies target core user exists. | Static verified. |
| XP amount input | `#xp-amount` | Capture positive amount from 1 to 10000, then sign based on action. | Yes. | Backend enforces same limit/sign. | Static verified. |
| Action select | `#xp-action` | Choose grant or revoke. | Yes. | Backend allows only `admin_grant`/`admin_revoke`. | Static verified. |
| Description input | `#xp-description` | Capture required ledger description up to 200 chars. | Yes. | Backend enforces required max 200 chars. | Static verified. |
| Cancel button | `#cancel-xp-modal-btn` | Close modal without mutation. | Yes. | Not backend-dependent. | Static verified. |
| Submit XP Adjustment button | `#submit-xp-btn` | POST adjustment, show status, reload list. | Yes. | POST API is permission-gated, validated, transactional, and audited. | Needs authenticated fixture recheck. |

---

## Frontend Findings

### P2 - Leaderboard API failures are rendered as an empty leaderboard

Status: fixed locally; authenticated browser recheck still recommended.

Location:

- Template/JS: `frontend/platform/admin/community/leaderboard.html`, inline `loadLeaderboard()`

Problem:

`loadLeaderboard()` only updates `entries` when `res.ok` is true and only logs exceptions to the console. A 403, 500, network failure, or community DB outage leaves `entries` empty and `loading` false, so the operator sees "No leaderboard entries found" instead of an operational failure.

Expected:

Show a visible retryable error state for non-OK responses and fetch exceptions, and reserve the empty state for confirmed successful empty data.

Evidence:

The inline controller checks `if (res.ok) { ... }`, catches errors with `console.error(e)`, and has no error state variable.

Recommended fix:

Add `error` state, render a retry row/card, disable the empty state while `error` is set, and include the HTTP/API message where safe.

### P2 - XP adjustment modal lacks baseline dialog accessibility

Status: fixed locally; keyboard/mobile browser recheck still recommended.

Location:

- Template: `frontend/platform/admin/community/leaderboard.html`, `.admin-modal-overlay` and `.admin-modal`

Problem:

The modal has no `role="dialog"`, `aria-modal`, `aria-labelledby`, focus placement, focus restoration, Escape-to-close handler, or Tab containment. The close icon button also lacks an accessible name.

Expected:

Admin modals should be keyboard usable and screen-reader identifiable, following the modal baseline already added to other community admin pages.

Evidence:

Static template review found only `x-show`, `x-transition`, `@click.away`, and click handlers. No dialog ARIA/focus hooks are present.

Recommended fix:

Add dialog attributes, label the close button, move focus into the modal on open, restore focus on close, support Escape, and contain Tab while open.

### P2 - XP adjustment feedback uses blocking alerts

Status: fixed locally; authenticated browser recheck still recommended.

Location:

- Template/JS: `frontend/platform/admin/community/leaderboard.html`, inline `submitXpAdjustment()`

Problem:

Validation failures, backend failures, unexpected errors, and success feedback are all reported with `alert()`. That gives operators no durable inline state, is hard to cover in automated browser tests, and is a weak accessibility pattern for a sensitive admin workflow.

Expected:

The modal should render inline field errors, a submit-level error/success region, and a consistent admin toast or status message where appropriate.

Evidence:

Static review found `alert()` calls for missing user ID, invalid amount, successful adjustment, backend error, and unexpected error.

Recommended fix:

Replace blocking alerts with accessible inline errors and a visible success/error status region tied to the submit button lifecycle.

### P3 - Admin page depends on external CDN scripts

Status: fixed locally.

Location:

- Template: `frontend/platform/admin/community/leaderboard.html`, `<head>`

Problem:

The page loads HTMX from `unpkg.com` even though it does not use HTMX attributes, and loads Alpine from `cdn.jsdelivr.net` for the page controller. This creates avoidable third-party script dependency for an admin surface.

Expected:

Remove unused HTMX. For Alpine, either self-host the vendor script like other locally controlled assets or move this small controller to plain local JavaScript.

Evidence:

Static review found no `hx-*` attributes. Alpine attributes are present and require the external Alpine CDN script.

Recommended fix:

Delete the unused HTMX include and replace CDN Alpine dependency with a local static script or a page-specific vanilla JS controller.

---

## Backend Findings

### P1 - XP adjustment endpoint lacks fine-grained community.manage authorization

Status: fixed locally; authenticated API recheck still recommended.

Location:

- Backend: `backend/src/community/routes.rs`, `admin_award_xp`
- Page gate: `backend/src/admin/pages.rs`, `page_admin_generic`

Problem:

`POST /api/admin/community/users/:id/xp` only extracts `AdminUser`; it does not call `require_community_manage`. This allows any broad admin/super_admin session accepted by `AdminUser` to mutate a user's XP ledger and profile, even though this is a sensitive moderation/admin action.

Expected:

XP adjustment should require `community.manage`, and the page/API contract should match the sidebar permission model.

Evidence:

Nearby admin community mutation handlers call `require_community_manage(&state, &admin).await?`; `admin_award_xp` does not.

Recommended fix:

Call `require_community_manage` at the start of `admin_award_xp`, add an authorization regression test, and verify unauthorized admins receive 403 without writes.

### P1 - XP adjustment is not transactional and audit failure is ignored

Status: fixed locally; authenticated DB/audit recheck still recommended.

Location:

- Backend: `backend/src/community/routes.rs`, `admin_award_xp`
- Backend: `backend/src/community/xp.rs`, `award_xp` and `update_user_level`

Problem:

Admin XP adjustment writes an XP ledger row, updates `community_profiles.xp_total`, recalculates level, and then calls community audit logging as separate operations. If any later write fails after an earlier write succeeds, the XP ledger/profile/level/audit state can diverge. The audit log result is also not checked, so the API can report success without a durable admin audit record.

Expected:

Manual admin XP adjustment should be one database transaction with a locked target profile row, ledger insert, profile update, level recalculation, and required audit log commit.

Evidence:

`award_xp` executes multiple statements directly on `PgPool`, `admin_award_xp` calls `update_user_level` again after `award_xp`, and `crate::community::audit::log(...).await;` drops the result.

Recommended fix:

Implement an admin-specific transactional XP adjustment service that uses `BEGIN`, `SELECT ... FOR UPDATE`, inserts `xp_ledger`, updates profile/level, writes the audit row, and commits only if all steps succeed.

### P1 - XP revocation can drive community XP totals below zero

Status: fixed locally; migration/application recheck still recommended.

Location:

- Frontend: `frontend/platform/admin/community/leaderboard.html`, inline `submitXpAdjustment()`
- Backend: `backend/src/community/xp.rs`, `award_xp`

Problem:

The frontend turns `admin_revoke` into a negative amount, and the backend accepts the signed `custom_amount` without checking the user's current XP. `community_profiles.xp_total = xp_total + $1` can become negative, while the ledger still records the negative adjustment.

Expected:

XP totals should have a clear lower bound, normally zero, and revocation should reject amounts greater than the user's current XP or clamp through an explicit business rule.

Evidence:

`award_xp` accepts `custom_amount`, inserts it into `xp_ledger`, and applies `xp_total = xp_total + $1`. Migrations shown in `database/community/008_circles_xp.sql` and `database/069_apply_missing_community_schema.sql` do not add a non-negative `xp_total` constraint.

Recommended fix:

Add backend validation and a database check constraint for `community_profiles.xp_total >= 0`, with a safe migration/backfill plan for existing rows.

### P2 - Leaderboard read API lacks explicit community.view/community.manage authorization

Status: fixed locally; authenticated API recheck still recommended.

Location:

- Backend: `backend/src/community/routes.rs`, `admin_get_leaderboard`
- Page gate: `backend/src/admin/pages.rs`, `page_admin_generic`

Problem:

`GET /api/admin/community/leaderboard` only requires `AdminUser` and does not call `require_community_view_or_manage`. The page itself is protected by `community.view` or `community.manage`, but the API should enforce the same contract because it exposes user IDs, XP totals, levels, circle IDs, and streaks.

Expected:

The read API should require `community.view` or `community.manage`.

Evidence:

`admin_get_leaderboard` names the extractor `_admin` and immediately fetches the community pool and leaderboard rows.

Recommended fix:

Rename the extractor to `admin`, call `require_community_view_or_manage(&state, &admin).await?`, and add a route-level permission test.

### P2 - XP adjustment payload is under-validated

Status: fixed locally; authenticated API recheck still recommended.

Location:

- Backend: `backend/src/community/routes.rs`, `AdminAwardXpReq` and `admin_award_xp`

Problem:

The backend accepts any `reason_label`, unbounded `description`, and any signed `i32` amount. The frontend normally sends `admin_grant` or `admin_revoke`, but backend trust in frontend options is not sufficient for a sensitive admin mutation.

Expected:

Backend validation should allow only `admin_grant` and `admin_revoke`, enforce sign/action consistency, cap the maximum absolute adjustment, trim and length-limit description, and verify the target user/profile exists before writing.

Evidence:

`AdminAwardXpReq` is plain deserialization and `admin_award_xp` passes values directly into `award_xp`.

Recommended fix:

Add a typed validator for manual XP adjustment requests and return 400 for invalid reason, amount, description, or missing target user.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Inline JS syntax | Extract inline `document.addEventListener('alpine:init', ...)` block to `/tmp/leaderboard-inline.js`; run `node --check /tmp/leaderboard-inline.js`. | Syntax passes. | Passed with no syntax output. | Pass |
| Fixed JS syntax | `node --check frontend/platform/static/js/admin-community-leaderboard.js`. | Syntax passes. | Passed. | Pass |
| Backend build | `cd backend && cargo fmt --check` and `cd backend && cargo check`. | Format/build pass. | Passed. | Pass |
| Removed CDN/alert patterns | `rg "unpkg|cdn.jsdelivr|x-data|x-show|x-for|@click|alert\\("` against the page/script. | No matches. | No matches. | Pass |
| Unauthenticated page request | `curl -i http://localhost:8888/admin/community/leaderboard`. | 401 or login redirect; no admin HTML exposed. | HTTP 401 JSON `{"error":"Authentication required"}` with security headers. | Pass |
| Unauthenticated API request | `curl -i 'http://localhost:8888/api/admin/community/leaderboard?limit=1000'`. | 401; no leaderboard data exposed. | HTTP 401 JSON `{"error":"Authentication required"}` with security headers. | Pass |
| Authenticated leaderboard load | Open page with admin session and observe table/network/console. | Page loads rows or visible empty/error state. | Not run in this documentation-only audit. | Not run |
| Authenticated XP adjustment | Submit safe fixture grant/revoke and verify `xp_ledger`, `community_profiles`, and audit row. | Atomic write, correct level, audit row, visible success/failure. | Not run because backend issues require fixes before safe mutation testing. | Blocked |
| Keyboard/mobile modal pass | Open modal, navigate by keyboard, test Escape/focus trap/mobile layout. | Modal is keyboard and mobile usable. | Not run; static review found missing accessibility wiring. | Needs recheck |

---

## Automated Test Coverage

- `node --check /tmp/leaderboard-inline.js` passed for the original extracted inline script syntax.
- `node --check frontend/platform/static/js/admin-community-leaderboard.js` passed after the fix.
- `cd backend && cargo fmt --check` passed.
- `cd backend && cargo check` passed.
- Scoped `git diff --check` passed for touched backend/frontend/migration files.
- `rg` found no dedicated tests for `/api/admin/community/leaderboard` or `/api/admin/community/users/:id/xp`.
- Existing public leaderboard tests cover `/leaderboard`, `/api/leaderboard`, `/api/leaderboard/me`, and user privacy settings, not the admin community XP management page.

Recommended tests:

- Admin with `community.view` can read `/api/admin/community/leaderboard`; admin without that permission receives 403.
- Only admin with `community.manage` can POST XP adjustment.
- POST without valid CSRF returns 403 and creates no ledger/profile/audit changes.
- Grant and revoke use one transaction and write ledger/profile/level/audit consistently.
- Revocation greater than current XP is rejected and leaves profile unchanged.
- Modal keyboard/focus behavior and visible API error state in browser E2E.

---

## Security and Data Integrity Notes

- The global CSRF middleware is mounted in `backend/src/main.rs`, and `admin-permission-guard.js` injects `X-CSRF-Token` for mutating fetch calls when the CSRF cookie exists. The route still needs explicit no-CSRF E2E coverage because this mutation changes user XP state.
- XP is not money, but it is user-facing reputation/state and may gate community features. Treat admin adjustments as sensitive audit-required mutations.
- User IDs, XP totals, circle IDs, and streaks are admin-only data. The API should not rely on broad admin role alone when the page permission model already defines community-specific permissions.

---

## Final Status

`needs_recheck`

Severity counts:

- P0: 0
- P1: 3
- P2: 5
- P3: 1

Fixed locally:

- Backend permission checks and transactional XP adjustment.
- Non-negative XP validation and database constraints.
- Frontend error state, inline submit feedback, local JS, and modal accessibility.

Remaining recheck:

- Authenticated browser/API tests verify read, grant, revoke, CSRF rejection, audit log, and mobile/keyboard behavior.
