-- 175_qualified_at_distribute.sql
-- ──────────────────────────────────────────────────────────────────────────
-- F22 fix: re-spread synthetic `qualified_at` timestamps that were clustered
-- at the migration-168 deploy minute.
--
-- mig 168 backfilled `qualified_at = updated_at` for every row that was
-- already in a qualified status but missing qualified_at. For rows whose
-- updated_at hadn't been touched since INSERT, the backfill correctly used
-- the row's natural creation time. But rows whose updated_at had been bumped
-- by a different column-change near the deploy clustered at the migration
-- minute.
--
-- For our schema this is a P3 cosmetic concern (rollups bucket by DAY, not
-- by minute), but it's worth fixing so any future hourly rollup wouldn't
-- show a synthetic spike at deploy time.
--
-- Strategy: for rows where qualified_at = updated_at (synthetic), reset
-- qualified_at to `created_at` if created_at is older. This gives a more
-- defensible "when did this referral first become qualified" semantics
-- and naturally spreads the timestamps across the period when referrals
-- actually happened.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

UPDATE affiliate_referrals
   SET qualified_at = created_at
 WHERE qualified_at IS NOT NULL
   AND qualified_at = updated_at
   AND created_at < qualified_at
   AND status IN ('qualified', 'paid', 'under_holdback', 'first_investment_done');

COMMIT;

\echo '── Synthetic-cluster check after re-spread ──'
SELECT to_char(qualified_at, 'YYYY-MM-DD HH24:MI') AS minute,
       COUNT(*) AS n
  FROM affiliate_referrals
 WHERE status IN ('qualified', 'paid', 'under_holdback')
 GROUP BY minute
 HAVING COUNT(*) > 5
 ORDER BY n DESC LIMIT 5;
