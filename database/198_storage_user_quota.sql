-- ═══════════════════════════════════════════════════════════════════
-- Migration 179: Per-User Storage Quota
-- ═══════════════════════════════════════════════════════════════════
--
-- Materialised running counters per user so we can reject upload requests
-- BEFORE writing to GCS once a user has exceeded their tier's quota.
-- Without this, a single buggy / hostile client can fill the bucket
-- (cost-DoS) or pump GiB through the multipart parser before we notice.
--
-- Approach:
--   - `storage_user_quotas` keeps a `bytes_used` BIGINT counter per
--     (user_id, class) where class ∈ {avatar, post_image, asset_image,
--     asset_document, kyc_document, developer_logo}.
--   - The runtime quota for each class is held in code (`QUOTA_BYTES`
--     map in service.rs) rather than the DB. Reason: we want to bump
--     quotas via a code deploy + canary, not a database edit.
--   - Counter updates happen INSIDE the upload transaction so a partial
--     upload doesn't desync the counter from reality. Periodic
--     reconciliation (Phase 3) catches drift if any.
--
-- Default-row policy: rows are created lazily on first upload via UPSERT
-- with `ON CONFLICT DO UPDATE bytes_used = bytes_used + EXCLUDED.delta`.
-- Reading a non-existent row returns 0 (treated as "quota available").

CREATE TABLE IF NOT EXISTS storage_user_quotas (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    class           VARCHAR(40) NOT NULL
                    CHECK (class IN (
                        'avatar', 'post_image', 'asset_image',
                        'asset_document', 'kyc_document', 'developer_logo'
                    )),
    bytes_used      BIGINT NOT NULL DEFAULT 0 CHECK (bytes_used >= 0),
    file_count      INTEGER NOT NULL DEFAULT 0 CHECK (file_count >= 0),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, class)
);

-- Hot path: "what is my quota usage for class X" — single-row PK lookup.
CREATE INDEX IF NOT EXISTS idx_storage_quotas_user_class
    ON storage_user_quotas(user_id, class);

-- Reconciliation path: find users near their cap so support can pre-warn.
CREATE INDEX IF NOT EXISTS idx_storage_quotas_high_usage
    ON storage_user_quotas(bytes_used DESC);

COMMENT ON TABLE storage_user_quotas IS
    'Per-user-per-class storage usage counters. Drives pre-upload quota
     enforcement and admin "user nearing cap" alerts. Reconciled weekly
     against GCS-listing (Phase 3 reconciliation job).';
