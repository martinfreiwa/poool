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
