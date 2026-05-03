# Page Audit: Admin Developer Submissions

Date: 2026-04-27
Page: Admin Developer Submissions
URL: `/admin/developer-submissions`
Template: `frontend/platform/admin/developer-submissions.html`
Primary JS: `frontend/platform/static/js/admin-submissions.js`
Backend: `backend/src/admin/mod.rs`, `backend/src/admin/pages.rs`, `backend/src/admin/developer_projects.rs`
Tracker: `docs/issue-tracking/page-review-tracker.yml`
Status: fixed, needs authenticated runtime recheck

## Scope

This audit reviewed exactly one page: `/admin/developer-submissions`. The review covered the server-rendered admin page, the submissions list controller, the admin generic page gate, developer-project APIs used by the page, schema dependencies, and existing test references. This was documentation-only; no production application code was changed.

Runtime browser testing was not completed during the original audit because no backend was listening on `localhost:8888`. Static JS syntax validation passed.

## Fix Follow-Up: 2026-04-28

All five documented implementation issues were addressed in code:

- Added granular `submissions.review` and `submissions.approve` permissions, exposed them in the admin permissions API, granted them in `database/089_admin_submission_permissions.sql`, and gated the page plus developer-project APIs.
- Removed quick approve/publish from the list-page modal. Publishing now remains a full-review action and the backend enforces approved KYC, a linked asset, approval checklist completion, at least one asset document, at least one asset image, and submitted/in-review status before publishing.
- Changed the developer-projects list API to propagate DB failures instead of returning an empty queue, and changed the frontend to render a retryable load error rather than false zeroes.
- Made review audit log, note, and notification writes failure-blocking inside the status-change transaction. The detail-page auto-transition to `in_review` is now transactional with audit logging.
- Added dialog semantics, Escape handling, focus trapping/restoration, keyboard-sort support, `aria-sort`, a stronger quick-action accessible name, `revision_requested` status support, and corrected the empty-state column span.

Remaining issue: authenticated runtime/E2E verification still needs to run against a live backend with seeded admin/project data. This should verify permission denial, list error handling, quick reject/in-review, full-review approval gate failures and success, exact DB state changes, audit rows, notifications, and modal keyboard behavior.

## UI Inventory

| Element | Selector / location | Expected behavior | Wiring | Backend support | Status |
|---|---|---|---|---|---|
| Admin breadcrumb | `a[href="/admin/"]` in template lines 33-37 | Navigate back to admin dashboard | Link | `GET /admin/` | Working by static route mapping |
| Global admin search | `#admin-global-search` lines 40-48 | Global admin search | `admin-global-search.js` | Shared admin search routes, not page-specific | Not deeply audited here |
| KPI cards | `#stat-pending`, `#stat-approved`, `#stat-rejected`, `#stat-total-value` lines 68-129 | Show counts and total value from loaded projects | `updateStats()` lines 91-112 | `GET /api/admin/developer-projects` | Works if API succeeds; failures are masked as zeroes |
| Submission search | `#sub-search` lines 141-148 | Client-side search by project, title, developer, email, city | `input` listener lines 41-44 | Uses loaded API data | Wired |
| Status filter | `#filter-status` lines 150-158 | Filter by `developer_projects.status` | `change` listener lines 45-47 | Uses loaded API data | Wired, but omits `revision_requested` |
| Asset type filter | `#filter-type` lines 159-167 | Filter by `asset_type` | `change` listener lines 48-50 | Uses loaded API data | Wired |
| Count label | `#sub-count-label` lines 168-172 | Show filtered count | `applyFilters()` lines 148-150 | None | Wired |
| Sortable headers | `th[data-sort]` lines 182-190 | Sort client-side by selected field | `setupSorting()` lines 21-38 | Uses loaded API data | Wired, no aria-sort state |
| Loading row | `#submissions-table-body` lines 194-212 | Show loading spinner | Initial HTML only | API load should replace | Shows initially; no persistent API error state |
| Table rows | rendered in `renderTable()` lines 179-241 | Show project, type, developer, asset, status, amounts, progress, date, actions | `innerHTML` with escaped values | `GET /api/admin/developer-projects` | Mostly wired; backend DB failures look like empty data |
| Full review link | rendered link `/admin/developer-submission-review?id=...` lines 225-230 | Open full review page | Link | `GET /admin/developer-submission-review`, detail APIs | Wired |
| Quick action button | rendered button line 231 | Open quick review modal | Inline `onclick=openQuickModal(...)` lines 231-233 | None until submit | Wired, icon-only accessible name relies on `title` |
| Pagination | `#prev-page`, `#next-page`, `#pagination-info` lines 218-233 | Client-side page through 15-row slices | Listeners lines 51-63 | None | Wired |
| Review modal shell | `#review-modal` lines 239-302 | Modal for quick status changes | `openQuickModal()`, overlay click close | `POST /api/admin/developer-projects/:id/review` | Functionally wired; accessibility incomplete |
| Review notes | `#review-notes` lines 270-272 | Required for rejection | JS validation lines 281-289; backend validation lines 527-532 | Review API | Wired |
| Reject button | `#review-modal-reject` lines 278-284 | Reject project | JS posts `{action:"reject"}` | Review API updates status, unpublishes asset, notification, note, audit | Works in code path; audit/notification failures are partly swallowed |
| Mark in Review button | `#review-modal-in-review` lines 285-292 | Move project into review | JS posts `{action:"in_review"}` | Review API updates status, notification, audit | Works in code path; audit/notification failures are swallowed |
| Approve & Publish button | `#review-modal-approve` lines 293-299 | Approve and publish asset | JS posts `{action:"approve"}` | Review API sets project approved and asset published | High-risk bypass of full review gates |

## Backend And Database Mapping

The page route is registered in `backend/src/admin/mod.rs` for both `/admin/developer-submissions` and `/admin/developer-submissions.html`, using `page_admin_generic`. The generic admin page gate only requires an active `admin` or `super_admin` role for this route; there is no page-specific permission check for submissions review or asset publishing.

The list API is `GET /api/admin/developer-projects`. It joins `developer_projects`, `assets`, `users`, `user_profiles`, and latest `kyc_records`, plus a union branch for orphaned assets. The list API currently uses `.unwrap_or_default()` after `fetch_all`, so database read failure returns an empty list instead of a 500.

The quick modal posts to `POST /api/admin/developer-projects/:id/review`, which delegates to the notes/review handler. The handler accepts `approve`, `reject`, `request_revision`, and `in_review`, starts a transaction, and updates `developer_projects`, `assets`, `notifications`, `developer_project_notes`, and `audit_logs` depending on the action. Core schema dependencies exist in migrations for `developer_projects`, `developer_project_notes`, `asset_documents`, `asset_images`, `asset_milestones`, `notifications`, and `audit_logs`.

## Findings

### PAGE-ISSUE-0432 - Generic admin access can review and publish developer submissions

Severity: high
Category: security / authorization
Status: fixed

`/admin/developer-submissions` is served through `page_admin_generic` without a submissions-specific permission gate, and the API endpoints only require the broad `AdminUser` extractor. Any active `admin` or `super_admin` can list submissions and call the review mutation endpoints. Because quick approve can publish assets, this should require explicit permissions such as `submissions.review` for read/in-review/reject and `submissions.approve` or `assets.publish` for publish decisions.

Evidence:
- Page route uses `page_admin_generic` in `backend/src/admin/mod.rs` lines 114-115.
- Generic page checks special permissions for some admin areas, but not developer submissions, in `backend/src/admin/pages.rs` lines 189-249.
- API routes are registered in `backend/src/admin/mod.rs` lines 445-463.
- Handlers accept `AdminUser` without `require_permission()` in `backend/src/admin/developer_projects.rs` lines 15-18 and 495-500.

Fix:
Added `submissions.review` and `submissions.approve`, gated the page and all developer-project endpoints, and added a migration to grant the new permissions.

### PAGE-ISSUE-0433 - Quick approve can publish without full review gates

Severity: high
Category: functional / compliance
Status: fixed

The list page exposes a quick modal that can send `approve` directly. The backend approval path sets `developer_projects.status = 'approved'` and publishes the linked asset, but it does not enforce approved developer KYC, required documents, compliance checklist completion, asset completeness, maker/checker separation, or status-transition constraints. The modal copy directs admins to the full review page for document data room, financials, and KYC status, but still allows immediate publish from the lightweight list modal.

Evidence:
- Quick action copy and buttons are in `frontend/platform/static/js/admin-submissions.js` lines 254-272 and template lines 278-299.
- Approval path updates project status and publishes the asset in `backend/src/admin/developer_projects.rs` lines 629-660.
- The handler has no checks for `kyc_status`, checklist, documents, required asset fields, or current-status allowlist before publishing.

Fix:
Removed quick approve from the list modal. Backend approval now requires `submissions.approve`, approved developer KYC, linked asset, completed approval checklist, at least one document, at least one image, and an allowed status transition.

### PAGE-ISSUE-0434 - Submission list API masks database failures as an empty queue

Severity: high
Category: reliability / data integrity
Status: fixed

`GET /api/admin/developer-projects` converts any database read failure into an empty vector. The frontend then renders zero submissions and zero KPI values, making outages, schema drift, or query failures indistinguishable from a true empty queue. This is a high-risk admin workflow because operators can miss pending submissions or believe the queue is clear.

Evidence:
- Backend list query calls `.fetch_all(...).await.unwrap_or_default()` in `backend/src/admin/developer_projects.rs` lines 102-105.
- Frontend `loadSubmissions()` also converts non-OK responses and network errors to `allSubmissions = []` in `frontend/platform/static/js/admin-submissions.js` lines 75-86.

Fix:
The list API now returns an error on DB failure and the frontend renders a visible retryable error state separate from the true empty state.

### PAGE-ISSUE-0435 - Review side effects can commit without durable audit or notification records

Severity: high
Category: auditability / data integrity
Status: fixed

Several review decision paths log audit, notification, or note failures but still commit the status change. For a regulated admin workflow, approve/reject/revision/in-review state changes should either atomically persist required audit evidence and operator notes, or fail and roll back.

Evidence:
- Approval audit failure is logged but does not abort before commit in `backend/src/admin/developer_projects.rs` lines 685-699.
- Rejection unpublish, notification, note, and audit failures are logged but do not abort in lines 731-793.
- Revision notification, note, and audit failures are logged but do not abort in lines 827-875.
- In-review notification and audit failures are logged but do not abort in lines 961-993.
- Auto-transition on detail load also ignores update and audit errors in lines 254-272, which affects the related full-review page.

Fix:
Audit log, required note, and in-app notification writes now block status-change commits when they fail. The submitted-to-in-review auto-transition is also transactional with audit logging.

### PAGE-ISSUE-0436 - Quick review modal and table controls need accessibility recheck

Severity: medium
Category: accessibility / UX
Status: fixed

The modal is a styled `div` without `role="dialog"`, `aria-modal`, focus management, Escape close, or focus restoration. Sortable table headers are clickable but do not expose button semantics or `aria-sort`. The icon-only quick action button relies on `title` rather than a dependable accessible name. The empty filtered row uses `colspan="9"` even though the table has ten columns.

Evidence:
- Modal markup is in `frontend/platform/admin/developer-submissions.html` lines 239-302.
- Modal open/close logic is in `frontend/platform/static/js/admin-submissions.js` lines 248-279.
- Sort binding uses bare `th[data-sort]` click listeners in lines 21-38.
- Quick action icon-only button is rendered in lines 231-235.
- Empty state row uses `colspan="9"` in lines 173-176.

Fix:
Added dialog semantics, focus trap/restoration, Escape close, keyboard-sort behavior, `aria-sort`, `aria-label` for quick status review, `revision_requested` status support, and corrected the empty row colspan.

## Positive Checks

- The page uses integer cents fields from the API (`total_value_cents`, `total_raised_cents`, token prices) and formats them client-side only for display.
- The quick review POST includes `X-CSRF-Token` from the shared cookie helper.
- Row rendering escapes dynamic text before using `innerHTML` for most user-provided fields.
- Backend review mutations are wrapped in a transaction for the main status-update flow.
- `node --check frontend/platform/static/js/admin-submissions.js` passed.

## Tests And Commands Run

```bash
node --check frontend/platform/static/js/admin-submissions.js
```

Result: passed.

2026-04-28 rerun: passed.

```bash
curl -sS -o /tmp/poool_admin_developer_submissions_headers.txt -w '%{http_code}\n' http://localhost:8888/admin/developer-submissions || true
```

Result: `000`; local backend was not running on port 8888, so authenticated browser/runtime checks were not performed.

Static review commands:

```bash
sed -n '1,260p' docs/automation-prompts/DAILY_PAGE_AUDIT_PROMPT.md
sed -n '1,260p' docs/automation-prompts/PRODUCTION_READINESS_STANDARDS.md
sed -n '1,180p' docs/AGENT_DEVELOPMENT_PROMPT.md
sed -n '1,260p' docs/issue-tracking/page-review-tracker.yml
sed -n '1,340p' frontend/platform/admin/developer-submissions.html
sed -n '1,380p' frontend/platform/static/js/admin-submissions.js
rg -n "developer-submissions|developer-projects" backend/src/admin backend/src/main.rs tests
```

## Final Status

Final status: fixed, needs authenticated runtime recheck.

Severity counts:
- Critical: 0
- High: 4
- Medium: 1
- Low: 0
- Info: 0

Remaining verification should prioritize an authenticated browser/API run with seeded submissions, including permission denial, quick reject/in-review, full-review approval gates, audit rows, notifications, and mobile/keyboard modal behavior.
