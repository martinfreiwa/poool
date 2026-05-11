-- WS1.1 Phase A: nested comment replies (depth 1).
--
-- A reply is a comment whose parent_comment_id points to another comment on
-- the same post. The backend caps depth at 1 (replies cannot themselves be
-- replied to) to keep the UI manageable; deeper threading is an explicit
-- product decision left for a later epic.

ALTER TABLE comments
    ADD COLUMN IF NOT EXISTS parent_comment_id UUID NULL REFERENCES comments(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_comments_parent
    ON comments(parent_comment_id)
    WHERE parent_comment_id IS NOT NULL;
