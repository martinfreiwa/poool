-- ============================================================
-- POOOL Platform - Settings Input Wiring
-- Migration 078: Persist all current settings page fields
-- ============================================================

ALTER TABLE user_profiles
    ADD COLUMN IF NOT EXISTS middle_name VARCHAR(100),
    ADD COLUMN IF NOT EXISTS gender VARCHAR(30),
    ADD COLUMN IF NOT EXISTS social_twitter_url VARCHAR(512),
    ADD COLUMN IF NOT EXISTS social_linkedin_url VARCHAR(512),
    ADD COLUMN IF NOT EXISTS social_instagram_url VARCHAR(512),
    ADD COLUMN IF NOT EXISTS social_telegram_url VARCHAR(512),
    ADD COLUMN IF NOT EXISTS social_discord VARCHAR(100),
    ADD COLUMN IF NOT EXISTS social_website_url VARCHAR(512);

ALTER TABLE leaderboard_preferences
    ADD COLUMN IF NOT EXISTS bio VARCHAR(300);

ALTER TABLE developer_profiles
    ADD COLUMN IF NOT EXISTS github_url VARCHAR(512);
