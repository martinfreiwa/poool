# Financial Logic Audit: Cart And Checkout

Date: 2026-04-27

Status: issues found; production readiness blocked for checkout replay and pending-payment validation.

## Domain / Flow Audited

Cart-to-checkout purchase flow:

- User adds an approved asset to cart.
- User updates/removes cart quantities.
- User submits checkout with wallet or bank-style payment.
- Backend creates orders, order items, investments, invoices, wallet ledger rows, audit logs, affiliate commission side effects, and clears the cart.

## Files And Routes Reviewed

- `backend/src/cart/routes.rs`
  - `POST /cart/add`
  - `POST /cart/update`
  - `POST /cart/remove`
  - `GET /api/cart`
  - `GET /cart`
- `backend/src/payments/routes.rs`
  - `GET /checkout`
  - `POST /checkout`
  - `GET /api/wallets`
  - `GET /api/orders/latest`
- `backend/src/payments/service.rs`
  - `execute_checkout`
  - `calculate_platform_fee_cents`
  - `calculate_fx_deduction`
  - `cleanup_expired_orders`
  - `approve_order`
  - `reject_order`
- `backend/src/payments/models.rs`
- `frontend/platform/checkout.html`
- `frontend/platform/static/js/cart.js`
- `frontend/platform/static/js/property-detail-cart.js`
- `database/001_initial_schema.sql`
- `database/005_payments_checkout.sql`
- `database/013_admin_background_and_idempotency.sql`
- `tests/test_e2e_checkout.py`

## Tables Reviewed

- `cart_items`
- `orders`
- `order_items`
- `wallets`
- `wallet_transactions`
- `assets`
- `investments`
- `investment_limits`
- `invoices`
- `audit_logs`
- `idempotency_keys`
- `investment_disclosures_log`

## Financial Invariants Checked

- Money is stored as integer cents in Rust and `BIGINT` in PostgreSQL.
- Checkout uses a database transaction for the core money/order mutation.
- Wallet balance reads use `FOR UPDATE`.
- Asset availability reads use `FOR UPDATE`.
- Checkout must not trust client totals, token prices, or fees.
- Checkout must not create orders/investments without a valid payment path.
- Replays, double submits, and concurrent requests must not double charge or double reserve.
- State transitions must be explicit and reversible for pending bank orders.
- Ledger rows, orders, invoices, and audit logs must reconcile.

## Positive Findings

- Checkout subtotal and fee calculations are server-side and use integer cents / `Decimal`, not client-supplied totals.
- `execute_checkout` wraps the core mutation in a single transaction.
- Wallet payments lock the selected cash wallet row before balance validation and deduction.
- Asset availability is checked inside the checkout transaction, and asset rows are locked before token decrement.
- Wallet purchases write a negative `wallet_transactions` row linked to the order.
- Successful checkout creates `orders`, `order_items`, `investments`, `invoices`, and an audit log in the same transaction.
- KYC is checked in both add-to-cart and checkout submission.
- The global CSRF middleware covers `POST /checkout`; the checkout page sends `X-CSRF-Token`.

## Findings By Severity

### Critical: Checkout idempotency is optional and the browser does not send an idempotency key

Evidence:

- `backend/src/payments/routes.rs:534-605` only activates idempotency if the request includes `Idempotency-Key`.
- `frontend/platform/checkout.html:798-804` sends only `X-CSRF-Token`, not `Idempotency-Key`.
- `backend/src/payments/service.rs:376-385` locks asset rows but does not lock or consume the user's cart rows before building the order.
- `backend/src/payments/service.rs:707-712` clears cart rows at the end of the transaction.

Impact:

A double-click guard exists in one browser tab, but concurrent submits from multiple tabs, retries, mobile/web race, or a network replay can reach `execute_checkout` without an idempotency key. Because cart rows are not locked or claimed, two concurrent statements can observe the same cart before the first transaction deletes it. If asset availability and wallet balance are sufficient, duplicate orders, duplicate wallet deductions, duplicate investment increments, duplicate invoice issuance, and duplicate affiliate commission tracking are possible.

Recommended fix:

Require an idempotency key for checkout, generate it in the checkout page, scope uniqueness to `(user_id, request_path, request_method, key)`, lock the idempotency row through response persistence, and claim cart rows with `FOR UPDATE` or an atomic cart-to-order handoff before any charge/reservation. Add a concurrent checkout regression test that fires two `POST /checkout` requests for the same cart and asserts exactly one order, one ledger deduction, one investment increment, one invoice, and one cart clear.

### High: Any non-`wallet` payment method creates a pending order and reserves assets without server-side method validation

Evidence:

- `backend/src/payments/routes.rs:704-719` validates `payment_currency`, but not `payment_method`.
- `backend/src/payments/service.rs:486-542` deducts funds only when `payment_method == "wallet"`.
- `backend/src/payments/service.rs:550-558` treats every other method as `pending`.
- `backend/src/payments/service.rs:569-589` persists the submitted payment method.
- `backend/src/payments/service.rs:612-657` decrements `assets.tokens_available` and upserts `investments` even for non-wallet methods.
- `backend/src/payments/service.rs:631-637` records those investments as `funding_in_progress`.

Impact:

A direct request with `payment_method=anything` can create a pending order and reserve asset tokens without a wallet debit and without proving a supported bank-transfer flow. This can lock inventory, inflate pending investment state, update investment limits, issue invoices, and trigger downstream affiliate/referral logic for a payment method the backend never approved.

Recommended fix:

Whitelist checkout payment methods server-side, for example only `wallet` and `bank`. For `bank`, require a successfully stored proof URL or a durable provider intent/reference before reserving tokens. Reject unknown methods before entering `execute_checkout`.

### High: Bank-transfer checkout can proceed when proof upload fails or is missing

Evidence:

- `frontend/platform/checkout.html:765-778` requires proof in the browser only for selected bank payment.
- `backend/src/payments/routes.rs:642-675` attempts proof upload, but only logs a warning if upload fails.
- `backend/src/payments/routes.rs:721-727` passes `proof_url` to `execute_checkout` whether it is `Some` or `None`.
- `backend/src/payments/service.rs:569-589` inserts the order with nullable `proof_of_transfer_url`.

Impact:

Client validation can be bypassed, and GCS/storage failure still allows a pending order that reserves tokens and creates investment/invoice records. Operators may see pending orders that have no proof artifact to review, while inventory remains unavailable until cleanup/rejection.

Recommended fix:

For `payment_method=bank`, require a non-empty uploaded proof stored successfully in private storage before creating the order. If storage is unavailable, fail closed before token reservation. Consider adding explicit proof metadata and audit details for review/reconciliation.

### Medium: Investment limit usage is incremented before bank orders are actually paid

Evidence:

- `backend/src/payments/service.rs:660-667` increments `investment_limits.invested_12m_cents` for all checkout methods.
- Non-wallet methods remain `pending` at `backend/src/payments/service.rs:550-558`.
- `cleanup_expired_orders` and `reject_order` restore assets/investments, but this audit did not find corresponding annual-limit rollback in the reviewed cleanup/rejection paths.

Impact:

Pending or later-rejected bank orders can consume annual investment capacity even though no settled investment occurred. This can block users from legitimate purchases and cause compliance/reporting drift.

Recommended fix:

Either reserve limit capacity separately from settled investment usage or roll back `invested_12m_cents` in every pending-order rejection/expiry path. Add tests for bank pending, admin rejection, expiry cleanup, and wallet completed checkout.

### Medium: Money arithmetic has remaining unchecked multiplications in cart/checkout totals

Evidence:

- `backend/src/payments/service.rs:408` adds `asset_price * tokens_qty` into `subtotal_cents`.
- `backend/src/payments/service.rs:594` calculates per-item subtotal as `asset_price * tokens_qty`.
- `backend/src/cart/routes.rs` uses similar token quantity and price multiplication for cart totals and display.

Impact:

Normal production data is unlikely to overflow, but malicious or corrupt asset/token data could wrap or panic depending on build mode and context. Financial calculations should fail closed on overflow.

Recommended fix:

Use `checked_mul` and `checked_add` for all price-times-quantity and total accumulation paths, and return a safe checkout/cart error if the amount exceeds supported bounds.

## Missing Tests

- Concurrent double-submit checkout test with two or more simultaneous `POST /checkout` calls for the same cart.
- Required idempotency-key test: missing key rejected, first key succeeds, replay returns cached response, in-flight replay returns conflict, and same key for a different user cannot collide.
- Unknown payment method rejection test.
- Bank transfer proof-required test, including storage-upload failure.
- Pending bank order lifecycle test: reserve tokens, approve, reject, expire, restore assets, restore investment state, and reconcile investment limits.
- Exact platform-fee assertion in checkout E2E; current `tests/test_e2e_checkout.py` expects only asset subtotal deduction and can miss configured fee behavior.
- Wallet ledger reconciliation test across `wallets.balance_cents`, completed wallet transaction sum, order total, platform fee wallet credit, and invoice total.
- Overflow-boundary tests for token quantity and asset price multiplication.
- Cart row locking/claiming test proving stale cart rows cannot be checked out after another transaction clears them.

## Production Readiness Status

Blocked for production readiness.

The checkout flow has strong foundations for server-side pricing, integer money, transactionality, wallet locking, and audit/ledger creation. It is not production-ready until checkout idempotency is mandatory and backend validation closes the non-wallet reservation path.

## Recommended Fix Order

1. Make checkout idempotency mandatory and generated by the frontend; scope keys by user/path/method and persist response atomically.
2. Lock or atomically claim `cart_items` before order creation so a cart can only be consumed once.
3. Server-side whitelist `payment_method` and reject unknown methods.
4. Require stored proof/provider intent for bank-transfer orders before reserving tokens.
5. Fix investment-limit accounting for pending/rejected/expired bank orders.
6. Add concurrent checkout, replay, bank-proof, unknown-method, fee, and reconciliation tests.
7. Replace remaining unchecked price/quantity multiplications with checked arithmetic.
