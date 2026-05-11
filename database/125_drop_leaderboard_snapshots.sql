-- ═══════════════════════════════════════════════════════════════════
-- Migration 125: Drop unused leaderboard_snapshots table
-- ═══════════════════════════════════════════════════════════════════
--
-- The `leaderboard_snapshots` table was created in migration 023 to hold
-- historical rank snapshots, but no production code ever wrote or read
-- from it (`grep -r leaderboard_snapshots --include='*.rs'` returns zero
-- matches). The trend-tracking feature it was meant to support was
-- never built; the current leaderboard renders only the live precomputed
-- `leaderboard_scores` rows. Drop the dead table.
--
-- CASCADE removes the dependent index `idx_lb_snap_user_date` along with
-- the table.

DROP TABLE IF EXISTS leaderboard_snapshots CASCADE;
