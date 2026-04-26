-- Migration 088: Admin Reports permissions
-- Grants the explicit Reports page/export permission used by /admin/reports.

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, 'reports.generate'
FROM roles r
WHERE r.name IN ('super_admin', 'compliance', 'finance')
ON CONFLICT DO NOTHING;
