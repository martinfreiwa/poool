-- 056_circle_resource_delivery.sql
-- Phase 8 follow-up: versioned, permissioned delivery metadata for
-- Asset Circle resources. Raw storage object paths stay server-side;
-- clients use an authenticated delivery endpoint.

ALTER TABLE circle_resources
  ADD COLUMN IF NOT EXISTS file_name VARCHAR(240),
  ADD COLUMN IF NOT EXISTS mime_type VARCHAR(120),
  ADD COLUMN IF NOT EXISTS file_size_bytes BIGINT,
  ADD COLUMN IF NOT EXISTS sha256_hex CHAR(64),
  ADD COLUMN IF NOT EXISTS version_label VARCHAR(80) NOT NULL DEFAULT 'v1',
  ADD COLUMN IF NOT EXISTS published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS requires_download BOOLEAN NOT NULL DEFAULT TRUE,
  ADD CONSTRAINT circle_resources_file_size_nonnegative
    CHECK (file_size_bytes IS NULL OR file_size_bytes >= 0),
  ADD CONSTRAINT circle_resources_sha256_hex_format
    CHECK (sha256_hex IS NULL OR sha256_hex ~ '^[0-9a-f]{64}$');

CREATE INDEX IF NOT EXISTS idx_circle_resources_delivery_active
  ON circle_resources (circle_id, id, is_active, expires_at);

COMMENT ON COLUMN circle_resources.storage_object_path IS
  'Private storage path or gs:// URI. Never returned by list APIs; only resolved by the authenticated delivery endpoint.';
COMMENT ON COLUMN circle_resources.version_label IS
  'Human-readable resource version shown to Circle members, for example v1 or 2026-Q2.';
COMMENT ON COLUMN circle_resources.requires_download IS
  'When true, the delivery endpoint forces Content-Disposition attachment for private files.';
