-- 079: Affiliate admin management permission
-- Allows affiliate application review endpoints to use a dedicated RBAC permission
-- instead of relying only on broad admin role extraction.

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, 'affiliates.manage'
FROM roles r
WHERE r.name IN ('admin', 'super_admin')
ON CONFLICT DO NOTHING;
