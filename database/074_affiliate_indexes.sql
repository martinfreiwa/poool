-- Migration 074: Affiliate System — Performance Indexes & Payout Operations
-- Step 7 from docs/affiliate/AFFILIATE_ONBOARDING_GAPS.md
--
-- 1. Partial B-Tree index for the holdback worker (scans expiring holdbacks nightly)
-- 2. Composite index on affiliate_commissions for payout batching
-- 3. SELECT ... FOR UPDATE hint via index on affiliate_commissions status
-- 4. Index on referral_clicks for affiliate code lookups

-- Partial index: efficiently find all referrals whose holdback window has expired
-- Used exclusively by the nightly holdback worker
CREATE INDEX IF NOT EXISTS idx_affiliate_referrals_holdback_expiry
    ON affiliate_referrals (holdback_expires_at)
    WHERE status = 'under_holdback';

-- Composite index: payout batching queries filter by affiliate_id + status
CREATE INDEX IF NOT EXISTS idx_affiliate_commissions_affiliate_status
    ON affiliate_commissions (affiliate_id, status);

-- Index on source_order_id for checking if a commission's underlying trade is still active
CREATE INDEX IF NOT EXISTS idx_affiliate_commissions_source_order
    ON affiliate_commissions (source_order_id)
    WHERE status IN ('provisionally_tracked', 'on_hold');

-- Index on affiliate_referrals.referred_user_id for fast attribution lookups on signup
CREATE INDEX IF NOT EXISTS idx_affiliate_referrals_referred_user
    ON affiliate_referrals (referred_user_id);

-- Index on affiliate_referrals.affiliate_id + status for dashboard counts
CREATE INDEX IF NOT EXISTS idx_affiliate_referrals_affiliate_status
    ON affiliate_referrals (affiliate_id, status);

-- Index on referral_clicks.code for affiliate dashboard click counts
CREATE INDEX IF NOT EXISTS idx_referral_clicks_code
    ON referral_clicks (code);

-- Index on affiliates.referral_code for the landing page cookie flow
CREATE INDEX IF NOT EXISTS idx_affiliates_referral_code
    ON affiliates (referral_code)
    WHERE status = 'active';
