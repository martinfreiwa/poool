-- ============================================================
-- POOOL Platform - Phase 16.4 Core Abort
-- Add 'aborted' to assets and 'refunded' to investments
-- ============================================================

-- Safely drop constraint and add 'aborted'
ALTER TABLE assets DROP CONSTRAINT IF EXISTS assets_funding_status_check;
ALTER TABLE assets ADD CONSTRAINT assets_funding_status_check CHECK (funding_status IN (
    'upcoming', 'funding_open', 'funding_in_progress',
    'funded', 'rented', 'payout_pending', 'exited', 'aborted'
));

-- Safely drop constraint and add 'refunded'
ALTER TABLE investments DROP CONSTRAINT IF EXISTS investments_status_check;
ALTER TABLE investments ADD CONSTRAINT investments_status_check CHECK (status IN (
    'active', 'funded', 'rented', 'payout_pending',
    'in_process', 'funding_in_progress', 'exited', 'failed', 'refunded'
));
