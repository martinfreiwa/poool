-- Durable affiliate payout request queue.
-- Created from the affiliate dashboard payout request action so the UI cannot
-- report success based only on a best-effort email notification.

CREATE TABLE IF NOT EXISTS affiliate_payout_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    affiliate_id UUID NOT NULL REFERENCES affiliates(user_id) ON DELETE CASCADE,
    amount_cents BIGINT NOT NULL CHECK (amount_cents >= 5000),
    status VARCHAR(30) NOT NULL DEFAULT 'requested'
        CHECK (status IN ('requested', 'processing', 'paid', 'rejected', 'cancelled')),
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_by_admin_id UUID REFERENCES users(id),
    processed_at TIMESTAMPTZ,
    payout_batch_id UUID REFERENCES payout_batches(id),
    admin_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_affiliate_payout_requests_open
    ON affiliate_payout_requests (affiliate_id)
    WHERE status IN ('requested', 'processing');

CREATE INDEX IF NOT EXISTS idx_affiliate_payout_requests_status_created
    ON affiliate_payout_requests (status, created_at DESC);
