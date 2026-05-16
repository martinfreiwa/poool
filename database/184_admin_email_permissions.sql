-- 184: Admin email-marketing permissions
--
-- Adds granular permissions for the Emails & Marketing admin section so
-- the `AdminUser::require_permission(&pool, "emails.{view,edit,send}")`
-- gates added in backend/src/admin/emails.rs do not silently 403 the
-- existing admin/super_admin roles. Follows the pattern used in 087
-- (notifications) and 088 (reports).
--
-- * emails.view — read template list, KPI dashboard, delivery logs,
--                 audience recipient counts
-- * emails.edit — create / update / delete templates
-- * emails.send — queue marketing campaigns

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, permission
FROM roles r
CROSS JOIN (
    VALUES
        ('emails.view'),
        ('emails.edit'),
        ('emails.send')
) AS perms(permission)
WHERE r.name IN ('admin', 'super_admin')
ON CONFLICT DO NOTHING;
