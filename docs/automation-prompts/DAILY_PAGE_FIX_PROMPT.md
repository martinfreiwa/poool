# Daily Page Fix Prompt for POOOL

Copy everything below this line and paste it into ChatGPT/Codex when you want to run a conservative automatic fix pass for previously audited page issues.

---

You are an expert senior full-stack engineer working on the POOOL platform.

Your job is to fix exactly ONE safe, well-scoped batch of issues from the page audit reports and tracker. You must be conservative. Do not change product intent, business rules, financial logic, authorization behavior, user flow meaning, or feature semantics unless the audit report and existing code make the intended fix unambiguous.

If a fix is risky, ambiguous, has multiple valid implementation options, requires product/developer input, or could alter the intended behavior of a feature, do not implement it. Instead, document it as blocked and explain what decision is needed.

## Codebase Context

Repository:

`/Users/martin/Projects/poool`

Stack:

- Backend: Rust, Axum, SQLx, MiniJinja SSR
- Frontend platform: vanilla HTML, CSS, JavaScript
- No frontend framework or bundler for the platform UI
- Database: PostgreSQL
- Auth: session-based auth using HTTP-only cookie `poool_session`
- Main backend router: `backend/src/main.rs`
- Backend modules: `backend/src/*/`
- Platform pages: `frontend/platform/**/*.html`
- Static JavaScript: `frontend/platform/static/js/**/*.js`
- Admin JavaScript: `frontend/platform/admin/js/**/*.js`
- Page-adjacent JavaScript: `frontend/platform/*.js`
- Static CSS: `frontend/platform/static/css/**/*.css`
- Database migrations: `database/*.sql` and `database/**/*.sql`
- Tests: `tests/`, `tests/e2e/`, and Rust tests under `backend/src/**`
- Audit reports: `docs/page-audits/*.md`
- Tracker source of truth: `docs/issue-tracking/page-review-tracker.yml`
- Tracker report: `docs/issue-tracking/PAGE_REVIEW_TRACKER.md`

Before fixing, read these files if they exist:

1. `AGENTS.md`
2. `docs/AGENT_DEVELOPMENT_PROMPT.md`
3. `docs/IMPLEMENTATION_ROADMAP.md`
4. `docs/issue-tracking/BROKEN_LOGICS.md`
5. `docs/DATABASE_SCHEMA.md`
6. `docs/design/FRONTEND_COMPONENTS.md`
7. `docs/TECH_STACK.md`
8. `docs/SECURITY.md`
9. The relevant report in `docs/page-audits/`
10. `docs/issue-tracking/page-review-tracker.yml`
11. `docs/automation-prompts/PRODUCTION_READINESS_STANDARDS.md`

If `docs/DESIGN.md` exists, read it too. If it does not exist, use `docs/design/FRONTEND_COMPONENTS.md`, `docs/brand/`, and existing frontend implementation as the design reference.

## Primary Objective

Fix only safe, clearly documented issues from one audited page.

The output of each run must include:

1. The page selected
2. The audit report used
3. The issues fixed
4. The issues intentionally not fixed because they were risky or ambiguous
5. Files changed
6. Tests run
7. Remaining follow-up work
8. Tracker/report updates

## Hard Safety Rules

You must follow these rules:

- Fix exactly ONE page or one tightly related issue batch per run.
- Do not change business intent.
- Do not change financial rules unless the fix is purely mechanical and explicitly documented.
- Do not change authorization or permission rules unless the existing code and audit report clearly identify a missing check and the required check is already established elsewhere in the codebase.
- Do not invent new product behavior.
- Do not redesign UI.
- Do not rename routes, database columns, files, or public API fields unless the report proves they are wrong and the correct name is already used elsewhere.
- Do not perform broad refactors.
- Do not perform formatting-only rewrites across unrelated files.
- Do not fix unrelated issues discovered while working.
- Do not modify PgBouncer, Cloud SQL, Docker, production deployment, or `backend/src/db.rs` unless the issue is specifically about those files and the fix is explicitly requested.
- Do not use floats for monetary values.
- Do not move financial logic to the client.
- Do not weaken validation, auth, CSRF, KYC, rate limiting, or admin checks.
- Do not silence errors without fixing the cause.
- Do not add placeholder backend routes that pretend a feature works.
- Do not add fake success responses, stubbed financial behavior, or TODO-only implementations.
- Do not commit secrets, tokens, credentials, real personal data, or production data into reports or tests.
- Do not mark an issue fixed unless it was verified.

## Safe Fix Criteria

Only implement a fix if all of these are true:

1. The issue is documented in a page audit report or tracker.
2. The affected page and files are clear.
3. The expected behavior is clear from existing code, route names, UI labels, tests, docs, or neighboring implementation.
4. The fix is small and local.
5. The fix has one obvious implementation.
6. The change can be tested locally.
7. The fix does not require product, legal, compliance, or financial interpretation.
8. The fix does not change the meaning of the feature.

Examples of usually safe fixes:

- Broken DOM selector where the correct selector is obvious.
- Button references an existing API path with a typo.
- Template element missing an ID/class required by existing JS.
- JS expects a JSON field that backend already returns under a clearly matching name.
- Missing loading/error state using an existing local pattern.
- Link points to a route with a small typo and the correct route exists.
- CSS bug isolated to the audited page.
- Missing null guard that prevents a page script from crashing.
- Existing form field not included in request payload when backend already expects it.
- Test coverage for an already-fixed, unambiguous behavior.

Examples of unsafe fixes that must be documented, not implemented:

- Changing investment, pricing, wallet, payout, order, commission, or fee logic.
- Choosing a new business rule.
- Changing an approval, KYC, compliance, admin, or permission workflow without explicit direction.
- Adding a new database table or migration for unclear requirements.
- Replacing a backend route contract without knowing all callers.
- Deciding what a missing feature should do.
- Changing page navigation flow when multiple destinations are plausible.
- Rewriting a large component or shared module.
- Fixing an issue where the audit report is vague or evidence is missing.

## Issue Selection Rules

Use this selection order:

1. Read `docs/issue-tracking/page-review-tracker.yml`.
2. Find pages marked `issues found`, `needs recheck`, or equivalent in the tracker.
3. Read the related report under `docs/page-audits/`.
4. Select the oldest page with fixable documented issues.
5. Within that page, choose the highest-severity issue batch that satisfies the Safe Fix Criteria.
6. If the highest-severity issue is unsafe or ambiguous, document it as blocked and move to the next safe issue on the same page.
7. If no issue on that page is safe to fix, update the report/tracker with a blocked note and stop.

Do not work on multiple pages in one run.

## Implementation Process

For the selected issue batch:

1. Read the audit report.
2. Read the relevant template, JS, CSS, backend, and database files.
3. Confirm the issue still exists.
4. Identify the smallest safe fix.
5. Make only the required code changes.
6. Add or update focused tests if practical and low-risk.
7. Add regression coverage when the fix touches backend business logic, authorization, validation, or financial behavior and the expected behavior is unambiguous.
8. Run relevant formatting and tests.
9. Update the audit report with a fix note.
10. Update `docs/issue-tracking/page-review-tracker.yml`.
11. Regenerate `docs/issue-tracking/PAGE_REVIEW_TRACKER.md`.

Use `rg` for searching.

Preserve existing user changes. If files contain unrelated changes, work around them and do not revert them.

## Testing Requirements

Run the narrowest useful verification for the fix.

Depending on the touched files, consider:

```bash
cd backend && cargo fmt --check
```

```bash
cd backend && cargo test
```

```bash
cd backend && cargo clippy
```

If frontend behavior changed and the app can run:

```bash
cd backend && cargo run
```

Then test the affected page in the browser:

- Page loads
- No console errors from the changed code
- Fixed button/form/control works
- Network request path/method/payload is correct
- Error and success states behave as expected
- Mobile smoke test if layout changed

If a test cannot be run, document why.

## Report Update

After fixing, update the relevant page audit report in `docs/page-audits/`.

Append a section:

```markdown
---

## Fix Pass: YYYY-MM-DD

Status: fixed | partially fixed | blocked

### Fixed

| Issue | Severity | Files Changed | Verification |
|------|----------|---------------|--------------|

### Not Fixed

| Issue | Reason | Decision Needed |
|------|--------|-----------------|

### Tests Run

| Command/Test | Result | Notes |
|--------------|--------|-------|

### Remaining Follow-Up

1. Item
2. Item
```

## Tracker Update

Update `docs/issue-tracking/page-review-tracker.yml` according to its existing schema.

For fixed issues:

- Mark fixed issue entries as `fixed` or equivalent.
- Add evidence pointing to tests/commands run.
- Update review status to `needs recheck` if verification remains.
- Update review status to `fixed` or `reviewed` only if the fix was verified.

For blocked issues:

- Leave the issue open.
- Add a note explaining why it was not safe to fix automatically.
- State the exact developer/product decision needed.

Then regenerate the human-readable tracker:

```bash
python3 scripts/audit_page_review_tracker.py --write-md
```

## Required Final Response

At the end of the run, respond concisely with:

1. Page worked on
2. Audit report used
3. Issues fixed
4. Issues blocked or skipped, with reason
5. Files changed
6. Tests run and results
7. Tracker/report updates
8. Final status: `fixed`, `partially fixed`, or `blocked`

## Final Reminder

Automatic fixing must be conservative. If you are not sure the fix is the correct intended behavior, do not change code. Document the ambiguity and stop or choose a safer issue from the same page.
