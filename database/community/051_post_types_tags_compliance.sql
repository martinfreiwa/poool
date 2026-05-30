-- 051_post_types_tags_compliance.sql
-- Phase 5: structured Circle/community post types, tags/flairs, and
-- compliance-ready post metadata.

ALTER TABLE posts
  ADD COLUMN IF NOT EXISTS content_tags TEXT[] NOT NULL DEFAULT '{}'::TEXT[];

UPDATE posts
SET content_tags = '{}'::TEXT[]
WHERE content_tags IS NULL;

ALTER TABLE posts
  ALTER COLUMN content_tags SET DEFAULT '{}'::TEXT[];

ALTER TABLE posts
  ALTER COLUMN content_tags SET NOT NULL;

ALTER TABLE posts DROP CONSTRAINT IF EXISTS posts_post_type_check;
ALTER TABLE posts
  ADD CONSTRAINT posts_post_type_check
  CHECK (
    post_type IN (
      'general',
      'discussion',
      'question',
      'market_insight',
      'property_update',
      'due_diligence',
      'poll',
      'announcement',
      'ama_question',
      'resource',
      'risk_discussion',
      'official_update',
      -- Legacy values kept readable/write-compatible while older code paths
      -- and historical rows are phased into the Phase 5 taxonomy.
      'milestone',
      'farm_update',
      'review'
    )
  );

ALTER TABLE circles
  ADD COLUMN IF NOT EXISTS required_post_tags TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
  ADD COLUMN IF NOT EXISTS allowed_post_types TEXT[] NOT NULL DEFAULT ARRAY[
    'general',
    'discussion',
    'question',
    'market_insight',
    'property_update',
    'due_diligence',
    'poll',
    'announcement',
    'ama_question',
    'resource',
    'risk_discussion',
    'official_update'
  ]::TEXT[];

CREATE INDEX IF NOT EXISTS idx_posts_content_tags_gin
  ON posts USING GIN (content_tags);

CREATE INDEX IF NOT EXISTS idx_posts_circle_type_created
  ON posts (circle_id, post_type, created_at DESC)
  WHERE is_hidden = false;

CREATE INDEX IF NOT EXISTS idx_circles_required_post_tags_gin
  ON circles USING GIN (required_post_tags);

COMMENT ON COLUMN posts.content_tags IS
  'Phase 5 normalized post tags/flairs. User payload is allowlisted server-side.';
COMMENT ON COLUMN circles.required_post_tags IS
  'Phase 5 optional Circle policy. All listed tags must be present on new Circle posts.';
COMMENT ON COLUMN circles.allowed_post_types IS
  'Phase 5 optional Circle policy. New Circle posts must use one of these server-allowlisted types.';
