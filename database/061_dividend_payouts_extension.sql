-- Migration 061: Extend dividend_payouts for Phase 9 payout execution
-- Purpose: Add wallet_credited, credited_at columns and relax investment_id constraint
-- Ref: Phase 9.4 — Dividend Payout Execution

-- 🔴 These columns are needed by dividends::service::execute_distribution()
--    which credits wallets and tracks the wallet transaction link.

-- 1. Add wallet_credited flag (tracks if payout has been credited to user wallet)
ALTER TABLE dividend_payouts
    ADD COLUMN IF NOT EXISTS wallet_credited BOOLEAN NOT NULL DEFAULT false;

-- 2. Add credited_at timestamp (when the wallet was credited)
ALTER TABLE dividend_payouts
    ADD COLUMN IF NOT EXISTS credited_at TIMESTAMPTZ;

-- 3. Relax investment_id to be nullable — Phase 9 distributions create payouts
--    per user/asset, not per investment. Legacy payouts still reference investment_id.
ALTER TABLE dividend_payouts
    ALTER COLUMN investment_id DROP NOT NULL;

-- 4. Add unique constraint for distribution_id + user_id to prevent duplicate payouts
--    (only if it doesn't already exist)
CREATE UNIQUE INDEX IF NOT EXISTS idx_div_payouts_dist_user
    ON dividend_payouts(distribution_id, user_id)
    WHERE distribution_id IS NOT NULL;
