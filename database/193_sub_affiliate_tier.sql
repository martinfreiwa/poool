-- Migration 193: 2-tier sub-affiliate program (Phase 4).
--
-- Industry-standard (Rewardful, PartnerStack, Impact): an affiliate can
-- refer OTHER affiliates and earn a smaller cut (`tier-2 rate`) on
-- those sub-affiliates' qualifying commissions.
--
-- Modelling:
--   * `affiliates.referred_by_affiliate_id` — the parent affiliate.
--     Self-referencing FK. NULL = top-level (no parent). Set ONCE at
--     affiliate creation; never reassigned (a trigger or app-level
--     guard could enforce — for MVP we trust the app).
--   * `affiliate_tier2_rate_bps` on `affiliate_programs` — the global
--     tier-2 rate. 250 bps (= 2.5%) default; per-program-specific so
--     a campaign can set tier-2 to 0 to disable nesting.
--   * `affiliate_commissions.parent_commission_id` — when a commission
--     row is a "tier-2 spawn" of another commission, this FK points
--     to the original. NULL = direct commission. Idempotent: at most
--     one tier-2 row per original commission (partial unique).
--
-- The commission engine spawns the tier-2 row INSIDE the same tx as the
-- original commission, so either both commit or both roll back.
--
-- Idempotent. Safe to re-run.

ALTER TABLE affiliates
    ADD COLUMN IF NOT EXISTS referred_by_affiliate_id UUID
        REFERENCES affiliates(user_id) ON DELETE SET NULL;

-- Block self-parenting (affiliate referring themselves).
ALTER TABLE affiliates
    DROP CONSTRAINT IF EXISTS affiliates_no_self_referred_by;
ALTER TABLE affiliates
    ADD CONSTRAINT affiliates_no_self_referred_by
    CHECK (referred_by_affiliate_id IS NULL
           OR referred_by_affiliate_id <> user_id);

CREATE INDEX IF NOT EXISTS idx_affiliates_referred_by
    ON affiliates (referred_by_affiliate_id)
    WHERE referred_by_affiliate_id IS NOT NULL;

ALTER TABLE affiliate_programs
    ADD COLUMN IF NOT EXISTS affiliate_tier2_rate_bps INTEGER NOT NULL DEFAULT 250
        CHECK (affiliate_tier2_rate_bps BETWEEN 0 AND 10000);

ALTER TABLE affiliate_commissions
    ADD COLUMN IF NOT EXISTS parent_commission_id UUID
        REFERENCES affiliate_commissions(id) ON DELETE SET NULL;

-- One tier-2 spawn per parent. Two tier-2 rows for the same parent
-- would double-pay.
CREATE UNIQUE INDEX IF NOT EXISTS idx_affiliate_commissions_one_tier2_per_parent
    ON affiliate_commissions (parent_commission_id)
    WHERE parent_commission_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_affiliate_commissions_parent
    ON affiliate_commissions (parent_commission_id)
    WHERE parent_commission_id IS NOT NULL;

COMMENT ON COLUMN affiliates.referred_by_affiliate_id IS
  'Phase-4 2-tier: the affiliate who referred THIS affiliate. NULL = top-level. Earns affiliate_tier2_rate_bps on every commission this affiliate earns.';
COMMENT ON COLUMN affiliate_commissions.parent_commission_id IS
  'Phase-4 2-tier: when set, this row is the tier-2 spawn paid to the parent affiliate.';
