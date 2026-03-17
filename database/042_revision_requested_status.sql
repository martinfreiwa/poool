-- ============================================================
-- Migration 042: Add revision_requested status + revision_notes
-- ============================================================

-- 1. Add revision_notes column to developer_projects
ALTER TABLE developer_projects ADD COLUMN IF NOT EXISTS revision_notes TEXT;

-- 2. Drop old check constraint and add new one with 'revision_requested'
ALTER TABLE developer_projects DROP CONSTRAINT IF EXISTS developer_projects_status_check;
ALTER TABLE developer_projects ADD CONSTRAINT developer_projects_status_check
    CHECK (status IN ('draft', 'submitted', 'in_review', 'approved', 'rejected', 'live', 'revision_requested'));
