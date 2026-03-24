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
    ('Intro',   0,        399999,  1.00, '#98FB96', 1),
    ('Plus',    400000,   999999,  2.00, '#027A48', 2),
    ('Pro',     1000000,  2999999, 3.00, '#7A5AF8', 3),
    ('Elite',   3000000,  9999999, 4.00, '#F79009', 4),
    ('Premium', 10000000, NULL,    5.00, '#0000FF', 5)
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
