# Page Audit: Community Circle Detail

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/community/circle-detail`
Template: `frontend/platform/admin/community/circle-detail.html`
JavaScript: inline script in `frontend/platform/admin/community/circle-detail.html`; shared `frontend/platform/static/js/admin-permission-guard.js`, `frontend/platform/static/js/admin-theme.js`, `frontend/platform/static/js/user-data.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/community/routes.rs`, `backend/src/community/circles.rs`

---

## Summary

The page can render the admin shell and has matching backend routes for circle detail loading and the visible actions. The destructive actions are not production-ready: the inline fetch calls omit CSRF headers, the backend allows any broad admin role to view and mutate circles, and force-transfer can create inconsistent circle ownership for arbitrary UUIDs. The page should remain `needs_recheck`.

Follow-up fix applied 2026-04-25: CSRF headers, community RBAC gates, force-transfer target/profile consistency checks, update field validation, and accessible status regions were added locally. Authenticated browser/API recheck is still recommended before closing the page.

---

## Tested Scope

- Reviewed `docs/automation-prompts/DAILY_PAGE_AUDIT_PROMPT.md`, `docs/automation-prompts/PRODUCTION_READINESS_STANDARDS.md`, `AGENTS.md`, `docs/AGENT_DEVELOPMENT_PROMPT.md`, `docs/IMPLEMENTATION_ROADMAP.md`, `docs/issue-tracking/BROKEN_LOGICS.md`, `docs/DATABASE_SCHEMA.md`, `docs/design/FRONTEND_COMPONENTS.md`, `docs/TECH_STACK.md`, `docs/SECURITY.md`, and `docs/DESIGN.md`.
- Static-reviewed the page template, inline JavaScript, admin page routing, community admin API routes, circle service functions, community circle migrations, and current tracker records.
- Ran inline JavaScript syntax check and `cargo check`.
- Did not execute mutating admin actions because this was a documentation-only audit and no safe authenticated admin fixture was established for destructive circle operations.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/circle-detail` | Clean admin route registered. |
| URL alias | `/admin/community/circle-detail.html` | Linked from circles list. |
| Template | `frontend/platform/admin/community/circle-detail.html` | Page shell plus all page logic inline. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Loaded, but page actions do not enforce page-specific permissions. |
| Shared JS | `frontend/platform/static/js/admin-theme.js` | Theme support. |
| Shared JS | `frontend/platform/static/js/user-data.js` | User/session display support. |
| CSS | `frontend/platform/static/css/admin.css` | Admin shell/components. |
| Backend page route | `GET /admin/community/circle-detail` | `page_admin_generic`; requires broad `AdminUser`. |
| Backend API route | `GET /api/admin/community/circles/:id` | Returns `{ circle, members }`. |
| Backend API route | `PUT /api/admin/community/circles/:id` | Updates name/description/avatar/public flag. |
| Backend API route | `POST /api/admin/community/circles/:id/transfer` | Force transfers ownership. |
| Backend API route | `DELETE /api/admin/community/circles/:id/members/:user_id` | Removes non-owner member. |
| Backend API route | `DELETE /api/admin/community/circles/:id` | Deletes circle. |
| Database table | `circles` | Circle metadata and ownership. |
| Database table | `circle_members` | Membership roles. |
| Database table | `community_profiles` | User community profile and circle pointer. |
| Database table | `circle_invites` | Deleted on circle deletion. |
| Database table | `circle_join_requests` | Cascades on circle deletion. |
| Database table | `community_audit_logs` | Best-effort audit writes after successful mutations. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Admin | `.admin-breadcrumbs a[href="/admin/"]` | Navigate to admin dashboard. | Yes, link. | Yes. | Not runtime-tested. |
| Breadcrumb Community | `.admin-breadcrumbs a[href="/admin/community/"]` | Navigate to community admin overview. | Yes, link. | Yes. | Not runtime-tested. |
| Breadcrumb Circles | `.admin-breadcrumbs a[href="/admin/community/circles.html"]` | Navigate to circles list. | Yes, link. | Yes. | Not runtime-tested. |
| Loading state | `#circle-detail-container` initial HTML | Show loading until API returns. | Yes. | Requires GET API. | Static-only verified. |
| Missing ID error | DOMContentLoaded branch when `id` absent | Show "No circle ID specified." | Yes. | No backend needed. | Static-only verified. |
| Circle detail loader | `loadCircleDetail(circleId)` | Fetch detail JSON and render page. | Yes. | `GET /api/admin/community/circles/:id`. | Route exists; not runtime-tested with fixture. |
| Error state | `loadCircleDetail` non-OK/catch | Show visible error. | Basic visible text. | Yes. | Static-only verified. |
| Disband Circle | button calling `deleteCircle(circleId)` | Confirm and delete circle. | Calls DELETE without CSRF. | Route exists. | Expected to fail CSRF in normal app. |
| Name field | `#edit-name` | Edit circle name. | Yes. | PUT route accepts optional `name`. | Mutating path expected to fail CSRF. |
| Emoji field | `#edit-emoji` | Edit avatar emoji. | Yes. | PUT route accepts optional `avatar_emoji`. | Mutating path expected to fail CSRF. |
| Visibility select | `#edit-visibility` | Toggle public/private. | Yes. | PUT route accepts optional `is_public`. | Mutating path expected to fail CSRF. |
| Description textarea | `#edit-desc` | Edit description. | Yes. | PUT route accepts optional `description`. | Mutating path expected to fail CSRF. |
| Save Changes | form `onsubmit="updateCircle(...)"` | Persist settings and refresh. | Calls PUT without CSRF. | Route exists. | Expected to fail CSRF in normal app. |
| Transfer User ID | `#transfer-user-id` | Enter new owner UUID. | Required text input. | POST expects UUID JSON. | Backend accepts arbitrary UUID shape. |
| Transfer | form `onsubmit="forceTransfer(...)"` | Confirm and transfer ownership. | Calls POST without CSRF. | Route exists. | Expected to fail CSRF; backend validation gap remains. |
| Stats cards | rendered in `renderDetail` | Show member count, XP, level. | Yes, data-driven. | GET route returns circle. | Could throw if response fields missing/null. |
| Members table | `.admin-table` in `membersHtml` | Show user ID, role, joined timestamp. | Yes. | GET route returns members. | Static-only verified. |
| Kick member | row button `onclick="kickMember(...)"` | Confirm and remove non-owner member. | Calls DELETE without CSRF. | Route exists. | Expected to fail CSRF in normal app. |
| Empty members state | `members.length === 0` | Show "No members found." | Yes. | GET route returns members. | Static-only verified. |

---

## Frontend Findings

### P1 - Mutating circle actions omit CSRF token

Location:

- Template: `frontend/platform/admin/community/circle-detail.html`
- JS: inline `updateCircle`, `forceTransfer`, `kickMember`, `deleteCircle`

Problem:

The page sends PUT/POST/DELETE requests with JSON headers only. Global CSRF middleware requires `X-CSRF-Token`, body token, or query token for mutating methods. Save, transfer, kick, and disband are therefore wired to real routes but should receive 403 CSRF errors in a normal session.

Expected:

Admin mutating fetches should read the CSRF token from the existing platform mechanism and send `X-CSRF-Token`, while preserving useful user-facing error messages.

Evidence:

`backend/src/auth/csrf.rs` enforces CSRF on POST/PUT/DELETE/PATCH. The inline fetch calls do not set `X-CSRF-Token`.

Recommended fix:

Use the shared CSRF helper pattern used by other admin pages or add one, then include the token on all state-changing circle calls.

### P2 - Inline HTML rendering makes state and error handling fragile

Location:

- Template: `frontend/platform/admin/community/circle-detail.html`
- JS: `renderDetail`

Problem:

The entire detail view is built through `innerHTML` with inline event handlers. Circle text fields are escaped, but the pattern makes future additions easy to get wrong, does not disable submit buttons while requests are in flight, and surfaces backend failures through generic `alert()` messages.

Expected:

Use DOM construction or a constrained renderer for data rows, attach listeners with `addEventListener`, disable mutating buttons during requests, and render structured inline errors.

Evidence:

`renderDetail` writes one large template string to `#circle-detail-container`; mutation handlers use `alert()` and do not parse backend JSON errors.

Recommended fix:

Move the inline script into a page-specific JS file and replace inline handlers with event delegation plus visible status regions.

### P3 - Dynamic status messages are not announced accessibly

Location:

- Template: `frontend/platform/admin/community/circle-detail.html`

Problem:

Loading, missing-ID, not-found, success, and failure states are plain injected text or blocking alerts without an `aria-live` status region. Keyboard users receive native confirms/alerts, but the page itself does not expose a stable announced state after API results.

Expected:

Add a visible status/error region with `role="status"` or `role="alert"` and focus management after failed loads or successful mutations.

Evidence:

`#circle-detail-container` is replaced with plain `<div>` error text and mutation feedback is `alert()`.

Recommended fix:

Add reusable admin alert/status markup and focus it after load/mutation failures.

---

## Backend Findings

### P1 - Circle admin page and APIs lack fine-grained community permission checks

Location:

- Page route: `backend/src/admin/pages.rs`
- API routes: `backend/src/community/routes.rs`

Problem:

The page route and all circle admin APIs require only `AdminUser`, which means any active `admin` or `super_admin` role can view, update, transfer, remove members from, or delete circles. Other admin areas have started using `require_permission`, but no `community.manage` or equivalent permission exists here.

Expected:

Read actions should require a community read/moderation permission, and destructive actions should require a stronger community management permission. Super admins can still inherit the permission through RBAC.

Evidence:

Handlers accept `AdminUser` but never call `admin.require_permission(...)`. The permissions inventory does not include community-specific admin permissions.

Recommended fix:

Create/grant community permissions, gate `/admin/community/*` pages and `/api/admin/community/*` APIs, and add authorization tests for read and destructive circle actions.

### P1 - Force transfer can create inconsistent ownership for arbitrary UUIDs

Location:

- Route: `POST /api/admin/community/circles/:id/transfer`
- Service: `backend/src/community/circles.rs`

Problem:

`admin_force_transfer_circle` accepts any UUID-shaped `new_owner_id`. If the UUID is not already a member, it inserts a `circle_members` owner row and updates `community_profiles`, but it does not verify that the user exists in the core DB or that a community profile row exists. Because `circle_members.user_id` has no cross-DB foreign key, the circle can end up owned by an ID that is not a real/community-profiled user, and the profile update can silently affect zero rows.

Expected:

The backend should verify the target user exists, ensure or create the target community profile inside the transaction, and fail if the target cannot be made consistent.

Evidence:

`circle_members.user_id` is a UUID without a FK in `database/community/008_circles_xp.sql`; `admin_force_transfer_circle` inserts the row and does not check `community_profiles` update row count.

Recommended fix:

Validate target user against the core user table or an approved bridge, upsert `community_profiles`, and assert affected rows before committing.

### P2 - Circle update relies on database errors instead of server validation

Location:

- Route: `PUT /api/admin/community/circles/:id`
- Service: `backend/src/community/circles.rs`

Problem:

Admin circle updates pass optional name, description, and emoji directly into SQL. The database constrains some fields, but the API does not return field-level validation for empty names, overlong names, overlong emoji strings, or descriptions above 500 characters.

Expected:

Backend should enforce the same semantic limits as the schema and return client-safe 400 responses that the page can display inline.

Evidence:

`AdminUpdateCircleReq` has raw optional strings and `admin_force_update_circle` executes `UPDATE circles SET ... RETURNING *` with no validation.

Recommended fix:

Add validation before SQL and document the limits in the page UI.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static route mapping | Compared template fetch paths with Axum route registration. | Every frontend API path has a backend route. | GET, PUT, transfer POST, member DELETE, and circle DELETE routes exist. | Pass |
| Inline JS syntax | Extracted inline script to `/tmp/circle-detail-inline.js`; ran `node --check`. | No syntax errors. | Passed. | Pass |
| Backend compile | Ran `cargo check` in `backend/`. | Backend compiles. | Passed with existing warnings. | Pass |
| CSRF contract review | Compared mutating fetch calls with global CSRF middleware. | Mutating calls include CSRF token. | No mutating call sends CSRF token. | Fail |
| Authz review | Checked page/API handlers for permission enforcement. | Community actions require fine-grained permission. | Only broad `AdminUser` is enforced. | Fail |
| Browser/admin fixture test | Load real page with admin session and circle fixture. | Page loads and actions behave. | Not run; no safe destructive fixture/session established. | Not run |

---

## Security Findings

- P1: CSRF headers are missing from all mutating circle actions, so production CSRF middleware should block them.
- P1: Circle read and destructive admin actions are gated only by broad admin role, not a community-specific permission.
- P1: Force-transfer can assign ownership to arbitrary UUID-shaped IDs without verifying real user/profile existence.
- P2: Best-effort audit logs are written after mutations, but audit write failures do not fail sensitive admin mutations.
- P2: Destructive actions use native confirms but no typed confirmation or reason capture for disband/force-transfer.

---

## Database Findings

- `circles`, `circle_members`, `circle_invites`, `circle_join_requests`, token-gate columns, and `community_audit_logs` exist in community migrations.
- Admin delete is transactional for `community_profiles`, `circle_members`, `circle_invites`, and `circles`; `circle_join_requests` depends on `ON DELETE CASCADE`.
- Admin remove member is transactional and protects owner removal.
- Force-transfer is transactional but does not verify target profile/user existence or affected row counts.
- Circle text constraints exist partly in SQL, but server-side validation is incomplete.

---

## Missing Tests

- Authorization tests for `/admin/community/circle-detail` and all `/api/admin/community/circles*` routes covering admin without community permission, community moderator, and super admin.
- CSRF integration tests for PUT/POST/DELETE circle admin endpoints.
- Force-transfer data integrity tests for non-existent user UUID, user without community profile, existing member, current owner, and full circle.
- Frontend/browser tests for missing ID, not found, successful load, save error, transfer error, kick error, and delete redirect.
- Accessibility smoke for keyboard navigation, status announcement, and mobile layout.

---

## Recommended Fix Order

1. Add CSRF headers to all mutating page fetches and show backend error messages inline.
2. Introduce and enforce community admin permissions for page access and circle read/mutation APIs.
3. Harden force-transfer with target user/profile validation and affected-row checks.
4. Add server-side field validation for circle update payloads.
5. Move inline page logic into a page-specific JS file with event listeners, request locking, and accessible status regions.

---

## Final Status

`needs_recheck`

Reason: The audited findings have local fixes, but the destructive workflows still need an authenticated circle fixture recheck for load, save, transfer, kick, delete, permission denial, and audit-log behavior.
