-- ═══════════════════════════════════════════════════════════════════
-- Migration 176: Rename `referral_revenue_cents` → `referral_network_value_cents`
-- ═══════════════════════════════════════════════════════════════════
--
-- The column has always stored `SUM(referee.purchase_value_cents)` — the
-- network volume the user's referrals contributed, NOT commission revenue
-- the user earned. The misnomer confused every dev reading the schema
-- cold. The frontend was already relabeled to "Network Volume" in May
-- 2026; this migration brings the database column name in line.
--
-- Code changes paired with this migration: rename the SQL refs in
-- backend/src/leaderboard/service.rs and the struct field in models.rs.
-- The JSON response field name changes accordingly — only the leaderboard
-- frontend consumes it (developer and investor pages), updated in the
-- same commit.

-- Idempotent: only renames when the old name still exists. Lets the
-- migration re-run on environments where someone already manually
-- applied the rename, without erroring.
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'leaderboard_scores'
      AND column_name = 'referral_revenue_cents'
  ) THEN
    ALTER TABLE leaderboard_scores
      RENAME COLUMN referral_revenue_cents TO referral_network_value_cents;
  END IF;
END $$;

-- The rank column for this metric keeps its name — it was already named
-- `rank_ref_revenue` (abbreviation), which still fits the new semantic.
-- Renaming it too would force a second round of code updates with no
-- semantic gain. Documented for future grep-ability.
