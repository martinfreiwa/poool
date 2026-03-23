-- Migration: Create super_admin account for support@traffic-creator.com and approve KYC
-- Following the same pattern as 057_timmytech_super_admin.sql
BEGIN;

DO $$
DECLARE
    v_user_id UUID;
    v_admin_role_id UUID;
    v_super_admin_role_id UUID;
    v_investor_role_id UUID;
BEGIN
    -- 1. Insert or Get User
    -- Password hash is a placeholder (Argon2id format), they should use 'forgot password' to set their real one
    INSERT INTO users (email, password_hash, email_verified, status)
    VALUES ('support@traffic-creator.com', '$argon2id$v=19$m=4096,t=3,p=1$c29tZXNhbHQ$RndZUnpwVnBIUnpWcFJ6VnBSelZwUnpWcFJ6VnBSelZwUnpWcFJ6VnA', TRUE, 'active')
    ON CONFLICT (email) DO UPDATE SET email_verified = TRUE, status = 'active'
    RETURNING id INTO v_user_id;

    -- 2. Ensure profile exists
    INSERT INTO user_profiles (user_id, first_name, last_name, display_name)
    VALUES (v_user_id, 'Support', 'Admin', 'Support Admin')
    ON CONFLICT (user_id) DO NOTHING;

    -- 3. Ensure wallets exist (USD default)
    INSERT INTO wallets (user_id, wallet_type, balance_cents, currency)
    VALUES (v_user_id, 'cash', 0, 'USD')
    ON CONFLICT (user_id, wallet_type, currency) DO NOTHING;

    INSERT INTO wallets (user_id, wallet_type, balance_cents, currency)
    VALUES (v_user_id, 'rewards', 0, 'USD')
    ON CONFLICT (user_id, wallet_type, currency) DO NOTHING;

    -- 4. Assign Roles (investor, admin, super_admin)
    SELECT id INTO v_investor_role_id FROM roles WHERE name = 'investor';
    SELECT id INTO v_admin_role_id FROM roles WHERE name = 'admin';
    SELECT id INTO v_super_admin_role_id FROM roles WHERE name = 'super_admin';

    IF v_user_id IS NOT NULL AND v_investor_role_id IS NOT NULL THEN
        INSERT INTO user_roles (user_id, role_id)
        VALUES (v_user_id, v_investor_role_id)
        ON CONFLICT DO NOTHING;
    END IF;

    IF v_user_id IS NOT NULL AND v_admin_role_id IS NOT NULL THEN
        INSERT INTO user_roles (user_id, role_id)
        VALUES (v_user_id, v_admin_role_id)
        ON CONFLICT DO NOTHING;
    END IF;

    IF v_user_id IS NOT NULL AND v_super_admin_role_id IS NOT NULL THEN
        INSERT INTO user_roles (user_id, role_id)
        VALUES (v_user_id, v_super_admin_role_id)
        ON CONFLICT DO NOTHING;
    END IF;

    -- 5. Approve KYC
    IF NOT EXISTS (SELECT 1 FROM kyc_records WHERE user_id = v_user_id AND status = 'approved') THEN
        INSERT INTO kyc_records (user_id, status, provider, verified_at)
        VALUES (v_user_id, 'approved', 'system_override', NOW());
    END IF;

    -- 6. Ensure investment limits exist
    -- Set a high limit for super admin (10M USD)
    INSERT INTO investment_limits (user_id, annual_limit_cents, invested_12m_cents, limit_year)
    VALUES (v_user_id, 1000000000, 0, EXTRACT(YEAR FROM NOW())::INTEGER)
    ON CONFLICT (user_id, limit_year) DO NOTHING;

END $$;

COMMIT;
