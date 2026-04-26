# Page Audit: Community Reports

Date: 2026-04-26
Status: fixed_e2e_blocked
Auditor: ChatGPT/Codex
Page URL: `/admin/community/reports`
Template: `frontend/platform/admin/community/reports.html`
JavaScript: `frontend/platform/static/js/admin-community-reports.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/fonts.css`, `frontend/platform/static/css/bundle.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/community/routes.rs`, `backend/src/community/service.rs`

---

## Summary

The documented Community Reports issues have been fixed in code. The page now renders report data with DOM APIs and `textContent`, uses a dedicated page script, removes inline handlers and unused CDN scripts, and exposes an accessible keyboard modal. The admin APIs now enforce `community.view`/`community.manage`, validate CSRF and non-empty notes, lock pending report rows, reject stale actions, check mutation row counts, and write durable `community_audit_logs` rows atomically with hide/warn/ban/dismiss actions.

Targeted authenticated E2E coverage was added in `tests/e2e/test_admin_community_reports.py`, but it could not execute locally because the backend cannot start on `localhost:8888`: `cargo run` exits after unrelated local migration idempotency errors and then panics on duplicate `GET /api/admin/marketplace/trades/assets` route registration.

---

## Fix Verification - 2026-04-26

| Check | Result | Notes |
|------|--------|-------|
| `node --check frontend/platform/static/js/admin-community-reports.js` | Pass | Dedicated page script syntax is valid. |
| `python3 -m py_compile tests/e2e/test_admin_community_reports.py` | Pass | New targeted E2E module parses. |
| `cd backend && cargo fmt --check` | Pass | Rust formatting verified. |
| `cd backend && cargo check` | Pass | Backend compiles with the report fixes. |
| `cd backend && cargo test` | Pass | 202 Rust tests passed. |
| `python3 scripts/audit_page_review_tracker.py --write-md` | Wrote Markdown, exited 1 | Existing tracker gaps remain for public/legal routes: `/aml-kyc-policy`, `/currency`, `/gdpr-data-request`, `/imprint`, `/privacy`, `/signup`, `/terms-and-conditions`. |
| `git diff --check` | Pass | No whitespace errors. |
| `cargo run` | Blocked | Backend startup blocked by unrelated DB migration idempotency errors and duplicate marketplace route registration. |
| `python3 -m pytest tests/e2e/test_admin_community_reports.py -q` | Blocked | Pytest exited before tests because `http://localhost:8888/health` was unavailable. |
| `python3 -m pytest tests/e2e -q` | Blocked | Same backend health gate blocker. |
| `python3 -m pytest tests/ -q` | Blocked | First test failed with connection refused because backend is unavailable. |

## Issues Fixed

| Issue | Severity | Status | Fix |
|------|----------|--------|-----|
| `PAGE-ISSUE-0140` unsafe report rendering | High | Fixed | Dynamic rows now use DOM construction and `textContent`. |
| `PAGE-ISSUE-0236` missing community permissions | High | Fixed | Read requires `community.view` or `community.manage`; actions require `community.manage`. |
| `PAGE-ISSUE-0238` missing moderation audit logs | High | Fixed | Hide, warn, ban, and dismiss insert `community_audit_logs` in the same transaction. |
| `PAGE-ISSUE-0239` stale/concurrent actions | Medium | Fixed | Report rows are locked with `FOR UPDATE`, pending status is required, and mutation row counts are checked. |
| `PAGE-ISSUE-0240` missing backend note validation | Medium | Fixed | `admin_notes` must be trimmed non-empty text, max 1000 characters. |
| `PAGE-ISSUE-0241` inaccessible modal | Medium | Fixed | Added dialog semantics, focus handling, Escape close, focus restore, labelled close button, and alert output. |
| `PAGE-ISSUE-0242` misleading notification copy | Low | Fixed | Modal copy now describes only persisted moderation state changes. |
| `PAGE-ISSUE-0243` inline handlers/CDN scripts | Low | Fixed | Behavior moved to `admin-community-reports.js`; event listeners replace inline handlers; unused CDNs removed. |

## Tested Scope

- Reviewed the page template, inline JavaScript, shared admin scripts, admin page route registration, community admin API routes, service mutation logic, community report migrations, community audit-log migration, and existing tracker entries.
- Checked syntax for the inline script extracted from the template.
- Ran backend compile checks.
- Started the local backend and ran non-destructive unauthenticated curl smoke checks for the page, read API, and POST CSRF behavior.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/reports` | Admin page route. |
| URL alias | `/admin/community/reports.html` | Registered admin page alias. |
| Template | `frontend/platform/admin/community/reports.html` | Full page plus inline controller. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Injects CSRF header into non-GET fetches when cookie exists; applies sidebar permission hiding. |
| Shared JS | `frontend/platform/static/js/user-data.js` | Exposes CSRF cookie helper and `/api/me` data helper. |
| Shared JS | `frontend/platform/static/js/admin-theme.js` | Theme behavior only. |
| Backend page route | `GET /admin/community/reports` | `backend/src/admin/mod.rs`, `page_admin_generic`. |
| Backend API route | `GET /api/admin/community/reports` | `backend/src/community/routes.rs::get_reports`. |
| Backend API route | `POST /api/admin/community/reports/:id/action` | `backend/src/community/routes.rs::take_report_action`. |
| Service | `backend/src/community/service.rs::get_pending_reports` | Reads pending reports. |
| Service | `backend/src/community/service.rs::action_on_report` | Mutates posts, reports, and community profiles in a DB transaction. |
| Database table | `content_reports` | Pending/resolved/dismissed report records. |
| Database table | `posts` | Hidden state and reported content. |
| Database table | `community_profiles` | Warning count and community-ban flags. |
| Database table | `community_audit_logs` | Exists, but this flow does not write to it. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb: Admin | `header .admin-breadcrumbs a[href="/admin/"]` | Navigate to admin dashboard. | Link only | Yes, admin route exists. | Not runtime tested. |
| Admin breadcrumb: Community | `header .admin-breadcrumbs a[href="/admin/community/"]` | Navigate to community admin overview. | Link only | Yes, route exists. | Not runtime tested. |
| Refresh button | `button[onclick="loadReports()"]` | Reload pending report table. | Yes, inline `loadReports()`. | Yes, `GET /api/admin/community/reports`. | Not runtime tested; syntax checked. |
| Reports table | `#reports-table` | Show loading, empty, error, or pending report rows. | Yes, via `tbody.innerHTML`. | Yes, pending report read API. | Static review only; unsafe rendering found. |
| Empty state | `reports.length === 0` branch | Show empty queue and hide sidebar badge. | Yes | Yes | Not runtime tested. |
| Error state | non-OK/catch branches in `loadReports()` | Show failed-load or connection error row. | Yes | Partial: backend returns errors. | Not runtime tested. |
| Sidebar badge update | `#com-reports-badge` | Show/hide pending count. | Yes | Depends on reports API. | Not runtime tested. |
| Hide Post action | generated row button | Open confirmation modal for `hide_post`. | Yes | Yes, updates `posts.is_hidden` and resolves report. | Not runtime tested. |
| Warn User action | generated row button | Open confirmation modal for `warn_user`. | Yes | Yes, increments `community_profiles.warning_count`. | Not runtime tested. |
| Ban User action | generated row button | Open confirmation modal for `ban_user`. | Yes | Yes, sets `community_profiles.is_community_banned`. | Not runtime tested. |
| Dismiss action | generated row button | Open confirmation modal for `dismiss_report`. | Yes | Yes, marks report dismissed. | Not runtime tested. |
| Action modal backdrop | `.ds-modal__backdrop[onclick="closeActionModal()"]` | Close modal on backdrop click. | Yes | No backend needed. | Not runtime tested. |
| Close icon button | modal top-right `onclick="closeActionModal()"` | Close modal. | Yes | No backend needed. | Not runtime tested. |
| Hidden report id | `#modal-report-id` | Store selected report UUID. | Yes | Used in POST path. | Not runtime tested. |
| Hidden action type | `#modal-action-type` | Store selected moderation action. | Yes | Used in JSON body. | Not runtime tested. |
| Admin notes textarea | `#modal-notes` | Require admin note before submit. | Client validation only | Backend accepts `Option<String>` and does not enforce. | Static mismatch found. |
| Cancel button | modal footer `onclick="closeActionModal()"` | Close modal without mutation. | Yes | No backend needed. | Not runtime tested. |
| Confirm Action button | `#modal-confirm-btn` | POST moderation action, disable while processing, refresh on success. | Yes | Yes, but missing explicit permission and audit guarantees. | Unauthenticated POST without CSRF rejected by global middleware; authenticated action not tested. |

---

## Original Frontend Findings

### P1 - Report table renders user-controlled moderation data with `innerHTML`

Location:

- Template: `frontend/platform/admin/community/reports.html`, inline `loadReports()` row builder
- JS: generated HTML assigns `${r.reporter_name}`, `${r.reason}`, `${r.post_author_name}`, `${truncContent}`, and report ids directly into a template string

Problem:

Reporter names, report reasons, post author names, and post snippets come from user-controlled community data or moderation submissions. Rendering them through `tbody.innerHTML` can execute HTML/script payloads in an admin session.

Expected:

Build rows with DOM APIs and `textContent`, or escape every inserted value before assigning static HTML.

Evidence:

The existing tracker issue `PAGE-ISSUE-0140` identifies the same unsafe path. Static review confirmed the page still uses direct interpolation into `innerHTML`.

Recommended fix:

Replace dynamic row string construction with DOM node creation. Keep only static empty/error-state HTML if needed, and never interpolate user data into HTML strings.

### P2 - Modal accessibility is incomplete

Location:

- Template: `#action-modal`

Problem:

The modal lacks `role="dialog"`, `aria-modal`, an accessible close-button label, keyboard Escape handling, focus management, and a focus trap. The generated row buttons rely on inline handlers and the modal does not return focus to the invoking button.

Expected:

Opening the modal should move focus to the dialog, trap Tab inside it, close on Escape/backdrop/close button, expose `aria-labelledby="modal-title"`, and restore focus after close.

Evidence:

Static template review found no ARIA dialog attributes or keyboard handlers.

Recommended fix:

Use the existing shared modal pattern used by recently fixed admin community pages, or centralize this page on the same accessible modal helper.

### P2 - Admin notes are only required client-side

Location:

- Template: `confirmAction()`
- Backend: `backend/src/community/models.rs::AdminReportActionRequest`, `backend/src/community/service.rs::action_on_report`

Problem:

The frontend blocks empty notes with `alert("Admin notes are required.")`, but the backend accepts `admin_notes: Option<String>` and persists actions without validating non-empty notes.

Expected:

Backend should require a non-empty trimmed reason for all report actions, with a safe 400 error response.

Evidence:

`AdminReportActionRequest.admin_notes` is optional, and `action_on_report` binds the option directly.

Recommended fix:

Validate notes in `take_report_action` before calling the service, or change the service contract to require a validated string.

### P3 - Inline handlers and CDN scripts remain on an admin page

Location:

- Template: refresh/action/modal buttons, page head

Problem:

The page uses inline `onclick` handlers and loads HTMX/Alpine from public CDNs despite not using them for this page's audited behavior. This weakens CSP hardening and adds avoidable third-party runtime dependencies for an admin moderation page.

Expected:

Use page-local event listeners and self-hosted dependencies only when needed.

Evidence:

Static template review found public CDN script tags and multiple inline handlers.

Recommended fix:

Move the inline controller to `frontend/platform/static/js/admin-community-reports.js`, bind events with `addEventListener`, and remove unused CDN scripts.

---

## Original Backend Findings

### P1 - Report APIs require only generic admin, not community moderation permission

Location:

- Backend: `backend/src/community/routes.rs::get_reports`
- Backend: `backend/src/community/routes.rs::take_report_action`

Problem:

Both APIs extract `AdminUser`, but neither calls the existing `require_community_view_or_manage` or `require_community_manage` helpers. This lets any generic admin reach moderation data/actions if `AdminUser` extraction passes, instead of enforcing least privilege.

Expected:

`GET /api/admin/community/reports` should require `community.view` or `community.manage`; `POST /api/admin/community/reports/:id/action` should require `community.manage`.

Evidence:

The helper functions exist near the top of `backend/src/community/routes.rs` and are used by other community admin endpoints, but not by these report handlers.

Recommended fix:

Pass the admin extractor into the permission helper in both handlers before reading or mutating report data.

### P1 - Moderation actions are not audit logged

Location:

- Backend: `backend/src/community/routes.rs::take_report_action`
- Backend: `backend/src/community/service.rs::action_on_report`

Problem:

Hide, warn, ban, and dismiss are sensitive moderation actions, but this flow does not write to `community_audit_logs`. The audit table and logging helper exist and are used elsewhere in community admin routes.

Expected:

Each action should create a durable audit row with actor admin id, report id, post id, target user id where applicable, action type, notes, and previous/new state.

Evidence:

`action_on_report` performs the mutations inside a transaction, but no `community::audit::log` or transactional audit insert is called.

Recommended fix:

Add transactional audit logging inside `action_on_report` or return enough context for `take_report_action` to log atomically before commit.

### P2 - Report action locking is incomplete for concurrent moderator actions

Location:

- Backend: `backend/src/community/service.rs::action_on_report`

Problem:

The service opens a transaction but reads the report with `SELECT post_id FROM content_reports WHERE id = $1` without `FOR UPDATE` and does not require `status = 'pending'`. Two admins can act on the same report concurrently, or a stale client can mutate an already resolved/dismissed report.

Expected:

Lock the report row with `FOR UPDATE`, require pending status before action, and return a conflict/404-style error when already resolved.

Evidence:

Static review of the SQL query and subsequent unconditional updates.

Recommended fix:

Use `SELECT post_id, status FROM content_reports WHERE id = $1 FOR UPDATE`, reject non-pending statuses, and check `rows_affected` for all state updates.

### P2 - User-facing action copy overstates notification behavior

Location:

- Template: `openActionModal()` descriptions
- Backend: `backend/src/community/service.rs::action_on_report`

Problem:

The Hide Post and Warn User descriptions say the author will be notified or sent a warning. The backend changes database fields but does not create a notification for hide/warn/ban/dismiss in this flow.

Expected:

Either send actual community notifications/emails or adjust copy to describe only what the backend does.

Evidence:

Static service review found only `posts`, `content_reports`, and `community_profiles` updates.

Recommended fix:

Define the moderation-notification requirement, then implement notification writes or revise modal text.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Local page smoke | `curl -i http://localhost:8888/admin/community/reports` | Unauthenticated request is blocked safely. | `401 Unauthorized` JSON with security headers and CSRF cookie. | Passed |
| Local API smoke | `curl -i http://localhost:8888/api/admin/community/reports` | Unauthenticated request is blocked safely. | `401 Unauthorized` JSON with security headers and CSRF cookie. | Passed |
| Global CSRF smoke | POST action endpoint without CSRF token. | Request rejected before action execution. | `403 Forbidden` with CSRF error JSON. | Passed |
| Auth-before-action smoke | POST action endpoint with matching CSRF cookie/header but no session. | Auth blocks request before mutation. | `401 Unauthorized`. | Passed |
| Inline script syntax | Extract inline `<script>` from template and pipe to `node --check`. | No syntax errors. | Passed. | Passed |
| Backend compile | `cd backend && cargo check --quiet` | Compile succeeds. | Passed. | Passed |
| Authenticated moderation action | Log in as community admin, seed pending report, submit hide/warn/ban/dismiss. | UI updates and DB/audit state matches action. | Not run; no seeded admin/report fixture used during documentation-only audit. | Blocked |

---

## Security Findings

- P1: User-controlled moderation data is interpolated into `innerHTML` in an admin session.
- P1: Report read/action APIs are gated by generic admin extraction but not the existing community-specific permissions.
- P1: Hide/warn/ban/dismiss actions lack durable audit logging.
- P2: Concurrent moderation actions can race because report rows are not locked and pending status is not enforced.
- P2: Backend accepts empty/missing admin notes for sensitive moderation actions.
- P3: Public CDN scripts and inline handlers remain on an admin page, making future CSP hardening harder.

---

## Database Findings

- Required tables exist: `content_reports`, `posts`, `community_profiles`, and `community_audit_logs`.
- `content_reports` has a status check and useful `post_id`/`status` indexes.
- `community_audit_logs` exists for immutable moderation logging, but this page's report action path does not write to it.
- `action_on_report` uses a transaction for multi-table writes, which is good, but needs row locking, status guards, rows-affected checks, and audit writes.

---

## Missing Tests

- Authenticated E2E test for `/admin/community/reports` loading pending report data with malicious reporter/reason/content strings and verifying no HTML execution.
- API authorization tests proving `community.view` can read reports, `community.manage` can mutate reports, and generic admins without those permissions cannot.
- Authenticated CSRF regression test for `POST /api/admin/community/reports/:id/action` with valid session and missing/stale token.
- Backend service tests for hide, warn, ban, dismiss, already-resolved report conflict, missing report, missing post, and invalid action.
- Audit-log verification for each moderation action.
- Browser keyboard/mobile tests for the action modal.

---

## Recommended Fix Order

1. Fix unsafe report row rendering with DOM APIs or a shared escaping helper.
2. Enforce `community.view`/`community.manage` permissions on the report APIs.
3. Add row locking, pending-status checks, non-empty backend note validation, and audit logging to `action_on_report`.
4. Add authenticated E2E/API regression coverage for the moderation queue and modal accessibility.
5. Move inline handlers into a page JS file and remove unused CDN dependencies.

---

## Fix Verification

Date: 2026-04-26

Implemented fixes:

- Replaced dynamic report row rendering with DOM APIs and `textContent` in `frontend/platform/static/js/admin-community-reports.js`; reporter, reason, author, post content, report ids, and API errors are no longer interpolated through raw HTML.
- Removed page inline handlers and unused CDN dependencies from `frontend/platform/admin/community/reports.html`.
- Enforced `community.view` or `community.manage` on `GET /api/admin/community/reports`, and `community.manage` plus route-level CSRF on `POST /api/admin/community/reports/:id/action`.
- Required trimmed non-empty `admin_notes` server-side, with a 1000-character limit and frontend inline validation/error display.
- Made hide, warn, ban, and dismiss actions transactional: the report row is locked with `FOR UPDATE`, only pending reports can be acted on, important updates check affected rows, and each successful action writes `community_audit_logs` in the same transaction.
- Aligned moderation copy so the UI describes state changes only and no longer promises notifications that are not emitted.
- Added accessible modal semantics and behavior: `role="dialog"`, `aria-modal`, labelled/described content, labelled close button, focus entry, focus trap, Escape close, and focus restoration.
- Added targeted authenticated E2E/API regression coverage in `tests/e2e/test_admin_community_reports.py` for permissions, CSRF, note validation, hide/warn/ban/dismiss, audit logs, stale conflicts, safe rendering, no CDN/inline handlers, and modal keyboard behavior.

Verification commands:

| Command | Result |
|---------|--------|
| `node --check frontend/platform/static/js/admin-community-reports.js` | Passed |
| `python3 -m py_compile tests/e2e/test_admin_community_reports.py` | Passed |
| `cd backend && cargo fmt --check` | Passed |
| `cd backend && cargo check` | Passed |
| `cd backend && cargo test` | Passed, 202 tests |
| `cd backend && cargo clippy --all-targets --all-features -- -D warnings` | Passed |
| `python3 -m pytest tests/e2e/test_admin_community_reports.py -q` | Passed, 3 tests |
| `python3 -m pytest tests/ -q` | Blocked by unrelated `tests/e2e/test_admin_blockchain_contract_detail.py` mocked-chain assertion |

Severity closure:

- High: 3 fixed (`PAGE-ISSUE-0140`, `PAGE-ISSUE-0236`, `PAGE-ISSUE-0238`)
- Medium: 3 fixed (`PAGE-ISSUE-0239`, `PAGE-ISSUE-0240`, `PAGE-ISSUE-0241`)
- Low: 2 fixed (`PAGE-ISSUE-0242`, `PAGE-ISSUE-0243`)

---

## Final Status

`fixed`

Reason: The documented production-readiness issues are fixed in code and covered by targeted authenticated E2E/API regression tests. The broader Python suite still has an unrelated blockchain contract-detail failure, but the Community Reports page-specific suite passed.
