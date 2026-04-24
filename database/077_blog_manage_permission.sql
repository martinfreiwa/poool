-- Migration 077: Blog admin RBAC permission
-- Grants the v1 Sanity-backed blog dashboard permission to super admins.

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, 'blog.manage'
FROM roles r
WHERE r.name = 'super_admin'
ON CONFLICT DO NOTHING;
