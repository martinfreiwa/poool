-- ============================================================
-- POOOL Platform – Rewards Seed Data (002b)
-- ============================================================
-- Seeds rewards_balances, user_tiers, referral_codes for test@poool.app
-- Also cleans up any rogue test tiers from the tiers table.
--
-- Prerequisites: 004_rewards_schema.sql must be applied.
--                test@poool.app user must exist.
--
-- Run: psql -d poool -f database/002b_seed_rewards.sql
-- ============================================================

BEGIN;

DO $$
DECLARE
    v_user_id     UUID;
    v_intro_id    INT;
BEGIN
    -- ── Get test user ─────────────────────────────────────────
    SELECT id INTO v_user_id FROM users WHERE email = 'test@poool.app' LIMIT 1;

    IF v_user_id IS NULL THEN
        RAISE EXCEPTION 'User test@poool.app not found – run 002_seed_data.sql first';
    END IF;

    -- ── Remove rogue/test tiers that are not the real 5 ──────
    -- First, reassign any user_tiers that point to a rogue tier → Intro
    SELECT id INTO v_intro_id FROM tiers WHERE name = 'Intro' LIMIT 1;

    IF v_intro_id IS NULL THEN
        RAISE EXCEPTION 'Intro tier not found – run 004_rewards_schema.sql first';
    END IF;

    UPDATE user_tiers
    SET tier_id = v_intro_id
    WHERE tier_id NOT IN (SELECT id FROM tiers WHERE name IN ('Intro','Plus','Pro','Elite','Premium'));

    -- Delete rogue tiers (e.g. TestTier)
    DELETE FROM tiers
    WHERE name NOT IN ('Intro', 'Plus', 'Pro', 'Elite', 'Premium');

    RAISE NOTICE 'Cleaned up rogue tiers ✓';

    -- ── Seed rewards_balances ─────────────────────────────────
    -- cashback = $50 (5000 cents), referrals = $30 (3000 cents), promotions = $20 (2000 cents)
    INSERT INTO rewards_balances (user_id, cashback, referrals, promotions)
    VALUES (v_user_id, 5000, 3000, 2000)
    ON CONFLICT (user_id) DO UPDATE SET
        cashback    = 5000,
        referrals   = 3000,
        promotions  = 2000,
        updated_at  = NOW();

    RAISE NOTICE 'rewards_balances seeded: cashback=5000, referrals=3000, promotions=2000 ✓';

    -- ── Seed user_tiers ───────────────────────────────────────
    -- invested_12m = $2,500 (250000 cents) → sits in Plus range (>= $10,000)
    -- Use a more realistic amount: $5,000 = 500000 cents → mid-range between Intro and Plus
    INSERT INTO user_tiers (user_id, tier_id, invested_12m)
    VALUES (v_user_id, v_intro_id, 500000)
    ON CONFLICT (user_id) DO UPDATE SET
        tier_id      = v_intro_id,
        invested_12m = 500000,
        updated_at   = NOW();

    RAISE NOTICE 'user_tiers seeded: tier=Intro, invested_12m=500000 ✓';

    -- ── Seed referral_codes ───────────────────────────────────
    INSERT INTO referral_codes (user_id, code)
    VALUES (v_user_id, 'POOOL001')
    ON CONFLICT (user_id) DO UPDATE SET
        code = 'POOOL001';

    RAISE NOTICE 'referral_codes seeded: code=POOOL001 ✓';

END $$;

COMMIT;

-- ── Verification ──────────────────────────────────────────────
\echo ''
\echo '=== Rewards Seed Verification ==='

SELECT 'rewards_balances' AS table_name,
       cashback, referrals, promotions,
       (cashback + referrals + promotions) AS total
FROM rewards_balances rb
JOIN users u ON u.id = rb.user_id
WHERE u.email = 'test@poool.app';

SELECT 'user_tiers' AS table_name, t.name AS tier, ut.invested_12m
FROM user_tiers ut
JOIN tiers t ON t.id = ut.tier_id
JOIN users u ON u.id = ut.user_id
WHERE u.email = 'test@poool.app';

SELECT 'referral_codes' AS table_name, code
FROM referral_codes rc
JOIN users u ON u.id = rc.user_id
WHERE u.email = 'test@poool.app';

SELECT 'tiers' AS table_name, id, name, min_invest, cashback_pct, sort_order
FROM tiers
ORDER BY sort_order;
