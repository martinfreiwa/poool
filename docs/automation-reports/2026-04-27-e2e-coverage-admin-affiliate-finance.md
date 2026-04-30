# E2E Coverage Gap Audit: Admin Affiliate Finance

Date: 2026-04-27
Automation: Daily POOOL E2E Coverage Gap Tracker
Selected scope: `/admin/affiliate-finance` and `/api/admin/rewards/affiliates/payouts/pending` plus `/api/admin/rewards/affiliates/:id/payout`
Report path: `docs/automation-reports/2026-04-27-e2e-coverage-admin-affiliate-finance.md`

## Production Readiness Verdict

Coverage is not adequate for production. This flow releases affiliate commission money from the affiliate treasury into user cash wallets, so release confidence requires committed authenticated E2E coverage with database assertions for integer-cent balances, payout batches, wallet transaction rows, audit logs, tax-document gating, authorization, and CSRF.

## Existing Coverage Found

- `tests/test_e2e_affiliate.py` covers affiliate onboarding, referral attribution, dashboard earnings, and the affiliate payout request notification endpoint. It does not open `/admin/affiliate-finance`, does not call the admin batch payout endpoint, and does not verify treasury debit, cash-wallet credit, payout batches, wallet transactions, or audit rows.
- `tests/test_e2e_affiliate_full_funnel.py` calls `POST /api/admin/rewards/affiliates/:id/payout`, but it is stale against the current implementation. It seeds or checks outdated wallet assumptions, including `wallet_type = 'default'`, and does not set `affiliates.tax_document_gcs_path`, while the current payout route credits `wallet_type = 'cash'` and blocks payout without a tax document.
- `tests/admin/test_admin_features.py` only performs broad financial admin API smoke checks for orders, deposits, and treasury. It does not cover affiliate finance or payout release.
- `tests/admin/test_affiliate_route_contract_static.py` checks that the affiliate finance nav item is permission-mapped to `affiliates.manage`. It does not verify runtime page access, API authorization, CSRF, payout behavior, or UI state.
- No committed Playwright test was found for `frontend/platform/admin/affiliate-finance.html` or `frontend/platform/admin/js/admin-affiliate-finance.js`.

Related documentation evidence:

- `docs/page-audits/2026-04-25-admin-affiliate-finance.md`
- `docs/route-contract-audits/2026-04-27-route-contract-admin-affiliate-finance.md`
- `docs/page-review-tracker.yml` issues `PAGE-ISSUE-0034` through `PAGE-ISSUE-0038`

## Missing Coverage

- Happy path: admin with `affiliates.manage` loads the finance page, sees a seeded payable affiliate, opens the payout modal, executes the payout, and the row disappears or updates deterministically.
- Backend state: payout creates exactly one `payout_batches` row, marks only the selected payable commission IDs as `paid`, sets `payout_batch_id`, debits the affiliate treasury wallet, credits the affiliate `cash` wallet, writes matching `wallet_transactions`, and writes `AFFILIATE_BATCH_PAYOUT_EXECUTED` in `audit_logs`.
- Financial integrity: all assertions use integer cents, verify the exact payout total, and prove no unsummed payable commission row is accidentally marked paid.
- Tax gate: affiliates without `tax_document_gcs_path` are blocked before mutation and the UI/API show a deterministic compliance error.
- Minimum threshold: totals below 5000 cents disable or reject payout and do not mutate commissions, wallets, batches, or audit logs.
- Authorization: unauthenticated users, non-admin users, and admins without `affiliates.manage` are blocked from the page, pending payout API, and payout mutation.
- CSRF: payout POST without a valid `X-CSRF-Token` fails and does not mutate state.
- Concurrency/idempotency: two admins or two requests attempting the same payout produce one committed payout and one no-op/conflict/error response without double crediting.
- UI error states: pending-list API failure, payout API failure, insufficient treasury, missing treasury wallet, and tax-gate rejection are visible and retryable without leaving buttons permanently disabled.
- Security rendering: seeded affiliate name, email, and referral code containing HTML/JavaScript-like text must not break the inline payout action or execute script.
- Accessibility/responsive: payout modal needs keyboard open/close, Escape, focus management, accessible dialog semantics, and a mobile viewport smoke.

## Suggested Test Files And Names

Primary Playwright/API test file:

- `tests/e2e/test_admin_affiliate_finance.py`

Suggested tests:

- `test_admin_affiliate_finance_requires_login_and_affiliates_manage`
- `test_admin_affiliate_finance_lists_pending_payouts_from_real_api`
- `test_admin_affiliate_finance_executes_payout_and_persists_ledger_state`
- `test_admin_affiliate_finance_blocks_missing_csrf_without_mutation`
- `test_admin_affiliate_finance_blocks_missing_tax_document`
- `test_admin_affiliate_finance_blocks_under_minimum_threshold`
- `test_admin_affiliate_finance_does_not_pay_unsummed_commissions`
- `test_admin_affiliate_finance_concurrent_payout_executes_once`
- `test_admin_affiliate_finance_surfaces_api_errors_and_recovers_button_state`
- `test_admin_affiliate_finance_modal_keyboard_mobile_and_xss_fixture`

Backend-focused companion tests if concurrency is hard to express in Playwright:

- `backend/src/admin/rewards_tests.rs`
- `affiliate_batch_payout_updates_only_locked_commission_ids`
- `affiliate_batch_payout_requires_tax_document`
- `affiliate_batch_payout_writes_balanced_wallet_transactions_and_audit_log`

## Test Data Required

- Admin session with `affiliates.manage`.
- Admin session without `affiliates.manage`.
- Active affiliate with `referral_code`, profile data, and `tax_document_gcs_path`.
- Active affiliate without `tax_document_gcs_path`.
- One or more `affiliate_referrals` rows linked to payable `affiliate_commissions`.
- Affiliate treasury wallet with a known USD integer-cent balance and enough funds.
- Existing affiliate `cash` wallet fixture and a no-wallet case to verify create-or-credit behavior.
- Unsafe-looking but inert affiliate name/email/referral-code fixtures for rendering checks.
- Cleanup keyed by seeded user IDs, referral IDs, commission IDs, payout batch IDs, wallet transaction `external_ref_id`, and audit log entity IDs.

## Priority Order

1. HTTP+DB regression for authenticated happy-path payout, exact cents, payout batch, commission status, wallet transactions, and audit log.
2. Security negatives for unauthenticated access, missing `affiliates.manage`, missing CSRF, missing tax document, and under-threshold payout.
3. Financial integrity regression proving only locked/summed commission IDs are paid and concurrent payout executes once.
4. Playwright UI coverage for pending list, payout modal, success/error states, and retry behavior.
5. Keyboard, mobile, and XSS fixture coverage for the modal and payout rows.

## Minimum Regression Suite Before Release

- One committed test that seeds a tax-ready active affiliate with payable commissions, executes the real admin payout path, and verifies exact `payout_batches`, `affiliate_commissions`, `wallets`, `wallet_transactions`, and `audit_logs` state.
- One committed test that proves missing tax document, under-threshold totals, insufficient permissions, and missing CSRF do not mutate state.
- One committed concurrency or idempotency test proving the same payable commission set cannot be paid twice.
- One Playwright smoke for the admin finance page, payout modal, visible API error state, and keyboard/mobile behavior.

## Commands Run

Documentation/source audit only. No production application code was modified and no tests were added.

Read/search commands included:

```bash
rg -n "affiliate-finance|affiliate finance|affiliate_finance|affiliate.*payout|payout.*affiliate|/api/admin/rewards/affiliates|affiliate.*finance|commission|payout" tests backend/src frontend/platform docs -g '!target'
rg --files tests backend/src frontend/platform | rg 'affiliate|rewards|payout|finance|admin.*affiliate'
sed -n '1,260p' frontend/platform/admin/affiliate-finance.html
sed -n '1,180p' frontend/platform/admin/js/admin-affiliate-finance.js
sed -n '1000,1325p' backend/src/admin/rewards.rs
sed -n '250,330p' tests/test_e2e_affiliate_full_funnel.py
sed -n '1,210p' tests/test_e2e_affiliate.py
sed -n '210,250p' tests/admin/test_admin_features.py
sed -n '1,80p' tests/admin/test_affiliate_route_contract_static.py
sed -n '744,900p' docs/page-review-tracker.yml
sed -n '1,220p' docs/page-audits/2026-04-25-admin-affiliate-finance.md
```
