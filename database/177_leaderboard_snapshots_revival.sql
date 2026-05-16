-- ═══════════════════════════════════════════════════════════════════
-- Migration 177: Re-introduce leaderboard_snapshots for trend tracking
-- ═══════════════════════════════════════════════════════════════════
--
-- The original table (migration 023) was dropped in migration 125 because
-- nothing ever wrote to it. We bring it back in a slimmer shape to enable
-- daily trend snapshots, written by a tokio task in lib.rs alongside the
-- background refresh job.
--
-- Schema rationale:
--   - One row per (user, metric, date). Lets the UI render "your rank vs
--     7 days ago" without an expensive recomputation.
--   - `metric` is a small enum-shaped text column (allowlisted in the
--     writer) so we can later visualise any of the 6 ranking metrics, not
--     just `invested`.
--   - `metric_value` is a BIGINT — matches `leaderboard_scores`'s widest
--     metric column (cents). Bps and counts fit comfortably.
--   - Only snapshots VISIBLE users — privacy: an opted-out user's
--     historical rank shouldn't be reconstructible from the snapshots
--     table either. Enforced in the writer SQL, not the schema.

CREATE TABLE IF NOT EXISTS leaderboard_snapshots (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    metric          VARCHAR(20) NOT NULL
                    CHECK (metric IN (
                        'invested', 'assets', 'roi', 'affiliates',
                        'revenue', 'highest_inv'
                    )),
    rank            INTEGER NOT NULL,
    metric_value    BIGINT NOT NULL DEFAULT 0,
    snapshot_date   DATE NOT NULL DEFAULT (NOW() AT TIME ZONE 'UTC')::DATE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, metric, snapshot_date)
);

-- Hot path: "what was this user's rank on date X for metric M".
CREATE INDEX IF NOT EXISTS idx_lb_snap_user_metric_date
    ON leaderboard_snapshots(user_id, metric, snapshot_date DESC);

-- Cold path: "show me the leaderboard as of date X for metric M".
CREATE INDEX IF NOT EXISTS idx_lb_snap_metric_date_rank
    ON leaderboard_snapshots(metric, snapshot_date, rank);

-- Retention contract: 13 months of snapshots. After that they get pruned
-- by the housekeeping task (NOT IMPLEMENTED yet — when added, see
-- backend/src/lib.rs housekeeping spawn block).
COMMENT ON TABLE leaderboard_snapshots IS
    'Daily rank snapshots per user per metric. 13-month retention. Privacy: only opted-in users snapshotted.';
