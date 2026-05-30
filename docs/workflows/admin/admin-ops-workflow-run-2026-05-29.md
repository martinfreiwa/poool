# Admin/Ops Workflow Run - 2026-05-29

Environment:
- Local backend: `http://localhost:8888`
- Database: local `poool` plus local `poool_community`
- Browser/E2E harness: Playwright via pytest

Executed:
- `python3 -m pytest tests/e2e/test_admin_dashboard_index.py tests/e2e/test_admin_deposits.py tests/e2e/test_admin_kyc_smoke.py tests/e2e/test_admin_dividends.py tests/e2e/test_admin_support.py tests/e2e/test_admin_notifications.py tests/e2e/test_admin_reports_export.py tests/e2e/test_admin_settings.py tests/e2e/test_admin_ui_layout.py -q`

Result:
- Admin/Ops browser checks passed: `16 passed in 21.23s`
- Backend checks passed: `cargo fmt --check` and `cargo check`

Issues found and fixed during this run:
- Concurrent lazy creation of developer default teams could hit `one_default_team_per_developer`. The default-team insert now uses an atomic upsert against the partial unique index.
- Affiliate daily rollup worker used `ON CONFLICT (rollup_date, link_id)` after the schema was extended to include `currency` in the primary key. The worker now targets `(rollup_date, link_id, currency)`.
- Circle Ops background notification upserts did not match the current partial unique index predicates in the community database. Conflict predicates now match the live indexes.
- Expected dividend business-rule rejections were logged as `[P0-FINANCIAL]` errors. They now return `400` and log as warnings; internal/DB/payout failures still log as P0 errors.
- Local development reconciliation/invariant workers produced fatal drift errors against disposable and historical test data. They remain enabled outside development and can be explicitly enabled locally through `ENABLE_FINANCIAL_RECONCILIATION_WORKER=true` and `ENABLE_FINANCIAL_INVARIANT_WORKER=true`.

Workflow coverage added:
- Admin dashboard safe rendering, refresh/search cards, and layout smoke.
- Deposit confirmation permissions, CSRF rejection, audit, and UI readback.
- KYC dashboard and review modal smoke.
- Dividend draft/approve/execute duplicate/business-rule handling, wallet credits, payouts, and audit.
- Support ticket list/detail/reply/status update.
- Notifications validation/CSRF/send path.
- Reports CSV/export preview and admin settings pages.

Remaining gaps:
- This run covers the core Admin/Ops browser block, not every marketplace/blockchain/admin-content page.
- Redis-backed trading workers and outbound email delivery remain local disabled states.
