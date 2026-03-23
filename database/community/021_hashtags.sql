-- 021_hashtags.sql — UX.4: Hashtag Architecture
-- Normalized hashtag system with many-to-many relationship to posts

CREATE TABLE IF NOT EXISTS hashtags (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tag         VARCHAR(100) NOT NULL UNIQUE, -- lowercase, trimmed, no '#' prefix
    post_count  INTEGER NOT NULL DEFAULT 0,   -- denormalized for trending query
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_hashtags_tag ON hashtags(tag);
CREATE INDEX idx_hashtags_trending ON hashtags(post_count DESC) WHERE post_count > 0;

CREATE TABLE IF NOT EXISTS post_hashtags (
    post_id     UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    hashtag_id  UUID NOT NULL REFERENCES hashtags(id) ON DELETE CASCADE,
    PRIMARY KEY (post_id, hashtag_id)
);

CREATE INDEX idx_post_hashtags_hashtag_id ON post_hashtags(hashtag_id);

-- Trigger: Update hashtag post_count on insert/delete
CREATE OR REPLACE FUNCTION update_hashtag_post_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE hashtags SET post_count = post_count + 1 WHERE id = NEW.hashtag_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE hashtags SET post_count = post_count - 1 WHERE id = OLD.hashtag_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_hashtag_post_count
AFTER INSERT OR DELETE ON post_hashtags
FOR EACH ROW EXECUTE FUNCTION update_hashtag_post_count();
