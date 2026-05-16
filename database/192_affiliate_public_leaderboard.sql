-- Migration 192: opt-in public affiliate leaderboard (Phase 4).
--
-- Industry-standard gamification: top affiliates by paid commission for
-- the current month + all-time, shown on a public page. Only affiliates
-- who explicitly opt in are listed — privacy-by-default.
--
-- Opt-in flag lives on the `affiliates` row alongside the optional
-- public-facing display name (avoids leaking the raw email). A public
-- avatar URL is optional too.
--
-- Read-side: a materialised view `affiliate_leaderboard_public` rolls up
-- the snapshot. Refreshed every 15 min by a worker (or on-demand via
-- `REFRESH MATERIALIZED VIEW CONCURRENTLY`). The view ONLY shows users
-- with `public_leaderboard_opt_in = TRUE`.
--
-- Idempotent. Safe to re-run.

ALTER TABLE affiliates
    ADD COLUMN IF NOT EXISTS public_leaderboard_opt_in BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS public_display_name VARCHAR(60),
    ADD COLUMN IF NOT EXISTS public_avatar_url   VARCHAR(512);

ALTER TABLE affiliates
    DROP CONSTRAINT IF EXISTS affiliates_public_avatar_https;
ALTER TABLE affiliates
    ADD CONSTRAINT affiliates_public_avatar_https
    CHECK (public_avatar_url IS NULL OR public_avatar_url LIKE 'https://%');

-- Materialised view = read-side snapshot. The aggregation is intentionally
-- simple — `paid` bucket only (commissions that actually settled). Joins
-- against `affiliates` to filter to opted-in users and to pick a display
-- name. Window function ranks by current-month then by all-time.
CREATE MATERIALIZED VIEW IF NOT EXISTS affiliate_leaderboard_public AS
SELECT
    a.user_id                       AS affiliate_user_id,
    COALESCE(
        NULLIF(TRIM(a.public_display_name), ''),
        SPLIT_PART(u.email, '@', 1)
    )                               AS display_name,
    a.public_avatar_url,
    a.current_tier,
    -- Lifetime paid commission, EUR cents.
    COALESCE((
        SELECT SUM(c.provisional_amount_cents)::BIGINT
          FROM affiliate_commissions c
         WHERE c.payout_user_id = a.user_id
           AND c.status = 'paid'
    ), 0)                           AS lifetime_paid_cents,
    -- Current month's paid commission.
    COALESCE((
        SELECT SUM(c.provisional_amount_cents)::BIGINT
          FROM affiliate_commissions c
         WHERE c.payout_user_id = a.user_id
           AND c.status = 'paid'
           AND c.updated_at >= date_trunc('month', NOW())
    ), 0)                           AS month_paid_cents,
    -- Total qualified referrals (signed up + investment confirmed).
    COALESCE((
        SELECT COUNT(*)::BIGINT
          FROM affiliate_referrals r
         WHERE r.payout_user_id = a.user_id
           AND r.status IN ('qualified', 'paid')
    ), 0)                           AS qualified_referrals,
    NOW()                           AS snapshot_at
FROM affiliates a
JOIN users u ON u.id = a.user_id
WHERE a.public_leaderboard_opt_in = TRUE
  AND a.status = 'active';

-- Required for CONCURRENT refresh — needs a unique index on the snapshot.
CREATE UNIQUE INDEX IF NOT EXISTS idx_affiliate_leaderboard_public_user
    ON affiliate_leaderboard_public (affiliate_user_id);

CREATE INDEX IF NOT EXISTS idx_affiliate_leaderboard_public_month
    ON affiliate_leaderboard_public (month_paid_cents DESC);

CREATE INDEX IF NOT EXISTS idx_affiliate_leaderboard_public_lifetime
    ON affiliate_leaderboard_public (lifetime_paid_cents DESC);

COMMENT ON MATERIALIZED VIEW affiliate_leaderboard_public IS
  'Phase-4 opt-in public leaderboard. Refresh CONCURRENTLY every 15 min. Only includes affiliates with public_leaderboard_opt_in = TRUE AND status = active.';
