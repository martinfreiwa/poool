-- 083: Admin community management permissions
-- Grants community-specific RBAC permissions used by admin community pages/APIs.

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.permission
FROM roles r
CROSS JOIN (
    VALUES
        ('community.view'),
        ('community.manage')
) AS p(permission)
WHERE r.name IN ('admin', 'super_admin')
ON CONFLICT DO NOTHING;
