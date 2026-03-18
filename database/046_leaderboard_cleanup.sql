-- ═══════════════════════════════════════════════════════════════════
-- Migration 046: Leaderboard Schema Cleanup & Timeframe Support
-- ═══════════════════════════════════════════════════════════════════
-- This migration:
--   1. Drops legacy unused columns from the original composite scoring system
--   2. Drops obsolete rank columns (weekly/monthly were never populated)
--   3. Adds a composite index on investments to speed up timeframe-filtered queries
--   4. Drops orphaned indexes

-- 1. Drop dead columns from the pre-metric era
ALTER TABLE leaderboard_scores
    DROP COLUMN IF EXISTS invest_score,
    DROP COLUMN IF EXISTS referral_score,
    DROP COLUMN IF EXISTS tier_score,
    DROP COLUMN IF EXISTS diversity_score,
    DROP COLUMN IF EXISTS total_score,
    DROP COLUMN IF EXISTS rank_alltime,
    DROP COLUMN IF EXISTS rank_monthly,
    DROP COLUMN IF EXISTS rank_weekly;

-- 2. Drop orphaned legacy indexes
DROP INDEX IF EXISTS idx_lb_scores_total;
DROP INDEX IF EXISTS idx_lb_scores_rank_alltime;
DROP INDEX IF EXISTS idx_lb_scores_rank_monthly;
DROP INDEX IF EXISTS idx_lb_scores_rank_weekly;

-- 3. Composite index for timeframe-filtered investment lookups
--    Covers: WHERE status = 'active' AND purchased_at >= $date
CREATE INDEX IF NOT EXISTS idx_investments_user_status_purchased
    ON investments(user_id, status, purchased_at DESC);

-- 4. Composite index on referral_tracking for efficient referral aggregation
CREATE INDEX IF NOT EXISTS idx_referral_tracking_referrer_created
    ON referral_tracking(referrer_id, created_at DESC);
