-- 174_team_tier_backfill_reassert.sql
-- ──────────────────────────────────────────────────────────────────────────
-- F15 fix: re-run the team-tier backfill with the canonical
-- (mig-168 / gross_amount_cents) body of `recompute_team_tier()`.
--
-- Why: mig 166 contained an early version of `recompute_team_tier` that
-- joined `investments` via `source_order_id` (broken — in production that
-- column holds `orders.id`, see Bug 1+2). It also contained a DO-block at
-- the bottom that iterated every team and called recompute_team_tier(id).
-- Because the broken function body ran first, every team got volume=0
-- stamped into developer_teams.team_volume_12m_cents.
--
-- mig 168 corrected the function body BUT did not re-execute the backfill,
-- so the cached volume values lingered. This migration replays the
-- backfill against the correct body so the cache reflects real data.
--
-- Idempotent: recompute_team_tier handles being called repeatedly and only
-- writes to developer_team_tier_history when the tier actually changes.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

DO $$
DECLARE
    r RECORD;
    cnt INT := 0;
BEGIN
    FOR r IN
        SELECT id FROM developer_teams WHERE status <> 'terminated'
    LOOP
        PERFORM recompute_team_tier(r.id);
        cnt := cnt + 1;
    END LOOP;
    RAISE NOTICE 'F15: replayed recompute_team_tier for % active teams', cnt;
END $$;

COMMIT;

SELECT current_team_tier, COUNT(*) AS n,
       SUM(team_volume_12m_cents)/100.0 AS sum_eur_volume
  FROM developer_teams
 WHERE status <> 'terminated'
 GROUP BY current_team_tier
 ORDER BY sum_eur_volume DESC NULLS LAST;
