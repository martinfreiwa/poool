---
description: Implement industry-standard Bank Accounts & Cards infrastructure for deposits and withdrawals
---

## Payment Methods Implementation Workflow

This workflow details the process of adding fully functional, securely tokenized Bank Accounts and Cards to the POOOL platform. It follows industry standards for PCI-DSS compliance, meaning we **never store raw credit card numbers or sensitive bank details directly**. Instead, we integrate with a modern payment gateway (e.g., Stripe, Adyen, or Wise) and store secure tokens.

---

### Phase 1: Database Migration (Tokenized Payment Methods)

Create a new migration file `database/002_payment_methods.sql` to track linked payment methods securely.

```sql
-- database/002_payment_methods.sql

CREATE TABLE payment_methods (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- "card" or "bank_account"
    method_type         VARCHAR(20) NOT NULL 
                        CHECK (method_type IN ('card', 'bank_account')),
    
    -- The token returned by Stripe/Adyen (e.g., pm_1Iqx...)
    provider_token      VARCHAR(255) NOT NULL UNIQUE,
    provider_name       VARCHAR(50) NOT NULL DEFAULT 'stripe',
    
    -- Masked details (e.g., "4242", "Visa", "Chase Bank")
    last4               VARCHAR(4),
    brand               VARCHAR(50), 
    exp_month           INTEGER,
    exp_year            INTEGER,
    account_name        VARCHAR(255),
    currency            VARCHAR(3) DEFAULT 'USD',
    
    is_default          BOOLEAN NOT NULL DEFAULT FALSE,
    status              VARCHAR(20) NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'expired', 'failed', 'deleted')),
                        
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_payment_methods_user ON payment_methods(user_id);
CREATE INDEX idx_payment_methods_token ON payment_methods(provider_token);

-- Apply updated_at trigger
CREATE TRIGGER set_updated_at_payment_methods
BEFORE UPDATE ON payment_methods
FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
```

---

### Phase 2: Backend API Routes (Rust)

Update `backend/src/wallet/routes.rs` and the `main.rs` router to handle payment method APIs.

1. **Setup Intent Endpoint (Card/Bank Linking):**
   - Create a `POST /api/payment-methods/setup` route.
   - This endpoint calls the Payment Gateway (e.g., Stripe) to generate a `SetupIntent` or `Session` token.
   - Returns the `client_secret` to the frontend.

2. **Save Payment Method Endpoint:**
   - Create a `POST /api/payment-methods` route.
   - Accepts the `provider_token` generated securely on the frontend.
   - Validates the token with the Gateway.
   - Inserts a new record into the `payment_methods` DB table.

3. **List Payment Methods Endpoint:**
   - Create a `GET /api/payment-methods` route.
   - Queries the DB for active payment methods belonging to the user.
   - Returns masked data (last4, brand) to dynamically populate the UI.

4. **Delete Payment Method Endpoint:**
   - Create a `DELETE /api/payment-methods/:id` route.
   - Detaches the token from the Payment Gateway API.
   - Marks the DB record `status = 'deleted'`.

---

### Phase 3: Frontend Integration (Secure Capture)

Update `frontend/platform/wallet.html` to securely handle sensitive inputs.

1. **Include Gateway SDK:** 
   E.g., `<script src="https://js.stripe.com/v3/"></script>`

2. **Replace Static Inputs with Secure Elements:**
   - In the "Add Bank Details" and "Add Card" modals, remove `<input>` tags for Account Numbers, IBANs, and Card Numbers.
   - Replace them with empty `<div>` containers matching the Gateway Elements schema.
   
3. **Handle Submission Securely:**
   - On modal submit, prevent default action.
   - Call `stripe.confirmCardSetup()` or `stripe.collectBankAccountToken()`.
   - On success, take the resulting `payment_method_id` and send it to our backend (`POST /api/payment-methods`).
   
4. **Dynamic UI Rendering:**
   - Update the HTML sections that display cards and banks.
   - Fetch the user's saved methods utilizing `GET /api/payment-methods`.
   - Handle Empty states gracefully ("No cards added yet").

---

### Phase 4: Funding Operations (Deposits & Withdrawals)

Enhance existing deposit/withdraw functionality to utilize actual payment rails instead of virtual updates.

1. **Update `handle_deposit`:**
   - Modify the deposit route to accept an `amount` and a `payment_method_id`.
   - Instead of immediately incrementing the balance, initiate a real `PaymentIntent` / Charge via the Payment Gateway using the saved token.
   - Only on successful asynchronous Webhook confirmation (or synchronous success) should the `wallets` table `balance_cents` be updated.

2. **Update `handle_withdraw`:**
   - Modify the withdraw route to accept an `amount` and a target payout `payment_method_id`.
   - Deduct the balance immediately to prevent double spending.
   - Initiate a Payout request via the Payment Gateway.
   - Update the `wallet_transactions` status based on Webhook events (from `processing` to `completed` or `failed`).

---

### Phase 5: Security, Compliance, and Webhooks

1. **Webhooks Implementation:**
   - Create a `POST /api/webhooks/gateway` endpoint.
   - Verify signatures to prevent spoofing.
   - Process asynchronous success/failure events for deposits and withdrawals.

2. **KYC Verification Locks:**
   - Ensure users cannot add Bank Accounts or initiate Fiat deposits/withdrawals greater than certain thresholds unless their `kyc_records` status is `approved`.
   
3. **Idempotency Keys:**
   - Implement UUID idempotency keys on all API requests to the Payment Gateway to ensure database network failures don't result in double-charging users.

---

### Phase 6: Automated Testing

1. Add explicit tests to `tests/test_platform.py`:
   - ✅ Test payment method listing returns empty array for new users.
   - ✅ Mock payment gateway response to simulate adding a saved test card.
   - ✅ Mock idempotency and test deposit with the saved test card.
   - ✅ Verify PCI scope reduction (no raw PAN data is stored in the DB).
