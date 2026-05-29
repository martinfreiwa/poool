-- 14.8.24 — Platform-wide community settings (key/value store).
--
-- Admins toggle feature flags + content thresholds via /admin/community/settings.
-- Values are JSON-encoded strings so the same table holds booleans, ints, and
-- short strings without per-type columns. Validation lives in the Rust handler.

CREATE TABLE IF NOT EXISTS community_settings (
    key        VARCHAR(80) PRIMARY KEY,
    value      TEXT NOT NULL,
    description TEXT,
    updated_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed the initial set of recognised keys (the handler accepts these by name).
INSERT INTO community_settings (key, value, description) VALUES
    ('feature_dms_enabled',       'false', 'Master toggle for direct messages feature.'),
    ('feature_polls_enabled',     'true',  'Allow polls in post composer.'),
    ('feature_reviews_enabled',   'true',  'Allow asset reviews from verified owners.'),
    ('automod_threshold',         '3',     'Number of automod hits before automatic shadowban.'),
    ('report_review_sla_hours',   '48',    'Target review time for content reports (hours).')
ON CONFLICT (key) DO NOTHING;
