-- ═══════════════════════════════════════════════════════════════════
-- Rollback for 204_cddrp_run1_permission_slugs.sql
--
-- Removes every (role, permission) pair INSERTed by the forward migration.
-- ═══════════════════════════════════════════════════════════════════

DELETE FROM admin_permissions
WHERE permission IN (
    -- Villa subsystem
    'villa.operations.view',
    'villa.operations.write',
    'villa.operations.approve',
    'villa.valuations.view',
    'villa.valuations.write',
    'villa.valuations.approve',
    'villa.capex.view',
    'villa.capex.approve',
    'villa.developer_access.view',
    'villa.developer_access.manage',
    'villa.deduction_policy.view',
    'villa.deduction_policy.write',
    'villa.forecast.view',
    'villa.forecast.write',
    'villa.snapshot.run',
    -- Developer projects
    'developer_projects.view',
    'developer_projects.write',
    -- .view variants
    'kyc.view',
    'users.view',
    'support.view',
    -- Pre-existing gap
    'approvals.manage',
    'marketplace.edit',
    'platform.manage',
    'users.edit'
);
