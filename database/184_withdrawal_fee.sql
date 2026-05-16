-- ══════════════════════════════════════════════════════════════
-- 184_withdrawal_fee.sql
--
-- Make the withdrawal fee actually deducted from user balance (P1-5).
-- The setting `withdrawal_fee_cents` existed since migration 006 but
-- was never read by the handler — fee was advisory text only.
--
-- New `fee_cents` column lets us preserve the fee that applied at
-- submission time even if the admin tunes the setting before the
-- request is approved.
-- ══════════════════════════════════════════════════════════════

ALTER TABLE withdrawal_requests
    ADD COLUMN IF NOT EXISTS fee_cents BIGINT NOT NULL DEFAULT 0
        CHECK (fee_cents >= 0);

-- Convenience index: admins often filter "any withdrawal with a non-
-- zero fee for this month" when running reconciliation.
CREATE INDEX IF NOT EXISTS idx_withdrawal_requests_fee
    ON withdrawal_requests(fee_cents, created_at DESC)
    WHERE fee_cents > 0;
