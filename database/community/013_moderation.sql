-- 013_moderation.sql
-- Add thread locking, content tagging, pinned comments, and mod notes.

-- Thread Locking + Content Tagging (NSFW/Spoiler)
ALTER TABLE posts ADD COLUMN IF NOT EXISTS is_locked BOOLEAN DEFAULT false;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS content_tags TEXT[] DEFAULT '{}';

-- Pinned comments (Admin can pin a comment)
ALTER TABLE comments ADD COLUMN IF NOT EXISTS is_pinned BOOLEAN DEFAULT false;

-- Auto Moderation log (Optional)
CREATE TABLE IF NOT EXISTS auto_mod_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID REFERENCES posts(id) ON DELETE CASCADE,
    comment_id UUID REFERENCES comments(id) ON DELETE CASCADE,
    user_id UUID REFERENCES community_profiles(user_id) ON DELETE CASCADE,
    rule_triggered TEXT NOT NULL,
    action_taken TEXT NOT NULL, -- e.g., 'hidden', 'warned', 'deleted'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    CHECK (post_id IS NOT NULL OR comment_id IS NOT NULL)
);

-- Advanced Moderation Mutes
ALTER TABLE community_profiles ADD COLUMN IF NOT EXISTS muted_until TIMESTAMPTZ;
ALTER TABLE community_profiles ADD COLUMN IF NOT EXISTS mod_notes TEXT;
