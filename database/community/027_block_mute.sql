-- 14.8.2 — Self-service block + mute relationships.
--
-- Two parallel tables so the semantics stay distinct:
--   block  = hard cut. The target user cannot see actor's posts in their
--            feed, cannot DM, cannot mention. Reciprocal effect (actor also
--            won't see target's posts) is enforced in the feed query so a
--            block always disables visibility in both directions.
--   mute   = soft cut. Actor stops seeing target's posts in their feed and
--            stops receiving notifications about target's actions. The
--            target is not informed and remains free to interact otherwise.
--
-- Both tables are user-controlled (self-service), distinct from the
-- existing admin/moderation tables in 013_moderation.sql / 014_shadowban.sql.

CREATE TABLE IF NOT EXISTS block_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_user_id UUID NOT NULL,
    target_user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT block_relationships_no_self CHECK (actor_user_id <> target_user_id),
    CONSTRAINT block_relationships_unique UNIQUE (actor_user_id, target_user_id)
);

CREATE INDEX IF NOT EXISTS idx_block_relationships_actor ON block_relationships (actor_user_id);
CREATE INDEX IF NOT EXISTS idx_block_relationships_target ON block_relationships (target_user_id);

CREATE TABLE IF NOT EXISTS mute_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_user_id UUID NOT NULL,
    target_user_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT mute_relationships_no_self CHECK (actor_user_id <> target_user_id),
    CONSTRAINT mute_relationships_unique UNIQUE (actor_user_id, target_user_id)
);

CREATE INDEX IF NOT EXISTS idx_mute_relationships_actor ON mute_relationships (actor_user_id);
