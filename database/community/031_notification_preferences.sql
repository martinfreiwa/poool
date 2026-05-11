-- Phase 3 task 31: per-user notification preferences. Stored as a JSONB
-- blob keyed by notification type so we don't need a schema migration each
-- time a new notification type is added; the server treats absent keys as
-- "enabled".
--
-- Typical payload:
--   {
--     "mention": true,
--     "new_follower": true,
--     "post_like": false,
--     "post_comment": true,
--     "announcement": true,
--     "reward": true,
--     "level_up": true,
--     "system": true
--   }

CREATE TABLE IF NOT EXISTS notification_preferences (
    user_id    UUID PRIMARY KEY,
    prefs      JSONB NOT NULL DEFAULT '{}'::JSONB,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
