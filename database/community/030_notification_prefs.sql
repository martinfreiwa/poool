-- 14.8.15 — Per-user community notification preferences.
--
-- Per-channel boolean columns on community_profiles. Default TRUE so
-- existing users retain current behaviour (receive all notifications).
-- The columns cover the notification types currently produced by
-- community/notifications.rs.

ALTER TABLE community_profiles
    ADD COLUMN IF NOT EXISTS notif_post_like BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS notif_post_comment BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS notif_mention BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS notif_follow BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS notif_announcement BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS notif_ama BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS notif_challenge BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS notif_reward BOOLEAN NOT NULL DEFAULT TRUE;
