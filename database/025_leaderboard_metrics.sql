-- ═══════════════════════════════════════════════════════════════════
-- Migration 025: Added explicit metric columns to leaderboard_scores
-- ═══════════════════════════════════════════════════════════════════

ALTER TABLE leaderboard_scores 
ADD COLUMN IF NOT EXISTS total_invested_cents BIGINT DEFAULT 0,
ADD COLUMN IF NOT EXISTS asset_count INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS portfolio_roi_bps INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS affiliate_count INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS referral_revenue_cents BIGINT DEFAULT 0,
ADD COLUMN IF NOT EXISTS highest_investment_cents BIGINT DEFAULT 0,

-- Also add ranking columns so we can sort them per metric statically and support pagination
ADD COLUMN IF NOT EXISTS rank_invested INTEGER,
ADD COLUMN IF NOT EXISTS rank_assets INTEGER,
ADD COLUMN IF NOT EXISTS rank_roi INTEGER,
ADD COLUMN IF NOT EXISTS rank_affiliates INTEGER,
ADD COLUMN IF NOT EXISTS rank_ref_revenue INTEGER,
ADD COLUMN IF NOT EXISTS rank_highest_inv INTEGER;

-- Create indexes to optimize the API calls when ordering by metric or fetching a certain rank
CREATE INDEX IF NOT EXISTS idx_lb_scores_rank_invested ON leaderboard_scores(rank_invested);
CREATE INDEX IF NOT EXISTS idx_lb_scores_rank_assets ON leaderboard_scores(rank_assets);
CREATE INDEX IF NOT EXISTS idx_lb_scores_rank_roi ON leaderboard_scores(rank_roi);
CREATE INDEX IF NOT EXISTS idx_lb_scores_rank_affiliates ON leaderboard_scores(rank_affiliates);
CREATE INDEX IF NOT EXISTS idx_lb_scores_rank_ref_revenue ON leaderboard_scores(rank_ref_revenue);
CREATE INDEX IF NOT EXISTS idx_lb_scores_rank_highest_inv ON leaderboard_scores(rank_highest_inv);
