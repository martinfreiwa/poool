---
description: Full workflow to implement bank details and card details integration on the wallet page
---

# Add Payment Methods Integration (Cards & Banks) - Industry Standard / PCI Compliant

This workflow implements an industry-standard, bulletproof methodology for saving payment methods. **Crucially**, it ensures **PCI DSS compliance** by never allowing raw credit card numbers to touch or be processed by our backend servers. We use tokenization (e.g., Stripe) for cards, and secure storage for bank accounts.

## Overview

- **Database**: New `payment_methods` table focusing on tokens and metadata (masking).
- **Backend**: Rust endpoints that only accept processor tokens for cards, and encrypt/mask bank accounts.
- **Frontend**: Stripe Elements integration for the card modal (to tokenize the card securely) and HTMX for the bank modal, plus dynamic rendering.

---

## Phase 1: Database Schema

### Step 1.1 – Create the `payment_methods` table

Run against the PostgreSQL database:

```sql
CREATE TABLE IF NOT EXISTS payment_methods (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  -- 'card' or 'bank'
  method_type TEXT NOT NULL CHECK (method_type IN ('card', 'bank')),

  -- Token / External references
  processor_type TEXT,        -- e.g., 'stripe', 'xendit', 'manual'
  processor_token TEXT,       -- e.g., pm_1N5ABC..., or encrypted generic reference
  customer_id TEXT,           -- the processor's customer ID

  -- Masked Card/Bank metadata (SAFE TO STORE)
  brand TEXT,                 -- 'Visa', 'Mastercard', or bank name 'Chase', 'BCA'
  last_four TEXT,             -- **** 1234
  expiry_month INT,
  expiry_year INT,
  holder_name TEXT,

  -- Additional bank routing metadata (can be null for cards)
  routing_number TEXT,        -- e.g. SWIFT, ACH Routing, BSB (publicly known, safe)
  bank_country TEXT,

  -- Common fields
  label TEXT,                 -- user-friendly label, e.g. "My Main Card"
  is_default BOOLEAN NOT NULL DEFAULT false,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'failed')),

  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_payment_methods_user_id ON payment_methods(user_id);
```

### Step 1.2 – Add trigger for `updated_at`

```sql
CREATE OR REPLACE FUNCTION update_payment_methods_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_payment_methods_updated_at
  BEFORE UPDATE ON payment_methods
  FOR EACH ROW
  EXECUTE FUNCTION update_payment_methods_updated_at();
```

---

## Phase 2: Backend – Rust Module

### Step 2.1 – Create `src/payment_methods/` directory

Create 4 files:

| File | Purpose |
|------|---------|
| `mod.rs` | Module declaration |
| `models.rs` | Structs for API interaction / database models |
| `service.rs` | Database operations and external processor API calls (e.g., async-stripe) |
| `routes.rs` | Axum handlers |

### Step 2.2 – `src/payment_methods/models.rs`

Define the input structures. Notice that **NO RAW CARD NUMBERS ARE DEFINED HERE**.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AttachCardTokenForm {
    pub stripe_payment_method_id: String,  // Token generated client-side
    pub holder_name: String,
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddBankForm {
    pub bank_name: String,
    pub account_holder_name: String,
    pub account_number: String,            // Backend will mask immediately
    pub routing_code: String,              // SWIFT/IBAN
    pub bank_country: Option<String>,
    pub label: Option<String>,
}
```

### Step 2.3 – `src/payment_methods/service.rs`

Implement the logic securely:

1. **Card Tokenization logic (`attach_card`)**:
   - Accepts the `stripe_payment_method_id` from the frontend.
   - Makes a secure API call to Stripe (`stripe::PaymentMethod::retrieve`) to verify the token and fetch the `last4`, `brand`, `exp_month`, and `exp_year`.
   - Attaches the payment method to the Stripe Customer ID associated with the user.
   - Stores *only* the masked details and the Stripe token in the Postgres DB.

2. **Bank masking logic (`add_bank`)**:
   - Takes `account_number`, extracts the last 4 digits.
   - Hashes/encrypts the full account number if it needs to be used later for payouts, otherwise discard it after retaining the last 4 digits for display.

### Step 2.4 – `src/payment_methods/routes.rs`

Implement handlers that return HTMX fragments:

```
GET    /api/payment-methods           → list all methods (rendered into HTML fragments)
POST   /api/payment-methods/card      → securely attach card token
POST   /api/payment-methods/bank      → securely register bank account
DELETE /api/payment-methods/:id       → deactivate payment method
POST   /api/payment-methods/:id/default → set default
```

Register these routes in `main.rs`.

---

## Phase 3: Frontend – Secure Data Capture

### Step 3.1 – Client-Side Stripe Elements (Card Modal)

To remain PCI compliant, **do not use standard `<input>` fields for the credit card number**.

1. Import Stripe JS in `wallet.html`:
   ```html
   <script src="https://js.stripe.com/v3/"></script>
   ```
2. Create an empty container in the card modal for the Stripe Element:
   ```html
   <div id="add-card-modal" class="bank-modal-overlay" style="display: none;">
     ...
     <form id="stripe-card-form">
        <label>Cardholder Name</label>
        <input type="text" id="cardholder-name" required>
        
        <label>Card Details</label>
        <div id="card-element" class="form-input" style="padding: 12px;"><!-- Stripe Element injects here --></div>
        <div id="card-errors" role="alert" style="color: red;"></div>

        <button type="submit" id="submit-card-btn" class="save-bank-btn">Securely Save Card</button>
     </form>
   </div>
   ```
3. Initialize Stripe in JS:
   ```javascript
   const stripe = Stripe('pk_test_YOUR_STRIPE_PUBLIC_KEY');
   const elements = stripe.elements();
   const cardElement = elements.create('card');
   cardElement.mount('#card-element');

   document.getElementById('stripe-card-form').addEventListener('submit', async (e) => {
       e.preventDefault();
       const btn = document.getElementById('submit-card-btn');
       btn.disabled = true;

       const name = document.getElementById('cardholder-name').value;
       
       // Perform client-side tokenization
       const {paymentMethod, error} = await stripe.createPaymentMethod({
           type: 'card',
           card: cardElement,
           billing_details: { name: name }
       });

       if (error) {
           document.getElementById('card-errors').textContent = error.message;
           btn.disabled = false;
       } else {
           // Send the safe token to our backend via HTMX or Fetch
           const formData = new FormData();
           formData.append('stripe_payment_method_id', paymentMethod.id);
           formData.append('holder_name', name);
           
           htmx.ajax('POST', '/api/payment-methods/card', {
               source: '#stripe-card-form',
               target: '#wallet-cards-section',
               swap: 'innerHTML',
               values: {
                   stripe_payment_method_id: paymentMethod.id,
                   holder_name: name
               }
           }).then(() => {
               closeCardModal();
               cardElement.clear();
               btn.disabled = false;
           });
       }
   });
   ```

### Step 3.2 – Secure Bank Form

For bank details, utilize standard `hx-post` since bank account numbers, while sensitive PII, do not fall under PCI DSS. Ensure the endpoint operates strictly over HTTPS.

```html
<form class="bank-modal-form"
      hx-post="/api/payment-methods/bank"
      hx-target="#wallet-banks-section"
      hx-swap="innerHTML"
      hx-on::after-request="if(event.detail.successful) closeBankModal()">
      <!-- inputs for bank details, routing, swift code, etc. -->
</form>
```

---

## Phase 4: Dynamic UI Rendering

Update `backend/src/wallet/routes.rs` (`page_wallet`) so that it automatically filters saved methods (`SELECT * FROM payment_methods WHERE ...`) and injects them where `<!-- CARDS_PLACEHOLDER -->` and `<!-- BANKS_PLACEHOLDER -->` reside in `wallet.html`.

Each rendered entry should feature a 3-dot dropdown menu allowing the user to trigger:
- `hx-delete="/api/payment-methods/{id}"`
- `hx-post="/api/payment-methods/{id}/default"`

Both calls should re-render and swap the respective list.

---

## Compliance & Security Checklist

- [ ] **PCI DSS Compliance**: Raw card numbers (`PANs`) never hit the server. Handled entirely via Stripe.js.
- [ ] **Secure Storage**: Backend only stores `last4`, `brand`, and `processor_token`.
- [ ] **HTTPS Validation**: Forms and API requests are enforced over TLS.
- [ ] **Data Masking**: Account numbers correctly masked (`*** 4567`) in the backend before being written to the DB.
- [ ] **Idempotency**: External processor API calls are wrapped in retries and idempotency keys to prevent double-charging or duplicate registration.
- [ ] **Audit Logging**: Addition and removal of payment methods logs explicitly to the audit trail.
