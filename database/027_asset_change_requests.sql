-- ============================================================
-- 027: Asset Change Requests — Developer Edit-Review Workflow
-- ============================================================
-- Stores proposed edits to approved/live assets.
-- Changes are NOT applied until an admin approves them.
-- ============================================================

CREATE TABLE IF NOT EXISTS asset_change_requests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    developer_id    UUID NOT NULL REFERENCES users(id),
    -- Snapshot of original values for changed fields only
    original_values JSONB NOT NULL,
    -- Proposed new values (same keys as original_values)
    proposed_values JSONB NOT NULL,
    -- State machine: pending → approved | rejected
    status          VARCHAR(20) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'approved', 'rejected')),
    admin_notes     TEXT,
    reviewed_by     UUID REFERENCES users(id),
    reviewed_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_acr_asset     ON asset_change_requests(asset_id);
CREATE INDEX IF NOT EXISTS idx_acr_status    ON asset_change_requests(status);
CREATE INDEX IF NOT EXISTS idx_acr_developer ON asset_change_requests(developer_id);
CREATE INDEX IF NOT EXISTS idx_acr_created   ON asset_change_requests(created_at DESC);

-- Only ONE pending request per asset at a time
CREATE UNIQUE INDEX IF NOT EXISTS idx_acr_one_pending
    ON asset_change_requests(asset_id) WHERE status = 'pending';
