# Daily Page Audit: Admin Dividends

Date: 2026-04-28
Page: Admin Dividends
Route: `/admin/dividends` and `/admin/dividends.html`
Template: `frontend/platform/admin/dividends.html`
Primary JS: `frontend/platform/static/js/admin-dividends.js`
Backend: `backend/src/admin/mod.rs`, `backend/src/admin/treasury.rs`, `backend/src/dividends/service.rs`
Tracker: `docs/issue-tracking/page-review-tracker.yml`
Final status: `fixed_needs_runtime_recheck`

## Executive Summary

`/admin/dividends` is wired to real backend routes for previewing, approving, and executing dividend distributions. The backend uses integer cents, transactions for calculation/execution, row locks for execution, and wallet transaction rows for credited payouts.

2026-04-28 fix pass status: the code issues found in this audit were fixed locally. Dividend APIs now require `financials.payout.*` permissions; Phase 9 no longer approve+executes in one UI action; creator self-approval and self-execution are blocked server-side; execution records `distributed_by` plus an audit log in the payout transaction; payout persistence errors roll back; Phase 9 errors are surfaced instead of falling back to legacy calculation; admin-rendered rows avoid `innerHTML`; and the UI now describes the platform investment ledger instead of an on-chain snapshot.

2026-04-28 coverage pass status: authenticated HTTP/DB/browser lifecycle coverage has been added in `tests/e2e/test_admin_dividends.py`. The new test covers create, approve, execute, cancel, permission denial, CSRF rejection, maker/checker runtime behavior, exact cents, payout rows, wallet transactions, audit logs, double-execute protection, browser safe rendering, and focus behavior.

Final status is `fixed_needs_runtime_recheck` because the new E2E could not execute locally: no backend was reachable on `http://localhost:8888/health`, and an attempted temporary backend start failed when Cargo hit `No space left on device` while writing incremental build files.

## Scope And Evidence

- Reviewed `frontend/platform/admin/dividends.html`.
- Reviewed `frontend/platform/static/js/admin-dividends.js`.
- Reviewed route registration in `backend/src/admin/mod.rs`.
- Reviewed API handlers in `backend/src/admin/treasury.rs`.
- Reviewed lifecycle and payout service logic in `backend/src/dividends/service.rs`.
- Reviewed schema support in `database/060_dividend_distributions.sql` and `database/061_dividend_payouts_extension.sql`.
- Reviewed existing tests mentioning dividends in `tests/admin/test_admin_features.py`, `tests/admin/test_admin_dashboard.py`, and Rust dividend unit tests.

## UI Inventory

| Element | Selector / Location | Expected Behavior | Wiring | Backend Support | Status |
|---|---|---|---|---|---|
| Breadcrumb Admin link | `a[href="/admin/"]` | Navigate back to admin dashboard. | Native link. | Admin page route. | Wired. |
| Ownership snapshot note | Static info block | Explain ownership source and anti-sniping. | Static. | Backend snapshots the platform investment ledger and uses `investments.purchased_at` for holding days. | Fixed. |
| Stepper | `#step-1-label`, `#step-2-label`, `#step-3-label` | Show configure, preview, process progress. | JS class updates. | None. | Wired, visual-only. |
| Asset selector | `#asset-select` | Load published/live/funded assets. | `loadAssets()` calls `GET /api/admin/assets`. | Admin assets API. | Fixed; renders titles with DOM APIs. |
| Amount input | `#total-amount` | Enter USD amount and compute cents. | Client uses `Math.round(parseFloat(value) * 100)`. | Backend validates positive integer cents. | Wired; backend is source of truth. |
| Projected yield | `#projected-yield` | Show estimated annual yield. | Client-only calculation from admin asset data. | None. | Informational only; label says APY without period basis validation. |
| Period inputs | `#period-start`, `#period-end` | Select distribution period. | Defaulted to current month; client validates end after start. | Phase 9 create validates date format and ordering. | Wired. |
| Min holding days | `#min-holding-days` | Anti-sniping threshold. | Client sends parsed integer. | Backend bounds value to `0..=365`. | Fixed. |
| Preview button | `#btn-preview` | Create/calculates preview. | Calls Phase 9 `POST /api/admin/dividends/distributions`; server errors are shown. | Real routes exist. | Fixed. |
| Preview table | `#splits-body` | Show holders, eligibility, share, payout. | Renders with DOM APIs. | Calculation result includes payout data. | Fixed. |
| CSV export | `#btn-export-csv` | Export current preview. | Client builds quoted CSV from `currentSplits`. | None. | Fixed with formula-injection hardening. |
| Discard button | `#btn-cancel` | Reset workflow. | Reloads page. | None. | Wired. |
| Confirm / Submit for Approval | `#btn-process` | Queue distribution for separate approval. | Phase 9 path shows queued state; legacy path queues approval. | Real routes exist. | Fixed. |
| Processing overlay | `#processing-overlay` | Show batch progress. | JS toggles display. | None. | Wired. |
| Success message | `#success-message`, `#final-summary-text` | Show outcome. | Uses DOM APIs. | API response summary. | Fixed. |
| Start New Distribution | `#btn-start-new-distribution` | Reset page. | Unobtrusive listener. | None. | Fixed. |
| Distribution history table | `#distributions-history-body` | List recent distributions and actions. | Calls `GET /api/admin/dividends/distributions`. | Real route exists. | Fixed; visible errors and safe rendering. |
| Refresh button | `#btn-refresh-distributions` | Reload history. | Calls `loadDistributionHistory()`. | Real route exists. | Wired. |
| History Approve/Execute/Cancel | delegated `data-action` buttons | Approve, execute, cancel historical distributions. | Delegated handlers call Phase 9 APIs. | Real routes exist with financial permissions. | Fixed; runtime recheck pending. |

## Backend And Data Review

Route registration exists for:

- `GET /admin/dividends`, `GET /admin/dividends.html`
- `POST /api/admin/dividends/calculate`
- `POST /api/admin/dividends/process`
- `GET/POST /api/admin/dividends/distributions`
- `GET /api/admin/dividends/distributions/:dist_id`
- `POST /api/admin/dividends/distributions/:dist_id/approve`
- `POST /api/admin/dividends/distributions/:dist_id/execute`
- `POST /api/admin/dividends/distributions/:dist_id/cancel`

Database support exists:

- `dividend_distributions` stores asset, period, total cents, snapshot metadata, status, creator, approver, and lifecycle timestamps.
- `dividend_payouts` stores distribution linkage, tokens, basis points, holding days, eligibility, wallet credit status, and wallet transaction IDs.
- `wallets` and `wallet_transactions` support wallet credits and ledger rows.

Positive findings:

- Money is modeled in cents (`BIGINT`/`i64`) for distribution totals and payouts.
- Execution uses a DB transaction and locks the distribution row plus payout rows before wallet credits.
- Execution refuses non-`approved` distributions.
- Wallet credits are pinned to `wallet_type = 'cash'` and `currency = 'USD'`.
- Credited payouts create `wallet_transactions` rows with type `dividend`.

## Fix Pass Update

Fixed in local working tree:

- `PAGE-ISSUE-0522`: `backend/src/admin/treasury.rs` now enforces `financials.payout.draft` for list/detail/calculate/create/process/cancel and `financials.payout.approve` for approve/execute.
- `PAGE-ISSUE-0523`: `frontend/platform/static/js/admin-dividends.js` no longer auto-approves and executes Phase 9 distributions; `backend/src/dividends/service.rs` blocks creator self-approval and self-execution.
- `PAGE-ISSUE-0424`: `database/090_dividend_distribution_execution_audit.sql` adds `distributed_by`, and execution writes `distributed_by` plus `dividend_distribution.executed` audit logs in the same transaction.
- `PAGE-ISSUE-0524`: `calculate_dividends` now propagates payout insert/update failures and checks affected rows before commit.
- `PAGE-ISSUE-0525`: Phase 9 create errors are displayed and no longer fall through to the legacy calculate route.
- `PAGE-ISSUE-0425`: asset, preview, success, and history rendering now use DOM APIs and delegated action handlers instead of `innerHTML` and inline `onclick`.
- `PAGE-ISSUE-0526`: copy now says splits come from the platform investment ledger, and the calculation uses `investments.purchased_at` for holding days.

Coverage added:

- `PAGE-ISSUE-0527`: `tests/e2e/test_admin_dividends.py` adds authenticated lifecycle E2E and browser coverage. Runtime execution remains pending because the backend was not reachable locally.

## Original Findings

### P0 - Dividend APIs allow any generic admin to draft, approve, execute, and cancel payouts

The sidebar maps dividends to `financials.payout.draft`, and the permission list also includes `financials.payout.approve`. The backend routes, however, only extract `AdminUser` and never call `admin.require_permission(...)`.

This affects both low-risk reads and high-risk mutations:

- `api_admin_dividends_calculate`
- `api_admin_dividends_process`
- `api_admin_dividends_list`
- `api_admin_dividends_create_distribution`
- `api_admin_dividends_distribution_detail`
- `api_admin_dividends_approve_distribution`
- `api_admin_dividends_execute_distribution`
- `api_admin_dividends_cancel_distribution`

Why it matters: any user with an admin role can trigger wallet-crediting dividend operations, regardless of finance payout permissions.

Recommended fix: enforce granular server-side permissions. Use `financials.payout.draft` for list/detail/calculate/create/process/cancel drafts as appropriate, and `financials.payout.approve` for approve and execute. Consider a separate `financials.payout.execute` permission if execution should be narrower than approval.

Tracker: `PAGE-ISSUE-0522`.

### P0 - Phase 9 flow bypasses four-eyes approval expectations

The primary `Confirm & Distribute Funds` handler calls `approve` and then `execute` in the same browser flow for Phase 9 distributions. The history table also exposes direct approve and execute actions. The legacy route queues an `admin_approval_requests` record for `dividend.process`, but the Phase 9 path does not use that queue or enforce a different approver/executor.

Why it matters: a single admin session can calculate, approve, and execute a payout that credits real wallet balances. That contradicts the legacy copy and weakens financial control.

Recommended fix: decide the canonical workflow. Either route Phase 9 through the existing approval queue, or add explicit maker/checker enforcement in `approve_distribution`/`execute_distribution`, including creator != approver and approver/executor constraints.

Tracker: `PAGE-ISSUE-0523`.

### P1 - Dividend execution does not record the executing admin

`api_admin_dividends_execute_distribution` discards `AdminUser`, and `execute_distribution` does not accept an actor. The distribution row records `approved_by`, but not `distributed_by`, and execution does not write an `audit_logs` row for the wallet-crediting operation.

Why it matters: payout execution is a financial mutation. Operators need an immutable actor trail for who executed the payout, not just who approved it.

Recommended fix: pass the admin user ID into execution, add `distributed_by` or a durable audit log entry, and write it in the same transaction as distribution status and wallet credits.

Tracker: `PAGE-ISSUE-0424`.

### P1 - Calculation swallows durable payout persistence failures

`calculate_dividends` inserts payout rows and later updates `eligible_holders`, but both operations discard their results with `let _ = ...`. A database constraint, type, duplicate, or schema issue can leave the distribution marked `calculated` while payout rows or the eligible-holder count were not persisted correctly.

Why it matters: the UI can show a successful preview and store a calculated distribution, but execution later fails with no eligible uncredited payouts or with data that does not match the preview.

Recommended fix: propagate payout insert/update errors, assert affected rows where appropriate, and roll back the transaction if the durable payout set does not match the returned preview.

Tracker: `PAGE-ISSUE-0524`.

### P1 - Phase 9 failures fall back to the legacy calculation path

When `POST /api/admin/dividends/distributions` returns a non-2xx response, the JS does not surface that error. It continues into the legacy `/api/admin/dividends/calculate` path. A duplicate period, server-side validation failure, or Phase 9 persistence failure can therefore be masked by a legacy preview that does not include the same lifecycle or anti-sniping persistence.

Why it matters: admins may believe they are using the Phase 9 anti-sniping workflow while the page has silently switched to legacy behavior.

Recommended fix: remove automatic fallback for non-2xx Phase 9 responses. Show the server error and require an explicit legacy-mode action if legacy remains supported.

Tracker: `PAGE-ISSUE-0525`.

### P2 - Dividend admin UI renders asset, status, and payout data through `innerHTML`

The page renders asset option labels, preview holder emails, history asset names, period/status fields, and inline action buttons through template strings assigned to `innerHTML`.

Why it matters: asset titles and account emails can contain user-controlled or operator-entered data. Admin pages are high-value XSS targets.

Recommended fix: build rows and options with DOM APIs and `textContent`, and attach action listeners with delegated events/data attributes instead of inline `onclick`.

Tracker: `PAGE-ISSUE-0425`.

### P2 - UI claims on-chain ownership snapshots, but backend snapshots off-chain investments

The page says distribution splits are calculated from on-chain ERC-1155 ownership. The backend calculation snapshots `investments` rows and uses the investment acquisition timestamp for holding days. The schema has `snapshot_block`, but the calculation does not fill it.

Why it matters: for tokenized assets, payout eligibility and share calculations must be clear about the authoritative source. If on-chain and off-chain records drift, this page can present stronger assurances than the implementation provides.

Recommended fix: either implement the on-chain snapshot/source reconciliation or change the page copy to say the current workflow uses the platform investment ledger. Persist `snapshot_block` when on-chain snapshots are actually used.

Tracker: `PAGE-ISSUE-0526`.

### P2 - Existing dividend test coverage is stale or too narrow

Rust unit tests cover holding-day helpers and selected eligibility math. The admin Python feature test still checks `/api/admin/dividends/pending`, which is not a registered route. There is no committed authenticated E2E for the current Phase 9 create/approve/execute/cancel workflow, no permission-denial test for finance permissions, and no DB assertion for wallet credits, payout rows, and audit logs.

Recommended fix: add authenticated HTTP/DB tests for create/approve/execute/cancel, CSRF rejection, permission denial, maker/checker rules, exact cents, skipped wallet behavior, audit logs, and double-execute prevention.

## Runtime / Automated Checks

| Command | Result | Notes |
|---|---:|---|
| `node --check frontend/platform/static/js/admin-dividends.js` | Pass | JS parses successfully. |
| `python3 -m pytest tests/admin/test_admin_dividends_static.py -q` | Pass | Static regression checks cover the remediation contracts. |
| `cargo fmt --check` | Pass | Backend Rust formatting is clean. |
| `CARGO_TARGET_DIR=/tmp/poool-dividends-current cargo test dividends::service --quiet` | Pass | 6 Rust dividend unit tests passed; command also reported a zero-test binary target. |
| `python3 -m py_compile tests/e2e/test_admin_dividends.py` | Pass | New E2E test syntax is valid. |
| `python3 -m pytest tests/e2e/test_admin_dividends.py -q` | Blocked | Backend health gate exited before test execution because `http://localhost:8888/health` was unreachable. |
| `CARGO_TARGET_DIR=/tmp/poool-dividends-current cargo run` | Blocked | Temporary backend start failed with `No space left on device` while writing Cargo incremental build files. |
| `python3 scripts/audit_page_review_tracker.py --write-md` | Pass | Regenerated `docs/issue-tracking/PAGE_REVIEW_TRACKER.md`; tracker audit reported no missing route/template/file references. |

Authenticated browser and mutating runtime coverage is committed but was not executed because no local backend was reachable, and the temporary backend start hit local disk exhaustion. The high-risk payout path should be verified by running the committed E2E against a live backend after disk space is available.

## Severity Counts

- Fixed this pass: Critical 2, High 3, Medium 3
- Remaining open: 0
- Low: 0

## Final Status

`fixed_needs_runtime_recheck`

Reason: all documented code and coverage gaps have local fixes, but the committed authenticated lifecycle/browser E2E still needs runtime execution against a live backend.

## Fix Next

1. Free local build disk space and start a clean backend on `localhost:8888`.
2. Run `python3 -m pytest tests/e2e/test_admin_dividends.py -q`.
