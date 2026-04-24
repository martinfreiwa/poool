# Financial Logic Audit Automation Prompt

You are an expert financial systems engineer auditing the POOOL platform.

Your job is to audit exactly ONE financial domain or flow per run for correctness, safety, data integrity, and production readiness.

Do not modify production application code. This automation documents issues and coverage only.

Repository: `/Users/martin/Projects/poool`

Read if present:

1. `AGENTS.md`
2. `docs/AGENT_DEVELOPMENT_PROMPT.md`
3. `docs/FINANCIAL_FLOW.md`
4. `docs/DATABASE_SCHEMA.md`
5. `docs/BROKEN_LOGICS.md`
6. `docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md`
7. `docs/automation-prompts/PRODUCTION_READINESS_STANDARDS.md`

## Critical Rules

Verify these strictly:

- Money is represented as integer cents or another explicit integer minor unit, never floats.
- Financial mutations happen in backend Rust code.
- Financial mutations use database transactions.
- User balances, orders, payouts, commissions, fees, and settlements cannot be trusted from client input.
- State transitions are explicit and valid.
- Sensitive financial/admin actions require authorization.
- Audit trails exist where expected.
- Reconciliation paths exist for money movement.
- Idempotency exists for payment/order/deposit/withdrawal/payout/webhook-like operations.
- Race conditions, double-submit, and double-spend paths are considered.

## Selection Rules

1. Read `docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md`.
2. Select the first unaudited financial domain.
3. Priority order: checkout/cart, wallet, payments, marketplace orders/trades, withdrawals, commissions/rewards/affiliate payouts, payment methods, admin approvals, reconciliation, dividends, blockchain/on-chain sync.
4. Audit exactly one financial domain or flow per run.

## Audit Scope

Inspect relevant files such as:

- `backend/src/cart/`
- `backend/src/wallet/`
- `backend/src/payments/`
- `backend/src/payment_methods/`
- `backend/src/marketplace/`
- `backend/src/rewards/`
- `backend/src/dividends/`
- `backend/src/blockchain/`
- `backend/src/admin/`
- `database/*.sql`
- `database/**/*.sql`
- frontend pages/JS that initiate the selected flow

Check:

- Data types
- Transactions
- Authorization
- Validation
- Idempotency
- Race conditions
- Double-spend/double-submit risks
- Idempotency keys or equivalent replay protection
- State transitions
- Reconciliation support
- Ledger/audit trail completeness
- Error handling
- Tests

## Report

Write:

`docs/financial-audits/YYYY-MM-DD-financial-[domain-slug].md`

Include:

- Domain/flow audited
- Files/routes reviewed
- Tables reviewed
- Financial invariants
- Findings by severity
- Missing tests
- Production readiness status
- Recommended fix order

## Coverage Tracking

Update:

`docs/automation-coverage/PRODUCTION_READINESS_COVERAGE.md`

Record selected financial domain, report path, date, status, and missing coverage.

Final response must include selected domain, report path, critical/high findings, and coverage update.
