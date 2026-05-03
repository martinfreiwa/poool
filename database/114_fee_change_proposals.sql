-- ─────────────────────────────────────────────────────────────────
-- 114_fee_change_proposals.sql
-- Marketplace fee changes that require dual approval and/or scheduling.
--
-- A "proposal" is a queued fee change. When status='approved' and
-- effective_at <= NOW(), a worker promotes it into fee_configurations
-- via the existing create-fee path.
--
-- Approver MUST be a different admin than the requester (4-eyes).
-- ─────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS fee_change_proposals (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scope           VARCHAR(15) NOT NULL CHECK (scope IN ('platform', 'asset', 'developer')),
    asset_id        UUID REFERENCES assets(id),
    developer_id    UUID REFERENCES users(id),
    taker_fee_bps   INTEGER NOT NULL CHECK (taker_fee_bps >= 0 AND taker_fee_bps <= 1000),
    maker_fee_bps   INTEGER NOT NULL CHECK (maker_fee_bps >= 0 AND maker_fee_bps <= 1000),
    reason          TEXT NOT NULL,

    requested_by    UUID NOT NULL REFERENCES users(id),
    requested_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- NULL = "apply now (after approval)". Future timestamp = scheduled.
    effective_at    TIMESTAMPTZ,

    -- Whether dual approval is required. Computed at proposal creation
    -- based on delta vs. current active config (>0.5pp = required).
    requires_approval BOOLEAN NOT NULL DEFAULT TRUE,

    status          VARCHAR(16) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'approved', 'rejected', 'applied', 'cancelled', 'expired')),

    approved_by     UUID REFERENCES users(id),
    approved_at     TIMESTAMPTZ,
    rejected_reason TEXT,

    applied_at      TIMESTAMPTZ,
    applied_fee_id  UUID REFERENCES fee_configurations(id),

    -- 4-eyes: a different admin must approve. NULL approved_by is allowed
    -- only for the auto-apply path (small-delta + immediate, no separate
    -- approver recorded — the audit log captures the requester+actor).
    CONSTRAINT chk_no_self_approval
        CHECK (approved_by IS NULL OR approved_by <> requested_by)
);

CREATE INDEX IF NOT EXISTS idx_fee_proposals_status_eff
    ON fee_change_proposals (status, effective_at);
CREATE INDEX IF NOT EXISTS idx_fee_proposals_requested_at
    ON fee_change_proposals (requested_at DESC);
