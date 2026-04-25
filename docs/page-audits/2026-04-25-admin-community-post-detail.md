# Page Audit: Admin Community Post Detail

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/community/post-detail`
Template: `frontend/platform/admin/community/post-detail.html`
JavaScript: inline script in `frontend/platform/admin/community/post-detail.html`; shared `frontend/platform/static/js/admin-permission-guard.js`, `frontend/platform/static/js/admin-theme.js`, `frontend/platform/static/js/user-data.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/fonts.css`, `frontend/platform/static/css/bundle.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/community/routes.rs`

---

## Summary

The page route is registered and guarded for admin community access, and the page has real backend endpoints for loading a post, hiding a post, locking/unlocking a thread, editing tags, and pinning/unpinning comments. The audit found serious production-readiness gaps: several post detail APIs only require broad admin access instead of `community.view`/`community.manage`, moderation mutations are not transactional or reliably audited, stale post IDs can return success, and the inline renderer injects community/report fields with `innerHTML`.

Final status is `needs_recheck` after backend hardening, safe rendering, and authenticated browser verification.

---

## Tested Scope

- Reviewed `frontend/platform/admin/community/post-detail.html`, including all inline JavaScript and dynamic markup.
- Reviewed route registration in `backend/src/admin/mod.rs` and generic admin page permission checks in `backend/src/admin/pages.rs`.
- Reviewed admin post/comment APIs in `backend/src/community/routes.rs`.
- Reviewed community tables/migrations for `posts`, `comments`, `reactions`, `content_reports`, moderation fields, and `community_audit_logs`.
- Checked existing tests for page/API coverage.
- Ran inline JavaScript syntax extraction with Node.
- Attempted local HTTP smoke against `localhost:8888`; no server was running.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/post-detail?id=:post_id` | Clean URL and `.html` alias are registered. |
| Template | `frontend/platform/admin/community/post-detail.html` | Inline script renders the full page body from API JSON. |
| Component | `frontend/platform/admin/components/sidebar.html` | Included by the template. |
| JS | inline script | Owns load, render, hide, lock, pin, and tag actions. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Injects CSRF headers for mutating fetch calls when the `csrf_token` cookie exists. |
| CSS | `frontend/platform/static/css/admin.css` | Primary admin UI styles. |
| Backend page route | `GET /admin/community/post-detail` | `page_admin_generic`, with community permission redirect guard. |
| Backend API route | `GET /api/admin/community/posts/:id` | Loads post, comments, reactions, reports. |
| Backend API route | `POST /api/admin/community/posts/:id/hide` | Hides post with reason. |
| Backend API route | `POST /api/admin/community/posts/:id/lock` | Locks/unlocks thread. |
| Backend API route | `POST /api/admin/community/posts/:id/tags` | Replaces post content tags. |
| Backend API route | `POST /api/admin/community/comments/:id/pin` | Pins/unpins comment. |
| Database table | `posts` | Content, hidden state, lock state, tags. |
| Database table | `comments` | Comment content and pin state. |
| Database table | `reactions` | Reaction inventory. |
| Database table | `content_reports` | Report reason/status inventory. |
| Database table | `community_audit_logs` | Expected moderation audit trail. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `nav.admin-breadcrumbs a[href="/admin/"]` | Navigate to admin dashboard. | Link | Yes, page route expected. | Static verified only. |
| Community breadcrumb | `a[href="/admin/community/"]` | Navigate to community overview. | Link | Yes. | Static verified only. |
| Posts breadcrumb | `a[href="/admin/community/posts.html"]` | Navigate to posts list. | Link | Yes. | Static verified only. |
| Loading state | `#post-detail-container` initial child | Show loading until detail API resolves. | Yes | Requires detail API. | Static verified; no timeout/abort state. |
| Missing ID error | `DOMContentLoaded` no `id` branch | Show error when `id` query param missing. | Yes | No backend needed. | Static verified. |
| Post detail fetch | `loadPostDetail(postId)` | GET post, comments, reactions, reports. | Yes | Route exists. | Not runtime verified; server unavailable. |
| Post content card | `${p.content}` in `innerHTML` | Display sanitized content. | Yes | API returns `content_sanitized` fallback. | Unsafe rendering risk if sanitized field is absent or malformed. |
| Content tags | `${p.content_tags.map(...)}` | Display tags. | Yes | Tags route/table exist. | Unsafe rendering and no backend validation. |
| Edit Tags button | `onclick="editTags(...)"` | Prompt for comma-separated tags and POST replacement. | Yes | Route exists. | Backend lacks permission, validation, audit, and stale-row checks. |
| Lock/Unlock Thread button | `onclick="toggleLock(...)"` | Confirm and POST lock state. | Yes | Route exists. | Backend lacks permission, transaction, reliable audit, and stale-row checks. |
| Hide Post button | `onclick="hidePost(...)"` | Prompt reason and POST hide. | Yes | Route exists. | Backend lacks permission, validation, transaction, reliable audit, and stale-row checks. |
| Comments table | `commentsHtml` | Show comment author, content, date, pin action. | Yes | Detail API returns comments. | Unsafe rendering and no visible failure state beyond generic alert. |
| Pin/Unpin comment button | `onclick="togglePinComment(...)"` | POST comment pin state and reload. | Yes | Route exists with permission and transaction. | Static verified; no runtime fixture. |
| Reports table | `reportsHtml` | Show reporter, reason, status, date. | Yes | Detail API returns reports. | Report reason/status injected as HTML. |
| Reactions list | `reactionsHtml` | Show reaction type and author. | Yes | Detail API returns reactions. | Author/type injected as HTML. |
| External HTMX script | `https://unpkg.com/htmx.org@1.9.10` | Library load. | Not used by page script. | No backend needed. | External dependency on admin page. |
| External Alpine script | `https://cdn.jsdelivr.net/.../alpinejs...` | Library load. | Not used by page script. | No backend needed. | External dependency on admin page. |

---

## Frontend Findings

### P1 - Dynamic admin renderer injects community data with `innerHTML`

Location:

- Template: `frontend/platform/admin/community/post-detail.html:99`
- Template: `frontend/platform/admin/community/post-detail.html:120`
- Template: `frontend/platform/admin/community/post-detail.html:136`
- Template: `frontend/platform/admin/community/post-detail.html:146`
- Backend source data: `backend/src/community/routes.rs:1460`, `backend/src/community/routes.rs:1480`, `backend/src/community/routes.rs:1514`

Problem:

The page builds HTML strings containing post content, comment content, author names, report reasons/statuses, reaction types, and tags, then assigns them through `post-detail-container.innerHTML`. Some content uses `content_sanitized` when present, but the backend falls back to raw content and other displayed fields are not sanitized for HTML insertion.

Expected:

Render untrusted/community-controlled values with `textContent` or DOM construction. If rich text is required, sanitize server-side and client-side contractually, and never mix raw string interpolation with user/admin data.

Evidence:

`p.content`, `c.content`, `r.reason`, `r.status`, `r.author_name`, `p.author_name`, and `content_tags` are interpolated directly into HTML templates.

Recommended fix:

Move the page behavior into a dedicated JS file, build DOM nodes with `textContent`, and only use static developer-controlled HTML snippets.

### P2 - Prompt/alert/confirm moderation UX has weak accessibility and state handling

Location:

- Template JS: `frontend/platform/admin/community/post-detail.html:228`
- Template JS: `frontend/platform/admin/community/post-detail.html:247`
- Template JS: `frontend/platform/admin/community/post-detail.html:266`
- Template JS: `frontend/platform/admin/community/post-detail.html:283`

Problem:

Moderation actions rely on browser prompts/alerts/confirms, do not disable buttons during in-flight requests, do not expose persistent success/error regions, and do not surface backend error bodies.

Expected:

Use accessible admin modals or inline forms with labels, focus management, `aria-live` status, in-flight disabled states, retry behavior, and backend error details.

Evidence:

Each mutating action uses `alert()` for failure and immediately reloads on success; no `aria-busy`, no status region, no keyboard-managed dialog.

Recommended fix:

Replace prompt/alert/confirm actions with the established admin modal/status pattern used by recently hardened community admin pages.

### P2 - External CDN scripts are loaded on an admin page without being used

Location:

- Template: `frontend/platform/admin/community/post-detail.html:11`
- Template: `frontend/platform/admin/community/post-detail.html:12`

Problem:

The page loads HTMX from unpkg and Alpine from jsDelivr, but no HTMX or Alpine behavior is used in this template. This creates avoidable external dependency and CSP/supply-chain exposure on an admin moderation surface.

Expected:

Remove unused external scripts or self-host/pin them consistently with the platform policy.

Evidence:

No `hx-*`, `x-*`, or Alpine component usage exists in the template.

Recommended fix:

Delete both script tags for this page unless a tested local dependency is required.

---

## Backend Findings

### P1 - Post detail APIs lack community-specific permission checks

Location:

- `backend/src/community/routes.rs:1309`
- `backend/src/community/routes.rs:1342`
- `backend/src/community/routes.rs:1380`
- `backend/src/community/routes.rs:1397`

Problem:

`GET /api/admin/community/posts/:id`, `POST /hide`, `POST /lock`, and `POST /tags` only extract `AdminUser`. They do not call `require_community_view_or_manage` or `require_community_manage`, even though the page route and sidebar use community permissions.

Expected:

The detail read endpoint should require `community.view` or `community.manage`; hide, lock/unlock, and tag mutation endpoints should require `community.manage`.

Evidence:

Nearby hardened comment endpoints call `require_community_manage` at `backend/src/community/routes.rs:1946`, while the post detail endpoints do not.

Recommended fix:

Add explicit permission checks to all admin post detail endpoints and cover them with permission-denied tests.

### P1 - Post moderation mutations are non-transactional and can report success for stale IDs

Location:

- `backend/src/community/routes.rs:1317`
- `backend/src/community/routes.rs:1350`
- `backend/src/community/routes.rs:1388`

Problem:

Hide, lock/unlock, and tag updates run direct `UPDATE` statements without `FOR UPDATE`, without checking affected row count or previous state, and without one transaction that includes audit logging. A stale UUID can return success with zero rows changed.

Expected:

Moderation mutations should lock the target post, validate existence, update state, write `community_audit_logs`, and commit atomically.

Evidence:

The handlers call `.execute(&c_pool).await?` and then return success. The result row count is ignored.

Recommended fix:

Use the pattern already present in comment pin/hide/delete: begin transaction, `SELECT ... FOR UPDATE`, update, audit with `log_community_admin_action_tx`, then commit.

### P1 - Post hide and lock audit logging is best-effort; tag edits are not audited

Location:

- `backend/src/community/routes.rs:1323`
- `backend/src/community/routes.rs:1361`
- `backend/src/community/routes.rs:1380`

Problem:

Hide and lock call `crate::community::audit::log(...).await` but ignore the result; tag edits do not write a community audit row at all. Admin moderation actions can therefore persist without a durable moderation audit trail.

Expected:

All moderation mutations should write durable audit entries atomically with the state change.

Evidence:

The audit future is awaited without `?`, and `admin_update_post_tags` accepts `_admin` and only runs an `UPDATE`.

Recommended fix:

Use `log_community_admin_action_tx` for hide, lock/unlock, and tags, including previous and new values.

### P2 - Tag update accepts unbounded arbitrary tag payloads

Location:

- `backend/src/community/routes.rs:1375`
- `backend/src/community/routes.rs:1388`

Problem:

The API accepts any `Vec<String>` and writes it into `posts.content_tags` with no count, length, vocabulary, normalization, or duplicate limits.

Expected:

Enforce a small allowlist or normalized max-count/max-length policy for moderation tags such as `NSFW`, `Spoiler`, `Needs Review`, or `Misleading`.

Evidence:

No validation function is called before binding `payload.tags`.

Recommended fix:

Validate tags server-side and return a 400 with a clear error for invalid input.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Inline JS syntax | Extracted inline `<script>` from template to `/tmp/post-detail-inline.js`; ran `node --check /tmp/post-detail-inline.js`. | Syntax passes. | Passed. | Pass |
| Route/static mapping | Reviewed admin route registration and community API router. | Page/API routes exist. | Routes exist. | Pass |
| Unauthenticated HTTP smoke | `curl 'http://localhost:8888/admin/community/post-detail?id=00000000-0000-0000-0000-000000000000'`. | 401/redirect if server running. | Could not connect to `localhost:8888`. | Blocked |
| Existing automated coverage search | Searched tests for post detail/admin post API coverage. | Tests cover detail read and moderation mutations. | No matching tests found. | Fail |
| Authenticated browser flow | Load page with admin session, fixture post, comments, reports; exercise hide/lock/tags/pin. | Visible success/error states and persisted backend changes. | Not run; no local server/session fixture. | Blocked |

---

## Security Findings

- P1: Admin post detail read/mutation APIs need explicit `community.view`/`community.manage` authorization.
- P1: Community/admin-controlled fields are interpolated into `innerHTML`, creating an admin-facing XSS risk.
- P1: Moderation audit logging is not atomic/reliable for post hide/lock, and tag edits are unaudited.
- P2: External CDN scripts are loaded on the admin page despite no page usage.
- CSRF: The shared admin fetch interceptor can add `X-CSRF-Token`, but these post moderation handlers do not currently call `require_csrf_header`. This should be aligned with the platform’s admin CSRF policy during the backend fix.

---

## Database Findings

- Required tables and columns exist: `posts.is_locked`, `posts.content_tags`, `comments.is_pinned`, `content_reports`, and `community_audit_logs`.
- Post hide/lock/tag handlers do not use transactions or row locks, despite updating moderation state and requiring an audit trail.
- Tag payload constraints are not enforced by schema or handler validation.
- Post mutation handlers ignore affected row count, allowing false success for nonexistent post IDs.

---

## Missing Tests

- Backend authorization tests for `GET /api/admin/community/posts/:id`, `/hide`, `/lock`, and `/tags` with admin users lacking community permissions.
- Backend mutation tests for nonexistent post IDs returning 404.
- Backend transaction/audit tests proving hide, lock/unlock, and tag edits write `community_audit_logs` atomically.
- Backend validation tests for tag count/length/allowlist.
- Frontend/browser tests for detail load, missing ID state, API failure state, hide/lock/tag/pin success and failure paths.
- Accessibility tests for replacement modals/status regions once prompt/alert/confirm is removed.
- XSS regression test with malicious post/comment/report/tag fixtures rendered as text, not executable HTML.

---

## Recommended Fix Order

1. Add backend `community.view`/`community.manage` permission checks and CSRF enforcement consistently for all post detail APIs.
2. Make post hide, lock/unlock, and tag updates transactional with row locks, row-existence checks, validation, and atomic `community_audit_logs` writes.
3. Replace `innerHTML` interpolation with DOM construction and `textContent`.
4. Replace prompt/alert/confirm flows with accessible admin modals and visible status regions.
5. Remove unused external HTMX/Alpine CDN scripts or self-host/pin only if required.
6. Add targeted backend and browser regression coverage.

---

## Final Status

`needs_recheck`

Reason: The page has real routes and backend support, but security, auditability, data integrity, rendering safety, UX, and test coverage gaps remain. Recheck after backend hardening and safe-rendering fixes with an authenticated admin fixture.
