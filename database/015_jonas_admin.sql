BEGIN;

DO $$
DECLARE
    v_user_id UUID;
    v_super_admin_role_id UUID;
BEGIN
    -- Insert Jonas user
    INSERT INTO users (email, password_hash, email_verified, status)
    VALUES ('jonas.freiwald@poool.app', '$2b$12$LQv3c1VqBWrtQGR9P1QGTeO9C.oTqO4Vl.oYtO4Vl.oYtO4Vl.oYt', TRUE, 'active')
    ON CONFLICT (email) DO NOTHING;
    
    SELECT id INTO v_user_id FROM users WHERE email = 'jonas.freiwald@poool.app';
    
    -- Ensure profile exists
    INSERT INTO user_profiles (user_id, first_name, last_name)
    VALUES (v_user_id, 'Jonas', 'Freiwald')
    ON CONFLICT (user_id) DO NOTHING;

    -- Ensure wallets exist
    INSERT INTO wallets (user_id, wallet_type, balance_cents, currency)
    VALUES (v_user_id, 'cash', 0, 'USD')
    ON CONFLICT (user_id, wallet_type, currency) DO NOTHING;

    INSERT INTO wallets (user_id, wallet_type, balance_cents, currency)
    VALUES (v_user_id, 'rewards', 0, 'USD')
    ON CONFLICT (user_id, wallet_type, currency) DO NOTHING;
    
    -- Assign role
    SELECT id INTO v_super_admin_role_id FROM roles WHERE name = 'super_admin';
    
    IF v_user_id IS NOT NULL AND v_super_admin_role_id IS NOT NULL THEN
        INSERT INTO user_roles (user_id, role_id)
        VALUES (v_user_id, v_super_admin_role_id)
        ON CONFLICT DO NOTHING;
    END IF;

END $$;

COMMIT;
