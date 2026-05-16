-- 168: Pre-computed weekly + monthly leaderboard tables
--
-- Closes the last "weekly/monthly recomputed live per request" performance
-- gap from the 2026-05-16 production-readiness audit (Bereich 1 + 7).
--
-- BEFORE: GET /api/leaderboard?timeframe=weekly ran a 6-CTE SELECT over
--         investments + referral_tracking with ROW_NUMBER() on every request.
--         O(n log n) per call → P99 latency climbs sharply past ~10k active
--         investors.
--
-- AFTER:  Two new tables shadow the shape of `leaderboard_scores` but hold
--         metrics filtered by the timeframe cutoff. A background worker
--         (see lib.rs leaderboard task) refreshes them on the same cadence
--         as the all-time table. The read path (get_rankings_timeframed)
--         becomes a flat indexed SELECT, identical to the all-time path.
--
-- Shape note: identical column set to leaderboard_scores so the upsert
-- code in service.rs can reuse `metric_columns()` and the same row-mapping
-- helpers without a parallel set of structs.

BEGIN;

-- ── Weekly ─────────────────────────────────────────────────────────
CREATE TABLE leaderboard_scores_weekly (
    user_id                  UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    total_invested_cents     BIGINT NOT NULL DEFAULT 0,
    asset_count              INTEGER NOT NULL DEFAULT 0,
    portfolio_roi_bps        INTEGER NOT NULL DEFAULT 0,
    affiliate_count          INTEGER NOT NULL DEFAULT 0,
    referral_network_value_cents   BIGINT NOT NULL DEFAULT 0,
    highest_investment_cents BIGINT NOT NULL DEFAULT 0,
    rank_invested            INTEGER,
    rank_assets              INTEGER,
    rank_roi                 INTEGER,
    rank_affiliates          INTEGER,
    rank_ref_revenue         INTEGER,
    rank_highest_inv         INTEGER,
    computed_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_lb_weekly_rank_invested   ON leaderboard_scores_weekly (rank_invested)   WHERE rank_invested   IS NOT NULL;
CREATE INDEX idx_lb_weekly_rank_assets     ON leaderboard_scores_weekly (rank_assets)     WHERE rank_assets     IS NOT NULL;
CREATE INDEX idx_lb_weekly_rank_roi        ON leaderboard_scores_weekly (rank_roi)        WHERE rank_roi        IS NOT NULL;
CREATE INDEX idx_lb_weekly_rank_affiliates ON leaderboard_scores_weekly (rank_affiliates) WHERE rank_affiliates IS NOT NULL;
CREATE INDEX idx_lb_weekly_rank_ref_rev    ON leaderboard_scores_weekly (rank_ref_revenue) WHERE rank_ref_revenue IS NOT NULL;
CREATE INDEX idx_lb_weekly_rank_highest    ON leaderboard_scores_weekly (rank_highest_inv) WHERE rank_highest_inv IS NOT NULL;

-- ── Monthly ────────────────────────────────────────────────────────
CREATE TABLE leaderboard_scores_monthly (
    user_id                  UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    total_invested_cents     BIGINT NOT NULL DEFAULT 0,
    asset_count              INTEGER NOT NULL DEFAULT 0,
    portfolio_roi_bps        INTEGER NOT NULL DEFAULT 0,
    affiliate_count          INTEGER NOT NULL DEFAULT 0,
    referral_network_value_cents   BIGINT NOT NULL DEFAULT 0,
    highest_investment_cents BIGINT NOT NULL DEFAULT 0,
    rank_invested            INTEGER,
    rank_assets              INTEGER,
    rank_roi                 INTEGER,
    rank_affiliates          INTEGER,
    rank_ref_revenue         INTEGER,
    rank_highest_inv         INTEGER,
    computed_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_lb_monthly_rank_invested   ON leaderboard_scores_monthly (rank_invested)   WHERE rank_invested   IS NOT NULL;
CREATE INDEX idx_lb_monthly_rank_assets     ON leaderboard_scores_monthly (rank_assets)     WHERE rank_assets     IS NOT NULL;
CREATE INDEX idx_lb_monthly_rank_roi        ON leaderboard_scores_monthly (rank_roi)        WHERE rank_roi        IS NOT NULL;
CREATE INDEX idx_lb_monthly_rank_affiliates ON leaderboard_scores_monthly (rank_affiliates) WHERE rank_affiliates IS NOT NULL;
CREATE INDEX idx_lb_monthly_rank_ref_rev    ON leaderboard_scores_monthly (rank_ref_revenue) WHERE rank_ref_revenue IS NOT NULL;
CREATE INDEX idx_lb_monthly_rank_highest    ON leaderboard_scores_monthly (rank_highest_inv) WHERE rank_highest_inv IS NOT NULL;

COMMIT;
