-- CO.7: scheduled posts. The composer can set a future timestamp; the
-- post row is written immediately so edits/cancellation work, but feed
-- queries hide it until `scheduled_for <= NOW()`. NULL = publish now.

ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS scheduled_for TIMESTAMPTZ NULL;

-- Partial index keeps the cost of the WHERE clause in the feed query at
-- ~zero — only the small set of pending scheduled posts is indexed.
CREATE INDEX IF NOT EXISTS idx_posts_scheduled_for_pending
    ON posts(scheduled_for)
    WHERE scheduled_for IS NOT NULL;
