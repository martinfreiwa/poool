-- Migration 016: Referral Analytics (Clicks & Metrics)

-- Track individual clicks on referral links
CREATE TABLE IF NOT EXISTS referral_clicks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(32) NOT NULL REFERENCES referral_codes(code) ON DELETE CASCADE,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for quick metric counts
CREATE INDEX idx_referral_clicks_code ON referral_clicks(code);
