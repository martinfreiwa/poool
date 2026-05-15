-- 2026-05-15: drop the legacy per-column notification preference fields on
-- community_profiles. The delivery filter (`notify_user`) reads from the
-- JSONB blob in `notification_preferences` instead, and that's the table
-- both UIs now write through.
--
-- These columns were never consulted by anything that actually delivers
-- notifications — they were a dead settings surface. No data migration
-- needed: any user who toggled a setting under the old model didn't
-- actually mute anything.

ALTER TABLE community_profiles
    DROP COLUMN IF EXISTS notif_post_like,
    DROP COLUMN IF EXISTS notif_post_comment,
    DROP COLUMN IF EXISTS notif_mention,
    DROP COLUMN IF EXISTS notif_follow,
    DROP COLUMN IF EXISTS notif_announcement,
    DROP COLUMN IF EXISTS notif_ama,
    DROP COLUMN IF EXISTS notif_challenge,
    DROP COLUMN IF EXISTS notif_reward;
