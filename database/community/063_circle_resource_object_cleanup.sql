-- 063_circle_resource_object_cleanup.sql
-- Phase 8 follow-up: physical object cleanup metadata for private Circle
-- Resource files after lifecycle/retention soft deletion.

ALTER TABLE circle_resources
  ADD COLUMN IF NOT EXISTS storage_deleted_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS storage_delete_attempts INTEGER NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS storage_delete_last_error TEXT,
  ADD COLUMN IF NOT EXISTS storage_delete_next_attempt_at TIMESTAMPTZ;

ALTER TABLE circle_resource_versions
  ADD COLUMN IF NOT EXISTS storage_deleted_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS storage_delete_attempts INTEGER NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS storage_delete_last_error TEXT,
  ADD COLUMN IF NOT EXISTS storage_delete_next_attempt_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_circle_resources_object_cleanup_due
  ON circle_resources (deleted_at ASC, storage_delete_next_attempt_at ASC)
  WHERE storage_object_path IS NOT NULL
    AND deleted_at IS NOT NULL
    AND storage_deleted_at IS NULL
    AND legal_hold = FALSE;

CREATE INDEX IF NOT EXISTS idx_circle_resource_versions_object_cleanup_due
  ON circle_resource_versions (resource_id, storage_delete_next_attempt_at ASC)
  WHERE storage_object_path IS NOT NULL
    AND storage_deleted_at IS NULL;

COMMENT ON COLUMN circle_resources.storage_deleted_at IS
  'Timestamp when the private backing object was physically deleted or confirmed absent.';
COMMENT ON COLUMN circle_resources.storage_delete_attempts IS
  'Retry counter for physical storage cleanup attempts.';
COMMENT ON COLUMN circle_resources.storage_delete_last_error IS
  'Last bounded error message from physical storage cleanup.';
COMMENT ON COLUMN circle_resources.storage_delete_next_attempt_at IS
  'Backoff timestamp before the next physical storage cleanup attempt.';
COMMENT ON COLUMN circle_resource_versions.storage_deleted_at IS
  'Timestamp when this version backing object was physically deleted or confirmed absent.';
