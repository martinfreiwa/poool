-- 14.8.12 — Nested comment replies (depth cap of 2).
--
-- Self-FK from comments.parent_comment_id → comments.id. NULL on top-level
-- comments. The depth cap (no reply to a reply) is enforced in Rust
-- service code, not by a DB constraint.
--
-- posts.comment_count stays a flat total — counts every comment regardless
-- of depth.

ALTER TABLE comments
    ADD COLUMN IF NOT EXISTS parent_comment_id UUID NULL
        REFERENCES comments(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_comments_parent ON comments (parent_comment_id);
