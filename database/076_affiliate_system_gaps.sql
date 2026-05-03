-- ============================================================
-- Migration 076: Affiliate System Gap Fixes
-- Date: 2026-03-31
-- Closes: GAP-08, GAP-10, GAP-11 from docs/affiliate/affiliate_system_fix_plan.md
-- ============================================================

-- ── GAP-08: Policy Versioning ─────────────────────────────────────────────────
-- Track which policy version the affiliate last accepted.
-- When CURRENT_POLICY_VERSION > accepted_policy_version, the dashboard
-- will return policy_reacceptance_required: true.
ALTER TABLE affiliates
    ADD COLUMN IF NOT EXISTS accepted_policy_version TEXT NOT NULL DEFAULT '1.0';

-- Backfill existing affiliates so they don't get forced re-acceptance immediately.
UPDATE affiliates SET accepted_policy_version = '1.0' WHERE accepted_policy_version IS NULL;


-- ── GAP-10: Tax Document Storage ──────────────────────────────────────────────
-- Stores the GCS object path for the uploaded W-9 or W-8BEN tax form.
-- Payout is blocked in api_admin_affiliate_batch_payout until this is non-NULL.
ALTER TABLE affiliates
    ADD COLUMN IF NOT EXISTS tax_document_gcs_path TEXT DEFAULT NULL;


-- ── GAP-11: Affiliate Custom Marketing Materials ───────────────────────────────
-- Tracks affiliate-uploaded custom creative assets pending admin brand review.
CREATE TABLE IF NOT EXISTS affiliate_materials (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    affiliate_id      UUID NOT NULL REFERENCES affiliates(user_id) ON DELETE CASCADE,
    asset_name        TEXT NOT NULL,
    gcs_path          TEXT NOT NULL,
    status            TEXT NOT NULL DEFAULT 'pending_review'
                          CHECK (status IN ('pending_review', 'approved', 'rejected')),
    review_note       TEXT DEFAULT NULL,
    reviewed_by       UUID DEFAULT NULL REFERENCES users(id),
    reviewed_at       TIMESTAMPTZ DEFAULT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for fast admin review queue lookups
CREATE INDEX IF NOT EXISTS idx_affiliate_materials_status
    ON affiliate_materials(status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_affiliate_materials_affiliate
    ON affiliate_materials(affiliate_id);


-- ── GAP-06: Referral State Machine Additions ──────────────────────────────────
-- Ensure the CHECK constraint on affiliate_referrals.status allows all states
-- the application now uses (kyc_approved, first_investment_done).
-- We drop and recreate the constraint to add the new states.
DO $$
BEGIN
    -- Drop existing status constraint if any
    ALTER TABLE affiliate_referrals
        DROP CONSTRAINT IF EXISTS affiliate_referrals_status_check;

    -- Re-add with all valid states
    ALTER TABLE affiliate_referrals
        ADD CONSTRAINT affiliate_referrals_status_check
        CHECK (status IN (
            'attributed',
            'registered',
            'kyc_approved',
            'first_investment_done',
            'under_holdback',
            'qualified',
            'disqualified',
            'expired',
            'paid'
        ));
EXCEPTION
    WHEN undefined_column THEN NULL;
END $$;


-- ── Audit log index (improves fraud ring detection query) ─────────────────────
CREATE INDEX IF NOT EXISTS idx_audit_logs_actor_ip
    ON audit_logs(actor_user_id, ip_address)
    WHERE ip_address IS NOT NULL;
