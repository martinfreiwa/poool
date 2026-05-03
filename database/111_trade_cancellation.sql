-- 111_trade_cancellation.sql
-- Adds admin cancellation support to trade_history.
-- Extends on_chain_status CHECK to include 'cancelled' and stores
-- the cancelling admin, reason, and timestamp for audit/compliance.

ALTER TABLE trade_history
    DROP CONSTRAINT IF EXISTS trade_history_on_chain_status_check;

ALTER TABLE trade_history
    ADD CONSTRAINT trade_history_on_chain_status_check
        CHECK (on_chain_status IN ('pending', 'submitted', 'confirmed', 'failed', 'cancelled'));

ALTER TABLE trade_history
    ADD COLUMN IF NOT EXISTS cancelled_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS cancelled_by UUID REFERENCES users(id),
    ADD COLUMN IF NOT EXISTS cancellation_reason TEXT;

-- Partial index for pending workload queries (operators reach here often).
CREATE INDEX IF NOT EXISTS idx_trade_pending_executed_at
    ON trade_history (executed_at DESC)
    WHERE on_chain_status = 'pending';
