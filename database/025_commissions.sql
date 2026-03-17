-- 025_commissions.sql
-- Adds payout_settings and commissions tables for the affiliate commissions feature.

-- Payout settings: stores the user's preferred payout configuration
CREATE TABLE IF NOT EXISTS payout_settings (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    payment_method VARCHAR(50) NOT NULL DEFAULT 'paypal',  -- 'paypal' | 'bank_transfer'
    account_email  VARCHAR(255),
    full_name      VARCHAR(255),
    street_address VARCHAR(500),
    postcode       VARCHAR(20),
    city           VARCHAR(100),
    country        VARCHAR(100),
    vat_number     VARCHAR(50),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_payout_settings_user UNIQUE (user_id)
);

-- Commissions: stores individual commission payout records
CREATE TABLE IF NOT EXISTS commissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    period_start    DATE NOT NULL,
    period_end      DATE NOT NULL,
    amount_cents    BIGINT NOT NULL DEFAULT 0,
    payment_method  VARCHAR(50) NOT NULL DEFAULT 'paypal',
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',  -- 'pending' | 'paid' | 'failed'
    paid_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_commissions_period UNIQUE (user_id, period_start, period_end)
);

CREATE INDEX IF NOT EXISTS idx_commissions_user_id ON commissions(user_id);
CREATE INDEX IF NOT EXISTS idx_commissions_period ON commissions(user_id, period_start DESC);

-- Seed sample commission data for the dev user
DO $$
DECLARE
    dev_user_id UUID;
BEGIN
    SELECT id INTO dev_user_id FROM users WHERE email = 'martin@poool.com' LIMIT 1;
    IF dev_user_id IS NULL THEN
        SELECT id INTO dev_user_id FROM users ORDER BY created_at ASC LIMIT 1;
    END IF;

    IF dev_user_id IS NOT NULL THEN
        -- Insert payout settings
        INSERT INTO payout_settings (user_id, payment_method, account_email, full_name, street_address, postcode, city, country)
        VALUES (dev_user_id, 'paypal', 'martin@poool.com', 'Martin Freiwald', '6 Arrenberg''sche Höfe', '42117', 'Wuppertal', 'Germany')
        ON CONFLICT (user_id) DO NOTHING;

        -- Insert sample commissions with realistic varied amounts
        INSERT INTO commissions (user_id, period_start, period_end, amount_cents, payment_method, status, paid_at) VALUES
            (dev_user_id, '2026-02-15', '2026-02-28',  25000, 'paypal', 'pending', NULL),
            (dev_user_id, '2026-01-15', '2026-01-31',  37500, 'paypal', 'paid',    '2026-02-01 00:00:00+00'),
            (dev_user_id, '2025-12-15', '2025-12-31',  15000, 'paypal', 'paid',    '2026-01-01 00:00:00+00'),
            (dev_user_id, '2025-11-15', '2025-11-30',  50000, 'paypal', 'paid',    '2025-12-01 00:00:00+00'),
            (dev_user_id, '2025-10-15', '2025-10-31',  10000, 'paypal', 'paid',    '2025-11-01 00:00:00+00'),
            (dev_user_id, '2025-09-15', '2025-09-30', 143963, 'paypal', 'paid',    '2025-10-01 00:00:00+00')
        ON CONFLICT (user_id, period_start, period_end) DO NOTHING;
    END IF;
END $$;
