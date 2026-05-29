-- ═══════════════════════════════════════════════════════════════════
-- Migration 204: CDDRP Run 1 — permission slug seeds
--
-- Closes the gap between permission slugs USED IN CODE and slugs
-- ACTUALLY SEEDED in the admin_permissions table. Without this, the
-- ~24 affected `require_permission(...)` call sites fail closed for
-- every sub-role (compliance, finance, support) — they only pass for
-- super_admin/admin because those two hold the 'all' wildcard.
--
-- Slugs added this session (CDDRP Run 1, audit doc:
-- docs/security-audits/2026-05-19-cddrp-run-1.md):
--
--   Villa subsystem (15):
--     villa.operations.{view,write,approve}
--     villa.valuations.{view,write,approve}
--     villa.capex.{view,approve}
--     villa.developer_access.{view,manage}
--     villa.deduction_policy.{view,write}
--     villa.forecast.{view,write}
--     villa.snapshot.run
--
--   Developer applications/projects review (2):
--     developer_projects.view
--     developer_projects.write
--
--   .view variants alongside existing .read (3 — parallel grants so
--   the new code and the old code both work; CONVENTION DRIFT to be
--   reconciled in a separate cleanup):
--     kyc.view (parallels kyc.read)
--     users.view (parallels users.read)
--     support.view (parallels support.read)
--
-- Pre-existing gap (4 — were already used in code before this session
-- but never seeded; granting to super_admin only as a conservative
-- default — sub-role grants need explicit product input):
--     approvals.manage
--     marketplace.edit
--     platform.manage
--     users.edit
--
-- Role-grant rationale per slug captured inline below. super_admin
-- gets EVERY new slug. admin already covers them via the 'all'
-- wildcard (no INSERT needed for admin). Sub-role grants follow the
-- existing patterns established in migrations 006 / 056 / 077–089.
-- ═══════════════════════════════════════════════════════════════════

-- 1. Villa subsystem — super_admin gets all 15
INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.perm FROM roles r,
(VALUES
    ('villa.operations.view'),
    ('villa.operations.write'),
    ('villa.operations.approve'),
    ('villa.valuations.view'),
    ('villa.valuations.write'),
    ('villa.valuations.approve'),
    ('villa.capex.view'),
    ('villa.capex.approve'),
    ('villa.developer_access.view'),
    ('villa.developer_access.manage'),
    ('villa.deduction_policy.view'),
    ('villa.deduction_policy.write'),
    ('villa.forecast.view'),
    ('villa.forecast.write'),
    ('villa.snapshot.run')
) AS p(perm)
WHERE r.name = 'super_admin'
ON CONFLICT DO NOTHING;

-- 2. Villa read-only views for compliance (KYC/AML oversight needs visibility
-- into villa operations/valuations/capex/developer-access/deduction-policy)
INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.perm FROM roles r,
(VALUES
    ('villa.operations.view'),
    ('villa.valuations.view'),
    ('villa.valuations.approve'),   -- compliance can approve valuations (independent oversight)
    ('villa.capex.view'),
    ('villa.developer_access.view'),
    ('villa.deduction_policy.view')
) AS p(perm)
WHERE r.name = 'compliance'
ON CONFLICT DO NOTHING;

-- 3. Villa reads + finance-relevant approvals for finance (treasury/payout sign-off)
INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.perm FROM roles r,
(VALUES
    ('villa.operations.view'),
    ('villa.operations.approve'),    -- finance signs off on monthly payouts
    ('villa.valuations.view'),
    ('villa.valuations.approve'),    -- finance signs off on NAV-impacting valuations
    ('villa.capex.view'),
    ('villa.capex.approve'),         -- finance reviews capex impact on distributable
    ('villa.deduction_policy.view'),
    ('villa.forecast.view')
) AS p(perm)
WHERE r.name = 'finance'
ON CONFLICT DO NOTHING;

-- 4. Developer-applications review surface (developer_projects.*)
-- super_admin gets both; compliance + support get read access for
-- triage; admin inherits via 'all'.
INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.perm FROM roles r,
(VALUES
    ('developer_projects.view'),
    ('developer_projects.write')
) AS p(perm)
WHERE r.name = 'super_admin'
ON CONFLICT DO NOTHING;

INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, 'developer_projects.view' FROM roles r
WHERE r.name IN ('compliance', 'support')
ON CONFLICT DO NOTHING;

-- 5. .view variants — parallel grants matching the existing .read holders
-- (codebase has drift: some handlers use .read, newer handlers use .view).
-- Granting both keeps every handler working until the convention is
-- consolidated in a future cleanup migration.
INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.perm FROM roles r,
(VALUES ('kyc.view'), ('users.view'), ('support.view')) AS p(perm)
WHERE r.name = 'super_admin'
ON CONFLICT DO NOTHING;

-- compliance: had kyc.read + users.read → now also gets kyc.view + users.view
INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.perm FROM roles r,
(VALUES ('kyc.view'), ('users.view')) AS p(perm)
WHERE r.name = 'compliance'
ON CONFLICT DO NOTHING;

-- support: had support.read + users.read → now also gets support.view + users.view
INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.perm FROM roles r,
(VALUES ('support.view'), ('users.view')) AS p(perm)
WHERE r.name = 'support'
ON CONFLICT DO NOTHING;

-- 6. Pre-existing gap — slugs USED IN CODE but never seeded.
-- Granting to super_admin only (admin inherits via 'all'). Sub-role
-- grants need explicit product input — leave empty here.
INSERT INTO admin_permissions (role_id, permission)
SELECT r.id, p.perm FROM roles r,
(VALUES
    ('approvals.manage'),
    ('marketplace.edit'),
    ('platform.manage'),
    ('users.edit')
) AS p(perm)
WHERE r.name = 'super_admin'
ON CONFLICT DO NOTHING;

-- Done.
