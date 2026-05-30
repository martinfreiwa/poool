-- 059_circle_resource_versions.sql
-- Phase 8 follow-up: explicit Circle resource version history and
-- document lifecycle metadata. Member-facing resource APIs still use
-- authenticated delivery endpoints and never expose storage paths.

CREATE TABLE IF NOT EXISTS circle_resource_versions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  resource_id UUID NOT NULL REFERENCES circle_resources(id) ON DELETE CASCADE,
  circle_id UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
  version_label VARCHAR(80) NOT NULL DEFAULT 'v1',
  url TEXT,
  storage_object_path TEXT,
  file_name VARCHAR(240),
  mime_type VARCHAR(120),
  file_size_bytes BIGINT,
  sha256_hex CHAR(64),
  requires_download BOOLEAN NOT NULL DEFAULT TRUE,
  published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ,
  change_note TEXT,
  is_current BOOLEAN NOT NULL DEFAULT FALSE,
  created_by UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (
    (url IS NOT NULL AND length(trim(url)) > 0 AND storage_object_path IS NULL)
    OR (storage_object_path IS NOT NULL AND length(trim(storage_object_path)) > 0 AND url IS NULL)
  ),
  CONSTRAINT circle_resource_versions_file_size_nonnegative
    CHECK (file_size_bytes IS NULL OR file_size_bytes >= 0),
  CONSTRAINT circle_resource_versions_sha256_hex_format
    CHECK (sha256_hex IS NULL OR sha256_hex ~ '^[0-9a-f]{64}$')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_circle_resource_versions_current
  ON circle_resource_versions (resource_id)
  WHERE is_current = TRUE;

CREATE INDEX IF NOT EXISTS idx_circle_resource_versions_resource_created
  ON circle_resource_versions (resource_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_circle_resource_versions_circle_created
  ON circle_resource_versions (circle_id, created_at DESC);

INSERT INTO circle_resource_versions (
  resource_id,
  circle_id,
  version_label,
  url,
  storage_object_path,
  file_name,
  mime_type,
  file_size_bytes,
  sha256_hex,
  requires_download,
  published_at,
  expires_at,
  change_note,
  is_current,
  created_by,
  created_at
)
SELECT
  r.id,
  r.circle_id,
  r.version_label,
  CASE WHEN r.storage_object_path IS NULL THEN r.url ELSE NULL END,
  r.storage_object_path,
  r.file_name,
  r.mime_type,
  r.file_size_bytes,
  r.sha256_hex,
  r.requires_download,
  r.published_at,
  r.expires_at,
  'Backfilled current version from circle_resources',
  TRUE,
  r.created_by,
  r.created_at
FROM circle_resources r
WHERE NOT EXISTS (
  SELECT 1
  FROM circle_resource_versions v
  WHERE v.resource_id = r.id
);

COMMENT ON TABLE circle_resource_versions IS
  'Audit-friendly version history for Circle resources. Storage object paths are for backend delivery only and must not be exposed to member-facing APIs.';
COMMENT ON COLUMN circle_resource_versions.is_current IS
  'Exactly one current version per resource is enforced by idx_circle_resource_versions_current.';
COMMENT ON COLUMN circle_resource_versions.change_note IS
  'Manager-facing note explaining why this version was added or replaced.';
