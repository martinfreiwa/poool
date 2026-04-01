-- Table for payment disputes/chargebacks
CREATE TABLE payment_disputes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    transaction_id UUID REFERENCES wallet_transactions(id),
    provider VARCHAR(50) NOT NULL, -- stripe, ocbc, mangopay
    provider_dispute_id VARCHAR(255) NOT NULL UNIQUE,
    amount_cents BIGINT NOT NULL,
    currency VARCHAR(3) NOT NULL,
    reason VARCHAR(255),
    status VARCHAR(50) NOT NULL DEFAULT 'open', -- open, under_review, won, lost
    evidence_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for searching disputes by user
CREATE INDEX idx_payment_disputes_user ON payment_disputes(user_id);
-- Index for provider fast lookup
CREATE INDEX idx_payment_disputes_provider_id ON payment_disputes(provider_dispute_id);
