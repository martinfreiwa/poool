-- 158: Admin-Permission für Team-Affiliate-Verwaltung
--
-- Gewährt admin/super_admin die Permission `affiliates.team_manage`. Damit
-- können Admin-Tools (z.B. Team aktivieren/sperren, Mitarbeiter umhängen,
-- Konflikte lösen) sauber gegated werden, getrennt von der allgemeinen
-- `affiliates.manage`-Permission (Application-Review, Payouts).
--
-- Hinweis: Developer-Accounts haben keine admin_permissions-Row. Ihre
-- Team-Management-Endpoints werden auf Application-Layer via DeveloperUser-
-- Extractor + Team-Ownership-Check geprüft, nicht über admin_permissions.

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, 'affiliates.team_manage'
FROM roles r
WHERE r.name IN ('admin', 'super_admin')
ON CONFLICT DO NOTHING;
