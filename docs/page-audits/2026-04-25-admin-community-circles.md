# Page Audit: Admin Community Circles

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/community/circles`
Template: `frontend/platform/admin/community/circles.html`
JavaScript: `frontend/platform/static/js/admin-community-circles.js`; shared `frontend/platform/static/js/admin-permission-guard.js`, `frontend/platform/static/js/admin-theme.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/community/routes.rs`, `backend/src/community/circles.rs`

---

## Summary

The Admin Community Circles page was audited and then fixed in the local working tree. It now uses self-hosted vanilla JavaScript, requires `community.manage` for the page and list/delete APIs, writes circle disband audit logs inside the delete transaction, shows retryable API errors, and has bounded search/filter pagination plus accessible async status and delete dialog behavior.

The page still needs authenticated browser recheck with a safe admin/circle fixture before closing the E2E review category.

---

## Fix Applied

Fixed on 2026-04-25:

- `PAGE-ISSUE-0110`: Added `community.manage` checks to `/admin/community/circles`, `/admin/community/circles.html`, `GET /api/admin/community/circles`, and `DELETE /api/admin/community/circles/:id`.
- `PAGE-ISSUE-0111`: Moved destructive delete audit logging into the same community DB transaction and included cleanup metadata.
- `PAGE-ISSUE-0112`: Added visible retryable API error state instead of false empty state.
- `PAGE-ISSUE-0113`: Removed CDN HTMX/Alpine from this page and moved behavior to `frontend/platform/static/js/admin-community-circles.js`.
- `PAGE-ISSUE-0114`: Added aria-live status, explicit destructive labels, and a focus-managed delete dialog.
- `PAGE-ISSUE-0115`: Added server-side `limit`, `offset`, `search`, and `visibility` support plus frontend search/filter/pagination controls.

---

## Tested Scope

- Static template review of `frontend/platform/admin/community/circles.html`.
- Inline Alpine component review for `loadCircles()` and `confirmDelete()`.
- Shared admin JS review for CSRF fetch interception and permission guard behavior.
- Backend route registration review in `backend/src/admin/mod.rs` and `backend/src/community/routes.rs`.
- Backend service review in `backend/src/community/circles.rs`.
- Community DB table/column verification with `psql -d poool_community`.
- Unauthenticated runtime smoke on alternate local port `8893`.
- Syntax/compile checks with Node and Cargo.

Destructive admin actions were not executed because no safe authenticated admin fixture was available for this documentation-only audit.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/circles` | Registered as admin generic page route. |
| URL alias | `/admin/community/circles.html` | Sidebar and detail back-link use `.html` alias. |
| Template | `frontend/platform/admin/community/circles.html` | Inline Alpine component owns all page behavior. |
| JS | `frontend/platform/static/js/admin-community-circles.js` | Self-hosted page controller for list, filters, pagination, errors, and delete dialog. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Installs global CSRF fetch interceptor and includes `nav-com-circles` permission mapping. |
| Shared JS | `frontend/platform/static/js/admin-theme.js` | Theme/notification helper only. |
| Backend page route | `GET /admin/community/circles` | `backend/src/admin/mod.rs`, `page_admin_community_circles`; requires `community.manage`. |
| Backend API route | `GET /api/admin/community/circles` | Lists bounded circles; supports `search`, `visibility`, `limit`, `offset`; requires `community.manage`. |
| Backend API route | `DELETE /api/admin/community/circles/:id` | Disbands/deletes circle with transactional audit log; requires `community.manage`. |
| Backend API route | `GET /api/admin/community/circles/:id` | Used by detail page, not directly by this list page. |
| Backend API route | `PUT /api/admin/community/circles/:id` | Used by detail page, not directly by this list page. |
| Backend API route | `POST /api/admin/community/circles/:id/transfer` | Used by detail page, not directly by this list page. |
| Backend API route | `DELETE /api/admin/community/circles/:id/members/:user_id` | Used by detail page, not directly by this list page. |
| Database table | `circles` | Verified in `poool_community`; expected columns exist. |
| Database table | `circle_members` | Used during deletion cleanup. |
| Database table | `circle_invites` | Used during deletion cleanup. |
| Database table | `circle_join_requests` | FK cascades on circle delete. |
| Database table | `community_profiles` | Unlinked when circle is deleted. |
| Database table | `community_audit_logs` | Used for best-effort audit logging after delete. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Admin link | `a[href="/admin/"]`, lines 29-34 | Navigate to admin dashboard. | Native link | Yes | Not authenticated-browser tested. |
| Breadcrumb Community link | `a[href="/admin/community/"]`, lines 29-34 | Navigate to community admin index. | Native link | Yes | Not authenticated-browser tested. |
| Refresh button | `button @click="loadCircles()"`, lines 48-51 | Reload circle data and stats. | Yes | `GET /api/admin/community/circles` | Static verified; authenticated runtime not executed. |
| Total Circles stat | `x-text="circles.length"`, line 59 | Show fetched circle count. | Yes | Depends on list API | Static verified. |
| Avg Members stat | `x-text="avgMembers"`, line 63 | Average `member_count`. | Yes | Depends on list API | Static verified. |
| Total Circle XP stat | `x-text="totalXP.toLocaleString()"`, line 67 | Sum `total_xp`. | Yes | Depends on list API | Static verified. |
| Loading state | `x-show="loading"`, lines 76-79 | Show while fetch is in progress. | Yes | No | Static verified; no aria-live. |
| Circles table | `.admin-table`, lines 81-130 | Show circle rows when data exists. | Yes | List API | Static verified. |
| Circle row name/id | `x-text="circle.name"`, `x-text="circle.id"`, lines 98-101 | Render escaped row data. | Yes | List API returns `Circle` | Uses `x-text`, XSS-safe. |
| Level fields | `x-text="'Lvl ' + circle.level"`, `circle.level_name`, lines 105-108 | Render level metadata. | Yes | Community DB columns exist. | Static/DB verified. |
| Members count | `x-text="circle.member_count + ' / ' + circle.max_members"`, line 110 | Render capacity. | Yes | Community DB columns exist. | Static/DB verified. |
| Total XP cell | `x-text="circle.total_xp.toLocaleString()"`, line 112 | Render formatted XP. | Yes | Community DB columns exist. | Static/DB verified. |
| Created cell | `new Date(circle.created_at).toLocaleDateString()`, line 113 | Render local date. | Yes | Community DB column exists. | No invalid-date fallback. |
| Visibility badge | `x-text="circle.is_public ? 'Public' : 'Private'"`, line 115 | Render public/private state. | Yes | Community DB column exists. | Static/DB verified. |
| View action | `:href="'/admin/community/circle-detail.html?id=' + circle.id"`, line 119 | Open detail page for circle. | Native Alpine href | Detail route exists | Static verified. |
| Disband action | `button @click="confirmDelete(circle)"`, lines 122-124 | Confirm and DELETE circle. | Yes | `DELETE /api/admin/community/circles/:id` | Not executed with auth; unauthenticated DELETE rejected by CSRF. |
| Empty state | `x-show="!loading && circles.length === 0"`, lines 132-136 | Show when no circles. | Yes | No | Also shown after fetch failure, causing ambiguity. |

---

## Frontend Findings

### P2 - Circle list API failures render as a false empty state

Location:

- Template: `frontend/platform/admin/community/circles.html:164`
- JS: inline `loadCircles()`

Problem:

`loadCircles()` only updates `circles` when `res.ok` is true and only logs caught exceptions. Non-2xx responses, JSON parse failures, and network errors leave `circles` as `[]`, then the page shows "No circles found."

Expected:

The page should distinguish loading, empty, and error states. Failed admin API calls should show a visible retryable error and avoid implying that there are no circles.

Evidence:

Static review of lines 164-176; unauthenticated `GET /api/admin/community/circles` returns `401 {"error":"Authentication required"}`, which this component would silently treat as empty if it occurred in-page.

Recommended fix:

Add `error` state, handle non-2xx responses by parsing `{error}` when possible, show a visible `role="alert"` message with Retry, and clear error only after a successful fetch.

### P2 - Page depends on third-party CDN scripts for core admin behavior

Location:

- Template: `frontend/platform/admin/community/circles.html:11`
- Template: `frontend/platform/admin/community/circles.html:12`

Problem:

HTMX and Alpine are loaded from external CDNs, and Alpine is required for the page to load data or disband a circle. HTMX is not used by this page at all.

Expected:

Production admin pages should self-host required runtime scripts or remove unused libraries. Core admin controls should not depend on third-party CDN availability.

Evidence:

Static template review found external `https://unpkg.com/htmx.org@1.9.10` and `https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js`; no `hx-*` attributes are present on the page.

Recommended fix:

Remove HTMX from this page if unused and serve Alpine from the platform static bundle, or move the page behavior to a self-hosted vanilla JS file.

### P3 - Async status and destructive controls need stronger accessibility affordances

Location:

- Template: `frontend/platform/admin/community/circles.html:76`
- Template: `frontend/platform/admin/community/circles.html:122`
- JS: inline `confirmDelete()`

Problem:

Loading, empty, success, and failure states are not announced via stable `aria-live`/`role="alert"` regions. The disband button is icon-only with a `title`, and destructive confirmation/success/failure use native `confirm()`/`alert()` rather than the admin modal/status pattern.

Expected:

Async status should be announced accessibly. Icon-only destructive buttons should have explicit `aria-label`, and destructive actions should use a focus-managed confirmation modal or shared confirmation component.

Evidence:

Static template review of lines 76-79, 122-124, and 179-198.

Recommended fix:

Add `aria-live` status/error regions, `aria-label="Disband circle: {name}"`, and use a focus-managed admin confirmation dialog with visible loading/success/error states.

### P3 - No search, filtering, or pagination for an unbounded admin table

Location:

- Template: `frontend/platform/admin/community/circles.html:81`
- Backend: `backend/src/community/circles.rs:1091`

Problem:

The page fetches and renders every circle in one table. There are no search, visibility filters, sort controls, pagination controls, or server-side limit/offset parameters.

Expected:

Admin list pages should support at least search and pagination once the dataset is not trivially small.

Evidence:

Static review found `SELECT * FROM circles ORDER BY created_at DESC` with no limit and no table controls beyond Refresh.

Recommended fix:

Add query parameters for search, visibility, sort, limit, and offset, then add matching admin table controls and empty/error states per filter.

---

## Backend Findings

### P1 - Admin circle routes lack fine-grained community permission checks

Location:

- Backend route: `backend/src/community/routes.rs:3636`
- Backend route: `backend/src/community/routes.rs:3645`
- Shared guard: `frontend/platform/static/js/admin-permission-guard.js`

Problem:

`GET /api/admin/community/circles` and `DELETE /api/admin/community/circles/:id` require only `AdminUser`, which checks broad `admin` or `super_admin` role membership. The page does not enforce a specific `community.read`, `community.manage`, or moderation permission before listing or disbanding circles.

Expected:

Circle list should require a community read/moderation permission; disband should require a higher-risk community manage/moderation permission. UI visibility must be backed by server-side authorization.

Evidence:

Handlers accept `AdminUser` but do not call `admin.require_permission(...)`. `PAGE_PERMISSION_MAP` in `admin-permission-guard.js` has no community circle nav/action permission mapping.

Recommended fix:

Create or use existing community admin permissions, add server-side `require_permission` calls to list/detail/update/delete/member/transfer routes, and align sidebar/page action visibility to those permissions.

### P1 - Destructive circle deletion can commit without durable audit logging

Location:

- Backend route: `backend/src/community/routes.rs:3645`
- Backend service: `backend/src/community/circles.rs:1099`

Problem:

`admin_delete_circle()` commits the destructive database transaction, then `admin_delete_circle` route calls `community::audit::log(...).await` and ignores the result. If audit logging fails, the circle is still deleted with no durable record.

Expected:

Sensitive admin moderation actions should be auditable. The audit log should be committed atomically with the mutation, or the route should fail/compensate if logging cannot be written.

Evidence:

Deletion is transactional in `circles.rs`, but audit logging happens afterward in `routes.rs` and its result is not propagated.

Recommended fix:

Either write the audit record inside the same community DB transaction or make audit logging failure fail the request before commit. Include actor, target circle, previous state, and cleanup counts in metadata.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Backend compile | `cd backend && cargo check` | Backend compiles. | Passed with 29 existing warnings. | Pass |
| Page JS syntax | `node --check frontend/platform/static/js/admin-community-circles.js` | Script parses. | Passed. | Pass |
| CDN dependency scan | `rg "unpkg.com/htmx|cdn.jsdelivr.net/npm/alpine|x-data|x-text|@click|Alpine" frontend/platform/admin/community/circles.html frontend/platform/static/js/admin-community-circles.js` | No page-local CDN/Alpine dependencies remain. | No matches. | Pass |
| Unauthenticated page protection | `curl -i http://127.0.0.1:8893/admin/community/circles` | Anonymous user blocked. | `401 Unauthorized` JSON. | Pass |
| Unauthenticated list API protection | `curl -i http://127.0.0.1:8893/api/admin/community/circles` | Anonymous user blocked. | `401 Unauthorized` JSON. | Pass |
| CSRF protection on DELETE | `curl -i -X DELETE http://127.0.0.1:8893/api/admin/community/circles/00000000-0000-0000-0000-000000000000` | Missing CSRF blocked. | `403 Forbidden` with CSRF error. | Pass |
| Community schema check | `psql -d poool_community` information_schema queries | Required circle tables/columns exist. | `circles`, `circle_members`, `circle_invites`, `circle_join_requests`, `community_profiles`, and `community_audit_logs` exist; expected circle columns exist. | Pass |
| Authenticated list/delete browser test | Real admin session, load page, click Refresh/View/Disband with safe fixture. | Data loads, actions behave and log correctly. | Not run; no safe authenticated admin fixture for destructive flow. | Not run |

---

## Security Findings

- Fixed P1: Circle list/delete page and APIs now require `community.manage`.
- Fixed P1: Circle deletion audit logging now happens inside the same delete transaction.
- Fixed P2: This page no longer depends on CDN Alpine or unused HTMX.
- No direct XSS issue found in the row renderer because user/server values are rendered with `x-text`, not raw HTML.
- CSRF middleware rejected an unauthenticated DELETE without a token. The page relies on `admin-permission-guard.js` global fetch interception to attach `X-CSRF-Token` for authenticated destructive calls.

---

## Database Findings

- `poool_community.circles` has the fields expected by `Circle`: `avatar_emoji`, `level`, `level_name`, `is_public`, `max_members`, token-gate fields, and timestamps.
- `circle_members`, `circle_invites`, `circle_join_requests`, `community_profiles`, and `community_audit_logs` exist.
- Fixed: circle listing now uses bounded `LIMIT/OFFSET` with optional search and visibility filter.
- Fixed: `admin_delete_circle()` deletes/unlinks `community_profiles`, `circle_members`, `circle_invites`, and `circles` in one transaction and writes cleanup counts plus prior circle state to `community_audit_logs`. `circle_join_requests` remains covered by FK cascade.

---

## Missing Tests

- E2E test for authenticated `/admin/community/circles` page load with at least one circle fixture.
- E2E test for list API failure showing a visible error instead of empty state.
- Authorization tests proving non-community admins cannot list/disband circles once fine-grained permissions are added.
- CSRF test for authenticated `DELETE /api/admin/community/circles/:id`.
- Integration test that disband deletes/unlinks all expected circle records and writes an audit log atomically.
- Accessibility/browser test for keyboard focus, icon-button accessible names, and mobile table behavior.

---

## Recommended Fix Order

1. Run authenticated browser recheck with a safe admin/circle fixture.
2. Add automated E2E/integration coverage for list, permission denial, failed API state, and audited disband.
3. Consider extending the same community permission/audit pattern to adjacent admin circle detail actions.

---

## Final Status

`needs_recheck`

Reason: The documented code findings were fixed locally, but authenticated browser and destructive-flow verification still need a safe fixture.
