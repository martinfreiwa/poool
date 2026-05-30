-- Add the configurable referral reward amount used by the admin rewards API.
-- Monetary values are stored as integer cents.
ALTER TABLE tiers
    ADD COLUMN IF NOT EXISTS referral_bonus BIGINT NOT NULL DEFAULT 0;
