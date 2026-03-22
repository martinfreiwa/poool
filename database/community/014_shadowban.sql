ALTER TABLE community_profiles ADD COLUMN IF NOT EXISTS is_shadowbanned BOOLEAN DEFAULT false;
