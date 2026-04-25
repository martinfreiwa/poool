# Page Audit: Community Comments

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/community/comments`
Template: `frontend/platform/admin/community/comments.html`
JavaScript: inline Alpine component in `frontend/platform/admin/community/comments.html`; shared `frontend/platform/static/js/admin-permission-guard.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/community/routes.rs`

---

## Summary

The page is implemented enough to load a global comments table, filter the in-memory result set, hide comments, and permanently delete comments through backend routes. It is not production-ready because destructive moderation routes only require the broad `AdminUser` role, do not use a fine-grained community moderation permission, and execute the comment mutation separately from the audit log. Several frontend states also hide failures, so an operator can see an empty table or local success state when the backend did not perform the expected moderation action.

Fix update: the documented code findings were fixed on 2026-04-25. The page remains `needs_recheck` only because authenticated browser/E2E verification with a disposable comment fixture still needs to be run.

---

## Fix Update - 2026-04-25

Fixed:

- Enforced `community.view` / `community.manage` permissions on global comment list, hide, delete, and pin routes.
- Made hide, delete, and pin comment moderation transactional with `community_audit_logs`.
- Added audit details for target user, previous state, reason, post context, and content preview where applicable.
- Missing comment IDs now return `404 Not Found` instead of success.
- Comment list `limit` is bounded to the server maximum.
- Replaced CDN Alpine/HTMX use on this page with self-hosted vanilla JS.
- Added visible API error/retry state so failed loads no longer render as "No comments found."
- Added labels for toolbar controls and `rel="noopener noreferrer"` for context links.

Still needs recheck:

- Authenticated admin browser pass with a disposable comment fixture.
- DB verification for hide/delete/pin state and audit log rows.
- Console, keyboard, and responsive smoke on the rendered page.

---

## Tested Scope

- Static review of `frontend/platform/admin/community/comments.html`.
- Static review of shared admin CSRF/permission wrapper in `frontend/platform/static/js/admin-permission-guard.js`.
- Static review of sidebar community links in `frontend/platform/static/js/admin-sidebar-loader.js`.
- Static review of `GET /admin/community/comments` registration in `backend/src/admin/mod.rs`.
- Static review of `GET /api/admin/community/comments`, `POST /api/admin/community/comments/:id/hide`, `DELETE /api/admin/community/comments/:id`, and related pin route in `backend/src/community/routes.rs`.
- Static review of `comments` and `community_audit_logs` schema migrations.
- Unauthenticated local curl smoke against the already-running backend on `http://localhost:8888`.
- Existing targeted Rust community tests.

Authenticated browser testing and safe destructive mutation testing were not performed because no authenticated admin test session and disposable comment fixture were available in this audit run.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/comments` | Admin page route, also available as `/admin/community/comments.html`. |
| Template | `frontend/platform/admin/community/comments.html` | Inline Alpine component owns all page behavior. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Adds CSRF header to mutating fetch calls and hides mapped admin nav items. |
| Shared JS | `frontend/platform/static/js/admin-sidebar-loader.js` | Contains `nav-com-comments`; not mapped by permission guard. |
| CSS | `frontend/platform/static/css/admin.css` | Admin layout and table/button primitives. |
| CSS | `frontend/platform/static/css/bundle.css` | Shared bundle. |
| CSS | `frontend/platform/static/css/fonts.css` | Font loading. |
| Backend page route | `GET /admin/community/comments` | Registered in `backend/src/admin/mod.rs`, handled by `page_admin_generic`. |
| Backend API route | `GET /api/admin/community/comments` | Returns latest comments with author display names. |
| Backend API route | `POST /api/admin/community/comments/:id/hide` | Sets `comments.is_hidden = true`. |
| Backend API route | `DELETE /api/admin/community/comments/:id` | Permanently deletes a comment. |
| Backend API route | `POST /api/admin/community/comments/:id/pin` | Used by post detail page, adjacent comment moderation route. |
| Database table | `comments` | `database/community/002_comments.sql`, plus `is_pinned` in `013_moderation.sql`. |
| Database table | `community_audit_logs` | `database/community/018_community_audit_log.sql`. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb: Admin | `frontend/platform/admin/community/comments.html:30` | Navigate to admin home. | Link. | `GET /admin/` expected. | Not browser-tested. |
| Admin breadcrumb: Community | `frontend/platform/admin/community/comments.html:32` | Navigate to community admin overview. | Link. | `GET /admin/community/` registered. | Not browser-tested. |
| Refresh button | `frontend/platform/admin/community/comments.html:48` | Reload latest comments, disable while loading. | `@click="loadComments()"`, `:disabled="loading"`. | `GET /api/admin/community/comments?limit=200`. | Inline JS syntax passed; unauth API requires auth. |
| Search input | `frontend/platform/admin/community/comments.html:61` | Filter by author or content in loaded comments. | `x-model="searchQuery"`, computed filter. | No backend search. | Static review only; lacks explicit label. |
| Status filter | `frontend/platform/admin/community/comments.html:66` | Filter visible or hidden comments. | `x-model="statusFilter"`, computed filter. | No backend filter. | Static review only; lacks explicit label. |
| Results count | `frontend/platform/admin/community/comments.html:75` | Show filtered and total loaded comments. | `x-text` bindings. | Depends on loaded API result. | Static review only. |
| Loading state | `frontend/platform/admin/community/comments.html:85` | Show spinner while loading. | `x-show="loading"`. | N/A. | Static review only. |
| Comments table | `frontend/platform/admin/community/comments.html:91` | Render comments when loaded. | `x-for="c in filteredComments"`. | `GET /api/admin/community/comments`. | Static review only. |
| Author cell | `frontend/platform/admin/community/comments.html:105` | Show initials, name, and user ID. | `x-text` bindings. | Uses `author_name`, `user_id`. | Static review only. |
| Content cell | `frontend/platform/admin/community/comments.html:117` | Show truncated sanitized comment. | `x-text`; XSS-safe rendering. | API returns sanitized content fallback. | Static review only. |
| Date cell | `frontend/platform/admin/community/comments.html:120` | Format created date/time. | `new Date(c.created_at)`. | API returns timestamp. | Static review only. |
| Helpful count | `frontend/platform/admin/community/comments.html:124` | Show helpful count. | `x-text`. | API returns `helpful_count`. | Static review only. |
| Status badge | `frontend/platform/admin/community/comments.html:130` | Show Visible or Hidden. | `x-text`, `:style`. | API returns `is_hidden`; hide mutation updates local state. | Static review only. |
| Context link | `frontend/platform/admin/community/comments.html:134` | Open public post context in new tab. | Dynamic `:href`, `target="_blank"`. | `GET /community/post/:id` registered in `backend/src/main.rs`. | Static review only; missing `rel="noopener"`. |
| Hide button | `frontend/platform/admin/community/comments.html:138` | Confirm, POST hide, mark row hidden. | `@click="hideComment(c)"`. | `POST /api/admin/community/comments/:id/hide`. | Unauth mutation returned CSRF 403; authenticated mutation not run. |
| Delete button | `frontend/platform/admin/community/comments.html:142` | Confirm, DELETE comment, remove row locally. | `@click="deleteComment(c)"`. | `DELETE /api/admin/community/comments/:id`. | Unauth mutation returned CSRF 403; authenticated mutation not run. |
| Clear Filters button | `frontend/platform/admin/community/comments.html:156` | Reset search and status filters. | Inline Alpine assignment. | No backend needed. | Static review only. |
| No filtered results empty state | `frontend/platform/admin/community/comments.html:153` | Show when filters remove all loaded rows. | `x-show`. | No backend needed. | Static review only. |
| No comments empty state | `frontend/platform/admin/community/comments.html:160` | Show when API returns no rows. | `x-show`. | Depends on API. | Also shown after failed load because failures are swallowed. |

---

## Frontend Findings

### P2 - Load failures are silently rendered as an empty comments table

Location:

- Template/JS: `frontend/platform/admin/community/comments.html:206`

Problem:

`loadComments()` only assigns comments when `res.ok` is true and only logs caught exceptions. Non-OK responses, bad JSON, network failures, and backend 5xx responses clear `loading` and leave `comments` as `[]`, which renders "No comments found." This makes an operational outage look like a legitimate empty moderation queue.

Expected:

The page should keep an explicit `error` state, show a retryable error banner/message, and avoid presenting failed loads as verified empty data.

Evidence:

Static review of lines 206-218. Unauthenticated `GET /api/admin/community/comments?limit=1` returned `401 Authentication required`; the current component would silently display the global empty state if this happened inside a browser session without redirect handling.

Recommended fix:

Track `errorMessage`, throw on `!res.ok`, show an error state above the table, and keep previous comments until a successful refresh replaces them.

### P3 - Form controls lack explicit accessible labels

Location:

- Template: `frontend/platform/admin/community/comments.html:61`
- Template: `frontend/platform/admin/community/comments.html:66`

Problem:

The search input and status select rely on placeholder/option text and visual context rather than persistent labels or `aria-label`. This is weaker for screen-reader and voice-control users.

Expected:

Add visible labels or `aria-label="Search comments"` and `aria-label="Filter comments by status"`.

Evidence:

Static template review found no `label`, `aria-label`, or `aria-labelledby` attributes for either control.

Recommended fix:

Add labels using the admin form pattern or concise `aria-label` attributes if visible labels would disrupt the toolbar.

### P3 - New-tab context links omit noopener protection

Location:

- Template: `frontend/platform/admin/community/comments.html:134`

Problem:

The Context link opens a new tab with `target="_blank"` but does not set `rel="noopener noreferrer"`.

Expected:

New-tab links should include `rel="noopener noreferrer"`.

Evidence:

Static template review.

Recommended fix:

Add `rel="noopener noreferrer"` to the context anchor.

### P2 - Admin page depends on third-party CDN scripts

Location:

- Template: `frontend/platform/admin/community/comments.html:11`
- Template: `frontend/platform/admin/community/comments.html:12`

Problem:

The page loads HTMX from `unpkg.com` and Alpine from `cdn.jsdelivr.net`. This creates an availability and supply-chain dependency for an admin moderation page, and there is no local fallback.

Expected:

Admin production pages should use self-hosted, pinned static assets or a documented integrity/fallback strategy.

Evidence:

Static template review; CSP currently permits both CDN hosts.

Recommended fix:

Self-host HTMX/Alpine under `frontend/platform/static/js/vendor/` or add a pinned integrity/fallback policy consistently across admin pages.

---

## Backend Findings

### P1 - Comment moderation routes lack fine-grained community permissions

Location:

- Backend: `backend/src/community/routes.rs:1615`
- Backend: `backend/src/community/routes.rs:1661`
- Backend: `backend/src/community/routes.rs:1688`
- Shared JS: `frontend/platform/static/js/admin-permission-guard.js:38`
- Sidebar: `frontend/platform/static/js/admin-sidebar-loader.js:93`

Problem:

Global comment listing, hide, delete, and adjacent pin routes require only `AdminUser`, which accepts any active `admin` or `super_admin` role. The codebase has `AdminUser::require_permission`, but no community comment route uses it. The sidebar permission guard also has no `nav-com-*` entries, so community admin navigation is not hidden by permission.

Expected:

Read access should require a community read/moderation permission, and hide/delete/pin should require a stronger community moderation permission. The sidebar guard should map `nav-com-comments` and related community pages to the same permission model.

Evidence:

Static review of route handler signatures and `PAGE_PERMISSION_MAP`. `rg` found no `community.*` permissions in migrations or permission guard mappings.

Recommended fix:

Introduce explicit permissions such as `community.view` and `community.moderate`, grant them to intended admin roles, call `admin.require_permission(&state.db, "...")` in each route, and add `nav-com-comments` to `PAGE_PERMISSION_MAP`.

### P1 - Destructive comment mutations are not atomic with audit logging

Location:

- Backend: `backend/src/community/routes.rs:1669`
- Backend: `backend/src/community/routes.rs:1695`
- Backend: `backend/src/community/audit.rs:10`

Problem:

Hide and delete execute the moderation mutation first, then call `community::audit::log`, whose implementation is fire-and-forget and intentionally does not fail the caller if audit insertion fails. Permanent delete can therefore succeed without an audit record, and the audit record omits `target_user_id`, previous state, and the frontend-provided hide reason.

Expected:

Sensitive moderation mutations should be transactional with the audit record, especially destructive permanent deletion. The audit details should include the reason, prior hidden state, comment author, and enough context for compliance review.

Evidence:

Static review of `admin_hide_comment`, `admin_delete_comment`, and `community/audit.rs`.

Recommended fix:

Use a community DB transaction, fetch the target comment with row lock, write the mutation and `community_audit_logs` row in the same transaction, and fail the request if audit logging fails.

### P2 - Hide and delete return success for missing comment IDs

Location:

- Backend: `backend/src/community/routes.rs:1669`
- Backend: `backend/src/community/routes.rs:1695`

Problem:

Both handlers call `.execute()` but never inspect `rows_affected()`. A valid UUID that matches no comment returns `{"success": true}` and the frontend updates local UI as if moderation succeeded.

Expected:

Missing comments should return `404 Not Found` and should not write misleading audit records or mutate local UI.

Evidence:

Static review of the update/delete handlers.

Recommended fix:

Check `rows_affected() == 1`; return `AppError::NotFound` or equivalent for zero rows.

### P2 - Comment list limit is unbounded and silently defaults on invalid input

Location:

- Backend: `backend/src/community/routes.rs:1620`

Problem:

The `limit` query param is parsed with `.unwrap_or(200)` and passed directly to SQL. Invalid values silently fall back to 200, while very large positive values can request an oversized moderation payload.

Expected:

Reject invalid limits or clamp to a safe maximum. Document the maximum in the API contract.

Evidence:

Static review of `admin_get_comments`.

Recommended fix:

Parse explicitly, clamp to a bounded range such as `1..=200`, and return a safe validation error for invalid values if the platform standard prefers rejection over clamping.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Page auth smoke | `curl -i http://localhost:8888/admin/community/comments` | Unauthenticated request rejected. | `401 Unauthorized`, JSON `Authentication required`, security headers present. | Pass |
| API auth smoke | `curl -i 'http://localhost:8888/api/admin/community/comments?limit=1'` | Unauthenticated API request rejected. | `401 Unauthorized`, JSON `Authentication required`, security headers present. | Pass |
| Hide CSRF smoke | `curl -i -X POST /api/admin/community/comments/000.../hide` without CSRF/session | Request rejected before mutation. | `403 Forbidden`, CSRF error JSON. | Pass |
| Delete CSRF smoke | `curl -i -X DELETE /api/admin/community/comments/000...` without CSRF/session | Request rejected before mutation. | `403 Forbidden`, CSRF error JSON. | Pass |
| Inline Alpine syntax | Extract inline script and run `node --check` | No syntax errors. | Passed. | Pass |
| Authenticated list render | Log in as admin and load page. | Comments render or explicit empty/error state. | Not run; no admin browser session available. | Not run |
| Authenticated hide/delete | Use disposable comment fixture and verify DB + audit row. | Mutation and audit are atomic. | Not run; static review found blockers. | Not run |
| Mobile/keyboard smoke | Test toolbar, table actions, confirm dialogs. | Keyboard reachable and no overflow. | Not run. | Not run |

---

## Security Findings

- P1: Comment moderation routes lack fine-grained community moderation permissions; any broad admin role can list, hide, delete, or pin comments.
- P1: Hide/delete mutations are not atomic with audit logs, and audit details omit reason, target user, and prior state.
- P2: Permanent delete is available from the list with only a native confirm and no backend state machine for soft delete, retention, or reversible moderation.
- P3: Context new-tab links omit `rel="noopener noreferrer"`.
- P2: Admin page relies on third-party CDN scripts without local fallback or integrity controls.
- Positive: Visible comment content is rendered with `x-text`, and backend list returns `content_sanitized` when available.
- Positive: Unauthenticated page/API requests are rejected, and unauthenticated mutating curl requests fail CSRF validation.

---

## Database Findings

- `comments` exists with `id`, `post_id`, `user_id`, `content`, `content_sanitized`, `helpful_count`, `is_hidden`, and `created_at`; `is_pinned` is added by `013_moderation.sql`.
- `community_audit_logs` exists with actor, action, entity, target, details, and indexes.
- Hide does not persist the provided reason because `comments` has no `hidden_reason` column and the audit call passes no details.
- Delete hard-removes comment rows through `DELETE FROM comments`, which limits later moderation review unless audit logging is made durable and complete.
- The mutation and audit inserts do not share a transaction.

---

## Missing Tests

- Backend authorization tests proving non-community-moderator admins cannot list, hide, delete, or pin comments once fine-grained permissions exist.
- Backend tests for `404` on missing comment hide/delete/pin IDs.
- Backend tests proving hide/delete mutations and audit rows commit or roll back atomically.
- Backend tests that the hide reason, target user, and previous state are recorded in `community_audit_logs.details`.
- Frontend/browser test for API load failure rendering an error state rather than "No comments found."
- E2E test with a disposable comment fixture covering list, filter, hide, delete confirmation, DB state, and audit log state.

---

## Recommended Fix Order

1. Add `community.view` / `community.moderate` permissions, enforce them on list/hide/delete/pin routes, and map `nav-com-comments` in `admin-permission-guard.js`.
2. Make hide/delete transactional with durable audit logging and complete audit details.
3. Return `404` when hide/delete/pin target no rows, and clamp or validate `limit`.
4. Add visible frontend error state for failed list loads and failed mutations.
5. Add accessible labels and `rel="noopener noreferrer"` to the toolbar/context link.
6. Replace CDN script loading with self-hosted pinned admin assets or a documented fallback.

---

## Final Status

`needs_recheck`

Reason: The page has implemented read and moderation routes, but destructive moderation authorization, audit durability, missing-row handling, frontend error states, and authenticated end-to-end coverage need fixes and re-verification before this page should be marked completed.
