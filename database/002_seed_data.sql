-- ============================================================
-- POOOL Platform – Seed Data
-- ============================================================
-- Populates all empty tables with realistic test data.
-- Run: psql -d poool -f database/002_seed_data.sql
--
-- Prerequisites: 001_initial_schema.sql must be applied.
--                At least one user (test@poool.app) must exist.
-- ============================================================

BEGIN;

-- ── Get current user ID ─────────────────────────────────────
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
SELECT id INTO v_user_id FROM users WHERE email = 'test@poool.app' LIMIT 1;

IF v_user_id IS NULL THEN
    RAISE EXCEPTION 'No user found with email test@poool.app';
END IF;

-- Get wallet IDs
SELECT id INTO v_cash_wallet_id FROM wallets WHERE user_id = v_user_id AND wallet_type = 'cash';
SELECT id INTO v_rewards_wallet_id FROM wallets WHERE user_id = v_user_id AND wallet_type = 'rewards';

-- ── 1. Complete user profile ────────────────────────────────
UPDATE user_profiles SET
    first_name = 'Martin',
    last_name = 'Weber',
    phone_number = '+49 170 1234567',
    date_of_birth = '1990-05-15',
    country = 'DE'
WHERE user_id = v_user_id;

RAISE NOTICE 'User profile updated ✓';

-- ── 2. Insert Real Estate Assets ─────────────────────────────

-- Property 1: Luxury Clifftop Villa
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, lease_term_years, land_size_sqm, building_size_sqm,
    bedrooms, bathrooms, construction_status, year_built,
    location_city, location_country, location_address,
    location_lat, location_lng, location_description,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'Luxury Clifftop Villa with Ocean Views in Uluwatu',
    'luxury-clifftop-villa-uluwatu',
    'Stunning clifftop villa with infinity pool and panoramic Indian Ocean views.',
    'This exceptional 4-bedroom villa sits atop the dramatic limestone cliffs of Uluwatu, offering 180-degree views of the Indian Ocean. Features include an infinity edge pool, open-air living pavilion, professional kitchen, and private access to a secluded beach. The property generates consistent rental income through premium short-term vacation bookings.',
    'real_estate',
    'villa', 'Uluwatu', 'leasehold', 25, 800.00, 450.00,
    4, 5, 'ready', 2022,
    'Bali', 'ID', 'Jl. Pantai Suluban, Uluwatu, Pecatu, Bali 80364',
    -8.8113, 115.0887, 'Perched on the clifftops of Uluwatu with stunning ocean views, 15 minutes from Padang Padang Beach.',
    133400000, 13340, 10000, 1100,
    1050, 800, 8900,
    'funding_in_progress', TRUE, TRUE
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
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'Modern Surf Villa near Echo Beach in Canggu',
    'modern-surf-villa-canggu',
    'Contemporary villa steps from Echo Beach, Canggu''s most popular surf break.',
    'Modern 4-bedroom freehold villa located just 200m from Echo Beach in the heart of Canggu. Features open-concept living, a 12m pool, rooftop terrace with sunset views, and a dedicated surf board storage. Positioned in Bali''s fastest-growing tourist area with year-round high occupancy rates.',
    'real_estate',
    'villa', 'Canggu', 'freehold', 600.00, 380.00,
    4, 4, 'ready', 2023,
    'Bali', 'ID', 'Jl. Nelayan, Echo Beach, Canggu, Bali 80361',
    -8.6509, 115.1300,
    115000000, 11500, 10000, 2400,
    1200, 900, 8500,
    'funding_in_progress', TRUE, TRUE
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
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'Boutique Resort with 6 Villas in Central Ubud',
    'boutique-resort-ubud',
    'Luxury boutique resort with 6 private villas surrounded by rice terraces.',
    'A fully operational boutique resort featuring 6 individually designed villas, each with private pool, set amongst lush tropical gardens and rice paddies. Central Ubud location provides easy access to cultural attractions, restaurants, and yoga studios. Strong year-round bookings with premium nightly rates.',
    'real_estate',
    'commercial', 'Ubud', 'leasehold', 30, 2500.00, 1200.00,
    12, 14, 'ready', 2020,
    'Bali', 'ID', 'Jl. Kajeng, Ubud, Gianyar, Bali 80571',
    -8.5069, 115.2625,
    285000000, 28500, 10000, 3600,
    1400, 600, 7800,
    'funding_in_progress', FALSE, TRUE
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
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'Vacation Rental Villa with Temple Views',
    'vacation-rental-villa-uluwatu',
    'Charming 3-bedroom villa with views of the iconic Uluwatu Temple.',
    'A beautifully designed 3-bedroom villa offering stunning views of Uluwatu Temple and the ocean. Features traditional Balinese architecture with modern amenities, a private pool, outdoor dining area, and tropical garden. Generates excellent short-term rental income with high occupancy during peak season.',
    'real_estate',
    'villa', 'Uluwatu', 'leasehold', 20, 500.00, 280.00,
    3, 3, 'ready', 2021,
    'Bali', 'ID', 'Jl. Pura Uluwatu, Pecatu, Kuta, Bali 80361',
    -8.8295, 115.0849,
    78500000, 7850, 10000, 800,
    1300, 700, 9200,
    'funding_in_progress', FALSE, TRUE
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
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'Renovation Flip Project – Canggu Villa',
    'renovation-flip-canggu',
    'Value-add renovation opportunity to flip a dated villa in prime Canggu location.',
    'Short-term investment opportunity: Acquire and renovate a dated 3-bedroom villa on a prime 400sqm plot in central Canggu. Plans include full interior renovation, new pool, and landscaping. Target flip within 12-18 months for significant capital appreciation. Professional project management team in place.',
    'real_estate',
    'villa', 'Canggu', 'leasehold', 20, 400.00, 220.00,
    3, 3, 'renovation',
    'Bali', 'ID', 'Jl. Batu Bolong, Canggu, Bali 80361',
    -8.6483, 115.1345,
    45000000, 4500, 10000, 5500,
    0, 2500,
    'funding_open', FALSE, TRUE
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
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'New Development Project – 4 Villa Complex',
    'new-development-seminyak',
    'Ground-up development of 4 luxury villas in the heart of Seminyak.',
    'An exciting development project to build 4 luxury 2-bedroom villas on a premium 1200sqm plot in central Seminyak. Close to beach, restaurants, and nightlife. Units will be sold individually or operated as vacation rentals upon completion. Professional developer with proven track record.',
    'real_estate',
    'villa', 'Seminyak', 'leasehold', 25, 1200.00,
    8, 8, 'construction',
    'Bali', 'ID', 'Jl. Kayu Aya, Seminyak, Bali 80361',
    -8.6815, 115.1580,
    180000000, 18000, 10000, 7000,
    850, 1200,
    'funding_open', TRUE, TRUE
) RETURNING id INTO v_asset6;

-- Property 7: Funded property (for "Funded" tab)
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, lease_term_years, land_size_sqm, building_size_sqm,
    bedrooms, bathrooms, construction_status, year_built,
    location_city, location_country, location_address,
    location_lat, location_lng,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'Luxury Pool Villa – Fully Funded',
    'luxury-pool-villa-canggu-funded',
    'This villa has been fully funded and is now generating rental income.',
    'A stunning 3-bedroom pool villa in Berawa, Canggu. Currently fully occupied and generating strong monthly returns for investors. Managed by a professional villa management company with transparent monthly reporting.',
    'real_estate',
    'villa', 'Canggu', 'leasehold', 25, 500.00, 300.00,
    3, 4, 'ready', 2023,
    'Bali', 'ID', 'Jl. Pantai Berawa, Canggu, Bali 80361',
    -8.6538, 115.1400,
    95000000, 9500, 10000, 0,
    1100, 700, 9000,
    'funded', FALSE, TRUE
) RETURNING id INTO v_asset7;

-- Property 8: Exited property (for "Exited" tab)
INSERT INTO assets (
    id, title, slug, short_description, description, asset_type,
    property_type, area, lease_type, lease_term_years, land_size_sqm, building_size_sqm,
    bedrooms, bathrooms, construction_status, year_built,
    location_city, location_country, location_address,
    location_lat, location_lng,
    total_value_cents, token_price_cents, tokens_total, tokens_available,
    annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
    funding_status, featured, published
) VALUES (
    gen_random_uuid(),
    'Beachfront Retreat – Successfully Exited',
    'beachfront-retreat-sanur-exited',
    'This investment has successfully exited with a 42% total return to investors.',
    'A beautiful 2-bedroom beachfront retreat in Sanur that was acquired, renovated, and successfully sold after 18 months. All investors received their principal plus 42% total return. This property demonstrates the profit potential of Bali real estate investments.',
    'real_estate',
    'villa', 'Sanur', 'freehold', NULL, 350.00, 180.00,
    2, 2, 'ready', 2019,
    'Bali', 'ID', 'Jl. Pantai Karang, Sanur, Bali 80228',
    -8.6903, 115.2619,
    65000000, 6500, 10000, 0,
    0, 4200, 0,
    'exited', FALSE, TRUE
) RETURNING id INTO v_asset8;

-- ── 3. Insert Commodity Assets ───────────────────────────────

-- Commodity 1: Premium Bali Rice
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
    'Partner with established rice farmers in Tabanan to fund the next harvest cycle of premium organic Bali rice. The rice is sold to luxury hotels and exported to international markets. Fixed ROI with profit sharing above target.',
    'commodity',
    'PT Bali Rice Co.', 6, 1200,
    15000000, 22000000, 8000000,
    7000000, 14000000,
    4000000, 60, 10,
    'Bali', 'ID',
    5000000, 5000, 1000, 350,
    'funding_in_progress', TRUE, TRUE
) RETURNING id INTO v_com1;

-- Commodity 2: Organic Cacao
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
    'Support Bali''s growing artisan chocolate industry by investing in organic cacao bean production. Beans are fermented and dried on-site, then sold to premium chocolate makers worldwide. Strong commodity demand with increasing prices.',
    'commodity',
    'Bali Cacao Collective', 12, 1500,
    20000000, 30000000, 10000000,
    10000000, 20000000,
    6000000, 55, 10,
    'Bali', 'ID',
    8000000, 8000, 1000, 600,
    'funding_open', FALSE, TRUE
) RETURNING id INTO v_com2;

-- Commodity 3: Coffee (funded)
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
    'A fully funded investment in specialty-grade Arabica coffee grown on the volcanic slopes of Mount Batur. Beans are processed using the wet-hull method and sold to specialty roasters in Australia and Japan.',
    'commodity',
    'Kintamani Coffee Farmers', 9, 1100,
    'Bali', 'ID',
    3000000, 3000, 1000, 0,
    'funded', FALSE, TRUE
) RETURNING id INTO v_com3;


-- ── 4. Insert Asset Images ───────────────────────────────────

-- Property 1 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset1, '/images/villa1.jpg', 'Luxury Clifftop Villa exterior', 0, TRUE),
    (v_asset1, '/images/villa1_2.jpg', 'Infinity pool with ocean view', 1, FALSE),
    (v_asset1, '/images/villa1_3.jpg', 'Modern interior living area', 2, FALSE),
    (v_asset1, '/images/villa1_4.jpg', 'Ocean view terrace', 3, FALSE);

-- Property 2 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset2, '/images/villa2_1.jpg', 'Modern Surf Villa exterior', 0, TRUE),
    (v_asset2, '/images/villa2_2.jpg', 'Tropical garden and pool', 1, FALSE);

-- Property 3 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset3, '/images/villa3_1.jpg', 'Boutique Resort entrance', 0, TRUE),
    (v_asset3, '/images/villa3_2.jpg', 'Resort common area', 1, FALSE);

-- Property 4 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset4, '/images/villa4_1.jpg', 'Vacation Rental Villa', 0, TRUE),
    (v_asset4, '/images/villa4_2.jpg', 'Panoramic temple view', 1, FALSE);

-- Property 5 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset5, '/images/villa5.jpg', 'Renovation flip project', 0, TRUE);

-- Property 6 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset6, '/images/villa6.jpg', 'New development site', 0, TRUE);

-- Property 7 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset7, '/images/villa3_1.jpg', 'Funded pool villa', 0, TRUE),
    (v_asset7, '/images/villa1_2.jpg', 'Pool area', 1, FALSE);

-- Property 8 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset8, '/images/villa2_1.jpg', 'Beachfront retreat', 0, TRUE);

-- Commodity images (rice) – all 5 gallery images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_com1, '/static/images/commodities/rice/eduardo-prim-3u51-uLQICc-unsplash.webp', 'Bali rice terraces', 0, TRUE),
    (v_com1, '/static/images/commodities/rice/hoach-le-dinh-PeRt3uMmjYM-unsplash.webp', 'Paddy Rice plantation', 1, FALSE),
    (v_com1, '/static/images/commodities/rice/vrlibs-studio-h0cvg3O-LN0-unsplash.webp', 'Rice farming', 2, FALSE),
    (v_com1, '/static/images/commodities/rice/winston-chen-kXoEdaZ3SFw-unsplash.webp', 'Rice harvest', 3, FALSE),
    (v_com1, '/static/images/commodities/rice/zhao-yangjun-dDAzpSUAbgI-unsplash.webp', 'Rice field landscape', 4, FALSE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_com2, '/static/images/commodities/rice/hoach-le-dinh-PeRt3uMmjYM-unsplash.jpg', 'Cacao beans', 0, TRUE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_com3, '/static/images/commodities/rice/winston-chen-kXoEdaZ3SFw-unsplash.jpg', 'Coffee beans', 0, TRUE);


-- ── 5. Insert Investments (user owns tokens in some assets) ──

INSERT INTO investments (user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, total_rental_cents, status) VALUES
    (v_user_id, v_asset1, 50, 667000, 700000, 45000, 'active'),
    (v_user_id, v_asset2, 30, 345000, 365000, 22000, 'active'),
    (v_user_id, v_asset7, 100, 950000, 990000, 91500, 'active'),
    (v_user_id, v_asset8, 75, 487500, 692250, 0, 'exited');

-- Commodity investments
INSERT INTO investments (user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents, status) VALUES
    (v_user_id, v_com1, 10, 50000, 50000, 'active'),
    (v_user_id, v_com3, 20, 60000, 66600, 'active');


-- ── 6. Insert Cart Items ─────────────────────────────────────

INSERT INTO cart_items (user_id, asset_id, tokens_quantity, token_price_cents) VALUES
    (v_user_id, v_asset2, 5, 11500),
    (v_user_id, v_asset5, 10, 4500);


-- ── 7. Insert Asset Milestones ───────────────────────────────

INSERT INTO asset_milestones (asset_id, title, description, milestone_date, month_index, is_completed) VALUES
    (v_asset1, 'Funding Opened', 'Funding campaign launched on POOOL Platform', NOW() - INTERVAL '60 days', 0, TRUE),
    (v_asset1, '50% Funded', 'Reached 50% of funding target', NOW() - INTERVAL '30 days', 1, TRUE),
    (v_asset1, '89% Funded', 'Nearing full funding!', NOW() - INTERVAL '5 days', 2, TRUE),
    (v_asset1, 'Fully Funded', 'Target: 100% funded and property acquired', NOW() + INTERVAL '30 days', 3, FALSE),
    (v_asset1, 'First Rental Income', 'Expected first rental payout to investors', NOW() + INTERVAL '90 days', 4, FALSE);

INSERT INTO asset_milestones (asset_id, title, description, milestone_date, month_index, is_completed) VALUES
    (v_asset5, 'Funding Opened', 'Renovation project funding launched', NOW() - INTERVAL '10 days', 0, TRUE),
    (v_asset5, 'Property Acquired', 'Purchase of villa completed', NULL, 1, FALSE),
    (v_asset5, 'Renovation Start', 'Construction work begins', NULL, 2, FALSE),
    (v_asset5, 'Renovation Complete', 'All work finished, final inspection', NULL, 5, FALSE),
    (v_asset5, 'Exit / Sale', 'Property listed for sale', NULL, 8, FALSE);


-- ── 8. Insert Asset Documents ────────────────────────────────

INSERT INTO asset_documents (asset_id, document_type, title, file_url, file_size_bytes) VALUES
    (v_asset1, 'expose', 'Investment Expose – Clifftop Villa', '/docs/expose-clifftop-villa.pdf', 2456780),
    (v_asset1, 'appraisal', 'Independent Appraisal Report', '/docs/appraisal-clifftop-villa.pdf', 1234567),
    (v_asset1, 'proof_of_title', 'Certificate of Leasehold', '/docs/title-clifftop-villa.pdf', 345678),

    (v_asset2, 'expose', 'Investment Expose – Surf Villa Canggu', '/docs/expose-surf-villa.pdf', 1987654),
    (v_asset2, 'financial', 'Financial Projections 5-Year', '/docs/financials-surf-villa.pdf', 567890),

    (v_asset5, 'expose', 'Renovation Project Plan', '/docs/expose-renovation-flip.pdf', 3456789),
    (v_asset5, 'floor_plan', 'Proposed Floor Plans', '/docs/floorplan-renovation.pdf', 987654);


-- ── 9. Insert Asset Financials ───────────────────────────────

INSERT INTO asset_financials (asset_id, period_month, period_year, rental_income_cents, expenses_cents, net_income_cents, occupancy_rate_bps) VALUES
    (v_asset1, 1, 2026, 1200000, 285000, 915000, 9200),
    (v_asset1, 2, 2026, 1150000, 275000, 875000, 8800),
    (v_asset1, 3, 2026, 1350000, 295000, 1055000, 9500),

    (v_asset2, 1, 2026, 980000, 232000, 748000, 8700),
    (v_asset2, 2, 2026, 1020000, 240000, 780000, 9000),
    (v_asset2, 3, 2026, 950000, 225000, 725000, 8500);


-- ── 10. Insert Orders (completed purchases) ──────────────────

INSERT INTO orders (id, user_id, order_number, total_cents, status, payment_method, completed_at) VALUES
    (gen_random_uuid(), v_user_id, 'ORD-2026-0001', 667000, 'completed', 'wallet', NOW() - INTERVAL '45 days'),
    (gen_random_uuid(), v_user_id, 'ORD-2026-0002', 345000, 'completed', 'wallet', NOW() - INTERVAL '30 days'),
    (gen_random_uuid(), v_user_id, 'ORD-2026-0003', 950000, 'completed', 'wallet', NOW() - INTERVAL '15 days');


-- ── 11. Insert Notifications ─────────────────────────────────

INSERT INTO notifications (user_id, type, title, message, is_read) VALUES
    (v_user_id, 'investment', 'Investment Confirmed', 'Your investment of $6,670 in Luxury Clifftop Villa has been confirmed.', TRUE),
    (v_user_id, 'investment', 'Investment Confirmed', 'Your investment of $3,450 in Modern Surf Villa has been confirmed.', TRUE),
    (v_user_id, 'system', 'Welcome to POOOL!', 'Welcome aboard! Start exploring investment opportunities on the marketplace.', TRUE),
    (v_user_id, 'payout', 'Monthly Payout', 'You received a $45.75 dividend payout from Luxury Pool Villa.', FALSE),
    (v_user_id, 'system', 'KYC Required', 'Please complete your identity verification to unlock all features.', FALSE),
    (v_user_id, 'investment', 'Property Funded!', 'Luxury Pool Villa has been fully funded. Congratulations!', TRUE),
    (v_user_id, 'system', 'New Property Listed', 'A new investment opportunity "Renovation Flip Project" is now available.', FALSE);


-- ── 12. Insert KYC Record ────────────────────────────────────

INSERT INTO kyc_records (user_id, provider, status, document_type, pep_check_passed, sanctions_check) VALUES
    (v_user_id, 'sumsub', 'pending', 'passport', NULL, NULL);


-- ── 13. Insert Dividend Payouts ──────────────────────────────

INSERT INTO dividend_payouts (investment_id, user_id, asset_id, amount_cents, payout_type, status, paid_at)
SELECT
    inv.id,
    v_user_id,
    v_asset7,
    4575,
    'rental',
    'paid',
    NOW()
FROM investments inv
WHERE inv.user_id = v_user_id AND inv.asset_id = v_asset7
LIMIT 1;


-- ── 14. Insert Investment Limits ─────────────────────────────

INSERT INTO investment_limits (user_id, annual_limit_cents, invested_12m_cents, limit_year) VALUES
    (v_user_id, 50000000, 2559500, 2026);


-- ── 15. Insert Support Ticket ────────────────────────────────

INSERT INTO support_tickets (user_id, subject, message, status, priority) VALUES
    (v_user_id, 'How do I withdraw my dividends?', 'Hi, I received a dividend payout notification but I''m not sure how to withdraw the funds to my bank account. Can you help?', 'open', 'normal');


-- ── 16. Add audit log entries ────────────────────────────────

INSERT INTO audit_logs (actor_user_id, action, entity_type, metadata) VALUES
    (v_user_id, 'investment_created', 'investment', '{"asset": "Luxury Clifftop Villa", "tokens": 50, "amount_usd": 6670}'),
    (v_user_id, 'investment_created', 'investment', '{"asset": "Modern Surf Villa", "tokens": 30, "amount_usd": 3450}'),
    (v_user_id, 'investment_created', 'investment', '{"asset": "Luxury Pool Villa", "tokens": 100, "amount_usd": 9500}'),
    (v_user_id, 'cart_add', 'cart', '{"asset": "Modern Surf Villa", "tokens": 5}'),
    (v_user_id, 'profile_updated', 'user_profile', '{"fields": ["first_name", "last_name", "phone_number", "country"]}');

RAISE NOTICE '──────────────────────────────────────────';
RAISE NOTICE 'Seed data inserted successfully! ✓';
RAISE NOTICE '  Assets:       11 (8 real estate + 3 commodities)';
RAISE NOTICE '  Images:       18';
RAISE NOTICE '  Investments:  6';
RAISE NOTICE '  Cart items:   2';
RAISE NOTICE '  Milestones:   10';
RAISE NOTICE '  Documents:    7';
RAISE NOTICE '  Financials:   10';
RAISE NOTICE '  Orders:       3';
RAISE NOTICE '  Notifications: 7';
RAISE NOTICE '  KYC record:   1';
RAISE NOTICE '  Dividends:    1';
RAISE NOTICE '  Support tix:  1';
RAISE NOTICE '──────────────────────────────────────────';

END $$;

COMMIT;
