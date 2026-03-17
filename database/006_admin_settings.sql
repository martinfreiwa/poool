-- ═══════════════════════════════════════════════════════════════════
-- Migration 006: Admin Settings & RBAC Tables
-- Creates platform_settings, roles, user_roles, admin_permissions
-- for the admin settings page.
-- ═══════════════════════════════════════════════════════════════════

-- Enable UUID generation (idempotent)
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================
-- 1. roles
-- ============================================================
CREATE TABLE IF NOT EXISTS roles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(30) NOT NULL UNIQUE,
    description     VARCHAR(255)
);
INSERT INTO roles (name, description) VALUES
    ('investor', 'Standard investor with marketplace and portfolio access'),
    ('developer', 'Real estate developer who can list assets'),
    ('admin', 'Platform administrator'),
    ('super_admin', 'Super administrator with full access'),
    ('compliance', 'Compliance officer - KYC and AML access'),
    ('support', 'Support agent - ticket management'),
    ('finance', 'Finance manager - treasury and payouts')
ON CONFLICT (name) DO NOTHING;

-- ============================================================
-- 2. user_roles
-- ============================================================
CREATE TABLE IF NOT EXISTS user_roles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id         UUID NOT NULL REFERENCES roles(id),
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, role_id)
);
CREATE INDEX IF NOT EXISTS idx_user_roles_user ON user_roles(user_id);

-- ============================================================
-- 3. admin_permissions (per-role permission grants)
-- ============================================================
CREATE TABLE IF NOT EXISTS admin_permissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    role_id         UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission      VARCHAR(100) NOT NULL,
    UNIQUE (role_id, permission)
);

-- Seed default permissions
-- super_admin + admin get "all"
INSERT INTO admin_permissions (role_id, permission) 
SELECT r.id, 'all' FROM roles r WHERE r.name IN ('super_admin', 'admin')
ON CONFLICT DO NOTHING;

-- compliance
INSERT INTO admin_permissions (role_id, permission) 
SELECT r.id, p.perm FROM roles r, 
(VALUES ('kyc.read'), ('kyc.write'), ('users.read')) AS p(perm) WHERE r.name = 'compliance'
ON CONFLICT DO NOTHING;

-- support
INSERT INTO admin_permissions (role_id, permission) 
SELECT r.id, p.perm FROM roles r, 
(VALUES ('support.read'), ('support.write'), ('users.read')) AS p(perm) WHERE r.name = 'support'
ON CONFLICT DO NOTHING;

-- finance
INSERT INTO admin_permissions (role_id, permission) 
SELECT r.id, p.perm FROM roles r, 
(VALUES ('treasury.read'), ('treasury.write'), ('deposits.read'), ('deposits.write'), ('orders.read')) AS p(perm) WHERE r.name = 'finance'
ON CONFLICT DO NOTHING;

-- ============================================================
-- 4. platform_settings (key-value store)
-- ============================================================
CREATE TABLE IF NOT EXISTS platform_settings (
    key             VARCHAR(100) PRIMARY KEY,
    value           TEXT NOT NULL,
    value_type      VARCHAR(20) NOT NULL DEFAULT 'string' CHECK (value_type IN ('string', 'number', 'boolean', 'json')),
    description     VARCHAR(500),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by      UUID REFERENCES users(id)
);

INSERT INTO platform_settings (key, value, value_type, description) VALUES
    ('platform_name', 'POOOL Finance', 'string', 'Display name used across the platform'),
    ('support_email', 'support@poool.finance', 'string', 'Support contact email'),
    ('enable_registrations', 'true', 'boolean', 'Allow new user registrations'),
    ('require_kyc', 'true', 'boolean', 'Require KYC verification for investments'),
    ('platform_fee_percent', '2.50', 'number', 'Platform fee percentage on token purchases'),
    ('withdrawal_fee_cents', '500', 'number', 'Flat withdrawal fee in cents'),
    ('referral_commission_percent', '1.00', 'number', 'Referral commission percentage'),
    ('min_withdrawal_cents', '1000', 'number', 'Minimum withdrawal amount in cents'),
    ('maintenance_mode', 'false', 'boolean', 'Redirect all users to maintenance page')
ON CONFLICT (key) DO NOTHING;

-- ============================================================
-- Done! 🎉
-- ============================================================
