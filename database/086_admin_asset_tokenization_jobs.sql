-- 086: Admin asset tokenization hardening
-- Adds an idempotency guard for irreversible asset tokenization requests and
-- grants the dedicated high-risk permission only to super_admin.

CREATE TABLE IF NOT EXISTS asset_tokenization_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    requested_by UUID NOT NULL REFERENCES users(id),
    status VARCHAR(20) NOT NULL
        CHECK (status IN ('in_progress', 'succeeded', 'failed')),
    chain_network VARCHAR(40),
    factory_address VARCHAR(42),
    clone_address VARCHAR(42),
    chain_tx_hash VARCHAR(66),
    metadata_uri VARCHAR(512),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_asset_tokenization_jobs_active
    ON asset_tokenization_jobs(asset_id)
    WHERE status = 'in_progress';

CREATE INDEX IF NOT EXISTS idx_asset_tokenization_jobs_asset_created
    ON asset_tokenization_jobs(asset_id, created_at DESC);

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, 'blockchain.tokenize'
FROM roles r
WHERE r.name = 'super_admin'
ON CONFLICT DO NOTHING;
