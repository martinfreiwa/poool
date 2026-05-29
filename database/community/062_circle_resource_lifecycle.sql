-- 062_circle_resource_lifecycle.sql
-- Phase 8 follow-up: upload/retention/review lifecycle foundation for
-- Circle Resource documents. This intentionally stores lifecycle metadata
-- and manager actions, while binary upload transport remains a separate slice.

ALTER TABLE circle_resources
  ADD COLUMN IF NOT EXISTS upload_status VARCHAR(32) NOT NULL DEFAULT 'external',
  ADD COLUMN IF NOT EXISTS retention_policy VARCHAR(32) NOT NULL DEFAULT 'standard',
  ADD COLUMN IF NOT EXISTS retention_until TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS review_required_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS reviewed_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS reviewed_by UUID,
  ADD COLUMN IF NOT EXISTS legal_hold BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS deleted_by UUID,
  ADD COLUMN IF NOT EXISTS deletion_reason TEXT,
  ADD COLUMN IF NOT EXISTS document_lifecycle_notes TEXT;

ALTER TABLE circle_resource_versions
  ADD COLUMN IF NOT EXISTS upload_status VARCHAR(32) NOT NULL DEFAULT 'external',
  ADD COLUMN IF NOT EXISTS retention_policy VARCHAR(32) NOT NULL DEFAULT 'standard',
  ADD COLUMN IF NOT EXISTS retention_until TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS review_required_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS document_lifecycle_notes TEXT;

UPDATE circle_resources
   SET upload_status = CASE
         WHEN deleted_at IS NOT NULL THEN 'deleted'
         WHEN storage_object_path IS NOT NULL THEN 'uploaded'
         ELSE 'external'
       END
 WHERE upload_status = 'external'
   AND storage_object_path IS NOT NULL;

UPDATE circle_resource_versions
   SET upload_status = CASE
         WHEN storage_object_path IS NOT NULL THEN 'uploaded'
         ELSE 'external'
       END
 WHERE upload_status = 'external'
   AND storage_object_path IS NOT NULL;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'circle_resources_upload_status_allowed'
  ) THEN
    ALTER TABLE circle_resources
      ADD CONSTRAINT circle_resources_upload_status_allowed
      CHECK (upload_status IN ('external', 'pending_upload', 'uploaded', 'rejected', 'expired', 'deleted'));
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'circle_resources_retention_policy_allowed'
  ) THEN
    ALTER TABLE circle_resources
      ADD CONSTRAINT circle_resources_retention_policy_allowed
      CHECK (retention_policy IN ('standard', 'legal_hold', 'delete_after_expiry'));
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'circle_resource_versions_upload_status_allowed'
  ) THEN
    ALTER TABLE circle_resource_versions
      ADD CONSTRAINT circle_resource_versions_upload_status_allowed
      CHECK (upload_status IN ('external', 'pending_upload', 'uploaded', 'rejected', 'expired', 'deleted'));
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'circle_resource_versions_retention_policy_allowed'
  ) THEN
    ALTER TABLE circle_resource_versions
      ADD CONSTRAINT circle_resource_versions_retention_policy_allowed
      CHECK (retention_policy IN ('standard', 'legal_hold', 'delete_after_expiry'));
  END IF;
END
$$;

CREATE INDEX IF NOT EXISTS idx_circle_resources_lifecycle_review
  ON circle_resources (review_required_at ASC, updated_at DESC)
  WHERE review_required_at IS NOT NULL
    AND reviewed_at IS NULL
    AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_circle_resources_retention_due
  ON circle_resources (retention_until ASC)
  WHERE retention_until IS NOT NULL
    AND deleted_at IS NULL
    AND legal_hold = FALSE;

CREATE INDEX IF NOT EXISTS idx_circle_resources_upload_status
  ON circle_resources (upload_status, updated_at DESC);

COMMENT ON COLUMN circle_resources.upload_status IS
  'Manager-facing upload/delivery lifecycle state. Binary upload transport is intentionally separate from this metadata foundation.';
COMMENT ON COLUMN circle_resources.retention_policy IS
  'Document retention mode: standard, legal_hold, or delete_after_expiry.';
COMMENT ON COLUMN circle_resources.retention_until IS
  'Timestamp after which non-legal-hold documents can be reviewed for retention deletion.';
COMMENT ON COLUMN circle_resources.review_required_at IS
  'Timestamp by which a manager should review this resource for accuracy, expiry, or compliance.';
COMMENT ON COLUMN circle_resources.document_lifecycle_notes IS
  'Manager-facing lifecycle notes for upload, review, retention, and soft-delete decisions.';
