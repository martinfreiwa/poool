-- 140: Rewards admin management permission
-- Gates the user-rewards dashboard, balance adjustments, and tier mutation
-- endpoints. Previously these endpoints relied only on broad admin role
-- extraction and were accessible to any role that decoded as AdminUser
-- (including support / kyc_reviewer).

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, 'rewards.manage'
FROM roles r
WHERE r.name IN ('admin', 'super_admin')
ON CONFLICT DO NOTHING;
