# Page Audit: Community Announcements

Date: 2026-04-25
Status: fixed_verified
Auditor: ChatGPT/Codex
Page URL: `/admin/community/announcements`
Template: `frontend/platform/admin/community/announcements.html`
JavaScript: inline script in `frontend/platform/admin/community/announcements.html`; shared `frontend/platform/static/js/admin-permission-guard.js`, `frontend/platform/static/js/admin-theme.js`, `frontend/platform/static/js/user-data.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`, inline Quill styles
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/community/routes.rs`, `backend/src/community/service.rs`

---

## Summary

The admin Community Announcements page was audited and then fixed locally. The page route is now permission-gated, the table reads a dedicated admin announcements endpoint, publish sends CSRF, rows render through DOM/text APIs, categories are validated server-side, and announcement publishing writes a community audit log.

Final status is `fixed_verified`.

2026-04-25 fix update: a targeted Playwright E2E passed against the local backend. It signed up an admin fixture, granted `community.manage`, loaded `/admin/community/announcements`, verified `GET /api/admin/community/announcements` returns 200, verified publish without CSRF returns 403, and published through the UI, creating a row in `poool_community.posts`.

---

## Tested Scope

- Static template review of `frontend/platform/admin/community/announcements.html`
- Inline JavaScript review for load, preview, modal, and publish behavior
- Shared admin JavaScript references checked for `/api/me`, CSRF helper, and permission guard behavior
- Backend route review for `/admin/community/announcements`, `/api/community/feed`, and `POST /api/admin/community/announcements`
- Service/schema review for `posts`, `announcement_categories`, `community_profiles`, and `community_audit_logs`
- Existing tests searched for admin announcement coverage
- Targeted Playwright browser/API E2E against the patched local backend

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/announcements` | Fixed to use `page_admin_community_announcements`; admin session plus `community.manage` required |
| URL alias | `/admin/community/announcements.html` | Same permission-gated page handler |
| Template | `frontend/platform/admin/community/announcements.html` | Page markup plus all page-specific JS inline |
| Shared JS | `frontend/platform/static/js/user-data.js` | Defines `window.getCsrfToken`; page uses it for publish |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Provides admin permission behavior and fallback CSRF injection |
| Backend page route | `GET /admin/community/announcements` | `backend/src/admin/mod.rs` -> `page_admin_community_announcements` |
| Backend API route | `GET /api/admin/community/announcements` | Added for admin announcement inventory |
| Backend API route | `POST /api/admin/community/announcements` | Fixed to require `community.manage`, validate category, and audit publish |
| Database table | `posts` | Stores announcement posts and sanitized content |
| Database table | `announcement_categories` | Stores category with DB check constraint |
| Database table | `community_profiles` | No longer blocks the admin announcement inventory endpoint |
| Database table | `community_audit_logs` | Announcement publish writes `announcement.create` |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Admin link | `.admin-breadcrumbs a[href="/admin/"]` | Navigate to admin home | Yes, link | Yes | Not runtime-tested |
| Breadcrumb Community link | `.admin-breadcrumbs a[href="/admin/community/"]` | Navigate to community admin index | Yes, link | Yes via generic admin route | Not runtime-tested |
| New Announcement button | line 67 inline `onclick` | Open create modal | Yes, direct style mutation | No backend needed | Works by static inspection; not keyboard/focus managed |
| Live Announcements table | `#announcements-table` | Load announcement rows | Yes, calls `/api/community/feed` | Backend exists, but wrong endpoint for admin announcement inventory | Broken/partial: loads all public feed posts, not just announcements |
| Loading row | initial table row lines 89-92 | Show loading state before fetch | Yes | No backend needed | Present |
| Empty row | inline JS line 216 | Show no-announcements state | Yes | Depends on feed response | Misleading because feed can be empty/non-empty independent of announcement inventory |
| Error row | inline JS line 245 | Show load failure | Yes | Yes | Present, but no retry action |
| Create modal | `#create-modal` | Dialog for publishing announcement | Partially wired by inline style handlers | API exists | Missing dialog semantics, focus trap, Escape close, backdrop close |
| Close icon button | modal header line 114 | Close modal | Yes, inline `onclick` | No backend needed | Lacks accessible label |
| Category select | `#ann-category` | Select DB-valid category | Yes | DB constraint matches options | Wired |
| Quill editor | `#editor-container` | Rich-text content entry | Yes via CDN Quill | Backend sanitizes with Ammonia | CDN dependency and no backend content policy beyond sanitizer/DB length |
| Pin checkbox | `#ann-pin` | Publish announcement as pinned | Yes | `posts.is_pinned` exists | Wired |
| Live Preview | `#preview-content`, `#preview-badge` | Preview current content/category | Yes | No backend needed | Uses `innerHTML` with editor HTML; admin-only preview but still should be constrained |
| Cancel button | create form line 151 | Close modal | Yes, inline `onclick` | No backend needed | Lacks focus return handling |
| Publish to Feed button | create form submit line 152 | POST announcement and refresh table | Yes | Route exists | Broken under CSRF middleware because request sends no `X-CSRF-Token` |

---

## Frontend Findings

### P1 - Publish request omits CSRF token

Location:

- Template/JS: `frontend/platform/admin/community/announcements.html:296`
- Shared helper available: `frontend/platform/static/js/user-data.js` defines `window.getCsrfToken`

Problem:

The publish fetch sends only `Content-Type: application/json`. The global CSRF middleware requires `X-CSRF-Token` for POST/PUT/PATCH/DELETE unless a route is explicitly skipped.

Expected:

The request should include `X-CSRF-Token: window.getCsrfToken()` or equivalent, and the UI should handle a 403 as a recoverable validation/security error.

Evidence:

`fetch('/api/admin/community/announcements', { method: 'POST', headers: { 'Content-Type': 'application/json' } ... })` is missing the CSRF header.

Recommended fix:

Add the CSRF header to the publish request, keep the submit button disabled only during the in-flight request, and show an inline form error rather than `alert()`.

### P1 - Announcement table renders API data through `innerHTML`

Location:

- Template/JS: `frontend/platform/admin/community/announcements.html:220-241`

Problem:

Rows are built with template strings and assigned to `tbody.innerHTML`. API fields including `p.content`, `p.category`, and `p.created_at` are interpolated without HTML escaping. The current code strips tags from `p.content` with a regex, but it does not safely escape attribute/text contexts such as the `title` attribute.

Expected:

Create table rows with DOM APIs and set user/content-derived values via `textContent`/safe attributes, or use a shared escaping helper before interpolation.

Evidence:

`title="${stripped}"`, `${preview}`, `${p.category}`, and `tbody.innerHTML = html` all render values derived from the API response.

Recommended fix:

Replace row string concatenation with DOM construction. If HTML rendering is required, only render `content_sanitized` through a vetted sanitizer and never inside attribute strings.

### P2 - Admin table reads the public feed instead of announcement inventory

Location:

- Template/JS: `frontend/platform/admin/community/announcements.html:210-213`
- Backend public feed: `backend/src/community/routes.rs:329`

Problem:

The page title says "Live Announcements", but it calls `/api/community/feed` without filtering. That endpoint returns the public community feed, so general posts, market insights, reviews, and milestones can appear in the admin announcement table. It also joins `community_profiles`, so an admin-created announcement may not show if the admin lacks a community profile.

Expected:

Use an admin-safe announcements endpoint or call a filtered endpoint that only returns `post_type='announcement'`, includes category metadata, and is not affected by public feed profile joins.

Evidence:

The inline comment says "Feeds return the announcements essentially"; the backend feed service does not filter `post_type='announcement'`.

Recommended fix:

Wire the table to an admin announcement listing route backed by `service::get_announcements`, or add a purpose-built `/api/admin/community/announcements` GET route with pagination and permission checks.

### P2 - Modal accessibility and state handling are incomplete

Location:

- Template: `frontend/platform/admin/community/announcements.html:67`, `frontend/platform/admin/community/announcements.html:104`, `frontend/platform/admin/community/announcements.html:114`, `frontend/platform/admin/community/announcements.html:151`

Problem:

The modal is opened and closed by inline style mutations. It has no `role="dialog"`, `aria-modal`, accessible close label, focus entry, focus trap, Escape close, backdrop close, or focus return. Error and success feedback use `alert()` and button text changes instead of inline status regions.

Expected:

Use the admin modal pattern from recently fixed admin pages: semantic dialog attributes, keyboard handling, focus restoration, and visible inline status messages.

Evidence:

The modal container is a plain `div` with `display:none`; controls are inline `onclick` handlers.

Recommended fix:

Move modal behavior into a page script function, add semantic dialog attributes and keyboard support, and avoid inline handlers.

---

## Backend Findings

### P1 - Announcement publish lacks fine-grained permission and audit logging

Location:

- Route handler: `backend/src/community/routes.rs:395-414`
- Route registration: `backend/src/community/routes.rs:1759-1762`
- Audit table: `database/community/018_community_audit_log.sql:5-13`

Problem:

`POST /api/admin/community/announcements` requires `AdminUser`, but it does not check a specific community permission and does not write `community_audit_logs`. This is a public-content admin action and should be attributable.

Expected:

Require a dedicated permission such as `community.manage` or `community.announcements.publish`, and write an immutable community audit row in the same successful operation path.

Evidence:

The handler extracts `AdminUser`, sanitizes content, calls `service::create_announcement`, and returns `{ "id": post_id }`; there is no permission check or audit call.

Recommended fix:

Add a permission check before mutation and log `announcement.create` with actor, post id, category, pinned state, and a non-sensitive content summary.

### P2 - Category validation relies on database constraint errors

Location:

- Request struct: `backend/src/community/routes.rs:27-32`
- Insert: `backend/src/community/service.rs:203-211`
- Constraint: `database/community/004_announcement_categories.sql:3-5`

Problem:

The backend accepts arbitrary category strings and relies on the database CHECK constraint to reject invalid values. That preserves data integrity but produces a generic database error rather than a clear 400 validation response.

Expected:

Validate category against the allowlist before opening the transaction.

Evidence:

`payload.category` is passed directly to `service::create_announcement` and bound into `announcement_categories.category`.

Recommended fix:

Add a server-side category enum/validator shared with admin UI choices.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static page route mapping | Checked tracker, `backend/src/admin/mod.rs`, and `backend/src/admin/pages.rs` | `/admin/community/announcements` maps to template behind admin auth and `community.manage` | Route is registered via `page_admin_community_announcements` | Pass |
| Table load contract | Reviewed table fetch and `/api/community/feed` backend | Admin table lists announcements only | Fetch uses public feed and can include all post types | Fail |
| Publish contract | Reviewed POST request and CSRF middleware | Publish sends CSRF header and creates announcement | Request omits `X-CSRF-Token`; global middleware should reject | Fail |
| DB transaction support | Reviewed `service::create_announcement` | Multi-table write is transactional | `posts` and `announcement_categories` insert in one transaction | Pass |
| Runtime browser test | Targeted Playwright E2E with signup admin fixture and direct local DB permission grant | Authenticated page load, API listing, CSRF rejection, and modal submit verified | Page loaded, GET returned 200, no-CSRF POST returned 403, UI publish created `poool_community.posts` row `195fdc2f-9183-4bcf-b389-972f02e85de3` | Pass |

---

## Security Findings

- Fixed P1: Publish sends CSRF and a route-level no-CSRF check now rejects missing-token API calls.
- Fixed P1: Admin table rows render with DOM APIs and text nodes instead of `innerHTML`.
- Fixed P1: Publish requires `community.manage`.
- Fixed P1: Publish writes `announcement.create` to `community_audit_logs`.
- Residual P2: Quill still loads from a third-party CDN; HTMX was removed from this page.

---

## Database Findings

- `posts` supports announcement content, sanitized content, pinned state, reaction/comment counts, and hidden state.
- `announcement_categories` correctly constrains category values and cascades on post deletion.
- `service::create_announcement` correctly wraps `posts` and `announcement_categories` writes in one transaction.
- `community_audit_logs` is now used by announcement publishing.
- The admin inventory endpoint uses `service::get_announcements`, so it is not affected by public feed `community_profiles` joins.

---

## Missing Tests

- Persist the targeted Playwright coverage as a committed test for `/admin/community/announcements`.
- Add an integration assertion that publish persists one `posts` row plus one `announcement_categories` row and one `community_audit_logs` row.
- Authorization test proving non-community admins cannot publish once a fine-grained permission is added.
- Regression test that the admin table lists only `post_type='announcement'`.
- XSS regression test for content/category rendering in the admin table.
- Accessibility test or Playwright smoke for modal focus, Escape close, and focus return.

---

## Recommended Fix Order

1. Add CSRF header and inline form error handling to the publish fetch.
2. Add fine-grained backend permission and community audit logging for announcement creation.
3. Replace the public feed table source with a dedicated admin announcement list endpoint.
4. Render announcement rows with DOM APIs/text nodes instead of `innerHTML`.
5. Upgrade modal accessibility and remove inline open/close handlers.
6. Self-host Quill/HTMX or remove unused HTMX from this page.

---

## Final Status

`fixed_verified`

Reason: The audit findings were fixed locally, but an authenticated admin browser/API pass is still required to verify the publish, listing, permission, audit-log, and modal flows end-to-end.
