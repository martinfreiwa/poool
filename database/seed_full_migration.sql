-- ============================================================
-- POOOL Platform – Initial Database Schema
-- Migration 001: Complete Schema Setup
-- Generated from DATABASE_SCHEMA.md
-- ============================================================

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================
-- 1. users
-- ============================================================
CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           VARCHAR(255) NOT NULL UNIQUE,
    password_hash   VARCHAR(255),
    email_verified  BOOLEAN NOT NULL DEFAULT FALSE,
    avatar_url      VARCHAR(512),
    status          VARCHAR(20) NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'suspended', 'deleted', 'frozen')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_users_email ON users(email);

-- ============================================================
-- 2. user_profiles
-- ============================================================
CREATE TABLE user_profiles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    first_name      VARCHAR(100),
    last_name       VARCHAR(100),
    display_name    VARCHAR(200),
    date_of_birth   DATE,
    nationality     VARCHAR(3),
    address_line_1  VARCHAR(255),
    address_line_2  VARCHAR(255),
    city            VARCHAR(100),
    state_province  VARCHAR(100),
    postal_code     VARCHAR(20),
    country         VARCHAR(3),
    phone_number    VARCHAR(30),
    tax_id          VARCHAR(50),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- 3. user_sessions
-- ============================================================
CREATE TABLE user_sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token   VARCHAR(255) NOT NULL UNIQUE,
    ip_address      INET,
    user_agent      TEXT,
    remember_me     BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_sessions_token ON user_sessions(session_token);
CREATE INDEX idx_sessions_user ON user_sessions(user_id);

-- ============================================================
-- 4. oauth_accounts
-- ============================================================
CREATE TABLE oauth_accounts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider        VARCHAR(20) NOT NULL
                    CHECK (provider IN ('google', 'facebook', 'apple')),
    provider_id     VARCHAR(255) NOT NULL,
    provider_email  VARCHAR(255),
    access_token    TEXT,
    refresh_token   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (provider, provider_id)
);
CREATE INDEX idx_oauth_user ON oauth_accounts(user_id);

-- ============================================================
-- 5. kyc_records
-- ============================================================
CREATE TABLE kyc_records (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider            VARCHAR(50) NOT NULL DEFAULT 'sumsub',
    provider_ref_id     VARCHAR(255),
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'in_review', 'approved', 'rejected', 'expired')),
    rejection_reason    TEXT,
    document_type       VARCHAR(50),
    pep_check_passed    BOOLEAN,
    sanctions_check     BOOLEAN,
    verified_at         TIMESTAMPTZ,
    expires_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_kyc_user ON kyc_records(user_id);
CREATE INDEX idx_kyc_status ON kyc_records(status);

-- ============================================================
-- 6. wallets
-- ============================================================
CREATE TABLE wallets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    wallet_type     VARCHAR(20) NOT NULL
                    CHECK (wallet_type IN ('cash', 'rewards')),
    balance_cents   BIGINT NOT NULL DEFAULT 0
                    CHECK (balance_cents >= 0),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, wallet_type)
);
CREATE INDEX idx_wallets_user ON wallets(user_id);

-- ============================================================
-- 7. wallet_transactions
-- ============================================================
CREATE TABLE wallet_transactions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id           UUID NOT NULL REFERENCES wallets(id),
    type                VARCHAR(30) NOT NULL
                        CHECK (type IN (
                            'deposit', 'withdrawal', 'purchase',
                            'sale', 'dividend', 'reward', 'refund', 'fee'
                        )),
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled')),
    amount_cents        BIGINT NOT NULL,
    description         TEXT,
    external_ref_id     VARCHAR(255),
    related_order_id    UUID,
    metadata            JSONB,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at        TIMESTAMPTZ
);
CREATE INDEX idx_wtx_wallet ON wallet_transactions(wallet_id);
CREATE INDEX idx_wtx_type ON wallet_transactions(type);
CREATE INDEX idx_wtx_status ON wallet_transactions(status);
CREATE INDEX idx_wtx_created ON wallet_transactions(created_at DESC);

-- ============================================================
-- 8. assets
-- ============================================================
CREATE TABLE assets (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    developer_user_id   UUID REFERENCES users(id),
    title               VARCHAR(255) NOT NULL,
    slug                VARCHAR(255) NOT NULL UNIQUE,
    short_description   VARCHAR(500),
    description         TEXT,
    asset_type          VARCHAR(30) NOT NULL
                        CHECK (asset_type IN (
                            'real_estate', 'commercial_property', 'commodity',
                            'business', 'startup', 'land_plot'
                        )),

    -- Property-specific details
    property_type       VARCHAR(50),
    area                VARCHAR(100),
    lease_type          VARCHAR(30)
                        CHECK (lease_type IN ('leasehold', 'freehold') OR lease_type IS NULL),
    lease_term_years    INTEGER,
    land_size_sqm       DECIMAL(10, 2),
    building_size_sqm   DECIMAL(10, 2),
    bedrooms            INTEGER,
    bathrooms           INTEGER,
    construction_status VARCHAR(50)
                        CHECK (construction_status IN ('ready', 'construction', 'renovation') OR construction_status IS NULL),
    year_built          INTEGER,

    -- Location & Media
    location_city       VARCHAR(100),
    location_country    VARCHAR(3),
    location_address    VARCHAR(255),
    location_lat        DECIMAL(10, 7),
    location_lng        DECIMAL(10, 7),
    location_description TEXT,
    google_maps_url     VARCHAR(512),
    video_url           VARCHAR(512),

    -- Financial data
    total_value_cents   BIGINT NOT NULL,
    token_price_cents   BIGINT NOT NULL,
    tokens_total        INTEGER NOT NULL,
    tokens_available    INTEGER NOT NULL,

    -- Yield (Properties)
    annual_yield_bps    INTEGER,
    capital_appreciation_bps INTEGER,
    occupancy_rate_bps  INTEGER,

    -- Commodity specific
    operator_name       VARCHAR(255),
    term_months         INTEGER,
    fixed_roi_bps       INTEGER,
    revenue_min_cents   BIGINT,
    revenue_max_cents   BIGINT,
    expenses_cents      BIGINT,
    net_profit_min_cents BIGINT,
    net_profit_max_cents BIGINT,
    investor_payout_cents BIGINT,
    operator_split_pct  INTEGER,
    poool_split_pct     INTEGER,

    -- Status
    funding_status      VARCHAR(30) NOT NULL DEFAULT 'upcoming'
                        CHECK (funding_status IN (
                            'upcoming', 'funding_open', 'funding_in_progress',
                            'funded', 'rented', 'payout_pending', 'exited'
                        )),
    featured            BOOLEAN NOT NULL DEFAULT FALSE,
    published           BOOLEAN NOT NULL DEFAULT FALSE,

    -- Timestamps
    funding_start_at    TIMESTAMPTZ,
    funding_end_at      TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_assets_type ON assets(asset_type);
CREATE INDEX idx_assets_status ON assets(funding_status);
CREATE INDEX idx_assets_developer ON assets(developer_user_id);
CREATE INDEX idx_assets_slug ON assets(slug);

-- ============================================================
-- 9. asset_images
-- ============================================================
CREATE TABLE asset_images (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    image_url       VARCHAR(512) NOT NULL,
    alt_text        VARCHAR(255),
    sort_order      INTEGER NOT NULL DEFAULT 0,
    is_cover        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_asset_images_asset ON asset_images(asset_id);

-- ============================================================
-- 10. asset_milestones
-- ============================================================
CREATE TABLE asset_milestones (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    title           VARCHAR(255) NOT NULL,
    description     TEXT,
    milestone_date  TIMESTAMPTZ,
    month_index     INTEGER,
    is_completed    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_asset_milestones_asset ON asset_milestones(asset_id);

-- ============================================================
-- 11. asset_documents
-- ============================================================
CREATE TABLE asset_documents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    document_type   VARCHAR(50) NOT NULL
                    CHECK (document_type IN (
                        'proof_of_title', 'legal_basis', 'building_permit',
                        'site_plan', 'tax_npwp', 'tax_pbb', 'tax_bphtb',
                        'license_nib', 'id_card', 'owner_npwp',
                        'expose', 'appraisal', 'financial', 'floor_plan', 'other'
                    )),
    title           VARCHAR(255) NOT NULL,
    file_url        VARCHAR(512) NOT NULL,
    file_size_bytes BIGINT,
    is_investor_visible BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_asset_docs_asset ON asset_documents(asset_id);
CREATE INDEX idx_asset_docs_asset_investor_visible
  ON asset_documents(asset_id, is_investor_visible)
  WHERE is_investor_visible = TRUE;

-- ============================================================
-- 12. asset_financials
-- ============================================================
CREATE TABLE asset_financials (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id                    UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    period_month                INTEGER NOT NULL,
    period_year                 INTEGER NOT NULL,
    rental_income_cents         BIGINT DEFAULT 0,
    appreciation_cents          BIGINT DEFAULT 0,
    occupancy_rate_bps          INTEGER,
    expenses_cents              BIGINT DEFAULT 0,
    net_income_cents            BIGINT DEFAULT 0,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (asset_id, period_month, period_year)
);
CREATE INDEX idx_asset_fin_asset ON asset_financials(asset_id);

-- ============================================================
-- 13. investments
-- ============================================================
CREATE TABLE investments (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                 UUID NOT NULL REFERENCES users(id),
    asset_id                UUID NOT NULL REFERENCES assets(id),
    tokens_owned            INTEGER NOT NULL DEFAULT 0
                            CHECK (tokens_owned > 0),
    purchase_value_cents    BIGINT NOT NULL,
    current_value_cents     BIGINT NOT NULL,
    total_rental_cents      BIGINT NOT NULL DEFAULT 0,
    appreciation_pct_bps    INTEGER DEFAULT 0,
    status                  VARCHAR(30) NOT NULL DEFAULT 'active'
                            CHECK (status IN (
                                'active', 'funded', 'rented', 'payout_pending',
                                'in_process', 'funding_in_progress', 'exited'
                            )),
    payout_expected_at      TIMESTAMPTZ,
    purchased_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, asset_id)
);
CREATE INDEX idx_investments_user ON investments(user_id);
CREATE INDEX idx_investments_asset ON investments(asset_id);

-- ============================================================
-- 14. cart_items
-- ============================================================
CREATE TABLE cart_items (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    asset_id        UUID NOT NULL REFERENCES assets(id),
    tokens_quantity INTEGER NOT NULL DEFAULT 1
                    CHECK (tokens_quantity > 0),
    token_price_cents BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, asset_id)
);
CREATE INDEX idx_cart_user ON cart_items(user_id);

-- ============================================================
-- 15. orders
-- ============================================================
CREATE TABLE orders (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id),
    order_number        VARCHAR(30) NOT NULL UNIQUE,
    total_cents         BIGINT NOT NULL,
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled', 'refunded')),
    payment_method      VARCHAR(30),
    payment_ref_id      VARCHAR(255),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at        TIMESTAMPTZ
);
CREATE INDEX idx_orders_user ON orders(user_id);
CREATE INDEX idx_orders_status ON orders(status);

-- ============================================================
-- 16. order_items
-- ============================================================
CREATE TABLE order_items (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id            UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    asset_id            UUID NOT NULL REFERENCES assets(id),
    tokens_quantity     INTEGER NOT NULL,
    token_price_cents   BIGINT NOT NULL,
    subtotal_cents      BIGINT NOT NULL
);
CREATE INDEX idx_order_items_order ON order_items(order_id);

-- ============================================================
-- 17. dividend_payouts
-- ============================================================
CREATE TABLE dividend_payouts (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    investment_id       UUID NOT NULL REFERENCES investments(id),
    user_id             UUID NOT NULL REFERENCES users(id),
    asset_id            UUID NOT NULL REFERENCES assets(id),
    amount_cents        BIGINT NOT NULL,
    payout_type         VARCHAR(20) NOT NULL
                        CHECK (payout_type IN ('rental', 'exit', 'bonus')),
    status              VARCHAR(20) NOT NULL DEFAULT 'scheduled'
                        CHECK (status IN ('scheduled', 'processing', 'paid', 'failed')),
    scheduled_at        TIMESTAMPTZ,
    paid_at             TIMESTAMPTZ,
    wallet_tx_id        UUID REFERENCES wallet_transactions(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_dividends_user ON dividend_payouts(user_id);
CREATE INDEX idx_dividends_investment ON dividend_payouts(investment_id);

-- ============================================================
-- 18. notifications
-- ============================================================
CREATE TABLE notifications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           VARCHAR(255) NOT NULL,
    message         TEXT,
    type            VARCHAR(30) NOT NULL
                    CHECK (type IN ('kyc', 'investment', 'payout', 'system', 'promo')),
    is_read         BOOLEAN NOT NULL DEFAULT FALSE,
    action_url      VARCHAR(512),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_notifications_user ON notifications(user_id);
CREATE INDEX idx_notifications_unread ON notifications(user_id, is_read) WHERE is_read = FALSE;

-- ============================================================
-- 19. support_tickets
-- ============================================================
CREATE TABLE support_tickets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id),
    subject         VARCHAR(255) NOT NULL,
    message         TEXT NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'open'
                    CHECK (status IN ('open', 'in_progress', 'resolved', 'closed')),
    priority        VARCHAR(10) DEFAULT 'normal'
                    CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    assigned_to     UUID REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_support_user ON support_tickets(user_id);
CREATE INDEX idx_support_status ON support_tickets(status);

-- ============================================================
-- 20. user_settings
-- ============================================================
CREATE TABLE user_settings (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    totp_secret     VARCHAR(255),
    totp_enabled    BOOLEAN NOT NULL DEFAULT FALSE,
    language        VARCHAR(5) DEFAULT 'en',
    email_notifications   BOOLEAN DEFAULT TRUE,
    push_notifications    BOOLEAN DEFAULT TRUE,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- 21. roles (seed data included)
-- ============================================================
CREATE TABLE roles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(30) NOT NULL UNIQUE,
    description     VARCHAR(255)
);
INSERT INTO roles (name, description) VALUES
    ('investor', 'Standard-Investor mit Zugang zum Marketplace und Portfolio'),
    ('developer', 'Immobilien-Entwickler, der Assets einstellen kann'),
    ('admin', 'Plattform-Administrator');

-- ============================================================
-- 22. user_roles
-- ============================================================
CREATE TABLE user_roles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id         UUID NOT NULL REFERENCES roles(id),
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, role_id)
);
CREATE INDEX idx_user_roles_user ON user_roles(user_id);

-- ============================================================
-- 23. developer_projects
-- ============================================================
CREATE TABLE developer_projects (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    developer_id    UUID NOT NULL REFERENCES users(id),
    asset_id        UUID REFERENCES assets(id),
    project_name    VARCHAR(255) NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'draft'
                    CHECK (status IN ('draft', 'submitted', 'in_review', 'approved', 'rejected', 'live')),
    total_raised_cents      BIGINT DEFAULT 0,
    investors_count         INTEGER DEFAULT 0,
    funding_progress_bps    INTEGER DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_dev_projects_developer ON developer_projects(developer_id);

-- ============================================================
-- 24. audit_logs (IMMUTABLE – niemals UPDATE oder DELETE!)
-- ============================================================
CREATE TABLE audit_logs (
    id              BIGSERIAL PRIMARY KEY,
    actor_user_id   UUID REFERENCES users(id),
    action          VARCHAR(100) NOT NULL,
    entity_type     VARCHAR(50) NOT NULL,
    entity_id       UUID,
    previous_state  JSONB,
    new_state       JSONB,
    ip_address      INET,
    user_agent      TEXT,
    metadata        JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_audit_user ON audit_logs(actor_user_id);
CREATE INDEX idx_audit_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX idx_audit_action ON audit_logs(action);
CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);

-- ============================================================
-- 25. password_reset_tokens
-- ============================================================
CREATE TABLE password_reset_tokens (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      VARCHAR(255) NOT NULL UNIQUE,
    expires_at      TIMESTAMPTZ NOT NULL,
    used_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_prt_token ON password_reset_tokens(token_hash);

-- ============================================================
-- 26. investment_limits
-- ============================================================
CREATE TABLE investment_limits (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                 UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    annual_limit_cents      BIGINT NOT NULL DEFAULT 25000000,
    invested_12m_cents      BIGINT NOT NULL DEFAULT 0,
    available_cents         BIGINT GENERATED ALWAYS AS (annual_limit_cents - invested_12m_cents) STORED,
    limit_year              INTEGER NOT NULL,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, limit_year)
);
CREATE INDEX idx_inv_limits_user ON investment_limits(user_id);

-- ============================================================
-- Helper: updated_at trigger function
-- ============================================================
CREATE OR REPLACE FUNCTION trigger_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply updated_at trigger to all relevant tables
DO $$
DECLARE
    t TEXT;
BEGIN
    FOR t IN
        SELECT unnest(ARRAY[
            'users', 'user_profiles', 'wallets', 'assets',
            'investments', 'cart_items', 'support_tickets',
            'user_settings', 'developer_projects', 'kyc_records'
        ])
    LOOP
        EXECUTE format(
            'CREATE TRIGGER set_updated_at BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at()',
            t
        );
    END LOOP;
END;
$$;

-- ============================================================
-- Done! 🎉
-- ============================================================
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
    (v_asset1, '/images/villa1.webp', 'Luxury Clifftop Villa exterior', 0, TRUE),
    (v_asset1, '/images/villa1_2.webp', 'Infinity pool with ocean view', 1, FALSE),
    (v_asset1, '/images/villa1_3.webp', 'Modern interior living area', 2, FALSE),
    (v_asset1, '/images/villa1_4.webp', 'Ocean view terrace', 3, FALSE);

-- Property 2 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset2, '/images/villa2_1.webp', 'Modern Surf Villa exterior', 0, TRUE),
    (v_asset2, '/images/villa2_2.webp', 'Tropical garden and pool', 1, FALSE);

-- Property 3 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset3, '/images/villa3_1.webp', 'Boutique Resort entrance', 0, TRUE),
    (v_asset3, '/images/villa3_2.webp', 'Resort common area', 1, FALSE);

-- Property 4 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset4, '/images/villa4_1.webp', 'Vacation Rental Villa', 0, TRUE),
    (v_asset4, '/images/villa4_2.webp', 'Panoramic temple view', 1, FALSE);

-- Property 5 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset5, '/images/villa5.webp', 'Renovation flip project', 0, TRUE);

-- Property 6 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset6, '/images/villa6.webp', 'New development site', 0, TRUE);

-- Property 7 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset7, '/images/villa3_1.webp', 'Funded pool villa', 0, TRUE),
    (v_asset7, '/images/villa1_2.webp', 'Pool area', 1, FALSE);

-- Property 8 images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_asset8, '/images/villa2_1.webp', 'Beachfront retreat', 0, TRUE);

-- Commodity images (rice) – all 5 gallery images
INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_com1, '/static/images/commodities/rice/eduardo-prim-3u51-uLQICc-unsplash.webp', 'Bali rice terraces', 0, TRUE),
    (v_com1, '/static/images/commodities/rice/hoach-le-dinh-PeRt3uMmjYM-unsplash.webp', 'Paddy Rice plantation', 1, FALSE),
    (v_com1, '/static/images/commodities/rice/vrlibs-studio-h0cvg3O-LN0-unsplash.webp', 'Rice farming', 2, FALSE),
    (v_com1, '/static/images/commodities/rice/winston-chen-kXoEdaZ3SFw-unsplash.webp', 'Rice harvest', 3, FALSE),
    (v_com1, '/static/images/commodities/rice/zhao-yangjun-dDAzpSUAbgI-unsplash.webp', 'Rice field landscape', 4, FALSE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_com2, '/static/images/commodities/rice/hoach-le-dinh-PeRt3uMmjYM-unsplash.webp', 'Cacao beans', 0, TRUE);

INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover) VALUES
    (v_com3, '/static/images/commodities/rice/winston-chen-kXoEdaZ3SFw-unsplash.webp', 'Coffee beans', 0, TRUE);


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

INSERT INTO asset_documents (asset_id, document_type, title, file_url, file_size_bytes, is_investor_visible) VALUES
    (v_asset1, 'expose', 'Investment Expose – Clifftop Villa', '/docs/expose-clifftop-villa.pdf', 2456780, TRUE),
    (v_asset1, 'appraisal', 'Independent Appraisal Report', '/docs/appraisal-clifftop-villa.pdf', 1234567, TRUE),
    (v_asset1, 'proof_of_title', 'Certificate of Leasehold', '/docs/title-clifftop-villa.pdf', 345678, TRUE),

    (v_asset2, 'expose', 'Investment Expose – Surf Villa Canggu', '/docs/expose-surf-villa.pdf', 1987654, TRUE),
    (v_asset2, 'financial', 'Financial Projections 5-Year', '/docs/financials-surf-villa.pdf', 567890, TRUE),

    (v_asset5, 'expose', 'Renovation Project Plan', '/docs/expose-renovation-flip.pdf', 3456789, TRUE),
    (v_asset5, 'floor_plan', 'Proposed Floor Plans', '/docs/floorplan-renovation.pdf', 987654, TRUE);


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
-- database/002_payment_methods.sql

CREATE TABLE payment_methods (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- "card" or "bank_account"
    method_type         VARCHAR(20) NOT NULL 
                        CHECK (method_type IN ('card', 'bank_account')),
    
    -- The token returned by Stripe/Adyen (e.g., pm_1Iqx...)
    provider_token      VARCHAR(255) NOT NULL UNIQUE,
    provider_name       VARCHAR(50) NOT NULL DEFAULT 'stripe',
    
    -- Masked details (e.g., "4242", "Visa", "Chase Bank")
    last4               VARCHAR(4),
    brand               VARCHAR(50), 
    exp_month           INTEGER,
    exp_year            INTEGER,
    account_name        VARCHAR(255),
    currency            VARCHAR(3) DEFAULT 'USD',
    
    is_default          BOOLEAN NOT NULL DEFAULT FALSE,
    status              VARCHAR(20) NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'expired', 'failed', 'deleted')),
                        
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_payment_methods_user ON payment_methods(user_id);
CREATE INDEX idx_payment_methods_token ON payment_methods(provider_token);

-- Apply updated_at trigger
CREATE TRIGGER set_updated_at_payment_methods
BEFORE UPDATE ON payment_methods
FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
-- ═══════════════════════════════════════════════════════════════════
-- Migration 003: Settings Extensions
-- Adds currency and timezone columns to user_settings for
-- the Preferences tab on the Settings page.
-- ═══════════════════════════════════════════════════════════════════

-- Currency preference (ISO 4217 code)
ALTER TABLE user_settings
  ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'USD';

-- Timezone (IANA timezone identifier, e.g. 'America/Los_Angeles')
ALTER TABLE user_settings
  ADD COLUMN IF NOT EXISTS timezone VARCHAR(64) NOT NULL DEFAULT 'UTC';

-- Comment for documentation
COMMENT ON COLUMN user_settings.currency IS 'ISO 4217 currency code (USD, EUR, GBP, SGD, IDR)';
COMMENT ON COLUMN user_settings.timezone IS 'IANA timezone identifier (e.g. America/New_York)';
-- ═══════════════════════════════════════════════════════════════════
-- Migration 004: Rewards System Schema
-- ═══════════════════════════════════════════════════════════════════

-- Tier definitions (Intro, Plus, Pro, Elite, Premium)
CREATE TABLE IF NOT EXISTS tiers (
    id          SERIAL PRIMARY KEY,
    name        VARCHAR(32) NOT NULL UNIQUE,
    min_invest  BIGINT NOT NULL DEFAULT 0,   -- cents
    max_invest  BIGINT,                       -- cents, NULL = unlimited
    cashback_pct DECIMAL(5,2) NOT NULL DEFAULT 0,
    badge_color VARCHAR(7) NOT NULL DEFAULT '#D0D5DD',
    sort_order  INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed tier data
INSERT INTO tiers (name, min_invest, max_invest, cashback_pct, badge_color, sort_order) VALUES
    ('Intro',   0,        999999,  1.00, '#98FB96', 1),
    ('Plus',    1000000,  4999999, 2.00, '#027A48', 2),
    ('Pro',     5000000,  9999999, 3.00, '#7A5AF8', 3),
    ('Elite',   10000000, 24999999,4.00, '#F79009', 4),
    ('Premium', 25000000, NULL,    5.00, '#0000FF', 5)
ON CONFLICT (name) DO UPDATE SET
    min_invest = EXCLUDED.min_invest,
    max_invest = EXCLUDED.max_invest,
    cashback_pct = EXCLUDED.cashback_pct,
    badge_color = EXCLUDED.badge_color,
    sort_order = EXCLUDED.sort_order;

-- User tier tracking
CREATE TABLE IF NOT EXISTS user_tiers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tier_id         INT NOT NULL REFERENCES tiers(id),
    invested_12m    BIGINT NOT NULL DEFAULT 0,  -- cents invested in last 12 months
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Rewards balances
CREATE TABLE IF NOT EXISTS rewards_balances (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    cashback        BIGINT NOT NULL DEFAULT 0,  -- cents
    referrals       BIGINT NOT NULL DEFAULT 0,  -- cents
    promotions      BIGINT NOT NULL DEFAULT 0,  -- cents
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Referral codes
CREATE TABLE IF NOT EXISTS referral_codes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code            VARCHAR(32) NOT NULL UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Referral tracking
CREATE TABLE IF NOT EXISTS referral_tracking (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    referrer_id     UUID NOT NULL REFERENCES users(id),
    referred_id     UUID NOT NULL REFERENCES users(id),
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending, qualified, paid
    referrer_reward BIGINT NOT NULL DEFAULT 3000,  -- 30 USD in cents
    referred_reward BIGINT NOT NULL DEFAULT 3000,  -- 30 USD in cents
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    qualified_at    TIMESTAMPTZ,
    UNIQUE(referred_id)
);
-- ============================================================
-- POOOL Platform – Migration 005: Payments, Checkout & Invoicing
-- Adds multi-currency wallet support, deposit requests,
-- enhanced orders, and invoices.
-- ============================================================

-- ============================================================
-- 1. Extend wallets with currency
-- ============================================================
-- Add currency column (default 'USD' for existing rows)
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'USD';

-- Drop old unique constraint and create the new one with currency
ALTER TABLE wallets DROP CONSTRAINT IF EXISTS wallets_user_id_wallet_type_key;
ALTER TABLE wallets ADD CONSTRAINT wallets_user_id_wallet_type_currency_key
    UNIQUE (user_id, wallet_type, currency);

-- ============================================================
-- 2. Extend wallet_transactions with currency
-- ============================================================
ALTER TABLE wallet_transactions ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'USD';

-- ============================================================
-- 3. deposit_requests – Intent tracking for bank deposits
-- ============================================================
CREATE TABLE IF NOT EXISTS deposit_requests (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    currency            VARCHAR(3) NOT NULL,
    amount_cents        BIGINT NOT NULL CHECK (amount_cents > 0),
    provider            VARCHAR(30) NOT NULL
                        CHECK (provider IN ('stripe', 'ocbc', 'midtrans', 'mangopay', 'manual')),
    provider_reference  VARCHAR(255),
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'paid', 'expired', 'failed', 'cancelled')),
    payment_method      VARCHAR(50),
    expires_at          TIMESTAMPTZ,
    paid_at             TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_deposit_req_user ON deposit_requests(user_id);
CREATE INDEX IF NOT EXISTS idx_deposit_req_status ON deposit_requests(status);
CREATE INDEX IF NOT EXISTS idx_deposit_req_provider_ref ON deposit_requests(provider_reference);

-- ============================================================
-- 4. Extend orders with multi-currency & FX fields
-- ============================================================
ALTER TABLE orders ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'USD';
ALTER TABLE orders ADD COLUMN IF NOT EXISTS payment_currency VARCHAR(3);
ALTER TABLE orders ADD COLUMN IF NOT EXISTS fx_rate DECIMAL(18, 8);
ALTER TABLE orders ADD COLUMN IF NOT EXISTS fx_provider VARCHAR(50);

-- ============================================================
-- 5. invoices – Automated invoicing for completed orders
-- ============================================================
CREATE TABLE IF NOT EXISTS invoices (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_number      VARCHAR(30) NOT NULL UNIQUE,
    order_id            UUID NOT NULL REFERENCES orders(id),
    user_id             UUID NOT NULL REFERENCES users(id),
    company_entity      VARCHAR(255) NOT NULL DEFAULT 'POOOL GmbH',
    subtotal_cents      BIGINT NOT NULL,
    tax_cents           BIGINT NOT NULL DEFAULT 0,
    total_cents         BIGINT NOT NULL,
    currency            VARCHAR(3) NOT NULL DEFAULT 'USD',
    pdf_url             VARCHAR(512),
    status              VARCHAR(20) NOT NULL DEFAULT 'issued'
                        CHECK (status IN ('draft', 'issued', 'void')),
    notes               TEXT,
    issued_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_invoices_user ON invoices(user_id);
CREATE INDEX IF NOT EXISTS idx_invoices_order ON invoices(order_id);
CREATE INDEX IF NOT EXISTS idx_invoices_number ON invoices(invoice_number);

-- ============================================================
-- 6. invoice_sequence – Atomic invoice number generation
-- ============================================================
CREATE SEQUENCE IF NOT EXISTS invoice_number_seq START 1;

-- ============================================================
-- 7. Apply updated_at trigger to new tables
-- ============================================================
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_trigger WHERE tgname = 'set_updated_at' AND tgrelid = 'deposit_requests'::regclass
    ) THEN
        CREATE TRIGGER set_updated_at BEFORE UPDATE ON deposit_requests
        FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
    END IF;
END;
$$;

-- ============================================================
-- Done! 🎉
-- ============================================================
