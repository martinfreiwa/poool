-- ============================================================
-- POOOL Platform – Migration 040: Withdrawal Requests
-- Tracks manual withdrawal requests requiring admin approval
-- ============================================================

CREATE TABLE IF NOT EXISTS withdrawal_requests (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount_cents        BIGINT NOT NULL CHECK (amount_cents > 0),
    currency            VARCHAR(3) NOT NULL DEFAULT 'USD',
    payment_method_id   UUID REFERENCES payment_methods(id),
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'approved', 'rejected', 'cancelled')),
    admin_notes         TEXT,
    approved_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_withdraw_req_user ON withdrawal_requests(user_id);
CREATE INDEX IF NOT EXISTS idx_withdraw_req_status ON withdrawal_requests(status);

-- Set updated_at trigger
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'set_updated_at' AND tgrelid = 'withdrawal_requests'::regclass
    ) THEN
        CREATE TRIGGER set_updated_at BEFORE UPDATE ON withdrawal_requests
        FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
    END IF;
END;
$$;
