-- Add optional banner image URL to Expert AMAs. Admins can override the
-- default brand banner per AMA. Idempotent so the migration loop can replay
-- without erroring on partially-migrated environments.
ALTER TABLE amas ADD COLUMN IF NOT EXISTS banner_url TEXT;
