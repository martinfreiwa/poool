# Page Audit: Community Index

Date: 2026-04-25
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/admin/community/`
Template: `frontend/platform/admin/community/index.html`
JavaScript: inline script in `frontend/platform/admin/community/index.html`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/community/routes.rs`

---

## Summary

The admin Community Overview page is route-registered and protected at the page level, but it is not production-ready. The page has no mutating actions, but its read path has serious correctness and security issues: recent activity is rendered with string-built `innerHTML` from community feed data, the dashboard labels the public feed as "Recent Announcements", the stats API masks query/schema failures as zero counts, and the Total XP KPI queries a table/column pair that does not exist in migrations.

Final status is `needs_recheck` because the page can render, but the primary dashboard data cannot be trusted until these findings are fixed and verified with an authenticated admin session.

---

## Tested Scope

- Static template review of `frontend/platform/admin/community/index.html`.
- Static backend route review of `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, and `backend/src/community/routes.rs`.
- Database migration review for community posts, comments, reactions, profiles, circles, reports, and XP tables.
- Existing test search for page/API coverage.
- Runtime browser testing was not performed because this documentation-only audit did not start or mutate the local app/session state.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/` | Registered to generic admin renderer. |
| URL alias | `/admin/community/index.html` | Registered to generic admin renderer. |
| Template | `frontend/platform/admin/community/index.html` | Contains markup plus all page-specific JavaScript inline. |
| Component | `frontend/platform/admin/components/sidebar.html` | Included by the template. |
| Shared JS | `frontend/platform/static/js/user-data.js` | Loaded globally. |
| Shared JS | `frontend/platform/static/js/admin-theme.js` | Loaded globally. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Loaded globally. |
| Backend page route | `GET /admin/community/` | `page_admin_generic` maps clean URL to `admin/community/index.html`. |
| Backend API route | `GET /api/admin/community/stats` | Returns KPI counts from the community DB. |
| Backend API route | `GET /api/community/feed` | Public/community feed API used incorrectly for "Recent Announcements". |
| Database table | `community_profiles` | Active profile count; `xp_total` exists here. |
| Database table | `posts` | Total posts and feed rows. |
| Database table | `comments` | Total comments. |
| Database table | `reactions` | Total reactions. |
| Database table | `circles` | Circle count. |
| Database table | `content_reports` | Pending report count. |
| Missing table/view | `user_xp_totals.current_xp` | Queried by stats API, not defined in migrations. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `nav.admin-breadcrumbs a[href="/admin/"]` | Navigate back to admin home. | Link only. | `GET /admin/` expected elsewhere. | Static verified. |
| Active Profiles KPI | `#kpi-profiles` | Load count from `/api/admin/community/stats`. | Yes, `textContent`. | Yes. | Unverified runtime. |
| Total Posts KPI | `#kpi-posts` | Load count from `/api/admin/community/stats`. | Yes, `textContent`. | Yes. | Unverified runtime. |
| Total Comments KPI | `#kpi-comments` | Load count from `/api/admin/community/stats`. | Yes, `textContent`. | Yes. | Unverified runtime. |
| Total Reactions KPI | `#kpi-reactions` | Load count from `/api/admin/community/stats`. | Yes, `textContent`. | Yes. | Unverified runtime. |
| Active Circles KPI | `#kpi-circles` | Load count from `/api/admin/community/stats`. | Yes, `textContent`. | Yes. | Unverified runtime. |
| Total XP Distributed KPI | `#kpi-xp` | Load total XP from `/api/admin/community/stats`. | Yes, `textContent`. | Broken: queries missing `user_xp_totals.current_xp`. | Expected false zero if query fails. |
| Pending Reports KPI | `#kpi-reports` | Load pending moderation report count. | Yes, `textContent`. | Yes, but query errors are swallowed as zero. | Unverified runtime. |
| Recent Announcements table | `#recent-announcements-table` | Show latest announcements. | Yes, but via `innerHTML`. | Wrong API: uses `/api/community/feed`, not admin announcements. | Needs recheck. |
| View All | `a[href="/admin/community/announcements.html"]` | Navigate to announcement admin page. | Link only. | Page route exists and has explicit `community.manage` gate. | Static verified. |
| Create Announcement quick action | `.admin-quick-action[href="/admin/community/announcements.html"]` | Navigate to announcement admin page. | Link only. | Page route exists and has explicit `community.manage` gate. | Static verified. |

---

## Frontend Findings

### P1 - Recent activity renders feed data with string-built HTML

Location:

- Template: `frontend/platform/admin/community/index.html:177-211`
- JS: inline `DOMContentLoaded` handler

Problem:

The page builds table rows with template literals using `p.author_avatar`, `p.author_name`, `p.category`, and engagement fields, then assigns the complete string with `tbody.innerHTML`. These values come from `/api/community/feed`, where author names and avatars are user/profile-derived data. This violates the platform standard of not using `innerHTML` with user-generated data and creates an XSS/injection risk in an admin-only surface.

Expected:

Render rows with DOM APIs and `textContent`, validate/rewrite avatar URLs before assigning `img.src`, and avoid interpolating server values into HTML strings.

Evidence:

`frontend/platform/admin/community/index.html:193-211` interpolates feed response fields and assigns `tbody.innerHTML = html`.

Recommended fix:

Move this inline script into a page JS file and render the table with `document.createElement`, `textContent`, and safe `src` assignment.

### P2 - Recent Announcements uses the public community feed instead of announcements

Location:

- Template: `frontend/platform/admin/community/index.html:177-180`
- Backend announcement API: `backend/src/community/routes.rs:529-544`, `backend/src/community/service.rs:91-155`

Problem:

The table is titled "Recent Announcements" but fetches `/api/community/feed`, which returns general community posts from `service::get_community_feed`. This can display non-announcement user posts in an admin announcement panel and does not use the existing announcement-specific query.

Expected:

Fetch `/api/admin/community/announcements` or another announcement-specific read endpoint, then render only announcement rows.

Evidence:

`frontend/platform/admin/community/index.html:179` calls `/api/community/feed`; `service::get_community_feed` selects from `posts` without requiring `post_type = 'announcement'` or an `announcement_categories` join when no category is provided.

Recommended fix:

Use the admin announcement API already used by `frontend/platform/admin/community/announcements.html`, or add a small read-only "recent announcements" endpoint that returns the exact columns the overview table needs.

### P2 - Async failures leave stale loading placeholders

Location:

- Template: `frontend/platform/admin/community/index.html:121-124`, `frontend/platform/admin/community/index.html:160-175`, `frontend/platform/admin/community/index.html:178-215`

Problem:

If either fetch returns a non-OK response, the page does nothing visible. If an exception is thrown, the error is only logged to the console. KPI cards stay as `—`, and the table can stay on `Loading...` indefinitely.

Expected:

Show a visible retryable error state for stats and the recent announcements table. Admin pages should not require DevTools to discover backend/API failures.

Evidence:

Both fetch blocks only enter the success path for `res.ok`; catch blocks call `console.error` but do not update the DOM.

Recommended fix:

Render `Unable to load` rows/cards with a retry control and make failures clear to operators.

### P3 - Unused external CDN scripts remain on the admin page

Location:

- Template: `frontend/platform/admin/community/index.html:11-12`

Problem:

The page loads HTMX and Alpine from third-party CDNs, but this template does not use HTMX or Alpine attributes. This creates an unnecessary external runtime dependency and widens the admin page's script supply-chain surface.

Expected:

Remove unused CDN scripts or self-host required vendor scripts through the existing static asset path.

Evidence:

Static template review found no `hx-*`, `x-data`, `x-show`, or other Alpine/HTMX usage on this page.

Recommended fix:

Delete both CDN script tags from this page if still unused after moving page logic into a local JS file.

---

## Backend Findings

### P1 - Community stats API masks schema and database failures as zero counts

Location:

- Backend: `backend/src/community/routes.rs:729-780`
- Database: `database/community/008_circles_xp.sql`, `database/069_apply_missing_community_schema.sql`

Problem:

`GET /api/admin/community/stats` uses `.unwrap_or((0,))` for every count query. A missing table, broken column, permissions problem, or transient database error is silently reported to admins as `0`. This already hides a real schema mismatch: the code queries `SELECT COALESCE(SUM(current_xp), 0) FROM user_xp_totals`, but repo migrations define `community_profiles.xp_total` and do not define `user_xp_totals` or `current_xp`.

Expected:

Query the real XP source, likely `SELECT COALESCE(SUM(xp_total), 0) FROM community_profiles`, and propagate database errors through `AppError` so the frontend can show an error state.

Evidence:

`rg` found no migration defining `user_xp_totals` or `current_xp`. `backend/src/community/xp.rs` and migrations use `community_profiles.xp_total`.

Recommended fix:

Replace all per-query `.unwrap_or((0,))` fallbacks with `?`/`map_err(AppError::Database)`, fix the XP query source, and add coverage for the stats response.

### P2 - Stats API does not enforce the same fine-grained permission as the page

Location:

- Page gate: `backend/src/admin/pages.rs:162-169`
- API route: `backend/src/community/routes.rs:729-733`
- Permission helper: `backend/src/community/routes.rs:88-101`

Problem:

The HTML page requires `community.view` or `community.manage`, but `get_admin_stats` only extracts `AdminUser` and does not call `require_community_view_or_manage`. The API should use the same authorization contract as the page, especially because this is admin-only community-health data.

Expected:

`get_admin_stats` should accept a named `admin` parameter and call `require_community_view_or_manage(&state, &admin).await?` before reading community stats.

Evidence:

`backend/src/community/routes.rs:729-733` names the extractor `_admin` and immediately reads the community pool without a permission check.

Recommended fix:

Align the stats API with the community admin API permission helpers used elsewhere in `backend/src/community/routes.rs`.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Route registration static check | Reviewed `backend/src/admin/mod.rs` routes for `/admin/community/` and `/admin/community/index.html`. | Both routes resolve to the page renderer. | Routes exist. | Pass static. |
| Page permission static check | Reviewed `page_admin_generic` for `admin/community/` gating. | Page requires community permission. | Page requires `community.view` or `community.manage`. | Pass static. |
| Stats API static check | Reviewed `/api/admin/community/stats` handler. | Same community permission as page; DB errors visible. | Handler only requires `AdminUser` and swallows query errors. | Fail. |
| XP schema static check | Searched migrations and community code for `user_xp_totals`, `current_xp`, and `xp_total`. | Stats query matches migration schema. | `user_xp_totals.current_xp` not defined; `community_profiles.xp_total` exists. | Fail. |
| Recent table static check | Reviewed inline fetch/render logic. | Shows recent announcements only and renders safely. | Uses public feed and `innerHTML`. | Fail. |
| Runtime browser smoke | Not run in this documentation-only audit. | Authenticated page load, console, network, desktop/mobile checks. | Not performed. | Not run. |

---

## Security Findings

- P1: Admin page XSS/injection risk from string-built `innerHTML` with feed-derived author/profile data.
- P2: `/api/admin/community/stats` does not enforce the same `community.view`/`community.manage` permission contract as the page.
- No state-changing controls exist on this page, so CSRF exposure is not directly present on the overview itself.
- The page loads unused third-party CDN scripts on an admin surface.

---

## Database Findings

- `user_xp_totals.current_xp` is queried by `get_admin_stats` but is not defined in repository migrations.
- The current migration-backed XP source is `community_profiles.xp_total`.
- Required read tables for the other KPIs exist: `posts`, `comments`, `reactions`, `community_profiles`, `circles`, and `content_reports`.
- Because the handler swallows individual query failures, admins cannot distinguish a true zero count from an unavailable/broken table.

---

## Missing Tests

- Add a Rust/API test for `GET /api/admin/community/stats` that verifies permission enforcement.
- Add a Rust/API test for stats response shape using seeded community rows, including nonzero `community_profiles.xp_total`.
- Add a regression test that a missing/broken stats query returns an error instead of a false zero response.
- Add an authenticated browser/E2E smoke for `/admin/community/` covering KPI load, recent announcement rendering, error state, and mobile layout.
- Add a frontend/static regression check or Playwright fixture confirming user-derived feed/announcement values are not rendered through raw HTML interpolation.

---

## Recommended Fix Order

1. Fix `/api/admin/community/stats`: enforce `community.view`/`community.manage`, query `community_profiles.xp_total`, and propagate DB errors.
2. Replace the recent table data source with the announcement-specific admin API.
3. Rewrite the recent table renderer to use DOM construction and `textContent`.
4. Add visible error/retry states for stats and recent announcements.
5. Remove unused CDN dependencies from the page or self-host them if they become necessary.
6. Run an authenticated desktop/mobile browser recheck and add targeted API/E2E coverage.

---

## Final Status

`fixed`

Reason: The documented issues were fixed locally and verified with targeted authenticated E2E coverage.

---

## Fix Verification

Date: 2026-04-26
Status: fixed

Changes verified:

- `PAGE-ISSUE-0124`: Recent announcements are rendered by `frontend/platform/static/js/admin-community-index.js` with DOM construction and `textContent`; no feed-derived values are interpolated into `innerHTML`.
- `PAGE-ISSUE-0125`: `GET /api/admin/community/stats` now propagates database errors and reads total XP from `community_profiles.xp_total`.
- `PAGE-ISSUE-0126`: The overview fetches `/api/admin/community/announcements`, not `/api/community/feed`, and the E2E verifies a seeded general post does not render in the Recent Announcements table.
- `PAGE-ISSUE-0127`: The stats API now calls `require_community_view_or_manage`.
- `PAGE-ISSUE-0128`: Stats and announcement loaders have visible retryable error states.
- `PAGE-ISSUE-0129`: Unused external HTMX and Alpine CDN scripts were removed from this page.

Verification commands:

| Command | Result |
|---------|--------|
| `node --check frontend/platform/static/js/admin-community-index.js` | Passed |
| `python3 -m py_compile tests/e2e/test_admin_community_index.py` | Passed |
| `cargo fmt --check` | Passed |
| `cargo check` | Passed |
| `BASE_URL=http://localhost:8896 DATABASE_URL=postgres://martin@localhost/poool COMMUNITY_DATABASE_URL=postgres://martin@localhost/poool_community python3 -m pytest tests/e2e/test_admin_community_index.py -q` | Passed, 4 tests |

E2E coverage:

- Creates a safe authenticated admin session.
- Seeds community profile XP and announcement/general-post fixtures.
- Verifies KPI API success and `total_xp` equals the `community_profiles.xp_total` sum.
- Verifies Recent Announcements renders announcement rows only.
- Verifies unsafe seeded markup does not execute or render as markup in the table.
- Verifies visible retry states for stats and announcements API failures.
- Verifies no critical console/network failures on normal load.
- Verifies mobile smoke without horizontal overflow.
- Cleans up seeded users, sessions, community profiles, and posts.
