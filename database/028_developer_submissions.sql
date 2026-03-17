-- ============================================================
-- Migration 028: Developer Submissions Enhancements
-- Adds submission_step tracking, investor_share_bps, amenities
-- ============================================================

-- 1. Track which step of the submission flow an asset is on
ALTER TABLE assets ADD COLUMN IF NOT EXISTS submission_step INTEGER DEFAULT 1;
-- Values: 1=AssetType, 2=PropertyInfo, 3=Documents, 4=Content, 5=Submitted

-- 2. Investor share of profits (basis points, e.g. 7000 = 70%)
ALTER TABLE assets ADD COLUMN IF NOT EXISTS investor_share_bps INTEGER;

-- 3. Amenities as JSON array (e.g. ["pool","gym","parking","security"])
ALTER TABLE assets ADD COLUMN IF NOT EXISTS amenities JSONB DEFAULT '[]'::jsonb;

-- 4. Soft-delete support
ALTER TABLE assets ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

-- 5. Index for developer submissions page queries
CREATE INDEX IF NOT EXISTS idx_assets_submission_step
    ON assets(submission_step)
    WHERE developer_user_id IS NOT NULL AND deleted_at IS NULL;

-- 6. Index for developer listing with status filter
CREATE INDEX IF NOT EXISTS idx_dev_projects_status
    ON developer_projects(status);
