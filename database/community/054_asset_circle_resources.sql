-- 054_asset_circle_resources.sql
-- Phase 8: Asset Circle metadata, primary asset-circle mapping,
-- holder-only resource access, and private investor club defaults.

ALTER TABLE circles
  ADD COLUMN IF NOT EXISTS related_asset_id UUID NULL,
  ADD COLUMN IF NOT EXISTS is_primary_asset_circle BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS holder_only_documents BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN IF NOT EXISTS asset_circle_tabs TEXT[] NOT NULL DEFAULT ARRAY[
    'feed',
    'official_updates',
    'q_and_a',
    'documents',
    'yield_reports',
    'members',
    'risk_discussion',
    'about_asset'
  ]::TEXT[];

UPDATE circles
SET related_asset_id = token_gate_asset_id
WHERE related_asset_id IS NULL
  AND token_gate_asset_id IS NOT NULL;

UPDATE circles
SET visibility = 'private',
    join_policy = 'request',
    allow_cross_post = FALSE
WHERE private_investor_club = TRUE
   OR circle_type = 'private_investor';

CREATE UNIQUE INDEX IF NOT EXISTS idx_circles_primary_asset_circle
  ON circles (related_asset_id)
  WHERE is_primary_asset_circle = TRUE
    AND related_asset_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_circles_related_asset
  ON circles (related_asset_id, is_primary_asset_circle, created_at DESC)
  WHERE related_asset_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS circle_resources (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  circle_id UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
  asset_id UUID NULL,
  title VARCHAR(240) NOT NULL,
  description TEXT,
  resource_type VARCHAR(40) NOT NULL DEFAULT 'resource'
    CHECK (resource_type IN (
      'official_document',
      'report',
      'yield_report',
      'guide',
      'link',
      'photo_update',
      'community_resource'
    )),
  access_scope VARCHAR(32) NOT NULL DEFAULT 'member'
    CHECK (access_scope IN ('public', 'member', 'holder_only', 'admin_only')),
  url TEXT,
  storage_object_path TEXT,
  is_official BOOLEAN NOT NULL DEFAULT FALSE,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_by UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (
    (url IS NOT NULL AND length(trim(url)) > 0)
    OR (storage_object_path IS NOT NULL AND length(trim(storage_object_path)) > 0)
  )
);

CREATE INDEX IF NOT EXISTS idx_circle_resources_circle_active
  ON circle_resources (circle_id, is_active, access_scope, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_circle_resources_asset_active
  ON circle_resources (asset_id, is_active, access_scope, created_at DESC)
  WHERE asset_id IS NOT NULL;

COMMENT ON COLUMN circles.related_asset_id IS
  'Phase 8 logical core assets.id link for Asset Circles and Private Investor Clubs.';
COMMENT ON COLUMN circles.is_primary_asset_circle IS
  'At most one primary official Asset Circle can exist per related_asset_id.';
COMMENT ON COLUMN circles.holder_only_documents IS
  'When true, holder-only resource rows require current asset ownership before URLs are returned.';
COMMENT ON TABLE circle_resources IS
  'Permissioned Circle resource/document index. URLs are returned only after Circle and holder access checks.';
