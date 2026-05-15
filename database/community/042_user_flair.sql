-- UX.14: custom user flair. A short label (text + emoji) shown next to
-- the display name in feed posts and the profile modal. Free-form, capped
-- at 24 characters server-side. NULL = no flair.

ALTER TABLE community_profiles
    ADD COLUMN IF NOT EXISTS flair TEXT NULL;
