-- 066_circle_resource_version_review.sql
-- Phase 8 follow-up: review and comparison metadata for Circle Resource
-- versions. This makes version approval auditable without exposing storage
-- paths or merging review with the restore-current workflow.

ALTER TABLE circle_resource_versions
  ADD COLUMN IF NOT EXISTS review_status VARCHAR(32) NOT NULL DEFAULT 'pending',
  ADD COLUMN IF NOT EXISTS reviewed_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS reviewed_by UUID,
  ADD COLUMN IF NOT EXISTS review_note TEXT;

UPDATE circle_resource_versions
   SET review_status = 'approved',
       reviewed_at = COALESCE(reviewed_at, created_at),
       review_note = COALESCE(review_note, 'Backfilled approved current version')
 WHERE is_current = TRUE
   AND review_status = 'pending';

UPDATE circle_resource_versions
   SET review_status = 'superseded'
 WHERE is_current = FALSE
   AND review_status = 'pending'
   AND EXISTS (
         SELECT 1
           FROM circle_resource_versions current_version
          WHERE current_version.resource_id = circle_resource_versions.resource_id
            AND current_version.is_current = TRUE
       );

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'circle_resource_versions_review_status_allowed'
  ) THEN
    ALTER TABLE circle_resource_versions
      ADD CONSTRAINT circle_resource_versions_review_status_allowed
      CHECK (review_status IN ('pending', 'approved', 'rejected', 'superseded'));
  END IF;
END
$$;

CREATE INDEX IF NOT EXISTS idx_circle_resource_versions_review_queue
  ON circle_resource_versions (circle_id, review_status, created_at DESC)
  WHERE review_status = 'pending';

CREATE INDEX IF NOT EXISTS idx_circle_resource_versions_reviewed_at
  ON circle_resource_versions (circle_id, reviewed_at DESC)
  WHERE reviewed_at IS NOT NULL;

COMMENT ON COLUMN circle_resource_versions.review_status IS
  'Manager review state for a concrete resource version: pending, approved, rejected, or superseded.';
COMMENT ON COLUMN circle_resource_versions.review_note IS
  'Manager-facing reason or context captured when approving or rejecting a version.';
