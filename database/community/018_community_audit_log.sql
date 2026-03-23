-- Community-specific admin audit log.
-- Records all moderation & admin actions taken in the community system.
-- Immutable: rows are NEVER updated or deleted (compliance requirement).

CREATE TABLE IF NOT EXISTS community_audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_user_id UUID,                            -- admin who performed the action
    action VARCHAR(100) NOT NULL,                   -- e.g. 'post.hide', 'user.ban', 'circle.delete'
    entity_type VARCHAR(50) NOT NULL,               -- e.g. 'post', 'comment', 'user', 'circle', 'badge', 'ama', 'challenge'
    entity_id UUID,                                 -- ID of the affected entity
    target_user_id UUID,                            -- the user the action was taken against (if applicable)
    details JSONB DEFAULT '{}',                     -- extra context (reason, previous state, etc.)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for querying by entity
CREATE INDEX IF NOT EXISTS idx_community_audit_entity ON community_audit_logs(entity_type, entity_id);
-- Index for querying by actor (which admin did what)
CREATE INDEX IF NOT EXISTS idx_community_audit_actor ON community_audit_logs(actor_user_id, created_at DESC);
-- Index for querying actions against a specific user
CREATE INDEX IF NOT EXISTS idx_community_audit_target ON community_audit_logs(target_user_id, created_at DESC);
