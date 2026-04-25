-- 080: Admin asset management permissions
-- Grants asset-specific RBAC permissions used by admin asset detail APIs.

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.permission
FROM roles r
CROSS JOIN (
    VALUES
        ('assets.view'),
        ('assets.edit'),
        ('assets.publish')
) AS p(permission)
WHERE r.name IN ('admin', 'super_admin')
ON CONFLICT DO NOTHING;
