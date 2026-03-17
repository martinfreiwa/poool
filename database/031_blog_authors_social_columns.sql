-- =========================================================================
-- 031_blog_authors_social_columns.sql — Add missing social columns
-- =========================================================================
-- The blog_authors table was missing facebook_url, instagram_url, and
-- whatsapp columns that the Rust service layer expects. This caused
-- the /blog page to return 500 errors.
-- =========================================================================

ALTER TABLE blog_authors ADD COLUMN IF NOT EXISTS facebook_url VARCHAR(512);
ALTER TABLE blog_authors ADD COLUMN IF NOT EXISTS instagram_url VARCHAR(512);
ALTER TABLE blog_authors ADD COLUMN IF NOT EXISTS whatsapp VARCHAR(50);
