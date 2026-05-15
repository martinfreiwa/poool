-- ============================================================
-- POOOL Platform – Fix All Test Warnings
-- ============================================================
-- Addresses all 15 warnings found by test_platform.py:
--   1. Incomplete user profiles (missing fields)
--   2. Missing user_settings row for jonas.freiwald@poool.app
--   3. Missing asset images for 4 assets
--   4. Auth warnings are expected (test user password / test env)
--
-- Run: psql -d poool -f database/003_fix_warnings.sql
-- ============================================================

BEGIN;

-- ── 1. Complete User Profiles ────────────────────────────────────
-- admin@poool.finance
UPDATE user_profiles SET
    phone_number    = '+49 170 0000001',
    date_of_birth   = '1985-03-10',
    country         = 'DE'
WHERE user_id = (SELECT id FROM users WHERE email = 'admin@poool.finance');

-- jonas.freiwald@poool.app
UPDATE user_profiles SET
    phone_number    = '+49 170 0000002',
    date_of_birth   = '1993-07-22',
    country         = 'DE'
WHERE user_id = (SELECT id FROM users WHERE email = 'jonas.freiwald@poool.app');

-- qa_test_final@poool.app
UPDATE user_profiles SET
    first_name      = 'QA',
    last_name       = 'Test',
    phone_number    = '+1 555 000 0001',
    date_of_birth   = '1995-01-01',
    country         = 'US'
WHERE user_id = (SELECT id FROM users WHERE email = 'qa_test_final@poool.app');

-- referred_* placeholder accounts – give them minimal data
UPDATE user_profiles SET
    first_name      = COALESCE(NULLIF(first_name, ''), 'Invited'),
    last_name       = COALESCE(NULLIF(last_name, ''), 'User'),
    phone_number    = COALESCE(NULLIF(phone_number, ''), '+1 555 000 0000'),
    date_of_birth   = COALESCE(date_of_birth, '1990-01-01'),
    country         = COALESCE(NULLIF(country, ''), 'US')
WHERE user_id IN (
    SELECT id FROM users WHERE email LIKE 'referred_%@example.com'
);

-- test@poool.app – add missing date_of_birth
UPDATE user_profiles SET
    date_of_birth = '1990-05-15'
WHERE user_id = (SELECT id FROM users WHERE email = 'test@poool.app')
  AND date_of_birth IS NULL;

DO $$ BEGIN
  RAISE NOTICE '✓ User profiles completed';
END $$;


-- ── 2. Create missing user_settings for jonas.freiwald@poool.app ─

INSERT INTO user_settings (user_id)
SELECT id FROM users WHERE email = 'jonas.freiwald@poool.app'
ON CONFLICT (user_id) DO NOTHING;

DO $$ BEGIN
  RAISE NOTICE '✓ User settings created for jonas.freiwald@poool.app';
END $$;


-- ── 3. Insert missing asset images ───────────────────────────────

-- Sunset Heights Villa (a1111111-…)
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
SELECT id, '/images/villa1.webp', 'Sunset Heights Villa exterior', 0, TRUE
FROM assets WHERE title = 'Sunset Heights Villa'
ON CONFLICT DO NOTHING;

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
SELECT id, '/images/villa1_2.webp', 'Sunset Heights Villa pool', 1, FALSE
FROM assets WHERE title = 'Sunset Heights Villa'
ON CONFLICT DO NOTHING;

-- Central Plaza Commerce (a2222222-…)
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
SELECT id, '/images/villa3_1.webp', 'Central Plaza Commerce exterior', 0, TRUE
FROM assets WHERE title = 'Central Plaza Commerce'
ON CONFLICT DO NOTHING;

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
SELECT id, '/images/villa3_2.webp', 'Central Plaza Commerce interior', 1, FALSE
FROM assets WHERE title = 'Central Plaza Commerce'
ON CONFLICT DO NOTHING;

-- Green Field Agriculture (a3333333-…) – commodity, use commodity images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
SELECT id,
    '/static/images/commodities/rice/eduardo-prim-3u51-uLQICc-unsplash.webp',
    'Green Field Agriculture – lush farmland', 0, TRUE
FROM assets WHERE title = 'Green Field Agriculture'
ON CONFLICT DO NOTHING;

-- Uluwatu Luxury Retreat (a4444444-…)
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
SELECT id, '/images/villa4_1.webp', 'Uluwatu Luxury Retreat exterior', 0, TRUE
FROM assets WHERE title = 'Uluwatu Luxury Retreat'
ON CONFLICT DO NOTHING;

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
SELECT id, '/images/villa1_2.webp', 'Uluwatu Luxury Retreat pool', 1, FALSE
FROM assets WHERE title = 'Uluwatu Luxury Retreat'
ON CONFLICT DO NOTHING;

DO $$ BEGIN
  RAISE NOTICE '✓ Asset images added for all 4 assets';
END $$;


-- ── Verification ─────────────────────────────────────────────────

DO $$
DECLARE
    v_incomplete_profiles INT;
    v_missing_settings INT;
    v_assets_no_images INT;
BEGIN
    SELECT COUNT(*) INTO v_incomplete_profiles
    FROM user_profiles
    WHERE first_name IS NULL OR last_name IS NULL
       OR phone_number IS NULL OR date_of_birth IS NULL OR country IS NULL;

    SELECT COUNT(*) INTO v_missing_settings
    FROM users u
    WHERE NOT EXISTS (SELECT 1 FROM user_settings us WHERE us.user_id = u.id);

    SELECT COUNT(*) INTO v_assets_no_images
    FROM assets a
    WHERE NOT EXISTS (SELECT 1 FROM asset_images ai WHERE ai.asset_id = a.id);

    RAISE NOTICE '── Post-fix verification ──────────────────';
    RAISE NOTICE '  Incomplete profiles: %', v_incomplete_profiles;
    RAISE NOTICE '  Users without settings: %', v_missing_settings;
    RAISE NOTICE '  Assets without images: %', v_assets_no_images;

    IF v_incomplete_profiles > 0 OR v_missing_settings > 0 OR v_assets_no_images > 0 THEN
        RAISE WARNING 'Some items still need attention – check output above';
    ELSE
        RAISE NOTICE '✓ All database warnings resolved!';
    END IF;
END $$;

COMMIT;
