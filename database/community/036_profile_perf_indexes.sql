-- WS3.1: composite indexes for the per-user profile endpoints.
--
-- /api/community/profile/:id/posts and /comments and /activity all run
-- `WHERE user_id = $1 ORDER BY created_at DESC LIMIT N OFFSET M`.
-- Posts and comments only had single-column indexes on (user_id) and
-- (created_at DESC) separately, forcing a sort. The xp_ledger already
-- ships a composite via 008_circles_xp.sql; mirror that on the post and
-- comment tables.

CREATE INDEX IF NOT EXISTS idx_posts_user_created
    ON posts(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_comments_user_created
    ON comments(user_id, created_at DESC);
