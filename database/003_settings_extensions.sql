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
