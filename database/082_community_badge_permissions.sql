-- 082: Community admin badge permissions
-- Grants read/write community moderation permissions used by admin community badge APIs.

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
