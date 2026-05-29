-- Phase 1: Circle Feed as the default Circle destination.
-- Global community posts remain represented by circle_id IS NULL.

ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS circle_id UUID REFERENCES circles(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_posts_circle_visible_created
    ON posts(circle_id, created_at DESC)
    WHERE is_hidden = false;

CREATE INDEX IF NOT EXISTS idx_posts_global_visible_created
    ON posts(created_at DESC)
    WHERE is_hidden = false AND circle_id IS NULL;
