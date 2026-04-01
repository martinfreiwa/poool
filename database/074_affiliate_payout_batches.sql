-- Phase 19: Affiliate Payout Batch Engine Schema

CREATE TABLE payout_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    affiliate_id UUID NOT NULL REFERENCES users(id),
    total_amount_cents BIGINT NOT NULL,
    status VARCHAR(30) NOT NULL DEFAULT 'processing' CHECK (status IN ('processing', 'paid', 'failed')),
    created_by_admin_id UUID NOT NULL REFERENCES users(id),
    paid_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Note: affiliate_commissions already has payout_batch_id UUID.
-- Now we link it.
ALTER TABLE affiliate_commissions
ADD CONSTRAINT fk_affiliate_commissions_payout_batch 
FOREIGN KEY (payout_batch_id) REFERENCES payout_batches(id);

-- Add affiliate_treasury to allowed wallet types
ALTER TABLE wallets DROP CONSTRAINT IF EXISTS wallets_wallet_type_check;
ALTER TABLE wallets ADD CONSTRAINT wallets_wallet_type_check 
CHECK (wallet_type IN ('cash', 'rewards', 'platform_fee', 'affiliate_treasury', 'escrow'));

-- Create the affiliate_treasury wallet for the platform admin
DO $$
DECLARE
    admin_id UUID;
BEGIN
    SELECT id INTO admin_id FROM users WHERE email = 'admin@poool.app' LIMIT 1;
    IF admin_id IS NOT NULL THEN
        INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
        VALUES (admin_id, 'affiliate_treasury', 'USD', 10000000) -- Seed with $100,000 for testing payouts
        ON CONFLICT (user_id, wallet_type, currency) DO NOTHING;
    END IF;
END $$;
