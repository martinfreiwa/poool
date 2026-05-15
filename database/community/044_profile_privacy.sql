-- Profile-page privacy toggles (community-side flags only).
--
-- `is_public_profile`         — when FALSE, the /community/u/:id sub-page
--                               and member-directory listing are hidden
--                               from logged-out visitors.
-- `allow_dms_from_strangers`  — when FALSE, only users the profile owner
--                               follows back can open a DM thread with
--                               them. The DM create-thread route enforces.
--
-- Leaderboard visibility lives on `leaderboard_preferences.visible` in the
-- CORE DB and is wired separately by the route handler.

ALTER TABLE community_profiles
    ADD COLUMN IF NOT EXISTS is_public_profile BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS allow_dms_from_strangers BOOLEAN NOT NULL DEFAULT TRUE;
