-- Migration 078: Fine-grained Blog CMS permissions
-- Keeps blog.manage as the backwards-compatible umbrella permission while
-- exposing granular controls for read, edit, publish, archive, and import.

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, permission
FROM roles r
CROSS JOIN (
    VALUES
        ('blog.view'),
        ('blog.edit'),
        ('blog.publish'),
        ('blog.archive'),
        ('blog.import'),
        ('blog.manage')
) AS p(permission)
WHERE r.name = 'super_admin'
ON CONFLICT DO NOTHING;
