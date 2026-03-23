-- Drop old constraint and add new one allowing platform_fee
ALTER TABLE wallets DROP CONSTRAINT IF EXISTS wallets_wallet_type_check;
ALTER TABLE wallets ADD CONSTRAINT wallets_wallet_type_check CHECK (wallet_type IN ('cash', 'rewards', 'platform_fee'));

-- Ensure an admin user exists and inject a platform_fee wallet for them
DO $$
DECLARE
    admin_id UUID;
BEGIN
    SELECT id INTO admin_id FROM users WHERE email = 'admin@poool.app' LIMIT 1;
    IF admin_id IS NOT NULL THEN
        INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
        VALUES (admin_id, 'platform_fee', 'USD', 0)
        ON CONFLICT (user_id, wallet_type, currency) DO NOTHING;
    END IF;
END $$;
