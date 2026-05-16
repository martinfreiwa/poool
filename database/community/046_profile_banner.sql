-- 046: Cover-photo / banner URL on community_profiles
--
-- The Facebook-style hero on the user profile page (community-profile.html
-- 2026-05-16 rework) shows an "Edit cover" button to upload a banner image.
-- The column for the resulting URL was missing — added here.
--
-- Length-checked to discourage opaque base64 stashing; real URLs are <512.

BEGIN;

ALTER TABLE community_profiles
    ADD COLUMN IF NOT EXISTS banner_url TEXT;

ALTER TABLE community_profiles
    DROP CONSTRAINT IF EXISTS community_profiles_banner_url_len;
ALTER TABLE community_profiles
    ADD CONSTRAINT community_profiles_banner_url_len
    CHECK (banner_url IS NULL OR char_length(banner_url) <= 1024);

COMMIT;
