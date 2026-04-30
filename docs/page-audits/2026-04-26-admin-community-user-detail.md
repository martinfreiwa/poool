# Page Audit: Community User Detail

Date: 2026-04-26; continued 2026-04-27; fixed and runtime-rechecked 2026-04-28
Status: fixed
Auditor: ChatGPT/Codex
Page URL: `/admin/community/user-detail`
Template: `frontend/platform/admin/community/user-detail.html`
JavaScript: inline script in `frontend/platform/admin/community/user-detail.html`; shared `frontend/platform/static/js/admin-permission-guard.js`, `frontend/platform/static/js/admin-theme.js`, `frontend/platform/static/js/user-data.js`, `frontend/platform/static/js/csrf.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`
Backend Routes: `backend/src/admin/pages.rs`, `backend/src/admin/mod.rs`, `backend/src/community/routes.rs`

---

## Summary

The audited code issues have been fixed locally: dynamic detail rendering now uses DOM APIs and `textContent`, unused external CDN scripts were removed, moderation actions use a labelled dialog instead of native prompts, mutation fetches send the shared CSRF header, read APIs require `community.view`/`community.manage`, moderation APIs require `community.manage`, and moderation mutations are transactional, audited, target-row aware, and server-validate ban reasons.

Final status is `fixed` because the code-level fixes, static regression tests, broad formatting gate, isolated backend compile, and targeted authenticated browser/API E2E all pass against the current backend.

---

## 2026-04-28 Fix Pass

Fixed:

- `PAGE-ISSUE-0142`: Replaced unsafe profile, badge, post, mod-note, ban, avatar, and link rendering with DOM construction, `textContent`, and validated URL assignment.
- `PAGE-ISSUE-0323`: Added `community.view`/`community.manage` checks to community user read APIs and `community.manage` checks to moderation mutations.
- `PAGE-ISSUE-0324`: Wrapped ban, mute, shadowban, warn, and mod-note updates in transactions; checked `rows_affected`; propagated audit-log insert failures; inserted warning notifications in the same transaction; added mod-note audit logging.
- Mutation CSRF: Loaded `csrf.js` on the page and added `X-CSRF-Token` to moderation fetches.
- Ban validation: Enforced required/non-empty ban reasons and a 1000-character cap server-side.
- Runtime schema mismatch: Detail API now enriches `profile.display_name` from the core user bridge instead of selecting a nonexistent `community_profiles.display_name` column.
- P2 moderation UX: Replaced native `prompt`/`confirm`/`alert` flows with one labelled dialog, visible status text, frontend validation, and disabled buttons while requests are pending.
- P3 external dependencies: Removed unused HTMX and Alpine CDN scripts from the page.

Remaining verification items:

- None for the documented findings in this audit. Broader mobile visual and cross-browser coverage can be added later, but the audited code issues are fixed and targeted runtime coverage passes.

---

## Tested Scope

- Static review of `frontend/platform/admin/community/user-detail.html`.
- Static review of page registration in `backend/src/admin/mod.rs` and `backend/src/admin/pages.rs`.
- Static review of `GET /api/admin/community/users/:id/detail` and moderation mutation handlers in `backend/src/community/routes.rs`.
- Static review of schema support in `database/community/001_posts.sql`, `005_community_profiles.sql`, `006_social_layer.sql`, `013_moderation.sql`, and `014_shadowban.sql`.
- Safe unauthenticated runtime smoke against local `cargo run` server, refreshed on 2026-04-27.
- Inline JavaScript syntax check after extracting the page script to `/tmp/admin-community-user-detail-inline.js`, refreshed on 2026-04-27.
- Static regression suite `python3 -m pytest tests/admin/test_admin_community_user_detail_static.py -q`, refreshed on 2026-04-28.
- Authenticated API/browser E2E suite `BASE_URL=http://localhost:8893 python3 -m pytest tests/e2e/test_admin_community_user_detail.py -q`, refreshed on 2026-04-28.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/user-detail` | Clean admin route registered. |
| URL alias | `/admin/community/user-detail.html` | Linked from community user list. |
| Template | `frontend/platform/admin/community/user-detail.html` | Page shell plus all page-specific JS inline. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Hides sidebar entries based on admin permissions. |
| Shared JS | `frontend/platform/static/js/csrf.js` | Exposes `window.getCsrfToken()` for moderation mutation headers. |
| Backend page route | `GET /admin/community/user-detail` | `page_admin_generic`; requires admin plus `community.view` or `community.manage`. |
| Backend API route | `GET /api/admin/community/users/:id/detail` | Returns core user, community profile, badges, recent posts, and XP summary. |
| Backend API route | `POST /api/admin/community/users/:id/warn` | Increments warning count and sends notification. |
| Backend API route | `POST /api/admin/community/users/:id/mod-notes` | Updates moderator notes. |
| Backend API route | `POST /api/admin/community/users/:id/mute` | Sets or clears `muted_until`. |
| Backend API route | `POST /api/admin/community/users/:id/ban` | Sets `is_community_banned` and `ban_reason`. |
| Backend API route | `POST /api/admin/community/users/:id/shadowban` | Sets `is_shadowbanned`. |
| Database table | `community_profiles` | Profile, moderation status, counts, XP, notes. |
| Database table | `badges`, `user_badges` | Badge display. |
| Database table | `posts` | Recent post display. |
| Database table | `community_audit_logs` | Some mutation audit logging. |
| Database table | `notifications` | Warning notification path. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Breadcrumb Admin | `a[href="/admin/"]` | Navigate to admin dashboard. | Link only. | Yes. | Not browser-clicked. |
| Breadcrumb Community | `a[href="/admin/community/"]` | Navigate to community overview. | Link only. | Yes. | Not browser-clicked. |
| Breadcrumb Users | `a[href="/admin/community/users.html"]` | Navigate to community user list. | Link only. | Yes. | Not browser-clicked. |
| Loading state | `#user-detail-container` initial child | Show while API loads. | Yes. | N/A. | Static verified. |
| Missing id error | `URLSearchParams(...).get("id")` branch | Show a visible missing-ID error. | Yes via `replaceChildren`/`textContent`. | N/A. | Static verified. |
| Detail loader | `loadUserDetail(userId)` | Fetch detail JSON and render profile. | Yes. | Yes, route requires `community.view`/`community.manage`. | Authenticated E2E passed. |
| Profile header/avatar | `buildProfileCard()` / `buildAvatar()` | Show avatar or initials, name, email, ID, bio. | Yes, DOM/text APIs and safe avatar URL validation. | Yes. | Static pass. |
| Ban status panel | `buildNotice()` for ban state | Show current ban reason. | Yes, text nodes. | Yes. | Static pass. |
| Send Warning | `openWarningDialog()` | Dialog reason validation, CSRF POST warning, reload. | Yes. | Route requires `community.manage`, transaction, audit, notification. | Authenticated E2E passed. |
| Mute/Unmute | `openMuteDialog()` / `openUnmuteDialog()` | Dialog/confirmation, CSRF POST mute payload, reload. | Yes. | Route requires `community.manage`, transaction, audit, row check. | Authenticated API E2E passed. |
| Shadowban/Un-shadowban | `openShadowbanDialog()` | Confirm in dialog, CSRF POST shadowban payload, reload. | Yes. | Route requires `community.manage`, transaction, audit, row check. | Authenticated API E2E passed. |
| Ban/Unban | `openBanDialog()` | Dialog reason validation, CSRF POST ban payload, reload. | Yes. | Route requires `community.manage`, CSRF, transaction, audit, row check, server reason validation. | Authenticated API E2E passed. |
| Edit Mod Notes | `openModNotesDialog()` | Dialog notes validation, CSRF POST update, reload. | Yes. | Route requires `community.manage`, transaction, audit, row check. | Authenticated API E2E passed. |
| Level & XP card | rendered `xp.current_level`, `xp.current_level_name`, `xp.total_xp` | Display XP summary. | Yes. | Yes, `get_xp_summary`. | Static verified. |
| Badges list | `buildBadgesCard()` | Display badge count and badges. | Yes, DOM/text APIs. | Yes. | Static pass. |
| Recent Posts table | `buildRecentPostsCard()` | Link to post detail and show type/date. | Yes, DOM/text APIs and encoded post links. | Yes. | Static pass. |

---

## Frontend Findings

### P1 - Detail view injects unescaped user and moderation data (fixed)

Location:

- Template: `frontend/platform/admin/community/user-detail.html:113`, `frontend/platform/admin/community/user-detail.html:122`, `frontend/platform/admin/community/user-detail.html:128`, `frontend/platform/admin/community/user-detail.html:139`, `frontend/platform/admin/community/user-detail.html:150`, `frontend/platform/admin/community/user-detail.html:182`, `frontend/platform/admin/community/user-detail.html:202`, `frontend/platform/admin/community/user-detail.html:210`

Problem:

The page builds one large HTML string with `profile.display_name`, `profile.bio`, `profile.ban_reason`, `profile.mod_notes`, badge fields, post content, email, avatar URL, and IDs. These values are user-generated, moderator-entered, or database-derived and are inserted with `innerHTML` or raw attribute interpolation.

2026-04-28 status: fixed. Current template uses DOM construction, `textContent`, text nodes, `replaceChildren`, encoded post links, and safe avatar URL validation.

Expected:

Render user-provided values with `textContent` and safe DOM construction. URLs should be assigned through validated `URL`/attribute helpers, and post links should use `encodeURIComponent`.

Evidence:

Static review found four `innerHTML` assignment sites and broad template-literal interpolation. This is the same unresolved class tracked as `PAGE-ISSUE-0142`.

Recommended fix:

Move the inline page controller into a dedicated JS file, create DOM nodes explicitly, set text with `textContent`, validate image/link URLs, and keep any static empty/error markup separate from user data.

### P2 - Native prompt/alert/confirm flows are weak moderation UX (fixed)

Location:

- Template: `frontend/platform/admin/community/user-detail.html:274-394`

Problem:

Warning, mod-notes, mute, ban, and shadowban actions use native `prompt()`, `confirm()`, and `alert()` calls. They do not provide accessible dialog semantics, focus management, inline validation, loading/disabled states, or server error details.

2026-04-28 status: fixed. Current template uses one labelled `<dialog>`, visible `role=status` feedback, field validation, disabled buttons while saving, and server error text.

Expected:

Use accessible modals or the existing custom confirmation pattern with labelled fields, visible validation, disabled submit while pending, status regions, and server-provided error text.

Evidence:

Static scan found 4 `prompt()`, 3 `confirm()`, and 12 `alert()` usages in the page inline script.

Recommended fix:

Replace native dialogs with page-local accessible modals and bind controls with `addEventListener`.

### P3 - Page loads unused third-party script dependencies (fixed)

Location:

- Template: `frontend/platform/admin/community/user-detail.html:11-12`

Problem:

The page loads HTMX and Alpine from public CDNs but the audited behavior is plain inline JavaScript. This increases dependency and CSP surface for a sensitive admin page.

2026-04-28 status: fixed. Current template has no HTMX/Alpine CDN references and loads only local shared scripts.

Expected:

Remove unused scripts, or self-host and use them intentionally.

Evidence:

No `hx-*`, `x-data`, or Alpine component usage was found in the page body.

Recommended fix:

Remove the two CDN scripts when extracting the inline controller.

---

## Backend Findings

### P1 - Detail and moderation APIs do not enforce community permissions (fixed)

Location:

- `backend/src/community/routes.rs:1548`
- `backend/src/community/routes.rs:1609`
- `backend/src/community/routes.rs:1648`
- `backend/src/community/routes.rs:1690`
- `backend/src/community/routes.rs:1728`
- `backend/src/community/routes.rs:1774`
- `backend/src/community/routes.rs:4155`

Problem:

The page route requires admin plus `community.view` or `community.manage`, but the backing APIs only extract `AdminUser`. Any admin role can read community user details or perform moderation actions if they know the endpoints.

2026-04-28 status: fixed. Read APIs require `community.view`/`community.manage`; moderation APIs require `community.manage`.

Expected:

`GET /detail` and the user list should require `community.view` or `community.manage`. Warning, mute, ban, shadowban, mod-notes, badge, and XP mutations should require `community.manage`.

Evidence:

`page_admin_generic` checks `community.view` / `community.manage` for `admin/community/*`, but the listed API handlers do not call `require_community_view_or_manage()` or `require_community_manage()`.

Recommended fix:

Call the permission helpers at the start of each handler and add denial tests for an admin role without community permissions.

### P1 - Moderation mutations can silently succeed or partially persist (fixed)

Location:

- `backend/src/community/routes.rs:1617-1640`
- `backend/src/community/routes.rs:1660-1682`
- `backend/src/community/routes.rs:1698-1720`
- `backend/src/community/routes.rs:1736-1766`
- `backend/src/community/routes.rs:1782-1788`

Problem:

Ban, mute, shadowban, warn, and mod-notes updates do not check `rows_affected()`. They can return success for a UUID with no community profile. Audit logging is best-effort for ban/mute/shadowban/warn, and mod-notes has no audit log at all. `warn` increments `warning_count` before notification and audit, so failure after the update can leave partial state.

2026-04-28 status: fixed. Moderation writes are transactional, check target rows, propagate audit failures, audit mod-note updates, and keep warning notification creation in the same transaction.

Expected:

Moderation mutations should be transactional, verify the target profile exists, return 404 or conflict for stale targets, and persist the profile change, notification where applicable, and audit row atomically.

Evidence:

Handlers call `.execute(&c_pool).await?` and ignore the result count; `crate::community::audit::log(...).await;` is not propagated in several handlers.

Recommended fix:

Use a transaction, select/update the target with row existence checks, propagate audit insert failures, and audit mod-note changes with old/new length or a redacted diff.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static regression tests | `python3 -m pytest tests/admin/test_admin_community_user_detail_static.py -q` | Page has no unsafe rendering/native dialogs/CDNs; page loads CSRF helper; backend has permissions, transactions, row checks, audit logging, ban validation, and detail moderation fields. | 4 passed. | Pass |
| E2E regression suite | `BASE_URL=http://localhost:8893 python3 -m pytest tests/e2e/test_admin_community_user_detail.py -q` | Authenticated API/browser checks for permissions, CSRF, stale rows, audit persistence, safe rendering, and dialog warning flow. | 2 passed. | Pass |
| Inline JS syntax after fix | Extracted inline script to `/tmp/admin-community-user-detail-inline.js`; ran `node --check`. | Syntax passes. | Passed with no syntax output. | Pass |
| Unsafe pattern scan after fix | `rg "innerHTML|outerHTML|onclick=|prompt\\(|alert\\(|confirm\\(|https://unpkg|cdn.jsdelivr" frontend/platform/admin/community/user-detail.html` | No matches. | No matches. | Pass |
| Broad backend formatting | `cargo fmt --check` | Backend is formatted. | Passed after running `cargo fmt`. | Pass |
| Page-local Rust format | `rustfmt --edition 2021 --check backend/src/community/routes.rs` | Touched route file is formatted. | Passed. | Pass |
| Broad backend compile/runtime | `CARGO_TARGET_DIR=/tmp/poool-cargo-check-community cargo check`; `PORT=8893 SERVER_PORT=8893 CARGO_TARGET_DIR=/tmp/poool-cargo-check-community cargo run`; `curl http://localhost:8893/health` | Backend compiles/starts for E2E. | `cargo check` passed in an isolated target. Current backend started on `:8893`; health returned OK. | Pass |
| Unauthenticated page request | `curl -i http://localhost:8888/admin/community/user-detail?id=00000000-0000-0000-0000-000000000000` | Redirect to login, no page data. | `303 See Other`, `location: /auth/login`. | Pass |
| Unauthenticated detail API | `curl -i http://localhost:8888/api/admin/community/users/00000000-0000-0000-0000-000000000000/detail` | 401 JSON auth failure. | `401 Unauthorized`, `{"error":"Authentication required"}`. | Pass |
| Unauthenticated mutation CSRF | POST warning without cookies/token. | Request rejected. | `403 Forbidden` CSRF JSON. | Pass |
| Authenticated detail happy path | Load page with admin session and a real community user fixture. | Detail renders safely and no console errors. | E2E passed against current backend on `:8893`. | Pass |
| Authenticated moderation actions | Warn/mute/ban/shadowban/mod-notes with safe fixture. | State changes and audit/notification rows verified. | E2E API test passed against current backend on `:8893`. | Pass |

2026-04-27 continuation note: `cargo run` started successfully enough for safe smoke checks, but emitted existing local migration/idempotency errors during startup. The server still handled the unauthenticated page/API/CSRF probes above. No production application code was modified.

---

## Security Findings

- Fixed: Admin-only XSS risk from unescaped user, moderator, badge, and post data rendered through `innerHTML`.
- Fixed: Backing APIs lacked fine-grained `community.view` / `community.manage` authorization even though the HTML route is gated.
- Fixed: Moderation mutations could return success without updating a target profile and could leave unaudited or partially persisted state.
- CSRF middleware rejected an unauthenticated POST without a valid CSRF token, and the page now loads `csrf.js` and sends `X-CSRF-Token` for moderation fetches.

---

## Database Findings

- Required schema support exists for `community_profiles`, `posts`, `badges`, `user_badges`, `muted_until`, `mod_notes`, and `is_shadowbanned`.
- Fixed: moderation handlers now use transactions, row-count checks, propagated audit inserts, and mod-note audit logging.

---

## Remaining Issues

- None for the documented findings in this audit. Optional follow-up coverage: broader mobile visual and cross-browser dialog checks.

---

## Recommended Fix Order

1. Keep the targeted static and E2E regression files in the suite.
2. Add broader mobile visual and cross-browser dialog checks if this admin page receives further UI changes.

---

## Final Status

`fixed`

Reason: The documented code issues are fixed and static regression, formatting, isolated `cargo check`, health, and authenticated browser/API E2E coverage pass against the current backend.
