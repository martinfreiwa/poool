-- 084: Prevent new negative community XP totals.
--
-- NOT VALID avoids blocking deployment if legacy rows need separate cleanup,
-- while PostgreSQL still enforces the constraint for new and updated rows.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_community_profiles_xp_total_nonnegative'
    ) THEN
        ALTER TABLE community_profiles
            ADD CONSTRAINT chk_community_profiles_xp_total_nonnegative
            CHECK (xp_total >= 0) NOT VALID;
    END IF;
END $$;
