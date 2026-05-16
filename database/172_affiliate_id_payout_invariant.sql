-- 172_affiliate_id_payout_invariant.sql
-- ──────────────────────────────────────────────────────────────────────────
-- F23 fix: enforce the invariant that affiliate_commissions.affiliate_id =
-- payout_user_id, so we don't have two sources of truth that could drift.
--
-- Background: when mig 157 added link_id / attribution_user_id /
-- payout_user_id to make team-business commissions properly tracked, the
-- legacy column `affiliate_id` was kept for back-compat with old read paths
-- (admin/rewards.rs uses `affiliate_id` in many JOINs). The INSERT in
-- service.rs:check_and_track_affiliate_commission sets
-- `affiliate_id = payout_user_id` explicitly, so today the two columns
-- always match. But there's nothing PREVENTING drift if a future "transfer
-- team ownership" admin tool updates payout_user_id without affiliate_id.
--
-- Strategy:
--   1. Verify no existing drift (this migration ASSERTs).
--   2. Add a CHECK constraint that the two must always match on every
--      INSERT/UPDATE so any future code change that breaks the invariant
--      fails loudly instead of silently corrupting payouts.
-- ──────────────────────────────────────────────────────────────────────────

BEGIN;

-- 1. Drift check. Refuse to apply this migration if any row currently
-- violates the invariant — would require manual investigation first.
DO $$
DECLARE
    drift_count INT;
BEGIN
    SELECT COUNT(*) INTO drift_count
      FROM affiliate_commissions
     WHERE affiliate_id <> payout_user_id;
    IF drift_count > 0 THEN
        RAISE EXCEPTION
            'Cannot enforce affiliate_id = payout_user_id: % rows already drift. Investigate before re-applying.',
            drift_count;
    END IF;
END $$;

-- 2. Add the invariant. NOT VALID so we don't re-scan all rows; we already
-- proved the table is clean above. New writes get checked.
ALTER TABLE affiliate_commissions
    DROP CONSTRAINT IF EXISTS affiliate_commissions_id_payout_match;
ALTER TABLE affiliate_commissions
    ADD CONSTRAINT affiliate_commissions_id_payout_match
    CHECK (affiliate_id = payout_user_id) NOT VALID;

-- 3. Validate now (cheap O(rows) one-time scan) so the constraint is fully
-- enforced from this point.
ALTER TABLE affiliate_commissions
    VALIDATE CONSTRAINT affiliate_commissions_id_payout_match;

COMMENT ON CONSTRAINT affiliate_commissions_id_payout_match
    ON affiliate_commissions IS
    'Enforces the back-compat invariant set in service.rs:check_and_track_affiliate_commission. Any future code path that updates payout_user_id MUST also update affiliate_id (or vice-versa). Catches drift introduced by team-transfer / admin tooling at write time instead of weeks later in a payout reconciliation.';

COMMIT;
