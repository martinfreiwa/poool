-- UX.16: quote-reposts. A self-FK on `posts` lets a new post reference
-- another post being shared. We deliberately do NOT cascade — if the
-- quoted post is deleted, the quoting post stays alive but the read query
-- swallows the broken join and renders without the quote card.
-- Chains are flat by contract: the FE composer never lets you quote a post
-- that already quotes (one level deep).

ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS quoted_post_id UUID NULL;

ALTER TABLE posts
    DROP CONSTRAINT IF EXISTS posts_quoted_post_id_fkey;

ALTER TABLE posts
    ADD CONSTRAINT posts_quoted_post_id_fkey
        FOREIGN KEY (quoted_post_id) REFERENCES posts(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_posts_quoted_post_id
    ON posts(quoted_post_id)
    WHERE quoted_post_id IS NOT NULL;
