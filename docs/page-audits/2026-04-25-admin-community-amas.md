# Page Audit: Admin Community AMAs

Date: 2026-04-25
Status: fixed, E2E verified
Auditor: ChatGPT/Codex
Page URL: `/admin/community/amas`
Template: `frontend/platform/admin/community/amas.html`
JavaScript: inline script in `frontend/platform/admin/community/amas.html`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/fonts.css`, `frontend/platform/static/css/bundle.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/community/routes.rs`, `backend/src/community/amas.rs`

---

## Summary

The `/admin/community/amas` page has a working admin route and management surface for listing AMAs, creating AMAs, changing status, viewing questions, answering questions, and featuring questions. The documented CSRF, permission, draft-detail exposure, audit-log, validation, and modal accessibility issues were fixed on 2026-04-25.

Targeted Playwright E2E now verifies the authenticated admin CRUD/moderation flow for create, status, detail, answer, feature, community audit logs, modal keyboard behavior, and mobile modal layout.

---

## Fix Follow-Up

Date: 2026-04-25

Fixed:

- Added explicit CSRF-aware fetch handling on the AMA admin page.
- Replaced dynamic HTML string row rendering with DOM construction and delegated event handlers.
- Switched the admin question panel to a dedicated admin AMA detail endpoint.
- Added `community.view` and `community.manage` permission gates to the AMA page/API surface.
- Added `database/083_admin_community_permissions.sql` for community admin permissions.
- Blocked public authenticated draft AMA detail reads.
- Added backend validation for create/status/answer inputs.
- Made status updates return 404 for missing AMA IDs.
- Bound answer/feature mutations to the selected AMA ID.
- Added community audit logs for create, status, answer, and feature/unfeature actions.
- Added accessible dialog semantics, focus management, Escape handling, Tab containment, and loading button states for the modals.
- Fixed the shared admin fetch interceptor so it does not duplicate `X-CSRF-Token` when a page-level API wrapper already set it.
- Added `tests/e2e/test_admin_community_amas.py` for the authenticated browser CRUD/moderation regression.
- Added mobile-specific AMA page layout rules so the header action and modals remain clickable and contained on narrow visual viewports.
- Anchored open modals to the browser visual viewport to avoid off-screen centering on mobile emulation.
- Extended the AMA E2E with Tab trapping, Escape close/focus return, and authenticated mobile modal smoke coverage.

Verification:

- `node --check` on the inline page script passed.
- `cargo fmt --check` passed.
- `cargo check` passed with existing warnings.
- `cargo test --no-run` passed with existing warnings.
- Scoped `git diff --check` passed.
- `BASE_URL=http://localhost:8895 DATABASE_URL=postgres://martin@localhost/poool COMMUNITY_DATABASE_URL=postgres://martin@localhost/poool_community python3 -m pytest tests/e2e/test_admin_community_amas.py -q` passed.
- `BASE_URL=http://localhost:8896 DATABASE_URL=postgres://martin@localhost/poool COMMUNITY_DATABASE_URL=postgres://martin@localhost/poool_community python3 -m pytest tests/e2e/test_admin_community_amas.py -q` passed with 2 tests on 2026-04-26.

Remaining recheck:

- None for the documented AMA production-readiness gaps.

---

## Tested Scope

- Read `AGENTS.md`, `docs/AGENT_DEVELOPMENT_PROMPT.md`, `docs/IMPLEMENTATION_ROADMAP.md`, `BROKEN_LOGICS.md`, `docs/DATABASE_SCHEMA.md`, `docs/FRONTEND_COMPONENTS.md`, `docs/TECH_STACK.md`, `docs/SECURITY.md`, `docs/DESIGN.md`, and `docs/automation-prompts/PRODUCTION_READINESS_STANDARDS.md`.
- Reviewed `docs/page-review-tracker.yml` and selected the first fully unreviewed page according to tracker order.
- Reviewed the AMA admin template, inline JavaScript, backend page route, backend API route registration, community AMA service functions, and AMA migration.
- Checked for existing tests covering admin AMA routes.
- Ran inline JavaScript syntax validation with `node --check`.
- Initial audit did not modify production application code; the follow-up fix updated the page, backend handlers, shared admin CSRF interceptor, migration, docs, and E2E test coverage.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/amas` | Registered by `page_admin_generic`. |
| Route alias | `/admin/community/amas.html` | Registered by `page_admin_generic`. |
| Template | `frontend/platform/admin/community/amas.html` | Contains all page markup and inline JS. |
| Shared component | `frontend/platform/admin/components/sidebar.html` | Included by template. |
| JS | inline script | Owns list, create, status, detail, answer, and feature behavior. |
| JS | `frontend/platform/static/js/user-data.js` | Shared user data behavior. |
| JS | `frontend/platform/static/js/admin-theme.js` | Shared admin theme behavior. |
| JS | `frontend/platform/static/js/admin-permission-guard.js` | Frontend-only permission affordances. |
| Backend page route | `GET /admin/community/amas` | `backend/src/admin/mod.rs`. |
| Backend API route | `GET /api/admin/community/amas` | Lists all AMAs, including drafts. |
| Backend API route | `POST /api/admin/community/amas` | Creates AMA. CSRF-aware UI path verified by E2E. |
| Backend API route | `POST /api/admin/community/amas/:id/status` | Updates AMA status. CSRF-aware UI path verified by E2E. |
| Backend API route | `GET /api/admin/community/amas/:id` | Admin-only detail endpoint used by admin question panel. |
| Backend API route | `POST /api/admin/community/amas/:id/questions/:qid/answer` | Answers question. CSRF-aware UI path verified by E2E. |
| Backend API route | `POST /api/admin/community/amas/:id/questions/:qid/feature` | Toggles featured state. CSRF-aware UI path verified by E2E. |
| Database table | `amas` | AMA records and status. |
| Database table | `ama_questions` | User questions, answers, featured flag. |
| Database table | `ama_question_upvotes` | Upvote join table with trigger-maintained counts. |
| Database table | `community_audit_logs` | Exists, but AMA admin actions do not write to it. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `nav.admin-breadcrumbs` | Navigate to admin and community index. | Link navigation. | Page routes expected. | Static only; route existence not exhaustively tested. |
| New AMA button | line 45 | Open create modal. | Yes, inline `openCreateAmaModal()`. | No backend until submit. | Static behavior wired. |
| AMA table loading state | `#amas-table` | Show loading then list/empty/error rows. | Yes, `loadAmas()` and `renderAmas()`. | `GET /api/admin/community/amas`. | Authenticated admin path verified by E2E. |
| Empty AMA state | `renderAmas([])` | Show "No AMAs yet" row. | Yes. | Not needed. | Static behavior wired. |
| Error row | `loadAmas()` catch | Show failed-to-load row. | Yes. | Backend failures surface generically. | Static behavior wired. |
| Questions button | dynamic row button | Open detail panel and load questions. | Yes, `viewQuestions()`. | Uses `/api/admin/community/amas/:id`. | Authenticated admin path verified by E2E. |
| Status select | dynamic row select | Change AMA status. | Yes, `changeStatus()`. | `POST /api/admin/community/amas/:id/status`. | Authenticated admin path verified by E2E. |
| Detail panel close | line 81 | Hide panel and clear selected AMA. | Yes. | Not needed. | Static behavior wired. |
| Create modal | `#ama-modal` | Capture title, description, expert, schedule, status. | Yes. | `POST /api/admin/community/amas`. | Authenticated admin path verified by E2E. |
| Create modal title input | `#ama-title` | Required title. | Client checks non-empty. | DB `VARCHAR(300) NOT NULL`; no service validation. | Missing length validation/error clarity. |
| Create modal expert input | `#ama-expert-name` | Required expert name. | Client checks non-empty. | DB `VARCHAR(200) NOT NULL`; no service validation. | Missing length validation/error clarity. |
| Create modal scheduled input | `#ama-scheduled` | Optional datetime. | Converts to ISO string. | Expects `DateTime<Utc>`. | Browser invalid date edge cases not handled. |
| Create modal status select | `#ama-status` | Initial status. | Sends selected value. | DB/check validates known statuses indirectly. | Invalid/stale values can become DB errors. |
| Create cancel/close | lines 96, 132 | Close modal. | Yes. | Not needed. | Lacks modal a11y semantics/focus handling. |
| Create submit | line 133 | Create AMA and reload list. | Yes. | Backend exists. | Broken until CSRF header is added. |
| Answer modal | `#answer-modal` | Display question and submit answer. | Yes. | `POST /api/admin/community/amas/:id/questions/:qid/answer`. | Authenticated admin path verified by E2E. |
| Answer textarea | `#answer-text` | Required answer text. | Client checks non-empty only. | Backend accepts any string. | Missing length/server validation. |
| Feature button | dynamic row button | Toggle featured flag. | Yes. | `POST /api/admin/community/amas/:id/questions/:qid/feature`. | Authenticated admin path verified by E2E. |

---

## Frontend Findings

### P1 - Admin AMA mutations omit CSRF tokens

Location:

- Template: `frontend/platform/admin/community/amas.html:241`, `frontend/platform/admin/community/amas.html:264`, `frontend/platform/admin/community/amas.html:345`, `frontend/platform/admin/community/amas.html:360`
- Middleware: `backend/src/auth/csrf.rs`

Problem:

The page sends all state-changing fetches with only `Content-Type: application/json` or no headers. The global CSRF middleware requires `X-CSRF-Token` for all `POST`, `PUT`, `DELETE`, and `PATCH` requests outside webhook/static paths. The template also does not load `frontend/platform/static/js/csrf.js`.

Expected:

Every admin AMA mutation should include the CSRF token via shared fetch helper or explicit `X-CSRF-Token: window.getCsrfToken()`.

Evidence:

The inline fetch calls for create/status/answer/feature omit the header. The CSRF middleware rejects API mutations without it with a 403 JSON error.

Recommended fix:

Load the shared CSRF helper and route all inline fetch calls through a small local `jsonFetch()` wrapper that adds `Content-Type`, `X-CSRF-Token`, `credentials: 'same-origin'`, parses JSON errors, and disables submit controls during in-flight requests.

---

### P2 - Modal controls are mouse-oriented and lack dialog semantics

Location:

- Template: `frontend/platform/admin/community/amas.html:91`
- Template: `frontend/platform/admin/community/amas.html:139`

Problem:

The Create AMA and Answer Question overlays are plain `div` trees with inline styles. They do not set `role="dialog"`, `aria-modal="true"`, `aria-labelledby`, focus-on-open, focus return, Escape close, or focus trapping. This is a regression risk for keyboard and screen-reader users on a moderation page.

Expected:

Admin modals should follow the shared modal accessibility baseline already used on other hardened admin pages.

Evidence:

Open/close only toggles `style.display`; no keyboard handlers or ARIA attributes are present.

Recommended fix:

Move the inline modal behavior into page JS or a shared admin modal helper, add dialog semantics, focus management, Escape/backdrop close, and disabled/loading button states.

---

### P2 - Dynamic rows are rendered through large HTML strings

Location:

- Template: `frontend/platform/admin/community/amas.html:189`
- Template: `frontend/platform/admin/community/amas.html:301`

Problem:

The code uses `innerHTML` for dynamic AMA and question rows. User-controlled fields are escaped with `escHtml()`, which reduces XSS risk for current fields, but the pattern is fragile and mixes escaped data into inline event-handler attributes.

Expected:

Use DOM construction or trusted templates with event delegation, especially for question text inserted into `onclick` arguments.

Evidence:

`openAnswerModal('${q.id}', '${escHtml(q.question).replace(/'/g, "\\'")}')` builds JavaScript code inside HTML attributes from question data.

Recommended fix:

Render rows with `document.createElement`, put IDs/text in `data-*` attributes or closures, and bind actions through delegated listeners.

---

## Backend Findings

### P1 - Admin AMA APIs lack fine-grained community permissions

Location:

- Backend: `backend/src/community/routes.rs:3285`
- Backend: `backend/src/community/routes.rs:3307`
- Backend: `backend/src/community/routes.rs:3336`
- Backend: `backend/src/community/routes.rs:3352`
- Backend: `backend/src/community/routes.rs:3365`

Problem:

All admin AMA API handlers only extract `AdminUser`. They do not call `admin.require_permission(...)`, despite the admin permission system being available and used by other admin domains. Any user with broad `admin` or `super_admin` role can list drafts, create AMAs, publish/close/archive AMAs, answer user questions, and feature questions.

Expected:

Read actions should require a community read/moderation permission, and mutations should require a community manage/moderate permission. The page route should also enforce the same read permission instead of relying on generic page rendering.

Evidence:

The handlers accept `AdminUser` and proceed directly to the community DB. The page is registered with `page_admin_generic` in `backend/src/admin/mod.rs`.

Recommended fix:

Create or reuse explicit permissions such as `community.view`, `community.moderate`, and `community.manage`; grant them by migration; enforce them on the page route and API handlers.

---

### P1 - Public AMA detail endpoint exposes draft AMA details by UUID

Location:

- Frontend: `frontend/platform/admin/community/amas.html:284`
- Backend: `backend/src/community/routes.rs:3216`
- Backend: `backend/src/community/amas.rs:108`

Problem:

The admin question panel calls `/api/community/amas/:id`, which requires only a normal authenticated user. The service query fetches `FROM amas WHERE id = $1` without filtering out `draft` or otherwise unpublished statuses. As a result, any authenticated user who obtains or guesses a draft AMA UUID can read the draft AMA record and its questions.

Expected:

Public AMA detail should only return statuses visible to users, or an admin-only detail endpoint should be used for admin moderation.

Evidence:

The public list filters `status != 'draft'`, but detail does not. The admin list displays the first eight characters of AMA IDs and loads details via the public detail endpoint.

Recommended fix:

Add `GET /api/admin/community/amas/:id` for admin detail with permission checks, and update public `get_ama_detail` to reject `draft` AMAs for non-admin users.

---

### P2 - AMA create/status/answer inputs rely on DB or weak validation

Location:

- Backend: `backend/src/community/routes.rs:3296`
- Backend: `backend/src/community/amas.rs:248`
- Backend: `backend/src/community/amas.rs:283`
- Database: `database/community/009_amas.sql:7`

Problem:

Create AMA does not trim or validate title/expert length/status before inserting. Invalid status or overlong title/expert fields bubble up as database errors instead of controlled 400 responses. Answer submission only checks non-empty in the browser; the backend accepts arbitrary answer length.

Expected:

Backend validation should enforce required fields, max lengths matching schema, valid statuses, schedule sanity, and answer length with clear client-safe errors.

Evidence:

The create handler passes payload fields directly into `create_ama()`. The service passes status into the DB without the same explicit status allowlist used by `update_ama_status()`.

Recommended fix:

Add a `validate_create_ama` and `validate_answer` path before DB writes. Return `AppError::BadRequest` for invalid payloads.

---

### P2 - Status update reports success for missing AMA IDs

Location:

- Backend: `backend/src/community/amas.rs:303`
- Backend: `backend/src/community/routes.rs:3336`

Problem:

`update_ama_status()` executes an `UPDATE` but never checks `rows_affected()`. A valid UUID that matches no AMA returns `{"success": true}`.

Expected:

Missing AMA IDs should return 404 so the UI can show a real stale-row/error state.

Evidence:

The service executes the update and returns `Ok(())` in all three status branches without checking whether a row was changed.

Recommended fix:

Capture the `PgQueryResult`, check `rows_affected()`, and return `AppError::NotFound("AMA not found")` when zero rows change.

---

### P2 - AMA moderation mutations are not audit logged

Location:

- Backend: `backend/src/community/amas.rs:248`
- Backend: `backend/src/community/amas.rs:283`
- Backend: `backend/src/community/amas.rs:337`
- Backend: `backend/src/community/amas.rs:392`
- Audit table: `database/community/018_community_audit_log.sql`

Problem:

Create AMA, status change, answer question, and feature/unfeature question are admin-sensitive community moderation actions, but they do not write to `community_audit_logs` or `audit_logs`.

Expected:

Each admin AMA mutation should write actor, action, entity type, entity ID, target user where applicable, previous/new state, and timestamp.

Evidence:

Other community admin handlers call `crate::community::audit::log(...)`, but AMA handlers do not.

Recommended fix:

Wrap status/answer/feature changes in service-level logic that captures previous state and calls `community::audit::log`. For create, log the new AMA ID and initial status.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Tracker selection | Parsed `docs/page-review-tracker.yml` for in-progress and not-reviewed pages. | Select exactly one first unreviewed page. | Selected `admin.community.amas`. | Pass |
| Static template review | Inspected `frontend/platform/admin/community/amas.html`. | Inventory all visible controls and JS hooks. | Controls and inline fetches documented. | Pass |
| Backend route review | Inspected `backend/src/admin/mod.rs`, `backend/src/community/routes.rs`, `backend/src/community/amas.rs`. | Verify page route and API support. | Routes exist; permission, CSRF integration, audit, and validation gaps found. | Issues found |
| Database review | Inspected `database/community/009_amas.sql` and audit log references. | Verify AMA tables and constraints. | Core AMA tables exist; community audit table exists but is unused by AMA mutations. | Issues found |
| JS syntax | Extracted inline script and ran `node --check`. | No syntax errors. | Command exited 0. | Pass |
| Existing test coverage scan | Ran `rg` for admin AMA route/test coverage. | Locate tests or document gap. | No relevant tests found. | Missing coverage |
| Runtime browser/API test | Start backend and exercise page with admin session. | Page load, create, status, detail, answer, feature, audit-log evidence. | Targeted Playwright E2E passed against local backend on port 8895. | Pass |
| Keyboard/browser recheck | Exercise create and answer modals with Tab, Shift+Tab, Escape, and focus-return assertions. | Focus stays trapped while open and returns to opener on Escape. | Extended Playwright E2E passed against local backend on port 8896. | Pass |
| Mobile browser recheck | Load `/admin/community/amas` with mobile admin viewport, open create modal, verify focused field and modal bounds. | Page action is clickable and modal stays within viewport. | Initial E2E exposed mobile overlap/off-screen modal; scoped layout/visual viewport fix applied; rerun passed. | Pass |

---

## Security Findings

- P1: Admin AMA APIs only enforce broad admin role membership and should use fine-grained community permissions.
- P1: Public authenticated AMA detail endpoint can return draft AMA details and questions by UUID.
- P1: Frontend admin mutations omit CSRF tokens, so production middleware rejects them. This is a functional break and also proves the page is not aligned with the platform's CSRF contract.
- P2: AMA admin mutations are not audit logged.
- P2: Create/answer payloads need backend validation with controlled 400 responses.
- P2: Inline event handlers and dynamic `innerHTML` rendering should be replaced with safer event delegation/DOM construction.

---

## Database Findings

- `amas`, `ama_questions`, and `ama_question_upvotes` exist in `database/community/009_amas.sql`.
- `ama_questions.question` has a 10-500 character check, but `ama_questions.answer` has no length check and backend answer validation is weak.
- `amas.title`, `amas.expert_name`, and `amas.expert_title` have length-limited `VARCHAR` columns, but backend create validation does not enforce those limits before DB insert.
- `created_by`, `user_id`, and `answered_by` are UUIDs without foreign keys to core `users`, probably because this is the separate community database. The service should compensate with clear auth and audit records.
- `community_audit_logs` exists, but AMA mutation handlers do not write to it.

---

## Missing Tests

- Add an authenticated admin integration test for `GET /api/admin/community/amas` requiring the correct community permission.
- Add CSRF tests for `POST /api/admin/community/amas`, status, answer, and feature routes.
- Add authorization tests proving non-admin users and admins without community permissions cannot create, publish, answer, or feature AMAs.
- Add public detail tests proving draft AMAs are not visible through `/api/community/amas/:id`.
- Add validation tests for title/expert/status/scheduled_at/answer length.
- Add status update tests for nonexistent AMA IDs returning 404.
- Add E2E coverage for create modal validation, status changes, answer modal, feature toggle, network errors, keyboard modal behavior, and mobile layout.

---

## Recommended Fix Order

1. Add CSRF header support to all admin AMA fetch calls so mutations can work.
2. Add fine-grained community permissions to the page route and admin AMA API handlers.
3. Split admin AMA detail from public AMA detail, and block draft details on public endpoints.
4. Add backend validation for create/status/answer payloads and 404 handling for missing status targets.
5. Add community audit logging for create/status/answer/feature mutations.
6. Refactor inline HTML/event rendering and modal behavior for safer DOM updates and accessible keyboard support.
7. Add targeted integration/E2E tests for the fixed flows.

---

## Final Status

`fixed_e2e_verified`

Reason: The documented implementation findings were fixed and the targeted authenticated browser E2E now passes for the admin AMA CRUD/moderation path, modal keyboard behavior, and mobile modal layout.
