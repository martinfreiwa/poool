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
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_asset_docs_asset ON asset_documents(asset_id);

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
