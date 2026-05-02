-- ============================================================
-- 102. developer_projects: admin workflow fields
--   - is_test            : flag obvious test/dummy submissions
--                          (filtered out of default admin queue)
--   - assigned_admin_id  : reviewer assignment, multi-device,
--                          replaces localStorage MVP in admin UI
-- ============================================================

ALTER TABLE developer_projects
    ADD COLUMN IF NOT EXISTS is_test BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE developer_projects
    ADD COLUMN IF NOT EXISTS assigned_admin_id UUID REFERENCES users(id);

ALTER TABLE developer_projects
    ADD COLUMN IF NOT EXISTS assigned_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_dev_projects_assigned_admin
    ON developer_projects(assigned_admin_id)
    WHERE assigned_admin_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_dev_projects_is_test
    ON developer_projects(is_test)
    WHERE is_test = TRUE;
