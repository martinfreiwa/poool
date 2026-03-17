-- ═══════════════════════════════════════════════════════════════════
-- Migration 010: Advanced Admin RBAC & Four-Eyes Principle
-- Implements multi-party approval, admin invitations, and 
-- granular permission structures for the enterprise dashboard.
-- ═══════════════════════════════════════════════════════════════════

-- 1. admin_invitations (Secure Onboarding)
CREATE TABLE IF NOT EXISTS admin_invitations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           VARCHAR(255) NOT NULL UNIQUE,
    role_id         UUID NOT NULL REFERENCES roles(id),
    invited_by      UUID NOT NULL REFERENCES users(id),
    token_hash      VARCHAR(255) NOT NULL UNIQUE,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'accepted', 'expired', 'revoked')),
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at     TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_admin_inv_token ON admin_invitations(token_hash);

-- 2. admin_approval_requests (Four-Eyes Principle / Multi-Party Authorization)
CREATE TABLE IF NOT EXISTS admin_approval_requests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    requester_id    UUID NOT NULL REFERENCES users(id),
    approver_id     UUID REFERENCES users(id), -- Null if pending
    action_type     VARCHAR(50) NOT NULL, -- e.g. 'treasury.payout', 'user.suspend', 'settings.update'
    entity_type     VARCHAR(50) NOT NULL, -- e.g. 'wallet_transaction', 'user'
    entity_id       UUID,
    payload         JSONB NOT NULL, -- The data for the action to be performed
    status          VARCHAR(20) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'approved', 'rejected', 'expired')),
    rejection_reason TEXT,
    expires_at      TIMESTAMPTZ NOT NULL DEFAULT (NOW() + interval '24 hours'),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_approval_status ON admin_approval_requests(status);
CREATE INDEX IF NOT EXISTS idx_approval_requester ON admin_approval_requests(requester_id);

-- 3. Add Granular Permissions
-- Extension of existing permissions in Migration 006
INSERT INTO admin_permissions (role_id, permission) 
SELECT r.id, p.perm FROM roles r, 
(VALUES 
    ('admins.manage'),          -- Invite/Suspend admins
    ('roles.edit'),             -- Edit permission matrix
    ('pii.view'),               -- See unmasked sensitive data
    ('financials.payout.draft'),-- Initiate a payout request
    ('financials.payout.approve')-- Approve a payout request (Four-Eyes)
) AS p(perm) WHERE r.name = 'super_admin'
ON CONFLICT DO NOTHING;

-- Grant permissions to specific roles
INSERT INTO admin_permissions (role_id, permission) 
SELECT r.id, p.perm FROM roles r, 
(VALUES ('pii.view'), ('kyc.override')) AS p(perm) WHERE r.name = 'compliance'
ON CONFLICT DO NOTHING;

INSERT INTO admin_permissions (role_id, permission) 
SELECT r.id, p.perm FROM roles r, 
(VALUES ('financials.payout.draft'), ('treasury.read')) AS p(perm) WHERE r.name = 'finance'
ON CONFLICT DO NOTHING;

-- 4. User Role Extension: Time-Based & Geo-Fencing
ALTER TABLE user_roles ADD COLUMN IF NOT EXISTS authorized_ips INET[]; -- Allowlist
ALTER TABLE user_roles ADD COLUMN IF NOT EXISTS access_start_time TIME; -- Daily restriction
ALTER TABLE user_roles ADD COLUMN IF NOT EXISTS access_end_time TIME;   -- Daily restriction

-- Apply updated_at trigger to approval requests
CREATE TRIGGER set_updated_at BEFORE UPDATE ON admin_approval_requests FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- Done! 🎉
