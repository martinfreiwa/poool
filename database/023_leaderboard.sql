-- ═══════════════════════════════════════════════════════════════════
-- Migration 023: Leaderboard System
-- ═══════════════════════════════════════════════════════════════════

-- Precomputed scores for fast leaderboard rendering
CREATE TABLE IF NOT EXISTS leaderboard_scores (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invest_score    INTEGER NOT NULL DEFAULT 0,      -- 0–1000
    referral_score  INTEGER NOT NULL DEFAULT 0,      -- 0–1000
    tier_score      INTEGER NOT NULL DEFAULT 0,      -- 0–1000
    diversity_score INTEGER NOT NULL DEFAULT 0,      -- 0–1000
    total_score     INTEGER NOT NULL DEFAULT 0,      -- weighted composite
    rank_alltime    INTEGER,
    rank_monthly    INTEGER,
    rank_weekly     INTEGER,
    computed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);
CREATE INDEX IF NOT EXISTS idx_lb_scores_total ON leaderboard_scores(total_score DESC);
CREATE INDEX IF NOT EXISTS idx_lb_scores_rank_alltime ON leaderboard_scores(rank_alltime);
CREATE INDEX IF NOT EXISTS idx_lb_scores_rank_monthly ON leaderboard_scores(rank_monthly);
CREATE INDEX IF NOT EXISTS idx_lb_scores_rank_weekly ON leaderboard_scores(rank_weekly);

-- Historical snapshots for trend tracking
CREATE TABLE IF NOT EXISTS leaderboard_snapshots (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    total_score     INTEGER NOT NULL,
    rank_position   INTEGER NOT NULL,
    snapshot_type   VARCHAR(10) NOT NULL CHECK (snapshot_type IN ('daily', 'weekly', 'monthly')),
    snapshot_date   DATE NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, snapshot_type, snapshot_date)
);
CREATE INDEX IF NOT EXISTS idx_lb_snap_user_date ON leaderboard_snapshots(user_id, snapshot_date DESC);

-- Per-user leaderboard visibility preferences
CREATE TABLE IF NOT EXISTS leaderboard_preferences (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    visible         BOOLEAN NOT NULL DEFAULT FALSE,
    show_avatar     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);
