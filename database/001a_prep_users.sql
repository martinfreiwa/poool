-- Migration to prepare users for seed data
BEGIN;

-- 005_payments_checkout.sql adds wallets.currency; this seed runs before
-- that migration in fresh CI applies. Make this migration self-sufficient
-- by adding just the column up-front (idempotent via IF NOT EXISTS).
-- 005 still owns the constraint swap — leaving the OLD unique constraint
-- in place here lets ON CONFLICT (user_id, wallet_type) match correctly
-- and avoids fighting 005's ADD CONSTRAINT on re-apply.
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'USD';

-- Insert roles if they don't exist
INSERT INTO roles (name, description) VALUES 
('admin', 'Administrator'),
('super_admin', 'Super Administrator'),
('user', 'Regular User')
ON CONFLICT (name) DO NOTHING;

-- Insert test user
INSERT INTO users (email, password_hash, email_verified, status)
VALUES ('test@poool.app', '$2b$12$LQv3c1VqBWrtQGR9P1QGTeO9C.oTqO4Vl.oYtO4Vl.oYtO4Vl.oYt', TRUE, 'active')
ON CONFLICT (email) DO NOTHING;

-- Get user id
DO $$
DECLARE
    v_user_id UUID;
    v_admin_role_id UUID;
    v_super_admin_role_id UUID;
BEGIN
    SELECT id INTO v_user_id FROM users WHERE email = 'test@poool.app';
    
    -- Ensure profile exists
    INSERT INTO user_profiles (user_id, first_name, last_name)
    VALUES (v_user_id, 'Test', 'User')
    ON CONFLICT (user_id) DO NOTHING;

    -- Ensure wallets exist (with standard currency)
    INSERT INTO wallets (user_id, wallet_type, balance_cents, currency)
    VALUES (v_user_id, 'cash', 50000000, 'USD')
    ON CONFLICT (user_id, wallet_type) DO NOTHING;

    INSERT INTO wallets (user_id, wallet_type, balance_cents, currency)
    VALUES (v_user_id, 'rewards', 100000, 'USD')
    ON CONFLICT (user_id, wallet_type) DO NOTHING;
    
    -- Create admin user
    INSERT INTO users (email, password_hash, email_verified, status)
    VALUES ('admin@poool.app', '$2b$12$LQv3c1VqBWrtQGR9P1QGTeO9C.oTqO4Vl.oYtO4Vl.oYtO4Vl.oYt', TRUE, 'active')
    ON CONFLICT (email) DO NOTHING;
    
    SELECT id INTO v_user_id FROM users WHERE email = 'admin@poool.app';
    
    INSERT INTO user_profiles (user_id, first_name, last_name)
    VALUES (v_user_id, 'Admin', 'User')
    ON CONFLICT (user_id) DO NOTHING;
    
    -- Assign roles
    SELECT id INTO v_admin_role_id FROM roles WHERE name = 'admin';
    SELECT id INTO v_super_admin_role_id FROM roles WHERE name = 'super_admin';
    
    IF v_user_id IS NOT NULL THEN
        INSERT INTO user_roles (user_id, role_id)
        VALUES (v_user_id, v_admin_role_id)
        ON CONFLICT DO NOTHING;
        
        INSERT INTO user_roles (user_id, role_id)
        VALUES (v_user_id, v_super_admin_role_id)
        ON CONFLICT DO NOTHING;
    END IF;

END $$;

COMMIT;
