# Page Audit: Developer Submissions

Date: 2026-04-28
Status: needs_recheck
Auditor: ChatGPT/Codex
Page URL: `/developer/submissions`
Template: `frontend/platform/developer/submissions.html`
JavaScript: `frontend/platform/static/js/developer-submissions.js`
CSS: `frontend/platform/static/css/developer-submissions.css`, `frontend/platform/static/css/developer-ui.css`
Backend Routes: `backend/src/developer/mod.rs`, `backend/src/developer/routes.rs`

---

## Summary

The `/developer/submissions` page is largely implemented: it is developer-gated at the page level, loads the developer's asset drafts from `/api/developer/drafts`, renders searchable/sortable/paginated rows, and wires resume, duplicate, delete, and resubmit actions to backend routes.

The audit found issues that need recheck after fixes. The biggest gap is that the mutating action APIs used by this page do not consistently call the same `require_developer_api` gate as the listing API. There is also a data-integrity edge case where assets missing `developer_projects` rows can appear as drafts and then submit or duplicate into inconsistent state. The custom delete modals also need basic dialog accessibility.

Post-fix update: the three implementation issues documented in this report were fixed on 2026-04-28. The remaining issue is verification coverage: authenticated browser/E2E testing for the table and mutating actions still needs to be run with an isolated developer fixture.

---

## Tested Scope

- Static review of the page template, shared topbar/sidebar includes, page JavaScript, CSS references, backend route registration, backend handlers, schema/migration support, and existing tests.
- Runtime unauthenticated smoke checks against a local `cargo run` server:
  - `GET /developer/submissions` redirects to `/auth/login`.
  - `GET /api/developer/drafts` returns JSON 401.
- Non-destructive checks only. Draft duplicate/delete/resubmit were not executed because no isolated authenticated developer fixture was provided in this audit run.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/developer/submissions` | Developer submissions management page |
| Template | `frontend/platform/developer/submissions.html` | JS-driven table shell and empty/loading states |
| Component | `frontend/platform/components/developer-topbar.html` | Adds `Add Asset` CTA |
| Component | `frontend/platform/components/sidebar.html` | Developer navigation |
| Component | `frontend/platform/components/mobile-menu.html` | Mobile navigation |
| JS | `frontend/platform/static/js/developer-submissions.js` | Fetches and renders rows; owns filters, sorting, pagination, actions |
| CSS | `frontend/platform/static/css/developer-submissions.css` | Page layout, table, modal, responsive styling |
| CSS | `frontend/platform/static/css/developer-ui.css` | Shared developer dashboard overrides |
| Backend page route | `GET /developer/submissions` | Registered in `backend/src/developer/mod.rs`; handler uses `require_developer_page` |
| Backend API route | `GET /api/developer/drafts` | Lists current developer assets |
| Backend API route | `POST /api/developer/draft/:id/duplicate` | Duplicates an owned asset into a draft |
| Backend API route | `DELETE /api/developer/draft/:id` | Soft-deletes owned draft assets |
| Backend API route | `POST /api/developer/draft/:id/submit` | Resubmits draft or revision-requested assets |
| Database table | `assets` | Source of asset rows, ownership, status, `deleted_at`, `submission_step` |
| Database table | `developer_projects` | Review status and revision notes |
| Database table | `asset_images` | Cover image lookup and submit prerequisite |
| Database table | `investments` | Delete guard for assets with active investors |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Add Asset topbar CTA | `#header-add-asset-btn`, template lines 12-13 via `developer-topbar.html` | Navigate to `/developer/add-asset` | Link | Page route exists | Not authenticated-runtime tested |
| Stat filter cards | `.sub-stat[data-filter]`, lines 17-95 | Filter rows by status and update active state | Yes, `filterByCard` plus keyboard listener | Uses already-loaded `/api/developer/drafts` data | Static verified |
| Bulk delete bar | `#sub-bulk-bar`, lines 103-111 | Delete selected draft rows after confirmation | Yes, `confirmBulkDelete` | `DELETE /api/developer/draft/:id` | Not executed |
| Search input | `#sub-search-input`, line 119 | Search by title, type, or app ID | Yes, `searchSubmissions` | Client-side only after API load | Static verified |
| Sort dropdown | `#sub-sort-trigger`, lines 123-145 | Sort newest, oldest, name A-Z/Z-A | Yes | Client-side only | Static verified |
| Result count | `#sub-result-count`, line 147 | Show filtered/total count | Yes | Client-side only | Static verified |
| Select all checkbox | `#select-all-checkbox`, lines 156-159 | Select visible draft rows only | Yes | Delete backend for selected IDs | Static verified |
| Sortable columns | `th.sortable`, lines 162-180 | Sort by title, progress, status, created, updated | Yes | Client-side only | Static verified |
| Dynamic rows | `#submissions-tbody`, lines 185-187 | Render rows from API data | Yes | `GET /api/developer/drafts` | API auth smoke only |
| Pagination | `#sub-pagination`, line 190 | Page through rows 20 at a time | Yes | Client-side only | Static verified |
| Empty state CTA | `#empty-state-add-asset-btn`, lines 216-222 | Navigate to add first asset | Link | Page route exists | Not authenticated-runtime tested |
| Loading state | `#submissions-loading`, lines 225-229 | Show until list API resolves | Yes | `GET /api/developer/drafts` | Unauthenticated API returns 401 |
| Resume draft | Generated `.sub-icon-btn`, JS lines 360-363 | Store draft ID and navigate to current step URL | Yes, localStorage + navigation | Edit pages/API exist separately | Static verified |
| Duplicate draft | Generated duplicate buttons, JS lines 364-366 and 382-384 | POST duplicate, toast, reload | Yes | Route exists | Not executed |
| Delete draft | Generated delete button, JS lines 367-369 | Confirm then soft-delete | Yes | Route exists | Not executed |
| Revision edit | Generated `.sub-edit-btn`, JS lines 370-374 | Navigate to property-content for revision | Yes | Edit page/API exist separately | Static verified |
| Resubmit | Generated `.sub-resubmit-btn`, JS lines 375-378 | Confirm then POST submit | Yes | Route exists | Not executed |
| View details | Generated view button, JS lines 379-381 | Navigate to `/developer/asset-detail?id=...` | Yes | Page route exists | Static verified |
| Delete modals | `.sub-modal-overlay`, JS lines 651-668 and 765-782 | Confirm destructive delete | Partially | Delete route exists | Accessibility gaps found |
| Toast | `#submissions-toast` fallback, JS lines 809-824 | Show success/error feedback | Yes | Client-side only | Static verified |

---

## Frontend Findings

### P2 - Delete confirmation modals lack dialog accessibility

Status: fixed 2026-04-28

Location:

- Template: generated by `frontend/platform/static/js/developer-submissions.js` lines 651-668 and 765-782
- CSS: `frontend/platform/static/css/developer-submissions.css` modal styling

Problem:

The custom bulk-delete and single-delete overlays do not set `role="dialog"`, `aria-modal="true"`, a labelled title relationship, initial focus, focus trapping, Escape close behavior, or focus restoration to the triggering control. They are mouse-click usable but not production-grade for keyboard and assistive technology users.

Expected:

Opening a destructive confirmation should move focus into a labelled modal dialog, trap focus while open, close on Escape, restore focus on close, and expose the destructive/cancel actions with clear accessible names.

Evidence:

The modal HTML is injected as a bare `.sub-modal-overlay` and `.sub-modal`; close handling only watches overlay clicks and inline cancel button handlers.

Recommended fix:

Use the shared `window.pooolConfirm` component for delete confirmation, or refactor the modal helper to add proper dialog semantics, focus management, Escape handling, and focus restoration.

Fix implemented:

- `frontend/platform/developer/submissions.html` now loads `poool-confirm` before `developer-submissions`.
- `frontend/platform/static/js/developer-submissions.js` now routes both single and bulk delete confirmations through `confirmAction`, which uses `window.pooolConfirm` when available and native `window.confirm` as fallback.

---

## Backend Findings

### P1 - Mutating draft APIs bypass the developer-role API gate

Status: fixed 2026-04-28

Location:

- `backend/src/developer/routes.rs` lines 229-238 defines `require_developer_api`.
- `GET /api/developer/drafts` uses it at lines 1265-1273.
- `POST /api/developer/draft/:id/submit` uses only `middleware::get_current_user` at lines 1328-1336.
- `POST /api/developer/draft/:id/duplicate` uses only `middleware::get_current_user` at lines 1417-1426.
- `DELETE /api/developer/draft/:id` uses only `middleware::get_current_user` at lines 1523-1532.

Problem:

The page route and list API require an active `developer`, `admin`, or `super_admin` role, but the mutating APIs only require an authenticated user who owns the asset. If a user's developer role is removed or deactivated after assets were created, the list/page gate blocks them, but direct API calls can still submit, duplicate, or delete owned assets.

Expected:

Every developer action API exposed by this page should call `require_developer_api` before ownership checks, so page and API authorization match.

Evidence:

Static route review showed only the listing API calls `require_developer_api`; the three mutating handlers do not.

Recommended fix:

Replace the local `middleware::get_current_user` blocks in submit, duplicate, and delete with `require_developer_api(&jar, &state).await`, then keep ownership/status checks as additional object-level authorization.

Fix implemented:

- `backend/src/developer/routes.rs` now calls `require_developer_api(&jar, &state).await?` in submit, duplicate, and delete handlers before ownership checks.

### P1 - Missing `developer_projects` rows can produce false-success submit and orphan duplicate state

Status: fixed 2026-04-28

Location:

- `GET /api/developer/drafts`: `LEFT JOIN developer_projects` with `COALESCE(dp.status, 'draft')`, lines 1275-1288.
- `POST /api/developer/draft/:id/submit`: missing project status is allowed and the `UPDATE developer_projects ... WHERE asset_id = $1` result is not checked, lines 1353-1404.
- `POST /api/developer/draft/:id/duplicate`: clone project row is an `INSERT ... SELECT ... FROM developer_projects WHERE asset_id = $2`, lines 1500-1509, with no affected-row check.

Problem:

The list API presents any owned asset without a `developer_projects` row as a draft. From the page, that row can then be resubmitted or duplicated. Submit can return success after updating only `assets.submission_step`, because the `developer_projects` update may affect zero rows. Duplicate can create a new `assets` row but no matching `developer_projects` row if the source asset has none, again returning success.

Expected:

The backend should either enforce the invariant that every developer-owned asset has a `developer_projects` row before showing it on this page, or create/repair the missing row transactionally. Submit and duplicate should check affected rows and fail or repair if the project row is missing.

Evidence:

The SQL contracts allow missing project rows through the read path and do not verify mutation row counts on write paths.

Recommended fix:

Use an inner join if the page should only manage real developer projects, or add transactional upsert/repair logic for missing `developer_projects` rows. Check `rows_affected()` for project updates/inserts and return a safe error if the invariant is broken.

Fix implemented:

- Submit now loads the project status inside the transaction and fails closed if the project row is missing.
- Submit now checks that exactly one `developer_projects` row was updated.
- Duplicate now validates the source project row before cloning and inserts exactly one clone project row from explicit values.
- Delete now fails closed if the project row is missing instead of treating missing status as `draft`.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| Unauthenticated page access | `curl -i http://localhost:8888/developer/submissions` | Redirect to login | `303 See Other`, `location: /auth/login` | Pass |
| Unauthenticated list API | `curl -i http://localhost:8888/api/developer/drafts` | JSON 401 | `401 Unauthorized`, `{"error":"Please log in"}` | Pass |
| JS syntax | `node --check frontend/platform/static/js/developer-submissions.js` | No syntax errors | No output, exit 0 | Pass |
| Backend validation unit tests | `cargo test draft_validation_tests` | Relevant developer validation tests pass | 7 passed | Pass |
| Post-fix static regression tests | `python3 -m pytest tests/test_developer_submissions_static.py -q` | Developer API gates, project-row checks, and shared confirmation usage are covered | 3 passed | Pass |
| Authenticated browser workflow | Load page as developer; filter/search/sort; duplicate/delete/resubmit isolated fixtures | All actions and states work | Not run, no isolated authenticated developer fixture provided | Not run |

---

## Security Findings

- P1: Mutating draft APIs do not consistently enforce the active developer-role gate used by the page and list API.
- CSRF headers are sent for POST/DELETE via explicit `X-CSRF-Token` and the global fetch interceptor. Runtime CSRF rejection behavior for these specific routes was not tested in this run.
- Rendered row content is mostly escaped before `innerHTML`, including titles, statuses, type labels, revision notes, IDs, dates, and cover image URLs. `safeImageUrl` restricts image sources to HTTP(S).
- No monetary mutation is performed by this page. Draft asset financial fields are displayed indirectly by edit/detail pages, not this table.

---

## Database Findings

- Required tables/columns exist in schema/migrations: `assets.submission_step`, `assets.deleted_at`, `developer_projects.status`, `developer_projects.revision_notes`, `asset_images`, and `investments`.
- P1: The page read path tolerates missing `developer_projects` rows, but submit/duplicate write paths do not enforce or repair that relationship.
- Delete is a soft delete via `assets.deleted_at` and blocks assets with active investments. It is a single-table mutation plus a read guard, so no multi-table transaction issue was found for delete.
- Submit wraps the asset/project update in a transaction, but does not check that the project status update affected a row.
- Duplicate wraps asset/project clone in a transaction, but does not check that the `developer_projects` insert selected a source row.

---

## Missing Tests

- Authenticated Playwright test for `/developer/submissions` covering load, empty state, non-empty table, search, stat filters, sort dropdown, column sorting, pagination, and mobile layout.
- Runtime API authorization tests proving submit, duplicate, and delete reject authenticated owners whose developer role is inactive or missing.
- Runtime regression tests for missing `developer_projects` rows: list behavior, submit failure, duplicate failure, and delete failure.
- CSRF rejection tests for duplicate, delete, bulk delete, and resubmit.
- Browser accessibility test for delete confirmation focus handling, Escape behavior, and dialog semantics.

---

## Recommended Fix Order

1. Add authenticated API and browser coverage for the page actions.
2. Add CSRF rejection coverage for duplicate, delete, bulk delete, and resubmit.
3. Run mobile/keyboard/browser accessibility verification for the shared confirmation path.

---

## Final Status

`needs_recheck`

Reason: The page is implemented and unauthenticated guards pass, but authorization and data-integrity issues in the mutating action APIs need fixes and authenticated re-verification.

Post-fix reason: The implementation issues are fixed and covered by static regression tests. The page remains `needs_recheck` until authenticated browser/API tests verify the real table and mutation flows.
