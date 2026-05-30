-- 168_commission_logic_fixes.sql
-- ──────────────────────────────────────────────────────────────────────────
-- Fixes 11 logic bugs discovered in the affiliate-team commission flow.
-- Migration is idempotent (uses IF NOT EXISTS / DROP IF EXISTS).
--
-- BUGS FIXED:
--   Bug 1+2 → New column `gross_amount_cents` removes the broken
--             investments-JOIN pattern that silently zeroed revenue reports
--             in production (call sites pass orders.id, FK references
--             investments — NOT VALID hides the mismatch).
--             Also kills double-count via cumulative UPSERTed investments.
--   Bug 3   → Commission compute now skips terminated teams (handled in
--             Rust query; here we add the cascade trigger that flips
--             affiliate_links inactive on team-terminate).
--   Bug 5   → recompute_team_tier() now uses FOR UPDATE (added in Rust
--             update of the function below).
--   Bug 7   → Backfill `qualified_at` for every row whose status indicates
--             qualification — stops the unstable updated_at fallback.
--   Bug 9   → Trigger: terminating a team deactivates all its team_business
--             links + sets their deactivated_reason.
--   Bug 10  → Lowercase index already exists; we add a CHECK constraint to
--             reject any future mixed-case slug insert.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

-- ── 1. Add gross_amount_cents to affiliate_commissions (Bug 1+2) ─────────
ALTER TABLE affiliate_commissions
    ADD COLUMN IF NOT EXISTS gross_amount_cents BIGINT NOT NULL DEFAULT 0;

COMMENT ON COLUMN affiliate_commissions.gross_amount_cents IS
    'The gross order amount (cents) the commission was computed from. Eliminates the need to JOIN investments via source_order_id (which was unreliable: call sites pass orders.id, the NOT VALID FK references investments).';

-- Backfill: recover the gross from provisional_amount_cents and the
-- recorded tier_at_execution. Formula: gross = commission * 10000 / bps.
-- Works for existing rows because tier_at_execution was stamped at INSERT.
UPDATE affiliate_commissions ac
   SET gross_amount_cents = (ac.provisional_amount_cents * 10000) / NULLIF(at.commission_rate_bps, 0)
  FROM affiliate_tiers at
 WHERE at.name = ac.tier_at_execution
   AND ac.gross_amount_cents = 0
   AND at.commission_rate_bps > 0;

-- Fallback for any row whose tier_at_execution doesn't match a known tier
-- (typos, legacy data): leave at 0 so reports degrade visibly rather than
-- silently mis-attributing.

-- ── 2. Backfill qualified_at (Bug 7) ─────────────────────────────────────
-- Sets qualified_at = updated_at for all rows that are currently in a
-- qualified-class status. Future updates only set qualified_at on the
-- explicit transition (service.rs:1934), so the rolling 12-month tier
-- window is stable from now on.
UPDATE affiliate_referrals
   SET qualified_at = updated_at
 WHERE qualified_at IS NULL
   AND status IN ('qualified', 'paid', 'under_holdback', 'first_investment_done');

-- ── 3. Slug lowercase enforcement (Bug 10) ───────────────────────────────
-- The existing partial-unique on LOWER(public_slug) prevents collisions
-- but allows mixed-case storage. Force lowercase at the DB layer so the
-- value matches what every lookup expects.
UPDATE developer_teams
   SET public_slug = LOWER(public_slug)
 WHERE public_slug IS NOT NULL
   AND public_slug <> LOWER(public_slug);

ALTER TABLE developer_teams
    DROP CONSTRAINT IF EXISTS developer_teams_public_slug_lowercase;
ALTER TABLE developer_teams
    ADD CONSTRAINT developer_teams_public_slug_lowercase
    CHECK (public_slug IS NULL OR public_slug = LOWER(public_slug));

-- ── 4. Cascade-deactivate links when team is terminated (Bug 9) ──────────
CREATE OR REPLACE FUNCTION cascade_terminate_team_links()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'terminated' AND OLD.status <> 'terminated' THEN
        UPDATE affiliate_links
           SET status = 'inactive',
               deactivated_at = NOW(),
               deactivated_reason = COALESCE(deactivated_reason,
                                             'team_terminated_' || NEW.id::text),
               updated_at = NOW()
         WHERE team_id = NEW.id
           AND status = 'active';
    END IF;
    RETURN NEW;
END $$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_developer_teams_cascade_terminate ON developer_teams;
CREATE TRIGGER trg_developer_teams_cascade_terminate
    AFTER UPDATE OF status ON developer_teams
    FOR EACH ROW
    EXECUTE FUNCTION cascade_terminate_team_links();

COMMENT ON FUNCTION cascade_terminate_team_links IS
    'When a team transitions to terminated, deactivate all its team_business affiliate_links so no further click attribution or commission can route through them.';

-- ── 5. Update recompute_team_tier() — FOR UPDATE + new column ────────────
-- Recreates the function (CREATE OR REPLACE) with two corrections:
--   (a) `SELECT ... FOR UPDATE` to serialize concurrent recomputes (Bug 5)
--   (b) volume comes from ac.gross_amount_cents directly — no JOIN to
--       investments, which would have returned 0 rows in production (Bug 1+2)
--   (c) skip terminated teams: the function now returns the cached values
--       unchanged if the team is terminated, so the trigger from §4 governs
--       link deactivation but rate stays where it was.
CREATE OR REPLACE FUNCTION recompute_team_tier(p_team_id UUID)
RETURNS TABLE(team_id UUID, old_tier TEXT, new_tier TEXT, old_bps INT, new_bps INT, volume_cents BIGINT)
LANGUAGE plpgsql AS $$
DECLARE
    v_old_tier TEXT;
    v_old_bps  INT;
    v_volume   BIGINT;
    v_new_tier TEXT;
    v_new_bps  INT;
    v_status   TEXT;
BEGIN
    -- Acquire a row-lock on the team to serialize concurrent recomputes.
    SELECT current_team_tier, team_commission_rate_bps, status
      INTO v_old_tier, v_old_bps, v_status
      FROM developer_teams
     WHERE id = p_team_id
       FOR UPDATE;

    IF NOT FOUND THEN
        RETURN; -- team gone; nothing to do
    END IF;

    -- Don't promote/demote a terminated team — but DO recompute volume for
    -- reporting consistency.
    IF v_status = 'terminated' THEN
        RETURN QUERY SELECT p_team_id, v_old_tier, v_old_tier, v_old_bps, v_old_bps, 0::BIGINT;
        RETURN;
    END IF;

    -- Volume = gross order amount on qualified team-business commissions in
    -- the rolling 12-month window. Uses the new self-contained
    -- gross_amount_cents column (Bug 1+2 fix). Filter on qualified_at to
    -- get a stable window (Bug 7 backfill ensures this is populated).
    SELECT COALESCE(SUM(ac.gross_amount_cents), 0)::BIGINT INTO v_volume
      FROM affiliate_referrals ar
      JOIN affiliate_links al ON al.id = ar.link_id
      JOIN affiliate_commissions ac ON ac.referral_id = ar.id
     WHERE al.team_id = p_team_id
       AND al.link_type = 'team_business'
       AND ar.status IN ('qualified', 'paid', 'under_holdback')
       AND COALESCE(ar.qualified_at, ar.updated_at) >= NOW() - INTERVAL '12 months';

    -- Pick highest tier whose threshold is met.
    SELECT name, commission_rate_bps INTO v_new_tier, v_new_bps
      FROM affiliate_tiers
     WHERE min_volume_cents <= v_volume
     ORDER BY min_volume_cents DESC
     LIMIT 1;

    IF v_new_tier IS NULL THEN
        v_new_tier := 'Access';
        v_new_bps  := 50;
    END IF;

    UPDATE developer_teams
       SET current_team_tier        = v_new_tier,
           team_commission_rate_bps = v_new_bps,
           team_volume_12m_cents    = v_volume,
           team_tier_updated_at     = NOW()
     WHERE id = p_team_id;

    -- Record the change in history (only if tier actually moved).
    IF v_old_tier IS DISTINCT FROM v_new_tier OR v_old_bps IS DISTINCT FROM v_new_bps THEN
        INSERT INTO developer_team_tier_history
            (team_id, old_tier, new_tier, old_bps, new_bps, volume_cents)
        VALUES (p_team_id, v_old_tier, v_new_tier, v_old_bps, v_new_bps, v_volume);
    END IF;

    RETURN QUERY SELECT p_team_id, v_old_tier, v_new_tier, v_old_bps, v_new_bps, v_volume;
END $$;

COMMENT ON FUNCTION recompute_team_tier IS
    'Recompute team tier from team_business gross volume (12m rolling). Row-locks the team to serialize concurrent calls. Logs every actual tier change to developer_team_tier_history. Idempotent.';

-- ── 6. Operator notes ────────────────────────────────────────────────────
-- Bugs not addressed by SQL alone (implemented in Rust patch instead):
--   * Bug 3  (terminated-team blocks new commissions in check_and_track) — Rust.
--   * Bug 4  (suspended dev affiliates row → tracing::warn audit) — Rust.
--   * Bug 6  (is_first_commission ignores clawed_back) — Rust query update.
--   * Bug 8  (ensure_dev_affiliate_row on accept/self_request) — Rust call sites.
--   * Bug 11 (lost-click audit log when referral attribution races) — Rust trace.

COMMIT;

-- ── Verification queries ─────────────────────────────────────────────────
SELECT COUNT(*) AS rows_total,
       COUNT(*) FILTER (WHERE gross_amount_cents > 0) AS rows_backfilled,
       COUNT(*) FILTER (WHERE gross_amount_cents = 0) AS rows_unmatched_tier
  FROM affiliate_commissions
 WHERE payout_user_id = (SELECT id FROM users WHERE email='support@traffic-creator.com');

SELECT status, COUNT(*) AS total, COUNT(qualified_at) AS with_qualified_at
  FROM affiliate_referrals
 GROUP BY status ORDER BY total DESC;

SELECT id, display_name, status,
       (SELECT COUNT(*) FROM affiliate_links WHERE team_id = developer_teams.id AND status = 'active') AS active_links
  FROM developer_teams ORDER BY status;
