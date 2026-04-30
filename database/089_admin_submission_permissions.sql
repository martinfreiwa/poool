-- 089: Admin developer submission review permissions
-- Grants granular permissions used by developer submission queue/review APIs.

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.permission
FROM roles r
CROSS JOIN (
    VALUES
        ('submissions.review'),
        ('submissions.approve')
) AS p(permission)
WHERE r.name IN ('admin', 'super_admin')
ON CONFLICT DO NOTHING;
