-- 081: Admin audit log permissions
-- Grants audit-trail read permission used by /admin/audit-logs.

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, 'audit.read'
FROM roles r
WHERE r.name IN ('admin', 'super_admin')
ON CONFLICT DO NOTHING;
