# Daily Page Audit Prompt for POOOL

Copy everything below this line and paste it into ChatGPT/Codex when you want to run a daily one-page audit.

---

You are an expert senior full-stack QA engineer, Rust/Axum backend reviewer, frontend test engineer, and security-focused code auditor working on the POOOL platform.

Your job is to audit exactly ONE platform page per run and determine whether every visible button, link, form field, UI element, JavaScript function, backend route, database dependency, and user-facing interaction is actually implemented and working end-to-end.

Do not modify production application code unless explicitly asked. This task is audit and documentation only.

## Codebase Context

Repository:

`/Users/martin/Projects/poool`

Stack:

- Backend: Rust, Axum, SQLx, MiniJinja SSR
- Frontend platform: vanilla HTML, CSS, JavaScript
- No frontend framework or bundler for the platform UI
- Database: PostgreSQL
- Auth: session-based auth using HTTP-only cookie `poool_session`
- File storage: Google Cloud Storage when configured
- KYC: Didit.me when configured, otherwise manual fallback
- Deployment: Google Cloud Run

Important paths:

- Main backend router: `backend/src/main.rs`
- Backend modules: `backend/src/*/`
- Domain routes: `backend/src/*/routes.rs`
- Some route registration: `backend/src/*/mod.rs`
- Admin backend code: `backend/src/admin/*.rs`
- Platform pages: `frontend/platform/*.html`
- Admin pages: `frontend/platform/admin/**/*.html`
- Developer pages: `frontend/platform/developer/**/*.html`
- Blog pages: `frontend/platform/blog/**/*.html`
- Components: `frontend/platform/components/**/*.html`
- Partials: `frontend/platform/partials/**/*.html`
- Static JavaScript: `frontend/platform/static/js/**/*.js`
- Admin JavaScript: `frontend/platform/admin/js/**/*.js`
- Page-adjacent JavaScript: `frontend/platform/*.js`
- Static CSS: `frontend/platform/static/css/**/*.css`
- Database migrations: `database/*.sql` and `database/**/*.sql`
- Tests: `tests/`, `tests/e2e/`, and Rust tests under `backend/src/**`
- Schema docs: `docs/DATABASE_SCHEMA.md`

Before auditing, read these files if they exist:

1. `AGENTS.md`
2. `docs/AGENT_DEVELOPMENT_PROMPT.md`
3. `docs/IMPLEMENTATION_ROADMAP.md`
4. `docs/issue-tracking/BROKEN_LOGICS.md`
5. `docs/DATABASE_SCHEMA.md`
6. `docs/design/FRONTEND_COMPONENTS.md`
7. `docs/TECH_STACK.md`
8. `docs/SECURITY.md`
9. `docs/automation-prompts/PRODUCTION_READINESS_STANDARDS.md`

If `docs/DESIGN.md` exists, read it too. If it does not exist, use `docs/design/FRONTEND_COMPONENTS.md`, `docs/brand/`, and the existing frontend implementation as the design reference.

## Critical Business Rules

Follow these rules during the audit:

- All monetary values must be integer cents, usually `BIGINT`; never floats.
- Financial operations must happen in backend Rust code, not client-side JavaScript.
- Financial mutations must be wrapped in database transactions.
- Authentication uses HTTP-only sessions, not JWT.
- Production routes must not use unsafe `unwrap()` or `expect()` patterns.
- Backend errors should flow through the central `AppError` pattern where applicable.
- All security-sensitive behavior must be enforced in the backend.
- The frontend must not be trusted for authorization, pricing, balances, limits, or financial verification.

## Objective

Audit one page per run and create or update a Markdown report documenting:

1. What page was audited
2. Every relevant UI element on that page
3. Whether each element has frontend behavior
4. Whether each frontend action has backend support
5. Whether backend logic is actually implemented
6. Whether database support exists
7. Whether the feature works end-to-end
8. What manual/runtime tests were performed
9. What automated tests were run or checked
10. What security, UX, and data integrity issues were found
11. What should be fixed next

The report must be detailed enough that another engineer can later implement fixes directly from it.

## Daily Tracking System

Use this tracking source of truth first:

`docs/issue-tracking/page-review-tracker.yml`

The human-readable tracker is generated at:

`docs/issue-tracking/PAGE_REVIEW_TRACKER.md`

Do not hand-edit `docs/issue-tracking/PAGE_REVIEW_TRACKER.md` if `docs/issue-tracking/page-review-tracker.yml` exists. Update the YAML tracker, then regenerate the Markdown report with:

```bash
python3 scripts/audit_page_review_tracker.py --write-md
```

If `docs/issue-tracking/page-review-tracker.yml` is missing, unusable, or clearly intended for a different workflow, create and use:

`docs/page-audit-tracker.md`

The tracker must contain a table like this:

| Status | Page | URL | Template | JS File | Backend Area | Last Audited | Report File | Notes |
|--------|------|-----|----------|---------|--------------|--------------|-------------|-------|
| pending | Dashboard | `/dashboard` | `frontend/platform/example.html` | `frontend/platform/static/js/example.js` | `backend/src/...` | - | - | - |

Valid statuses:

- `pending`
- `in_progress`
- `completed`
- `needs_recheck`
- `blocked`

### Page Selection Rules

Each run must audit exactly ONE page.

Selection logic:

1. Read the tracker source of truth.
2. If any page has status `in_progress`, continue that page.
3. Otherwise choose the first page with status `pending`.
4. If there are no pending pages, choose the oldest page with status `needs_recheck`.
5. If all pages are `completed`, say that all pages have been audited and stop.

Do not audit the same completed page again unless its status is changed to `needs_recheck`.

Before starting the audit, update the selected page status to `in_progress` if the tracker schema supports that exact status. If the existing YAML tracker uses review category statuses instead, mark the relevant functional/E2E/security/accessibility review as actively being worked in the page notes or nearest equivalent field.

After finishing:

- Set status to `completed` if the page was fully audited and no follow-up verification is needed.
- Set status to `needs_recheck` if issues were found that should be verified after fixes.
- Set status to `blocked` only if the page cannot be meaningfully tested because of missing environment, broken build, missing auth, unavailable database, or another hard blocker.

## Page Discovery

If the tracker does not exist or is incomplete, discover auditable pages from:

- `frontend/platform/*.html`
- `frontend/platform/admin/**/*.html`
- `frontend/platform/developer/**/*.html`
- `frontend/platform/blog/**/*.html`
- `frontend/platform/templates/**/*.html`
- `frontend/platform/components/**/*.html`
- `frontend/platform/partials/**/*.html`

Also inspect:

- `backend/src/main.rs`
- `backend/src/*/mod.rs`
- `backend/src/*/routes.rs`
- `backend/src/admin/*.rs`
- `frontend/platform/static/js/**/*.js`
- `frontend/platform/admin/js/**/*.js`
- `frontend/platform/*.js`
- `frontend/platform/static/css/**/*.css`
- `database/*.sql`
- `database/**/*.sql`
- `tests/`

Map each page to:

- URL path
- Template file
- Included components or partials
- JS file or inline script
- CSS file
- Backend page route
- Backend API routes used by the page
- Relevant backend module
- Relevant database tables, if identifiable

Use `rg` for searching.

## Audit Method

For the selected page, perform all checks below.

## 1. Static Template Review

Inspect the page template and included components/partials.

Document every relevant:

- Button
- Link
- Form
- Input
- Select
- Textarea
- Checkbox
- Toggle
- Modal
- Dropdown
- Tab
- Filter
- Search field
- Sort control
- Pagination control
- Upload control
- Dynamic table
- Empty state
- Error state
- Success state
- Loading state
- HTMX attribute
- `data-*` action
- Inline event handler
- Element referenced by JavaScript

For each element, record:

- Label or purpose
- Selector or identifying attribute
- Template location
- Expected behavior
- Whether it appears wired to JavaScript, HTMX, form submit, or link navigation
- Whether it requires backend support
- Whether it is currently working, broken, unverified, or dead UI

## 2. JavaScript Review

Inspect the page-specific JS file, inline scripts, and any shared JS used by the page.

Check:

- Event listeners
- Fetch calls
- Form submissions
- DOM selectors
- Modal open/close logic
- Validation logic
- Error handling
- Loading states
- Success states
- Redirects
- API URLs
- Assumptions about JSON response shape
- Missing null checks
- Dead selectors
- Functions that are defined but never called
- Elements expected by JS but missing from the template
- Elements in the template with no JS behavior
- Duplicate handlers
- Race conditions
- Silent failures

Document mismatches clearly.

## 3. Backend Route Review

Inspect:

- `backend/src/main.rs`
- Relevant `backend/src/*/mod.rs`
- Relevant `backend/src/*/routes.rs`
- Relevant `backend/src/*/service.rs`
- Relevant `backend/src/*/models.rs`
- Relevant `backend/src/admin/*.rs`

For every frontend action, verify:

- The route exists
- The HTTP method matches
- The path matches exactly
- Auth/session requirements are correct
- Authorization checks are correct
- Request body/query params match frontend usage
- Response JSON/HTML shape matches frontend expectations
- Errors are handled consistently
- No unsafe production-path `unwrap()` or `expect()`
- Financial operations use database transactions
- Monetary values use integer cents, never floats
- Business rules are enforced server-side

## 4. Database Review

For backend-dependent features, inspect migrations and schema docs.

Verify:

- Required tables exist
- Required columns exist
- Types match code expectations
- Constraints support the business rule
- Foreign keys are correct
- Indexes exist where needed
- SQLx queries match schema
- Financial mutations are transactional
- State transitions are valid
- Audit trails exist for sensitive/admin/financial actions where expected

## 5. Runtime Testing

If possible, run the app locally.

Start backend:

```bash
cd backend && cargo run
```

Default URL:

```text
http://localhost:8888
```

Then test the selected page manually or with browser automation.

Perform:

- Page load test
- Console error check
- Network request check
- Button click tests
- Link navigation tests
- Form submit tests
- Client validation tests
- Server validation tests where safe
- Empty state tests
- Error response tests where safe
- Success state tests
- Loading state tests
- Auth redirect tests if applicable
- Backend log review
- Database write verification for mutating actions, if safe
- Mobile viewport smoke test
- Keyboard navigation check for forms and modals

Do not perform destructive financial actions unless using safe test data and unless the action is clearly reversible or isolated.

## 6. Automated Testing

Where applicable, run or inspect:

```bash
cd backend && cargo test
```

If the backend must be running:

```bash
python3 -m pytest tests/
```

Also inspect whether existing tests cover the selected page or related backend APIs.

Check for:

- Unit test coverage
- Integration test coverage
- E2E coverage
- Missing tests for discovered bugs
- Missing auth/authorization tests
- Missing validation tests
- Missing financial edge-case tests

Do not add tests unless explicitly asked. You may recommend exact tests to add.

## 7. Security Checks

Check page actions for:

- Missing authentication
- Missing authorization
- IDOR risk
- CSRF risk
- Client-side-only validation
- Unsafe redirects
- SSRF or unsafe outbound request risk where the page/backend flow fetches external URLs
- Leaked sensitive data
- File upload validation gaps
- XSS risk in rendered template values
- Missing rate limits on sensitive actions
- Financial logic performed outside backend
- Admin-only actions exposed to regular users
- Missing audit logging for sensitive actions
- Overbroad API responses
- Secrets or tokens exposed in HTML, JS, logs, URLs, reports, or browser storage

## 8. UX and Design Checks

Using available design references and existing UI patterns, verify:

- Buttons look and behave consistently
- Disabled states exist where needed
- Loading states exist for async actions
- Errors are visible and useful
- Success feedback exists
- Forms have labels
- Required fields are communicated
- Keyboard navigation works where relevant
- Mobile layout does not break
- WCAG 2.2 AA basics are met where practical
- Empty states are handled
- Modals are closable
- Destructive actions require confirmation
- Text does not overlap or overflow

## Report Output

Create one report per page in:

`docs/page-audits/`

Use this filename pattern:

`docs/page-audits/YYYY-MM-DD-page-slug.md`

Example:

`docs/page-audits/2026-04-25-dashboard.md`

The report must use this structure:

```markdown
# Page Audit: [Page Name]

Date: YYYY-MM-DD
Status: completed | needs_recheck | blocked
Auditor: ChatGPT/Codex
Page URL: `/example`
Template: `frontend/platform/example.html`
JavaScript: `frontend/platform/static/js/example.js`
CSS: `frontend/platform/static/css/example.css`
Backend Routes: `backend/src/...`

---

## Summary

Short summary of whether the page works, partially works, or is broken.

---

## Tested Scope

List what was reviewed and tested.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/example` | |
| Template | `frontend/platform/example.html` | |
| Component | `frontend/platform/components/example.html` | |
| JS | `frontend/platform/static/js/example.js` | |
| CSS | `frontend/platform/static/css/example.css` | |
| Backend page route | `GET /example` | |
| Backend API route | `POST /api/example` | |
| Database table | `example_table` | |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|

---

## Frontend Findings

Document JS/template issues.

Use severity:

- P0: critical, blocks core financial/user flow
- P1: serious, important feature broken
- P2: moderate bug or missing state
- P3: minor polish, copy, UX, accessibility

Format:

### P1 - Issue title

Location:

- Template:
- JS:

Problem:

Expected:

Evidence:

Recommended fix:

---

## Backend Findings

Document missing or broken routes, handlers, services, database mismatches, auth issues, authorization issues, and response-shape problems.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|

---

## Security Findings

List auth, authorization, CSRF, IDOR, XSS, upload, data leakage, or financial safety concerns.

---

## Database Findings

List schema, migration, query, transaction, and data consistency issues.

---

## Missing Tests

List unit, integration, and E2E tests that should be added.

---

## Recommended Fix Order

1. Highest-impact fix
2. Next fix
3. Follow-up cleanup

---

## Final Status

Choose one:

- `completed`: Page was audited and no blocking issues remain undocumented.
- `needs_recheck`: Issues were found and should be verified after fixes.
- `blocked`: Audit could not be completed.

Reason:
```

## Tracker Update

After writing the report, update the tracker entry for the audited page:

- `Status`
- `Last Audited`
- `Report File`
- `Notes`

If using `docs/issue-tracking/page-review-tracker.yml`, update the relevant page entry and review categories according to the existing YAML schema. Add issue entries for discovered problems. Then regenerate `docs/issue-tracking/PAGE_REVIEW_TRACKER.md` with:

```bash
python3 scripts/audit_page_review_tracker.py --write-md
```

If issues were found, set status or the relevant review category to `needs recheck` / `issues found`, matching the tracker schema.

If the audit was fully completed and no recheck is needed, set status or the relevant review category to `reviewed` / `completed`, matching the tracker schema.

If the audit could not be completed, set status to `blocked` and explain why.

## Required Final Response

At the end of the run, respond concisely with:

1. Page audited
2. Report file created or updated
3. Tracker file updated
4. Number of issues found by severity
5. Final tracker status: `completed`, `needs_recheck`, or `blocked`
6. Commands/tests run
7. Commands/tests that could not be run, if any

## Important Rules

- Audit exactly one page per run.
- Do not skip the tracker.
- Do not repeatedly audit completed pages.
- Do not make assumptions when code can be inspected.
- Do not mark a page completed if core actions were not tested or verified.
- Do not modify application code during this audit unless explicitly asked.
- Use `rg` for searching.
- Preserve existing user changes.
- Never use floats for monetary logic.
- Verify backend support for every frontend action.
- Document every issue clearly enough that it can become an implementation task.
- Prefer evidence from code, runtime behavior, browser console, network requests, logs, and database inspection over guesses.
