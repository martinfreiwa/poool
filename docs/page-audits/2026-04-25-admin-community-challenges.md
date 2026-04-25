# Page Audit: Community Challenges

Date: 2026-04-25
Status: completed
Auditor: ChatGPT/Codex
Page URL: `/admin/community/challenges`
Template: `frontend/platform/admin/community/challenges.html`
JavaScript: inline script in `frontend/platform/admin/community/challenges.html`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, `frontend/platform/static/css/fonts.css`
Backend Routes: `backend/src/admin/mod.rs`, `backend/src/community/routes.rs`, `backend/src/community/challenges.rs`

---

## Summary

The page shell, table loader, create modal, and active/inactive toggle are wired to real backend routes. The audit found release-blocking gaps for a production admin workflow: challenge admin APIs only require broad admin access and do not write community audit logs, the create form sends requirement/frequency values that do not match the actual challenge engine values, and several failure/security/accessibility states need recheck after fixes.

Fix update, 2026-04-25: The local working tree now gates the page and admin APIs with `community.manage`, records community audit logs for create/toggle, validates challenge requirement/frequency/numeric/badge contracts server-side, returns not found for stale toggle IDs, aligns form options with supported challenge constants, escapes the requirement display, maps `nav-com-challenges` to `community.manage`, and adds baseline modal dialog keyboard behavior. Targeted authenticated HTTP+DB and browser E2E passed for page load, create validation, create persistence/audit, toggle persistence/audit, stale-toggle 404, modal keyboard behavior, and mobile modal fit.

---

## Tested Scope

- Reviewed `frontend/platform/admin/community/challenges.html` template, inline JavaScript, modal controls, table rendering, and fetch calls.
- Verified page route registration in `backend/src/admin/mod.rs`.
- Verified API route registration and handlers in `backend/src/community/routes.rs`.
- Reviewed challenge service code in `backend/src/community/challenges.rs`.
- Reviewed challenge schema in `database/community/011_challenges.sql` and community audit-log schema in `database/community/018_community_audit_log.sql`.
- Searched backend and test files for challenge coverage.
- Ran inline JavaScript syntax check, YAML validation, cargo checks, and targeted authenticated HTTP+DB plus browser keyboard/mobile E2E after the local fix.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/community/challenges` | Admin page route. |
| URL alias | `/admin/community/challenges.html` | Registered alias. |
| Template | `frontend/platform/admin/community/challenges.html` | Contains all page UI and inline JS. |
| Shared JS | `frontend/platform/static/js/admin-permission-guard.js` | Adds CSRF header interceptor and sidebar permission hiding. |
| Shared JS | `frontend/platform/static/js/admin-theme.js` | Admin theme behavior. |
| Shared JS | `frontend/platform/static/js/user-data.js` | Shared user/session data behavior. |
| CSS | `frontend/platform/static/css/admin.css` | Admin layout and component styles. |
| CSS | `frontend/platform/static/css/bundle.css` | Shared platform bundle. |
| Backend page route | `GET /admin/community/challenges` | `page_admin_generic` in `backend/src/admin/mod.rs`. |
| Backend API route | `GET /api/admin/community/challenges` | Lists all challenges by `created_at DESC`. |
| Backend API route | `POST /api/admin/community/challenges` | Creates a challenge. |
| Backend API route | `POST /api/admin/community/challenges/:id/toggle` | Toggles `is_active`. |
| User API route | `GET /api/community/challenges` | Lists active challenges with user progress. |
| Database table | `challenges` | Challenge definitions. |
| Database table | `challenge_progress` | Per-user progress. |
| Database table | `community_audit_logs` | Exists but is not written by challenge admin mutations. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb: Admin | `a[href="/admin/"]` | Navigate to admin root. | Link only | Page route expected | Not runtime-tested. |
| Admin breadcrumb: Community | `a[href="/admin/community/"]` | Navigate to community admin root. | Link only | Page route registered | Not runtime-tested. |
| New Challenge button | `button[onclick="openCreateChallengeModal()"]` | Open create modal. | Yes, inline function | No backend until submit | Static review: wired. |
| Challenges table | `#challenges-table` | Render loading, empty, rows, or error. | Yes, `loadChallenges()` and `renderChallenges()` | `GET /api/admin/community/challenges` exists | Authenticated E2E page/API load passed. |
| Loading state | initial `<tbody>` row | Show loading until GET completes. | Yes | Yes | Static review: present. |
| Empty state | `renderChallenges([])` | Show "No challenges found." | Yes | Yes | Static review: present. |
| Load error state | `catch` in `loadChallenges()` | Show failed-load row. | Yes | Yes | Static review: present, but no retry action. |
| Create modal shell | `#challenge-modal` | Modal dialog for challenge creation. | Yes, dialog semantics and keyboard handling | POST route exists | Browser keyboard/mobile E2E passed. |
| Modal close button | close `button` with `✕` | Close and reset form. | Yes | Not needed | Labelled control with focus return verified in browser E2E. |
| Title input | `#challenge-title` | Required challenge title. | Yes | Backend accepts string | No length validation beyond DB constraint. |
| Description textarea | `#challenge-description` | Required description. | Yes | Backend accepts string | No length validation beyond DB/storage behavior. |
| XP reward input | `#challenge-xp` | Non-negative XP. | Yes, `parseInt` fallback | Backend clamps with `.max(0)` | No explicit validation or user feedback for invalid input. |
| Badge reward input | `#challenge-badge` | Optional badge reward ID. | Yes | Backend stores string | No badge existence validation. |
| Requirement type select | `#challenge-req-type` | Select engine-supported requirement type. | Yes | Backend stores string | Values mismatch several engine emitters/schema defaults. |
| Requirement target value input | `#challenge-req-val` | Positive integer target. | Yes, `parseInt` fallback | Backend clamps with `.max(1)` | No explicit validation or user feedback. |
| Frequency select | `#challenge-frequency` | Select supported recurrence. | Yes | Backend stores string | `once` mismatches seeded/default `one_time`; `monthly` is not documented in schema comments. |
| Cancel button | modal footer secondary button | Close/reset modal. | Yes | Not needed | Static review: wired. |
| Create Challenge button | modal footer primary button | POST create payload, close, reload table. | Yes | `POST /api/admin/community/challenges` exists | Static review: wired, but permission/audit/validation gaps. |
| Row status badge | generated in `renderChallenges()` | Show Active or Inactive. | Yes | Backed by `is_active` | Static review: wired. |
| Row Activate/Deactivate button | generated row button | Confirm then POST toggle. | Yes | `POST /api/admin/community/challenges/:id/toggle` exists | Static review: wired, but missing stale-ID detection and audit log. |

---

## Frontend Findings

### P1 - Create Form Uses Requirement And Frequency Values The Engine Does Not Emit

Location:

- Template: `frontend/platform/admin/community/challenges.html:109`
- JS: `frontend/platform/admin/community/challenges.html:217`
- Backend/schema: `database/community/011_challenges.sql:9`

Problem:

The admin form offers `write_post`, `invite_friend`, `leave_review`, `daily_login`, and `once`, but the backend challenge engine and seeded data use values such as `write_review`, `login_streak`, `join_circle`, `kyc_approved`, and `one_time`. Challenges created with the mismatched values can appear active but never progress because `increment_progress()` matches on exact `requirement_type`.

Expected:

The admin UI should offer only engine-supported values, labels should match the real event emitters, and the backend should reject unsupported requirement/frequency values.

Evidence:

`increment_progress()` looks up `WHERE requirement_type = $1`; existing emitters call values such as `login_streak`, `write_review`, and `join_circle`. The admin select sends different strings for several choices.

Recommended fix:

Create a shared backend allowlist for challenge requirement types/frequencies, validate POST input against it, and render the admin options from the same contract or keep a manually synchronized enum.

### P2 - Challenge Table Leaves Some API-Derived Fields Unescaped Inside `innerHTML`

Location:

- JS: `frontend/platform/admin/community/challenges.html:168`
- JS: `frontend/platform/admin/community/challenges.html:174`

Problem:

The table uses `tbody.innerHTML` for generated rows. `title`, `description`, and `frequency` are escaped, but `requirement_type` is transformed and inserted as `reqDisplay` without escaping. Because the backend accepts arbitrary `requirement_type`, a crafted record can become stored script/markup in the admin table.

Expected:

All API-derived values should be rendered via DOM text nodes or passed through context-safe escaping before insertion.

Evidence:

`reqDisplay` is built directly from `c.requirement_type` and inserted into a template string.

Recommended fix:

Render rows with DOM APIs or escape `reqDisplay`; also add backend enum validation so unsupported values cannot be persisted.

### P2 - Create Modal Lacks Keyboard Dialog Behavior

Location:

- Template: `frontend/platform/admin/community/challenges.html:81`
- JS: `frontend/platform/admin/community/challenges.html:193`

Problem:

The modal is a fixed `div` toggled with inline style. It has no `role="dialog"`, `aria-modal`, labelled dialog name, Escape close handler, backdrop close behavior, focus trap, initial focus placement, or focus restoration. The close button is an unlabeled `✕`.

Expected:

Modal behavior should meet the same baseline as other admin pages: semantic dialog attributes, labelled controls, keyboard trapping, Escape/backdrop close, and focus return to the opener.

Evidence:

Static template/JS review found only `style.display = 'flex'` and `style.display = 'none'` handlers.

Recommended fix:

Move modal behavior to a small controller that tracks the opener, focuses the title or first field, traps Tab, handles Escape/backdrop close, and restores focus on close.

---

## Backend Findings

### P1 - Challenge Admin APIs Lack Fine-Grained Permission Checks And Audit Logs

Location:

- Backend: `backend/src/community/routes.rs:945`
- Backend: `backend/src/community/routes.rs:961`
- Backend: `backend/src/community/routes.rs:989`
- Frontend permission map: `frontend/platform/static/js/admin-permission-guard.js:39`
- Audit table: `database/community/018_community_audit_log.sql:3`

Problem:

Challenge list/create/toggle handlers accept any `AdminUser` and discard the value with `let _ = admin`. The sidebar permission map also has no `nav-com-challenges` entry. Create/toggle mutations do not write `community_audit_logs`, despite challenge creation and activation being public-facing community governance actions.

Expected:

Admin challenge actions should require an explicit community/admin permission, navigation should hide the page for roles without that permission, and every create/toggle mutation should write an immutable audit record with actor, entity, action, and details.

Evidence:

The handlers use only the broad `AdminUser` extractor; no `require_permission` call or `community_audit_logs` insert appears in the challenge admin path.

Recommended fix:

Add a dedicated permission such as `community.challenges.manage` or a documented community admin permission; enforce it server-side for list/create/toggle; add `nav-com-challenges` to the permission map; write audit rows for challenge create and status changes.

### P2 - Toggle Endpoint Reports Success For Missing Challenge IDs

Location:

- Backend: `backend/src/community/challenges.rs:174`
- Backend: `backend/src/community/routes.rs:989`

Problem:

`admin_toggle_challenge()` executes `UPDATE challenges SET is_active = $1 WHERE id = $2` and ignores `rows_affected()`. A stale or nonexistent challenge ID returns `{ "success": true }`, so the UI cannot distinguish a real update from a no-op.

Expected:

The endpoint should return 404 or a typed application error when no challenge row was updated.

Evidence:

The SQL result is not inspected before returning `Ok(())`.

Recommended fix:

Check the `PgQueryResult`, return `AppError::NotFound` or equivalent when `rows_affected() == 0`, and add a regression test.

### P2 - Challenge Create Lacks Server-Side Contract Validation

Location:

- Backend: `backend/src/community/routes.rs:934`
- Backend: `backend/src/community/challenges.rs:144`
- Database: `database/community/011_challenges.sql:3`

Problem:

The create route accepts free-form strings for title, description, badge reward, requirement type, and frequency. Numeric fields are silently clamped with `.max(0)` / `.max(1)` instead of rejecting invalid input. There is no badge existence check, no allowed enum validation, no length validation before database write, and no clear client-facing validation response.

Expected:

The backend should reject invalid challenge contracts with explicit 4xx errors and should verify optional badge rewards against the badge table or a documented reward-code contract.

Evidence:

The service binds all strings directly into `INSERT INTO challenges` and clamps numeric values before insert.

Recommended fix:

Add request validation before insert: trim/non-empty/title-description length, XP range, positive target value, allowed requirement/frequency values, and optional badge reference validation.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Static page route mapping | Checked `backend/src/admin/mod.rs` for `/admin/community/challenges` and alias. | Page route exists. | Both clean and `.html` routes are registered. | Pass |
| API route mapping | Checked `backend/src/community/routes.rs` for list/create/toggle routes. | API routes exist with expected methods. | `GET/POST /api/admin/community/challenges` and `POST /api/admin/community/challenges/:id/toggle` exist. | Pass |
| Inline JS syntax | Extracted inline script and ran `node --check`. | No syntax errors. | Command passed. | Pass |
| Database schema support | Reviewed `database/community/011_challenges.sql`. | Required challenge tables exist. | `challenges` and `challenge_progress` exist. | Pass |
| Authenticated browser load | Open page as admin and inspect console/network. | Page loads without console/API errors. | Passed in `tests/e2e/test_admin_community_challenges.py`. | Pass |
| Safe create mutation | Submit a valid challenge and verify DB row/audit log. | Challenge persists and audit log records action. | Passed in `tests/e2e/test_admin_community_challenges.py`. | Pass |
| Safe toggle mutation | Toggle challenge status and verify DB row/audit log. | Status changes or 404s for stale ID; audit log records action. | Passed in `tests/e2e/test_admin_community_challenges.py`, including stale-toggle 404. | Pass |
| Modal keyboard check | Open modal, Tab cycle, Escape, close, focus return. | Keyboard accessible dialog behavior. | Passed in `tests/e2e/test_admin_community_challenges.py`. | Pass |
| Mobile modal check | Open modal at 375px viewport and verify no horizontal overflow/panel fit. | Mobile-safe dialog layout. | Passed in `tests/e2e/test_admin_community_challenges.py`. | Pass |

---

## Security Findings

- P1: Challenge admin APIs lack fine-grained server-side permission checks and do not write immutable community audit logs.
- P2: Backend accepts arbitrary challenge contract values, increasing invalid-state and stored-content risk.
- P2: Table rendering leaves `requirement_type` unescaped in `innerHTML`; backend enum validation would reduce this surface.
- CSRF: The page includes `admin-permission-guard.js`, whose fetch interceptor adds `X-CSRF-Token` for POST/PUT/PATCH/DELETE when the cookie exists. This was reviewed statically only.
- No money movement occurs on this page.

---

## Database Findings

- `challenges` and `challenge_progress` tables exist.
- `challenge_progress.user_id` is a raw UUID with no local FK in the community schema, matching other community tables but requiring application-level identity discipline.
- `challenges.requirement_type` and `challenges.frequency` are unconstrained strings; comments and seed rows document values but do not enforce them.
- `community_audit_logs` exists and supports `entity_type = 'challenge'`, but challenge admin mutations do not write to it.

---

## Missing Tests

- Backend unit/integration tests for `POST /api/admin/community/challenges` validation, unsupported enums, XP/target bounds, and optional badge validation.
- Backend test that `POST /api/admin/community/challenges/:id/toggle` returns 404 or equivalent for nonexistent IDs.
- Authorization tests proving non-community-admin roles cannot list/create/toggle challenges.
- Audit-log tests proving create/toggle writes immutable `community_audit_logs` rows.
- Broader browser coverage for empty/error state rendering remains useful; targeted authenticated create/toggle and modal keyboard/mobile E2E now exists.

---

## Recommended Fix Order

1. Define and enforce the backend challenge contract: allowed requirement types, frequencies, field lengths, numeric ranges, and badge reward validation.
2. Align the admin form options with the backend challenge engine emitters and seeded values.
3. Add fine-grained challenge permissions, sidebar permission mapping, and community audit-log writes for create/toggle.
4. Return 404 for stale toggle IDs and surface that error in the UI.
5. Replace row `innerHTML` rendering with DOM construction or complete escaping.
6. Upgrade the modal to accessible dialog behavior and add authenticated browser/E2E coverage.

---

## Final Status

`completed`

Reason: Code fixes are applied locally and targeted authenticated HTTP+DB plus browser keyboard/mobile E2E passed.
