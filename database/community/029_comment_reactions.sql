-- 14.8.6 — Comment reactions (separate from post reactions).
--
-- We keep posts.reaction_count and comments.helpful_count as denormalized
-- counters maintained by triggers, matching the post-reactions pattern.
-- A new comment_reactions table mirrors the reactions schema (same allowed
-- taxonomy: fire/insightful/clap/green) but is intentionally separate so
-- the existing post-reaction service is untouched.

ALTER TABLE comments
    ADD COLUMN IF NOT EXISTS reaction_count INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS comment_reactions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    comment_id      UUID NOT NULL REFERENCES comments(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL,
    reaction_type   VARCHAR(20) NOT NULL
                    CHECK (reaction_type IN ('fire', 'insightful', 'clap', 'green')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (comment_id, user_id, reaction_type)
);

CREATE INDEX IF NOT EXISTS idx_comment_reactions_comment ON comment_reactions (comment_id);
CREATE INDEX IF NOT EXISTS idx_comment_reactions_user ON comment_reactions (user_id);

CREATE OR REPLACE FUNCTION update_comment_reaction_count() RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE comments SET reaction_count = reaction_count + 1 WHERE id = NEW.comment_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE comments SET reaction_count = GREATEST(reaction_count - 1, 0) WHERE id = OLD.comment_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_comment_reaction_count ON comment_reactions;

CREATE TRIGGER trg_comment_reaction_count
AFTER INSERT OR DELETE ON comment_reactions
FOR EACH ROW EXECUTE FUNCTION update_comment_reaction_count();
