-- ============================================================
-- POOOL Platform – Database Schema Update
-- Migration 070: Developer Profiles
-- ============================================================

CREATE TABLE developer_profiles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    company_name    VARCHAR(255) NOT NULL,
    logo_url        VARCHAR(512),
    description     TEXT,
    website_url     VARCHAR(512),
    facebook_url    VARCHAR(512),
    instagram_url   VARCHAR(512),
    youtube_url     VARCHAR(512),
    x_url           VARCHAR(512),
    linkedin_url    VARCHAR(512),
    tiktok_url      VARCHAR(512),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Trigger for updated_at
CREATE TRIGGER set_updated_at_dev_profiles
    BEFORE UPDATE ON developer_profiles
    FOR EACH ROW
    EXECUTE FUNCTION trigger_set_updated_at();

-- Index for querying by user_id
CREATE INDEX idx_dev_profiles_user ON developer_profiles(user_id);
