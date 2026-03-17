-- ============================================================
-- POOOL Platform – Production Seed Data
-- ============================================================
-- Adapted from 002_seed_data.sql for production use.
-- Uses jonas.freiwald@poool.app as the seed user.
-- ============================================================

BEGIN;

DO $$
DECLARE
    v_user_id UUID;
    v_cash_wallet_id UUID;
    v_rewards_wallet_id UUID;

    -- Asset IDs
    v_asset1 UUID;
    v_asset2 UUID;
    v_asset3 UUID;
    v_asset4 UUID;
    v_asset5 UUID;
    v_asset6 UUID;
    v_asset7 UUID;
    v_asset8 UUID;
    -- Commodity assets
    v_com1 UUID;
    v_com2 UUID;
    v_com3 UUID;
BEGIN

-- Get user
SELECT id INTO v_user_id FROM users WHERE email = 'jonas.freiwald@poool.app' LIMIT 1;

IF v_user_id IS NULL THEN
    RAISE EXCEPTION 'No user found with email jonas.freiwald@poool.app';
END IF;

-- Get or create wallets
SELECT id INTO v_cash_wallet_id FROM wallets WHERE user_id = v_user_id AND wallet_type = 'cash';
IF v_cash_wallet_id IS NULL THEN
    INSERT INTO wallets (user_id, wallet_type, balance_cents) VALUES (v_user_id, 'cash', 0) RETURNING id INTO v_cash_wallet_id;
END IF;

SELECT id INTO v_rewards_wallet_id FROM wallets WHERE user_id = v_user_id AND wallet_type = 'rewards';
IF v_rewards_wallet_id IS NULL THEN
    INSERT INTO wallets (user_id, wallet_type, balance_cents) VALUES (v_user_id, 'rewards', 0) RETURNING id INTO v_rewards_wallet_id;
END IF;

RAISE NOTICE 'Using user: % (id: %)', 'jonas.freiwald@poool.app', v_user_id;

-- ── 1. Insert Real Estate Assets ─────────────────────────────

-- Property 1: Luxury Clifftop Villa
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, lease_term_years, land_size_sqm, building_size_sqm,
    bedrooms, bathrooms, construction_status, year_built,
    location_city, location_country, location_address,
    location_lat, location_lng, location_description,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
    funding_status, featured, published, term_months
) VALUES (
    gen_random_uuid(),
    'Luxury Clifftop Villa with Ocean Views in Uluwatu',
    'luxury-clifftop-villa-uluwatu',
    'Stunning clifftop villa with infinity pool and panoramic Indian Ocean views.',
    'This exceptional 4-bedroom villa sits atop the dramatic limestone cliffs of Uluwatu, offering 180-degree views of the Indian Ocean. Features include an infinity edge pool, open-air living pavilion, professional kitchen, and private access to a secluded beach.',
    'real_estate',
    'villa', 'Uluwatu', 'leasehold', 25, 800.00, 450.00,
    4, 5, 'ready', 2022,
    'Bali', 'ID', 'Jl. Pantai Suluban, Uluwatu, Pecatu, Bali 80364',
    -8.8113, 115.0887, 'Perched on the clifftops of Uluwatu with stunning ocean views.',
    133400000, 13340, 10000, 1100,
    1050, 800, 8900,
    'funding_in_progress', TRUE, TRUE, 36
) RETURNING id INTO v_asset1;

-- Property 2: Modern Surf Villa
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, land_size_sqm, building_size_sqm,
    bedrooms, bathrooms, construction_status, year_built,
    location_city, location_country, location_address,
    location_lat, location_lng,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
    funding_status, featured, published, term_months
) VALUES (
    gen_random_uuid(),
    'Modern Surf Villa near Echo Beach in Canggu',
    'modern-surf-villa-canggu',
    'Contemporary villa steps from Echo Beach, Canggu''s most popular surf break.',
    'Modern 4-bedroom freehold villa located just 200m from Echo Beach in the heart of Canggu.',
    'real_estate',
    'villa', 'Canggu', 'freehold', 600.00, 380.00,
    4, 4, 'ready', 2023,
    'Bali', 'ID', 'Jl. Nelayan, Echo Beach, Canggu, Bali 80361',
    -8.6509, 115.1300,
    115000000, 11500, 10000, 2400,
    1200, 900, 8500,
    'funding_in_progress', TRUE, TRUE, 60
) RETURNING id INTO v_asset2;

-- Property 3: Boutique Resort
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, lease_term_years, land_size_sqm, building_size_sqm,
    bedrooms, bathrooms, construction_status, year_built,
    location_city, location_country, location_address,
    location_lat, location_lng,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
    funding_status, featured, published, term_months
) VALUES (
    gen_random_uuid(),
    'Boutique Resort with 6 Villas in Central Ubud',
    'boutique-resort-ubud',
    'Luxury boutique resort with 6 private villas surrounded by rice terraces.',
    'A fully operational boutique resort featuring 6 individually designed villas, each with private pool.',
    'real_estate',
    'commercial', 'Ubud', 'leasehold', 30, 2500.00, 1200.00,
    12, 14, 'ready', 2020,
    'Bali', 'ID', 'Jl. Kajeng, Ubud, Gianyar, Bali 80571',
    -8.5069, 115.2625,
    285000000, 28500, 10000, 3600,
    1400, 600, 7800,
    'funding_in_progress', FALSE, TRUE, 48
) RETURNING id INTO v_asset3;

-- Property 4: Vacation Rental Villa
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, lease_term_years, land_size_sqm, building_size_sqm,
    bedrooms, bathrooms, construction_status, year_built,
    location_city, location_country, location_address,
    location_lat, location_lng,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
    funding_status, featured, published, term_months
) VALUES (
    gen_random_uuid(),
    'Vacation Rental Villa with Temple Views',
    'vacation-rental-villa-uluwatu',
    'Charming 3-bedroom villa with views of the iconic Uluwatu Temple.',
    'A beautifully designed 3-bedroom villa offering stunning views of Uluwatu Temple and the ocean.',
    'real_estate',
    'villa', 'Uluwatu', 'leasehold', 20, 500.00, 280.00,
    3, 3, 'ready', 2021,
    'Bali', 'ID', 'Jl. Pura Uluwatu, Pecatu, Kuta, Bali 80361',
    -8.8295, 115.0849,
    78500000, 7850, 10000, 800,
    1300, 700, 9200,
    'funding_in_progress', FALSE, TRUE, 36
) RETURNING id INTO v_asset4;

-- Property 5: Renovation Flip
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, lease_term_years, land_size_sqm, building_size_sqm,
    bedrooms, bathrooms, construction_status,
    location_city, location_country, location_address,
    location_lat, location_lng,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps,
    funding_status, featured, published, term_months
) VALUES (
    gen_random_uuid(),
    'Renovation Flip Project – Canggu Villa',
    'renovation-flip-canggu',
    'Value-add renovation opportunity to flip a dated villa in prime Canggu location.',
    'Short-term investment opportunity: Acquire and renovate a dated 3-bedroom villa on a prime 400sqm plot.',
    'real_estate',
    'villa', 'Canggu', 'leasehold', 20, 400.00, 220.00,
    3, 3, 'renovation',
    'Bali', 'ID', 'Jl. Batu Bolong, Canggu, Bali 80361',
    -8.6483, 115.1345,
    45000000, 4500, 10000, 5500,
    0, 2500,
    'funding_open', FALSE, TRUE, 18
) RETURNING id INTO v_asset5;

-- Property 6: New Development
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, lease_term_years, land_size_sqm,
    bedrooms, bathrooms, construction_status,
    location_city, location_country, location_address,
    location_lat, location_lng,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps,
    funding_status, featured, published, term_months
) VALUES (
    gen_random_uuid(),
    'New Development Project – 4 Villa Complex',
    'new-development-seminyak',
    'Ground-up development of 4 luxury villas in the heart of Seminyak.',
    'An exciting development project to build 4 luxury 2-bedroom villas on a premium 1200sqm plot.',
    'real_estate',
    'villa', 'Seminyak', 'leasehold', 25, 1200.00,
    8, 8, 'construction',
    'Bali', 'ID', 'Jl. Kayu Aya, Seminyak, Bali 80361',
    -8.6815, 115.1580,
    180000000, 18000, 10000, 7000,
    850, 1200,
    'funding_open', TRUE, TRUE, 24
) RETURNING id INTO v_asset6;

-- Property 7: Funded property
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, lease_term_years, land_size_sqm, building_size_sqm,
    bedrooms, bathrooms, construction_status, year_built,
    location_city, location_country, location_address,
    location_lat, location_lng,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
    funding_status, featured, published, term_months
) VALUES (
    gen_random_uuid(),
    'Luxury Pool Villa – Fully Funded',
    'luxury-pool-villa-canggu-funded',
    'This villa has been fully funded and is now generating rental income.',
    'A stunning 3-bedroom pool villa in Berawa, Canggu. Currently fully occupied.',
    'real_estate',
    'villa', 'Canggu', 'leasehold', 25, 500.00, 300.00,
    3, 4, 'ready', 2023,
    'Bali', 'ID', 'Jl. Pantai Berawa, Canggu, Bali 80361',
    -8.6538, 115.1400,
    95000000, 9500, 10000, 0,
    1100, 700, 9000,
    'funded', FALSE, TRUE, 60
) RETURNING id INTO v_asset7;

-- Property 8: Exited property
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, land_size_sqm, building_size_sqm,
    bedrooms, bathrooms, construction_status, year_built,
    location_city, location_country, location_address,
    location_lat, location_lng,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
    funding_status, featured, published, term_months
) VALUES (
    gen_random_uuid(),
    'Beachfront Retreat – Successfully Exited',
    'beachfront-retreat-sanur-exited',
    'This investment has successfully exited with a 42% total return to investors.',
    'A beautiful 2-bedroom beachfront retreat in Sanur that was acquired, renovated, and successfully sold.',
    'real_estate',
    'villa', 'Sanur', 'freehold', 350.00, 180.00,
    2, 2, 'ready', 2019,
    'Bali', 'ID', 'Jl. Pantai Karang, Sanur, Bali 80228',
    -8.6903, 115.2619,
    65000000, 6500, 10000, 0,
    0, 4200, 0,
    'exited', FALSE, TRUE, 18
) RETURNING id INTO v_asset8;

-- ── 2. Commodity Assets ──────────────────────────────────────

INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    operator_name, term_months, fixed_roi_bps,
    revenue_min_cents, revenue_max_cents, expenses_cents,
    net_profit_min_cents, net_profit_max_cents,
    investor_payout_cents, operator_split_pct, poool_split_pct,
    location_city, location_country,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'Premium Bali Rice – Harvest Cycle Q2 2026',
    'premium-bali-rice-q2-2026',
    'Invest in organic Bali rice production with fixed 12% ROI over 6 months.',
    'Partner with established rice farmers in Tabanan to fund the next harvest cycle.',
    'commodity',
    'PT Bali Rice Co.', 6, 1200,
    15000000, 22000000, 8000000,
    7000000, 14000000,
    4000000, 60, 10,
    'Bali', 'ID',
    5000000, 5000, 1000, 350,
    'funding_in_progress', TRUE, TRUE
) RETURNING id INTO v_com1;

INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    operator_name, term_months, fixed_roi_bps,
    revenue_min_cents, revenue_max_cents, expenses_cents,
    net_profit_min_cents, net_profit_max_cents,
    investor_payout_cents, operator_split_pct, poool_split_pct,
    location_city, location_country,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'Organic Cacao – Single Origin Bali',
    'organic-cacao-bali-2026',
    'Fund single-origin cacao production with 15% projected annual return.',
    'Support Bali''s growing artisan chocolate industry.',
    'commodity',
    'Bali Cacao Collective', 12, 1500,
    20000000, 30000000, 10000000,
    10000000, 20000000,
    6000000, 55, 10,
    'Bali', 'ID',
    8000000, 8000, 1000, 600,
    'funding_open', FALSE, TRUE
) RETURNING id INTO v_com2;

INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    operator_name, term_months, fixed_roi_bps,
    location_city, location_country,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'Specialty Coffee – Kintamani Highlands',
    'specialty-coffee-kintamani-2026',
    'Kintamani single-origin coffee production – fully funded.',
    'A fully funded investment in specialty-grade Arabica coffee.',
    'commodity',
    'Kintamani Coffee Farmers', 9, 1100,
    'Bali', 'ID',
    3000000, 3000, 1000, 0,
    'funded', FALSE, TRUE
) RETURNING id INTO v_com3;


-- ── 3. Asset Images ──────────────────────────────────────────

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset1, '/images/villa1.webp', 'Luxury Clifftop Villa exterior', 0, TRUE),
    (v_asset1, '/images/villa1_2.webp', 'Infinity pool with ocean view', 1, FALSE),
    (v_asset1, '/images/villa1_3.webp', 'Modern interior living area', 2, FALSE),
    (v_asset1, '/images/villa1_4.webp', 'Ocean view terrace', 3, FALSE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset2, '/images/villa2_1.webp', 'Modern Surf Villa exterior', 0, TRUE),
    (v_asset2, '/images/villa2_2.webp', 'Tropical garden and pool', 1, FALSE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset3, '/images/villa3_1.webp', 'Boutique Resort entrance', 0, TRUE),
    (v_asset3, '/images/villa3_2.webp', 'Resort common area', 1, FALSE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset4, '/images/villa4_1.webp', 'Vacation Rental Villa', 0, TRUE),
    (v_asset4, '/images/villa4_2.webp', 'Panoramic temple view', 1, FALSE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset5, '/images/villa5.webp', 'Renovation flip project', 0, TRUE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset6, '/images/villa6.webp', 'New development site', 0, TRUE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset7, '/images/villa3_1.webp', 'Funded pool villa', 0, TRUE),
    (v_asset7, '/images/villa1_2.webp', 'Pool area', 1, FALSE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset8, '/images/villa2_1.webp', 'Beachfront retreat', 0, TRUE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_com1, '/static/images/commodities/rice/eduardo-prim-3u51-uLQICc-unsplash.webp', 'Bali rice terraces', 0, TRUE);
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_com2, '/static/images/commodities/rice/hoach-le-dinh-PeRt3uMmjYM-unsplash.webp', 'Cacao beans', 0, TRUE);
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_com3, '/static/images/commodities/rice/winston-chen-kXoEdaZ3SFw-unsplash.webp', 'Coffee beans', 0, TRUE);


-- ── 4. Asset Milestones ──────────────────────────────────────

INSERT INTO asset_milestones (asset_id, title, description, milestone_date, month_index, is_completed) VALUES
    (v_asset1, 'Funding Opened', 'Funding campaign launched on POOOL Platform', NOW() - INTERVAL '60 days', 0, TRUE),
    (v_asset1, '50% Funded', 'Reached 50% of funding target', NOW() - INTERVAL '30 days', 1, TRUE),
    (v_asset1, '89% Funded', 'Nearing full funding!', NOW() - INTERVAL '5 days', 2, TRUE),
    (v_asset1, 'Fully Funded', 'Target: 100% funded and property acquired', NOW() + INTERVAL '30 days', 3, FALSE),
    (v_asset1, 'First Rental Income', 'Expected first rental payout to investors', NOW() + INTERVAL '90 days', 4, FALSE);

-- ── 5. Asset Documents ───────────────────────────────────────

INSERT INTO asset_documents (asset_id, document_type, title, file_url, file_size_bytes) VALUES
    (v_asset1, 'expose', 'Investment Expose – Clifftop Villa', '/docs/expose-clifftop-villa.pdf', 2456780),
    (v_asset1, 'appraisal', 'Independent Appraisal Report', '/docs/appraisal-clifftop-villa.pdf', 1234567),
    (v_asset2, 'expose', 'Investment Expose – Surf Villa Canggu', '/docs/expose-surf-villa.pdf', 1987654);

-- ── 6. Asset Financials ──────────────────────────────────────

INSERT INTO asset_financials (asset_id, period_month, period_year, rental_income_cents, expenses_cents, net_income_cents, occupancy_rate_bps) VALUES
    (v_asset1, 1, 2026, 1200000, 285000, 915000, 9200),
    (v_asset1, 2, 2026, 1150000, 275000, 875000, 8800),
    (v_asset1, 3, 2026, 1350000, 295000, 1055000, 9500),
    (v_asset2, 1, 2026, 980000, 232000, 748000, 8700),
    (v_asset2, 2, 2026, 1020000, 240000, 780000, 9000);

-- ── 7. Tiers (if not already seeded) ─────────────────────────

INSERT INTO tiers (name, min_invest, cashback_pct, badge_color, sort_order)
SELECT 'Bronze', 0, 0.00, '#CD7F32', 1
WHERE NOT EXISTS (SELECT 1 FROM tiers WHERE name = 'Bronze');

INSERT INTO tiers (name, min_invest, cashback_pct, badge_color, sort_order)
SELECT 'Silver', 100000, 1.50, '#C0C0C0', 2
WHERE NOT EXISTS (SELECT 1 FROM tiers WHERE name = 'Silver');

INSERT INTO tiers (name, min_invest, cashback_pct, badge_color, sort_order)
SELECT 'Gold', 500000, 2.00, '#FFD700', 3
WHERE NOT EXISTS (SELECT 1 FROM tiers WHERE name = 'Gold');

INSERT INTO tiers (name, min_invest, cashback_pct, badge_color, sort_order)
SELECT 'Platinum', 2000000, 3.00, '#E5E4E2', 4
WHERE NOT EXISTS (SELECT 1 FROM tiers WHERE name = 'Platinum');

-- ── 8. Roles (if not already seeded) ─────────────────────────

INSERT INTO roles (name) SELECT 'investor' WHERE NOT EXISTS (SELECT 1 FROM roles WHERE name = 'investor');
INSERT INTO roles (name) SELECT 'developer' WHERE NOT EXISTS (SELECT 1 FROM roles WHERE name = 'developer');
INSERT INTO roles (name) SELECT 'admin' WHERE NOT EXISTS (SELECT 1 FROM roles WHERE name = 'admin');

-- Assign admin role to jonas
INSERT INTO user_roles (user_id, role_id)
SELECT v_user_id, r.id FROM roles r WHERE r.name = 'admin'
AND NOT EXISTS (SELECT 1 FROM user_roles ur WHERE ur.user_id = v_user_id AND ur.role_id = r.id);

INSERT INTO user_roles (user_id, role_id)
SELECT v_user_id, r.id FROM roles r WHERE r.name = 'investor'
AND NOT EXISTS (SELECT 1 FROM user_roles ur WHERE ur.user_id = v_user_id AND ur.role_id = r.id);

-- ── 9. Rewards balance ───────────────────────────────────────

INSERT INTO rewards_balances (user_id, cashback, referrals, promotions)
SELECT v_user_id, 5000, 0, 0
WHERE NOT EXISTS (SELECT 1 FROM rewards_balances WHERE user_id = v_user_id);

-- ── 10. Referral code ────────────────────────────────────────

INSERT INTO referral_codes (user_id, code)
SELECT v_user_id, 'JONAS2026'
WHERE NOT EXISTS (SELECT 1 FROM referral_codes WHERE user_id = v_user_id);

-- ── 11. Investment limits ────────────────────────────────────

INSERT INTO investment_limits (user_id, annual_limit_cents, invested_12m_cents, limit_year)
SELECT v_user_id, 50000000, 0, 2026
WHERE NOT EXISTS (SELECT 1 FROM investment_limits WHERE user_id = v_user_id);


RAISE NOTICE '──────────────────────────────────────────';
RAISE NOTICE 'Production seed data inserted successfully! ✓';
RAISE NOTICE '  Assets:       11 (8 real estate + 3 commodities)';
RAISE NOTICE '  Images:       16';
RAISE NOTICE '  Milestones:   5';
RAISE NOTICE '  Documents:    3';
RAISE NOTICE '  Financials:   5';
RAISE NOTICE '  Tiers:        4';
RAISE NOTICE '  Roles:        3';
RAISE NOTICE '──────────────────────────────────────────';

END $$;

COMMIT;
