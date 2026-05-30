-- Platform wallets are not owned by an end user. Keep user ownership
-- mandatory for normal wallets while allowing one system wallet per type
-- and currency.
ALTER TABLE wallets
    ALTER COLUMN user_id DROP NOT NULL;

ALTER TABLE wallets
    DROP CONSTRAINT IF EXISTS wallets_user_required_unless_system_wallet;

ALTER TABLE wallets
    ADD CONSTRAINT wallets_user_required_unless_system_wallet
    CHECK (
        user_id IS NOT NULL
        OR wallet_type IN ('affiliate_treasury', 'platform_fee', 'escrow')
    );

CREATE UNIQUE INDEX IF NOT EXISTS wallets_system_wallet_type_currency_key
    ON wallets (wallet_type, currency)
    WHERE user_id IS NULL;
