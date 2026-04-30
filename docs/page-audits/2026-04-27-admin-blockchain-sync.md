# Page Audit: Blockchain Sync

Date: 2026-04-27
Status: fixed, E2E verified
Auditor: ChatGPT/Codex
Page URL: `/admin/blockchain-sync`
Template: `frontend/platform/admin/blockchain-sync.html`
JavaScript: `frontend/platform/static/js/admin-blockchain-sync.js`
CSS: `frontend/platform/static/css/admin.css`, `frontend/platform/static/css/bundle.css`, inline page styles
Backend Routes: `GET /admin/blockchain-sync`, `GET /admin/blockchain-sync.html`, `GET /api/admin/blockchain/sync`, `POST /api/admin/blockchain/force-kyc-sync/:user_id`

---

## Summary

`/admin/blockchain-sync` is implemented enough to render a Web3 operations dashboard from backend APIs, but it is not production-ready. The page and sync API are available to any generic admin, the Force Sync mutation can assign a chain wallet to a user without verifying approved KYC or active status, and the user update plus audit log are not atomic. The status API also masks database failures as healthy zero/empty values, which is dangerous for an operational blockchain monitor.

2026-04-28 fix pass: the six documented issues are fixed in code. The page now has `treasury.read` page/status gates, `blockchain.manage` Force Sync gating, active approved-KYC eligibility rechecks, transactional wallet assignment plus audit logging, propagated status DB errors, correct empty-queue badge state, and no unused external HTMX import.

Final status is `fixed, E2E verified`. Targeted static tests, JS syntax checks, isolated Rust checks, and authenticated browser/API E2E now pass for the documented page scope.

---

## 2026-04-28 Fix Verification

Fixed:

- PAGE-ISSUE-0355: `/admin/blockchain-sync` and `GET /api/admin/blockchain/sync` now require `treasury.read`; `nav-blockchain-sync` uses the same permission.
- PAGE-ISSUE-0356: Force Sync now requires `blockchain.manage` and re-checks active approved-KYC eligibility under `FOR UPDATE`.
- PAGE-ISSUE-0357: Force Sync now updates `users.chain_wallet_address` and writes `audit_logs` in one SQL transaction; audit failure aborts the mutation.
- PAGE-ISSUE-0358: The sync status API now propagates core DB read failures instead of rendering zero/empty operational data.
- PAGE-ISSUE-0359: Empty whitelist queues now update the count badge to `0 pending`.
- PAGE-ISSUE-0360: The unused external HTMX CDN script was removed.

Remaining issues:

- No remaining documented issues for `/admin/blockchain-sync`.
- Broader full-suite concurrency remains a workspace concern: several unrelated Cargo jobs were active during this run, so Rust verification used isolated build state.

---

## Tested Scope

- Reviewed `frontend/platform/admin/blockchain-sync.html`.
- Reviewed `frontend/platform/static/js/admin-blockchain-sync.js`.
- Reviewed page routing in `backend/src/admin/mod.rs` and `backend/src/admin/pages.rs`.
- Reviewed blockchain sync handlers in `backend/src/admin/blockchain.rs`.
- Reviewed admin permission helper behavior in `backend/src/admin/extractors.rs` and sidebar permission mapping in `frontend/platform/static/js/admin-permission-guard.js`.
- Reviewed schema support in `database/006_admin_settings.sql`, `database/058_blockchain_integration.sql`, and `database/059_onchain_balances.sql`.
- Checked existing coverage references in `docs/E2E_COVERAGE_TRACKER.md`.

---

## Route and File Map

| Type | Path / Route | Notes |
|------|--------------|-------|
| URL | `/admin/blockchain-sync` | Clean admin page route. |
| URL alias | `/admin/blockchain-sync.html` | Registered alias. |
| Template | `frontend/platform/admin/blockchain-sync.html` | KPI cards, whitelist queue table, config panel, terminal report. |
| JS | `frontend/platform/static/js/admin-blockchain-sync.js` | Fetches sync status, renders KPIs/queue/config/report, posts Force Sync. |
| Shared JS | `user-data.js`, `admin-permission-guard.js`, `admin-theme.js`, `admin-global-search.js`, dropdown scripts | Provides CSRF helper/fetch interceptor and admin shell behavior. |
| Backend page route | `GET /admin/blockchain-sync` | `page_admin_generic`; only generic admin role currently required. |
| Backend API route | `GET /api/admin/blockchain/sync` | `api_admin_blockchain_sync_status`; only `AdminUser` currently required. |
| Backend API route | `POST /api/admin/blockchain/force-kyc-sync/:user_id` | Mutates `users.chain_wallet_address`; only `AdminUser` currently required. |
| Database table | `platform_settings` | Stores indexer enablement, poll interval, and confirmation depth. |
| Database table | `chain_indexer_cursor` | Stores last synced block and timestamp. |
| Database table | `onchain_balances` | Counted for balance-entry KPI. |
| Database table | `trade_history` | Counted for settlement status KPIs. |
| Database table | `chain_settlement_batches` | Counted for failed/last/average batch KPIs. |
| Database table | `users` | Supplies pending whitelist users and stores `chain_wallet_address`. |
| Database table | `kyc_records` | Filters whitelist queue to approved KYC records. |
| Database table | `audit_logs` | Intended audit trail for Force Sync. |

---

## UI Element Inventory

| Element | Selector / Location | Expected Behavior | Frontend Wired? | Backend Wired? | Runtime Result |
|--------|---------------------|-------------------|-----------------|----------------|----------------|
| Admin breadcrumb | `a[href="/admin/"]` | Navigate to admin dashboard. | Link only. | Yes. | Static verified; not clicked. |
| Refresh | `button[onclick="window.location.reload();"]` | Reload current dashboard. | Inline handler. | No backend beyond page/API reload. | Static verified. |
| Event Indexer status | `#kpi-indexer-status` | Show Active/Disabled. | Yes. | Yes via `/api/admin/blockchain/sync`. | Static verified; backend masks DB errors. |
| Last Synced Block | `#kpi-last-block`, `#kpi-last-sync` | Show cursor block and update time. | Yes. | Yes via `chain_indexer_cursor`. | Static verified; backend masks cursor query failure. |
| Poll Interval | `#kpi-poll-interval`, `#kpi-confirmation-depth` | Show platform setting values. | Yes. | Yes via `platform_settings`. | Static verified; backend falls back on read/parse failure. |
| On-Chain Balance Records | `#kpi-balance-entries` | Show count of synced balances. | Yes. | Yes via `onchain_balances`. | Static verified; DB failure becomes `0`. |
| Settlement status | `#kpi-settlement-status` | Show worker enablement. | Yes. | Yes via `CHAIN_SETTLEMENT_ENABLED`. | Static verified. |
| Settlement KPIs | `#kpi-pending-trades`, `#kpi-submitted-trades`, `#kpi-confirmed-trades`, `#kpi-failed-batches`, `#kpi-last-batch`, `#kpi-avg-batch` | Show trade and batch health. | Yes. | Yes via `trade_history` and `chain_settlement_batches`. | Static verified; count failures become zero. |
| Whitelist count badge | `#whitelist-count-badge` | Show pending user count or empty state. | Partially. | Yes via queue query. | Empty queue leaves badge as `Loading...`. |
| Whitelist table | `#whitelist-tbody` | Render pending KYC-approved active users. | Yes. | Yes via `users`/`kyc_records`. | Static verified; queue query failure renders empty. |
| Force Sync | Inline `onclick="forceKycSync(...)"` button | Confirm, post sync request, update row. | Yes. | Yes, but backend validation/authorization is incomplete. | Not submitted; mutation is unsafe for audit run. |
| Blockchain config panel | `#cfg-network`, `#cfg-chain-id`, `#cfg-factory`, `#cfg-registry`, `#cfg-settlement`, `#cfg-rpc` | Show chain/env config. | Yes via `textContent`. | Yes via env vars. | Static verified. |
| System Report | `#event-log-terminal` | Render operational report. | Yes. | Uses API response. | Static verified; uses `innerHTML` with mostly developer/server-controlled values. |
| Loading state | Template placeholders and terminal loading text | Show pending load. | Yes. | N/A. | Static verified. |
| Error state | `setTerminalLog(...)` | Show API load failure. | Partially. | N/A. | Only terminal updates; KPI/table placeholders can stay stale/loading. |

---

## Frontend Findings

### P2 - Empty whitelist queue leaves the count badge stuck on Loading

Location:

- Template: `frontend/platform/admin/blockchain-sync.html:171`
- JS: `frontend/platform/static/js/admin-blockchain-sync.js:80`

Problem:

`renderWhitelistQueue()` returns early for an empty queue before updating `#whitelist-count-badge`. The table says "All Clear", but the badge still says `Loading...`.

Expected:

The badge should show `0 pending` or `All synced` in the same success state as the table.

Evidence:

The badge update happens only after the empty-queue return path.

Recommended fix:

Set the badge text/color before the early return, or move badge rendering into a helper that handles both empty and non-empty states.

### P3 - Page loads external HTMX although no HTMX behavior is used

Location:

- Template: `frontend/platform/admin/blockchain-sync.html:11`

Problem:

The page loads `https://unpkg.com/htmx.org@1.9.10`, but the template contains no `hx-*` attributes. This creates an unnecessary third-party dependency on an admin blockchain operations page.

Expected:

Remove the external script or self-host it only if this page starts using HTMX.

Evidence:

Static review found no HTMX attributes in the template.

Recommended fix:

Delete the unused CDN script from the page.

---

## Backend Findings

### P1 - Blockchain sync page and status API are overbroadly available to generic admins

Location:

- Page gate: `backend/src/admin/pages.rs:227`
- API handler: `backend/src/admin/blockchain.rs:1402`
- Sidebar mapping: `frontend/platform/static/js/admin-permission-guard.js:52`

Problem:

`/admin/blockchain-sync` falls through `page_admin_generic` with only an active `admin` or `super_admin` role, and `GET /api/admin/blockchain/sync` accepts any `AdminUser`. The page exposes blockchain operational status, settlement counts, RPC configuration, and a queue of KYC-approved user emails.

Expected:

The page and read API should require the same granular permission as related blockchain pages, such as `treasury.read` or a dedicated `blockchain.view`. The sidebar entry should use the same permission.

Evidence:

`page_admin_generic` has explicit gates for `admin/blockchain-contracts` and `admin/asset-tokenize`, but no branch for `admin/blockchain-sync`. The API never calls `admin.require_permission(...)`.

Recommended fix:

Add a page-route gate and API gate, align `nav-blockchain-sync` in `PAGE_PERMISSION_MAP`, and add denial tests for generic admins without the permission.

### P1 - Force Sync can assign a chain wallet to users without proving approved KYC or active status

Location:

- Backend: `backend/src/admin/blockchain.rs:1609`

Problem:

The handler comment says it verifies the user exists and has approved KYC, but the query only selects from `users` by ID. It does not join `kyc_records`, does not require `k.status = 'approved'`, and does not require `u.status = 'active'`. An admin who can call the endpoint can assign a wallet address to any user ID without the queue's KYC/active filters.

Expected:

The mutation should re-check the exact server-side eligibility predicate used by the queue: active user, latest/valid approved KYC, and missing wallet address.

Evidence:

The queue query at `backend/src/admin/blockchain.rs:1536` filters approved KYC and active users. The mutation query at `backend/src/admin/blockchain.rs:1611` does not.

Recommended fix:

Replace the user lookup with an eligibility query joining `kyc_records`, lock the user row if the mutation remains multi-step, and return a safe 400/409 when the user is not eligible.

### P1 - Force Sync update and audit log are not atomic, and audit failures are swallowed

Location:

- Backend: `backend/src/admin/blockchain.rs:1642`
- Backend: `backend/src/admin/blockchain.rs:1650`

Problem:

The handler updates `users.chain_wallet_address` and then inserts `audit_logs` in a separate statement. The audit insert ends with `.ok()`, so an audit failure is ignored after the sensitive compliance mutation has already committed.

Expected:

The user update and audit log should run in one SQL transaction. If the audit insert fails, the wallet assignment should roll back and the API should return an error.

Evidence:

No `pool.begin()` or transaction is used in the handler. The audit insert result is intentionally discarded.

Recommended fix:

Wrap the eligibility check, wallet assignment, and audit insert in one transaction. Use `SELECT ... FOR UPDATE` or a conditional `UPDATE ... WHERE chain_wallet_address IS NULL ... RETURNING` to avoid races.

### P2 - Sync status API silently converts database failures into healthy-looking zeros and empty queues

Location:

- Backend: `backend/src/admin/blockchain.rs:1412`
- Backend: `backend/src/admin/blockchain.rs:1447`
- Backend: `backend/src/admin/blockchain.rs:1452`
- Backend: `backend/src/admin/blockchain.rs:1473`
- Backend: `backend/src/admin/blockchain.rs:1536`

Problem:

Core status reads use `.ok()`, `.unwrap_or(0)`, `.unwrap_or(0.0)`, or `.unwrap_or_default()`. A schema issue, DB outage, or query error can render as disabled indexer, zero balances/trades/failures, and an empty whitelist queue.

Expected:

Operational dashboard reads should fail visibly unless a missing optional setting is explicitly expected. Zero values should only come from successful queries.

Evidence:

Every core count/queue query in the sync handler has a fallback instead of propagating `ApiError`.

Recommended fix:

Propagate DB errors for `chain_indexer_cursor`, `onchain_balances`, `trade_history`, `chain_settlement_batches`, and whitelist queue reads. Keep explicit defaults only for missing optional settings after a successful settings query.

---

## End-to-End Test Results

| Test | Steps | Expected | Actual | Result |
|------|-------|----------|--------|--------|
| JS syntax | `node --check frontend/platform/static/js/admin-blockchain-sync.js && node --check frontend/platform/static/js/admin-permission-guard.js` | No syntax errors. | Passed with no output. | Pass |
| Static regression tests | `python3 -m pytest tests/admin/test_blockchain_sync_static.py -q` | Page/API gates, Force Sync eligibility/audit transaction, DB error propagation, empty badge, and no CDN HTMX are covered. | 5 passed. | Pass |
| Authenticated browser/API E2E | `python3 -m pytest tests/e2e/test_admin_blockchain_sync.py -q` | Permission denial, unauthenticated denial, CSRF rejection, missing `blockchain.manage`, ineligible approved-KYC rejection, successful Force Sync wallet/audit persistence, repeat rejection, page render, empty/count badge, and console health are covered. | 1 passed. | Pass |
| Rust check | `CARGO_TARGET_DIR=/tmp/poool-page-audit-target cargo check` | Compile the backend in isolated build state. | Passed. | Pass |
| Rust format | `cd backend && cargo fmt --check` | Rust formatting passes. | Passed with no output. | Pass |
| Scoped diff whitespace | `git diff --check -- backend/src/admin/blockchain.rs backend/src/admin/pages.rs frontend/platform/admin/blockchain-sync.html frontend/platform/static/js/admin-blockchain-sync.js frontend/platform/static/js/admin-permission-guard.js tests/admin/test_blockchain_sync_static.py tests/e2e/test_admin_blockchain_sync.py docs/page-audits/2026-04-27-admin-blockchain-sync.md docs/page-review-tracker.yml docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md` | No whitespace errors in touched files. | Passed with no output. | Pass |
| Static page route review | Reviewed `backend/src/admin/mod.rs` and `backend/src/admin/pages.rs`. | Page route exists and auth behavior is identifiable. | Routes exist; generic admin-only gate found. | Pass |
| Static API route review | Reviewed `backend/src/admin/mod.rs` and `backend/src/admin/blockchain.rs`. | API routes exist and auth/validation behavior is identifiable. | Routes exist; permission and validation gaps found. | Pass |
| Mutation runtime test | Submit Force Sync against seeded E2E users. | Eligible active approved-KYC user gets one wallet assignment and one audit row; ineligible and repeated submissions do not mutate. | Covered by `tests/e2e/test_admin_blockchain_sync.py`. | Pass |

---

## Security Findings

- Fixed: Generic admin access is replaced by `treasury.read` for the page and status API.
- Fixed: Force Sync now requires `blockchain.manage` and re-checks active approved-KYC eligibility server-side.
- Fixed: Force Sync now persists the wallet assignment and audit row atomically.
- Verified: authenticated tests prove no-CSRF, no-permission, ineligible-user, repeat-submission, and happy-path audit behavior.

---

## Database Findings

- Required schema exists for `users.chain_wallet_address`, `chain_settlement_batches`, `onchain_balances`, `chain_indexer_cursor`, `platform_settings`, `kyc_records`, and `audit_logs`.
- Fixed: Force Sync now locks the target user with `FOR UPDATE`.
- Fixed: wallet assignment and `audit_logs` insert now run in a single transaction.
- Verified: DB-backed E2E confirms the user row and audit row commit together, and that ineligible users do not mutate state.

---

## Follow-Up Coverage

- Added authenticated E2E/API test for `/admin/blockchain-sync` access with `treasury.read`.
- Added denial test for generic admin without `treasury.read`.
- Added Force Sync API tests for no CSRF, no `blockchain.manage`, already has wallet, no approved KYC, and successful update plus audit row.
- Static regression covers DB error propagation and empty whitelist badge behavior.
- Optional future coverage: mobile-specific layout and keyboard confirmation-flow smoke if this page receives more UI changes.

---

## Recommended Fix Order

1. Continue monitoring broader Cargo/build contention in the shared workspace.
2. Extend browser coverage later for mobile-specific layout if this page receives more UI changes.

---

## Final Status

`fixed, E2E verified`

Reason: All six documented code issues were fixed and covered by static regression tests plus authenticated browser/API E2E. Isolated Rust check and Rust formatting also passed.
