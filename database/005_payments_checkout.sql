-- ============================================================
-- POOOL Platform – Migration 005: Payments, Checkout & Invoicing
-- Adds multi-currency wallet support, deposit requests,
-- enhanced orders, and invoices.
-- ============================================================

-- ============================================================
-- 1. Extend wallets with currency
-- ============================================================
-- Add currency column (default 'USD' for existing rows)
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'USD';

-- Drop old unique constraint and create the new one with currency
ALTER TABLE wallets DROP CONSTRAINT IF EXISTS wallets_user_id_wallet_type_key;
ALTER TABLE wallets ADD CONSTRAINT wallets_user_id_wallet_type_currency_key
    UNIQUE (user_id, wallet_type, currency);

-- ============================================================
-- 2. Extend wallet_transactions with currency
-- ============================================================
ALTER TABLE wallet_transactions ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'USD';

-- ============================================================
-- 3. deposit_requests – Intent tracking for bank deposits
-- ============================================================
CREATE TABLE IF NOT EXISTS deposit_requests (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    currency            VARCHAR(3) NOT NULL,
    amount_cents        BIGINT NOT NULL CHECK (amount_cents > 0),
    provider            VARCHAR(30) NOT NULL
                        CHECK (provider IN ('stripe', 'xendit', 'midtrans', 'mangopay', 'manual')),
    provider_reference  VARCHAR(255),
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'paid', 'expired', 'failed', 'cancelled')),
    payment_method      VARCHAR(50),
    expires_at          TIMESTAMPTZ,
    paid_at             TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_deposit_req_user ON deposit_requests(user_id);
CREATE INDEX IF NOT EXISTS idx_deposit_req_status ON deposit_requests(status);
CREATE INDEX IF NOT EXISTS idx_deposit_req_provider_ref ON deposit_requests(provider_reference);

-- ============================================================
-- 4. Extend orders with multi-currency & FX fields
-- ============================================================
ALTER TABLE orders ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'USD';
ALTER TABLE orders ADD COLUMN IF NOT EXISTS payment_currency VARCHAR(3);
ALTER TABLE orders ADD COLUMN IF NOT EXISTS fx_rate DECIMAL(18, 8);
ALTER TABLE orders ADD COLUMN IF NOT EXISTS fx_provider VARCHAR(50);

-- ============================================================
-- 5. invoices – Automated invoicing for completed orders
-- ============================================================
CREATE TABLE IF NOT EXISTS invoices (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_number      VARCHAR(30) NOT NULL UNIQUE,
    order_id            UUID NOT NULL REFERENCES orders(id),
    user_id             UUID NOT NULL REFERENCES users(id),
    company_entity      VARCHAR(255) NOT NULL DEFAULT 'POOOL GmbH',
    subtotal_cents      BIGINT NOT NULL,
    tax_cents           BIGINT NOT NULL DEFAULT 0,
    total_cents         BIGINT NOT NULL,
    currency            VARCHAR(3) NOT NULL DEFAULT 'USD',
    pdf_url             VARCHAR(512),
    status              VARCHAR(20) NOT NULL DEFAULT 'issued'
                        CHECK (status IN ('draft', 'issued', 'void')),
    notes               TEXT,
    issued_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_invoices_user ON invoices(user_id);
CREATE INDEX IF NOT EXISTS idx_invoices_order ON invoices(order_id);
CREATE INDEX IF NOT EXISTS idx_invoices_number ON invoices(invoice_number);

-- ============================================================
-- 6. invoice_sequence – Atomic invoice number generation
-- ============================================================
CREATE SEQUENCE IF NOT EXISTS invoice_number_seq START 1;

-- ============================================================
-- 7. Apply updated_at trigger to new tables
-- ============================================================
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'set_updated_at' AND tgrelid = 'deposit_requests'::regclass
    ) THEN
        CREATE TRIGGER set_updated_at BEFORE UPDATE ON deposit_requests
        FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
    END IF;
END;
$$;

-- ============================================================
-- Done! 🎉
-- ============================================================
