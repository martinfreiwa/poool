---
description: Advanced FinTech Payments, Multi-Currency Checkout, and Invoicing (Phase 4)
---

# Workflow: Advanced FinTech Payments & Multi-Currency Checkout

This workflow provides a deep, enterprise-grade architecture for handling Multi-Currency Payments (USD, IDR), Bank Deposits, Foreign Exchange (FX) during Checkout, and Automated Invoicing. It is designed for strict compliance, traceability, and transactional safety.

## 1. Database Schema Overhaul (The Ledger Concept)
To handle real financial flows accurately, we must expand the database to support multi-currency, Escrow accounts, and precise order tracking.

### A. Wallets & System Accounts
Users can hold multiple fiat currencies. Additionally, when a user buys an asset, the money shouldn't just "disappear"—it should move from the **User's Wallet** to an **Escrow/Asset Wallet**.
1. **`wallets` Table Updates:**
   - Add `currency VARCHAR(3) NOT NULL` (e.g., 'USD', 'IDR').
   - Modify the unique constraint: `UNIQUE (user_id, wallet_type, currency)`.
   - Ensure the system itself can have wallets (e.g., `user_id` can be NULL or assigned to a POOOL System UUID) to act as Escrow accounts for Assets being funded.
2. **`wallet_transactions` Table Updates:**
   - Instead of a simple `amount_cents`, implement a strict ledger-like structure if possible, but at minimum ensure `currency` is logged and `status` reflects clearing states (`pending`, `cleared`).

### B. Deposits (Intent vs. Settlement)
When a user wants to deposit IDR via a Virtual Account (VA), they first create an *Intent*.
1. **Create `deposit_requests` Table:**
   - `id` (UUID)
   - `user_id` (UUID)
   - `currency` (VARCHAR(3))
   - `amount_cents` (BIGINT)
   - `provider` (VARCHAR - 'xendit', 'stripe', 'mangopay')
   - `provider_reference` (VARCHAR - e.g., the VA number or payment intent ID)
   - `status` (VARCHAR - 'pending', 'paid', 'expired', 'failed')
   - `expires_at` (TIMESTAMPTZ)
   - `created_at` / `updated_at`

### C. Orders & FX (Foreign Exchange)
Assets are priced in a base currency (e.g., USD). If a user pays in IDR, we must lock in an exchange rate to guarantee the asset funding target is met.
1. **`orders` Table Updates:**
   - `asset_currency` (VARCHAR(3))
   - `total_asset_cents` (BIGINT)
   - `payment_currency` (VARCHAR(3))
   - `total_payment_cents` (BIGINT)
   - `fx_rate_applied` (DECIMAL(18,8) or BIGINT for scaled integer representation)
   - `fx_provider` (VARCHAR)

### D. Invoicing & Compliance
1. **Create `invoices` Table:**
   - `id` (UUID)
   - `invoice_number` (VARCHAR, sequential/unique sequence e.g., `INV-2026-0001`)
   - `order_id` (UUID)
   - `user_id` (UUID)
   - `company_entity` (VARCHAR - e.g., "POOOL LLC" vs "PT POOOL Indonesia", depending on currency/jurisdiction)
   - `subtotal_cents`, `tax_cents`, `total_cents`, `currency`
   - `pdf_url` (VARCHAR)
   - `status` ('draft', 'issued', 'void')
   - `issued_at` (TIMESTAMPTZ)

## 2. Bank Deposit Flow Implementation
### IDR via Xendit / Midtrans (Virtual Accounts)
1. User enters `Rp 10.000.000` on the Deposit page.
2. Backend calls Xendit API to generate a Virtual Account (VA).
3. Backend inserts into `deposit_requests` (`status = 'pending'`).
4. User transfers money via their local Indonesian bank.
5. Xendit sends a Webhook to `POST /api/webhooks/xendit`.
6. **Webhook Handler (Atomic Transaction):**
   - Verify webhook signature.
   - Look up `deposit_requests` by reference and verify it's not already `paid`.
   - Update `deposit_requests` to `paid`.
   - Look up or create the IDR `wallet` for the user.
   - Insert `wallet_transactions` (`type = 'deposit'`, `status = 'completed'`).
   - Add `amount_cents` to `wallets.balance_cents`.
   - Commit transaction.

### USD via Stripe or Wire Transfer (Manual)
1. For manual wire transfers, user submits a form indicating they sent `$50,000`. Status is `pending_verification`. Admin verifies bank statement and manually clicks "Approve" in the admin dashboard, triggering the wallet credit.

## 3. The Multi-Currency Checkout Engine
1. **Initiate Purchase:** User clicks "Buy 10 Tokens" of a USD-priced Asset, but selects their "IDR Wallet" for payment.
2. **Fetch Live FX:** Backend fetches USD/IDR rate (e.g., via OpenExchangeRates or a PSP Oracle). Add a small spread (e.g., 0.5%) if POOOL charges FX fees.
3. **Atomic Purchase Transaction (`sqlx::Transaction`):**
   - Check `asset.tokens_available >= 10`.
   - Calculate `total_asset_cents` (e.g., $1000 = 100,000 cents).
   - Calculate `total_payment_cents` in IDR (e.g., Rp 15.500.000).
   - Check if User's IDR wallet `balance_cents >= total_payment_cents`.
   - **Deduct User IDR Wallet:** `- Rp 15.500.000`.
   - **Credit System Escrow USD Wallet:** `+ $1000` (represents funds held for the asset developer). *Note: The actual FX conversion of corporate funds happens reconciling PSP balances, but the logical ledger must reflect the asset is funded in USD.*
   - Reduce `asset.tokens_available -= 10`.
   - Insert `orders` (status = `completed`, saving the exact FX rate used).
   - Insert or update `investments` (user owns 10 tokens now).
   - Commit transaction.

## 4. Automated Invoicing Generation
1. Immediately after the Checkout Engine commits successfully, spawn a background task (or push to a Redis queue like `Faktory` or `obang`).
2. **Generate PDF:**
   - Query the `orders`, `users`, and `assets` tables.
   - Use a Rust PDF crate (e.g., `printpdf`, `typst`) or call a microservice to render an HTML invoice template into a PDF.
   - The invoice must state: "Payment received in IDR (Rp 15.500.000) for Asset XXX priced at USD $1000. Exchange Rate applied: 15,500 IDR/USD".
3. **Store & Deliver:**
   - Upload PDF to AWS S3 / Cloudflare R2.
   - Save the S3 URL to `invoices.pdf_url`.
   - Send Email with PDF attachment to user.

## 5. Security & Idempotency Rules
- **Webhooks:** Must be strictly idempotent. If Xendit fires the "VA Paid" webhook twice, the database must not credit the user twice. Use `deposit_requests.status` or `external_ref_id` as unique constraints.
- **Race Conditions:** During checkout, `SELECT ... FROM assets WHERE id = $1 FOR UPDATE` to lock the asset row. This prevents two users from buying the last token at the exact same millisecond.
- **No Floats:** All balances, prices, and FX calculations must use integers (`BIGINT` cents) or `DECIMAL`/`Numeric` types.

## 6. Next Steps for Implementation
1. Write the **SQL Migrations** to upgrade `wallets`, `wallet_transactions` and create `deposit_requests`, `orders` (upgraded), and `invoices`.
2. Implement the **Deposit Request Route** and the **Bank Webhook Listener**.
3. Implement the **Checkout Engine** with the `FOR UPDATE` lock and FX conversion logic.
4. Implement the **PDF Invoice Generator**.
