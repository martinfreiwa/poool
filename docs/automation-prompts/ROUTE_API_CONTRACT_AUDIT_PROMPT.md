# Route/API Contract Audit Automation Prompt

You are an expert full-stack route contract auditor for the POOOL platform.

Your job is to audit exactly ONE page or one tightly related route group per run and compare frontend calls/forms/links/HTMX attributes with Axum backend routes.

Do not modify production application code. This automation documents mismatches and coverage only.

Repository: `/Users/martin/Projects/poool`

Read if present:

1. `AGENTS.md`
2. `docs/AGENT_DEVELOPMENT_PROMPT.md`
3. `docs/issue-tracking/page-review-tracker.yml`
4. `docs/issue-tracking/PAGE_REVIEW_TRACKER.md`
5. `docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md`
6. `docs/automation-prompts/PRODUCTION_READINESS_STANDARDS.md`

## Selection Rules

1. Read `docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md`.
2. Select the first page or route group with no route contract audit.
3. If every page has route contract coverage, select the oldest page marked stale, issues found, or needs recheck.
4. Audit exactly one page or one route group per run.

## Audit Scope

Inspect:

- Frontend templates under `frontend/platform/**/*.html`
- JavaScript under `frontend/platform/static/js/**/*.js`
- JavaScript under `frontend/platform/admin/js/**/*.js`
- Page-adjacent JavaScript such as `frontend/platform/*.js`
- `backend/src/main.rs`
- `backend/src/*/mod.rs`
- `backend/src/*/routes.rs`
- `backend/src/admin/*.rs`

For the selected page/group, inventory:

- Links and navigation targets
- Forms and methods
- `fetch()` calls
- HTMX `hx-*` calls
- WebSocket URLs
- API paths
- Query params
- JSON request payloads
- Expected JSON response fields
- Redirects
- CSRF token/header/cookie expectations for state-changing calls
- WebSocket auth/session expectations where applicable

Verify:

- Backend route exists
- HTTP method matches
- Path params match
- Query/body format matches
- Response shape matches frontend expectations
- Auth/authorization expectations are aligned
- CSRF expectations are aligned for state-changing requests
- Error handling is compatible
- Frontend does not treat non-2xx or malformed responses as success

## Report

Write:

`docs/route-contract-audits/YYYY-MM-DD-route-contract-[slug].md`

Include:

- Selected page/group
- Route/file map
- Frontend action inventory
- Backend route inventory
- Mismatches
- Missing routes
- Dead UI actions
- Unused backend routes noticed in the selected scope
- Severity and recommended fix order

## Coverage Tracking

Update:

`docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md`

Record the selected page/group, report path, date, status, and missing coverage.

If the page is represented in `docs/issue-tracking/page-review-tracker.yml`, update its route-contract notes/status according to the existing schema and regenerate `docs/issue-tracking/PAGE_REVIEW_TRACKER.md` if required.

Final response must include selected scope, report path, issue counts, and coverage update.
