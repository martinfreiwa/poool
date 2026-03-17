-- ============================================================
-- POOOL Platform
-- Migration 009: Legal Consents & Terms Versioning
-- ============================================================

-- 1. user_consents — immutable record of every T&C acceptance
CREATE TABLE IF NOT EXISTS user_consents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    terms_version   VARCHAR(50) NOT NULL DEFAULT '1.0',
    accepted_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address      VARCHAR(100),
    user_agent      TEXT
);

CREATE INDEX IF NOT EXISTS idx_user_consents_user ON user_consents(user_id);
CREATE INDEX IF NOT EXISTS idx_user_consents_version ON user_consents(terms_version);

-- 2. Seed platform_settings with legal versioning keys
INSERT INTO platform_settings (key, value, value_type, description) VALUES
    ('legal_terms_version',   '1.0',       'string',  'Current active Terms & Conditions version. Increment to prompt all users to re-accept.'),
    ('legal_privacy_version', '1.0',       'string',  'Current active Privacy Policy version.'),
    ('legal_last_updated',    '2026-03-08','string',  'Date when legal documents were last updated (YYYY-MM-DD).')
ON CONFLICT (key) DO NOTHING;
