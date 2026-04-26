-- Migration 085: Admin blockchain contract-control hardening
-- Tracks local control state for per-clone pause/unpause operations and
-- introduces an explicit high-risk permission separate from normal admin pages.

CREATE TABLE IF NOT EXISTS chain_contract_controls (
    contract_address VARCHAR(42) PRIMARY KEY,
    asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    is_paused BOOLEAN NOT NULL DEFAULT FALSE,
    last_action VARCHAR(20) NOT NULL CHECK (last_action IN ('pause', 'unpause', 'sync')),
    last_tx_hash VARCHAR(66),
    updated_by UUID REFERENCES users(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_chain_contract_controls_asset
    ON chain_contract_controls(asset_id);

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, 'blockchain.manage'
FROM roles r
WHERE r.name = 'super_admin'
ON CONFLICT DO NOTHING;
