# POOOL Production Readiness Standards

These standards apply to all POOOL production-readiness automations.

## Source Of Truth

- Repository: `/Users/martin/Projects/poool`
- Mandatory project context: `AGENTS.md`
- Mandatory development rules: `docs/AGENT_DEVELOPMENT_PROMPT.md`
- Tracker source of truth: `docs/issue-tracking/page-review-tracker.yml`
- Generated page tracker: `docs/issue-tracking/PAGE_REVIEW_TRACKER.md`
- Shared automation coverage tracker: `docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md`

If a generated Markdown tracker says to update a YAML source first, update the YAML source and regenerate the Markdown with the documented script.

## Verified Stack And Paths

- Backend: Rust, Axum, SQLx, MiniJinja SSR
- Frontend platform: vanilla HTML, CSS, JavaScript, no framework/bundler for platform UI
- Database: PostgreSQL 16
- Auth: HTTP-only session cookie `poool_session`
- Main backend router: `backend/src/main.rs`
- Backend domains: `backend/src/*/`
- Admin backend files: `backend/src/admin/*.rs`
- Route registration may live in `backend/src/main.rs`, `backend/src/*/mod.rs`, `backend/src/*/routes.rs`, and selected admin module files
- Platform root pages: `frontend/platform/*.html`
- Admin pages: `frontend/platform/admin/**/*.html`
- Developer pages: `frontend/platform/developer/**/*.html`
- Blog pages: `frontend/platform/blog/**/*.html`
- Components/partials/templates: `frontend/platform/components/**/*.html`, `frontend/platform/partials/**/*.html`, `frontend/platform/templates/**/*.html`
- Static JavaScript: `frontend/platform/static/js/**/*.js`, `frontend/platform/admin/js/**/*.js`, plus rare page-adjacent scripts such as `frontend/platform/*.js`
- Static CSS: `frontend/platform/static/css/**/*.css`
- Database migrations and schema SQL: `database/**/*.sql`, `database/*.sql`
- Tests: `tests/`, `tests/e2e/`, Rust unit/integration tests under `backend/src/**`
- Tracker regeneration script: `scripts/audit_page_review_tracker.py`

## Non-Negotiable POOOL Rules

- Money must use integer minor units, normally `i64` cents in Rust and `BIGINT` in PostgreSQL.
- No floats for money, fees, balances, investment amounts, commissions, payouts, reconciliation, or settlement logic.
- Financial mutations must run in backend Rust, never as trusted client-side logic.
- Multi-write financial mutations must use database transactions.
- Balance reads before writes must use appropriate locking, usually `SELECT ... FOR UPDATE`.
- Server-side authorization is required for all sensitive actions.
- Client-side validation is only UX; backend validation is mandatory.
- Auth uses HTTP-only sessions, not JWT.
- No unsafe production-path `unwrap()` or `expect()`.
- No PgBouncer/Cloud SQL/db connection changes unless the task specifically targets that infrastructure and the production architecture rules in `AGENTS.md` have been reread.

## Security Standard

Use an OWASP ASVS-style review mindset, with special attention to:

- Authentication and session lifecycle
- Authorization and object-level access control
- IDOR/BOLA
- CSRF on state-changing requests
- XSS from templates, JSON data, `innerHTML`, or unsafe HTML rendering
- SSRF and unsafe outbound requests
- File upload validation, storage permissions, and content-type trust
- Sensitive data exposure in HTML, JS, logs, URLs, errors, and API responses
- Rate limits and abuse protection for auth, support, uploads, financial actions, and admin actions
- Secure defaults for optional services such as Redis, GCS, Didit, Sentry, and OAuth providers
- Admin route exposure and role/permission boundaries
- Audit logging for financial, admin, KYC, and support-sensitive actions
- Secrets must never be committed or printed into reports

## Reliability And Data Integrity Standard

Check for:

- Idempotency on payments, orders, deposits, withdrawals, payouts, emails, and webhook-like flows
- Race conditions and double-submit/double-spend risks
- Explicit state machines for orders, KYC, withdrawals, settlements, disputes, and admin approvals
- Consistent rollback behavior on partial failure
- Reconciliation paths for money movement
- Background job retry behavior where applicable
- Clear failure modes for missing Redis/GCS/Didit/Sentry/OAuth config
- No silent error swallowing for core workflows

## Privacy, Compliance, And Operational Standard

Check for:

- Personal, KYC, financial, and admin-only data are minimized in responses and reports.
- GDPR-style export/delete flows are not broken by the audited change or issue.
- Logs contain enough operational context without leaking secrets, tokens, personal data, bank data, KYC documents, or payment data.
- Sentry/error monitoring paths are safe when configured and degrade cleanly when absent.
- Critical failures have actionable error messages for operators and safe messages for users.
- Release/deploy readiness is clear: blockers, rollback concerns, required migrations, and manual verification steps are documented.
- Migrations are backward-compatible where possible and do not assume production data shape without checks.
- Background jobs and scheduled tasks are observable enough to debug failed production runs.

## Frontend And UX Standard

Check for:

- All visible controls wired to real behavior or intentionally disabled
- Loading, success, empty, and error states
- Accessible names for buttons/links
- Form labels and useful validation messages
- Keyboard support and focus handling for modals, dropdowns, tabs, and forms
- Responsive mobile and desktop behavior
- No overlapping/overflowing text
- No broken image/font/static asset references
- No console errors during normal use
- No sensitive data stored in localStorage/sessionStorage unless explicitly justified

Use WCAG 2.2 AA as the accessibility target where practical.

## Testing Standard

Prefer evidence from:

- `cargo fmt --check`
- `cargo test`
- targeted Rust tests
- `cargo clippy` when practical
- `python3 -m pytest tests/` or targeted `tests/e2e/` commands when services are available
- browser/manual evidence for UI behavior
- network request inspection
- database verification for safe non-destructive test data

If a command cannot run, document why and what evidence was used instead.

## Automation Safety

- Audit automations must not modify production application code.
- Fix automations may only fix small, local, documented, unambiguous issues.
- Ambiguous, risky, product/business/legal/compliance/financial decisions must be documented as blocked, not guessed.
- Preserve user changes and unrelated working tree changes.
- Do not perform destructive data actions unless explicitly directed and safe test data is confirmed.
