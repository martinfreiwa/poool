-- 087: Admin notification permissions
-- Adds granular permissions for viewing and sending admin notifications.

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, permission
FROM roles r
CROSS JOIN (
    VALUES
        ('notifications.view'),
        ('notifications.send')
) AS perms(permission)
WHERE r.name IN ('admin', 'super_admin')
ON CONFLICT DO NOTHING;
