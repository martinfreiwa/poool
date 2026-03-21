-- ═══════════════════════════════════════════════════════════════════
-- Migration 056: Marketplace RBAC Permissions
-- Phase 0.11 — Add 3 marketplace permissions to the auth system:
--   marketplace.view    — View marketplace data (orderbook, trades, prices)
--   marketplace.manage  — Manage marketplace (orders, fees, settings, kill-switch)
--   marketplace.compliance — Compliance oversight (audit, alerts, OJK reports)
--
-- These permissions are granted to specific roles:
--   super_admin → all 3
--   admin       → already has 'all' (covers marketplace.*)
--   compliance  → marketplace.view + marketplace.compliance
--   finance     → marketplace.view
-- ═══════════════════════════════════════════════════════════════════

-- 1. Grant marketplace permissions to super_admin
INSERT INTO admin_permissions (role_id, permission) 
SELECT r.id, p.perm FROM roles r, 
(VALUES 
    ('marketplace.view'),
    ('marketplace.manage'),
    ('marketplace.compliance')
) AS p(perm) WHERE r.name = 'super_admin'
ON CONFLICT DO NOTHING;

-- 2. Grant marketplace.view + marketplace.compliance to compliance role
INSERT INTO admin_permissions (role_id, permission) 
SELECT r.id, p.perm FROM roles r, 
(VALUES 
    ('marketplace.view'),
    ('marketplace.compliance')
) AS p(perm) WHERE r.name = 'compliance'
ON CONFLICT DO NOTHING;

-- 3. Grant marketplace.view to finance role
INSERT INTO admin_permissions (role_id, permission) 
SELECT r.id, 'marketplace.view' FROM roles r WHERE r.name = 'finance'
ON CONFLICT DO NOTHING;

-- Done! 🎉
