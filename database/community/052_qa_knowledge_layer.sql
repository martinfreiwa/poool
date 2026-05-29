-- 052_qa_knowledge_layer.sql
-- Phase 6: Q&A, due-diligence, and knowledge-layer state.

ALTER TABLE posts
  ADD COLUMN IF NOT EXISTS qa_status VARCHAR(32) NOT NULL DEFAULT 'open',
  ADD COLUMN IF NOT EXISTS official_answer_comment_id UUID NULL REFERENCES comments(id) ON DELETE SET NULL,
  ADD COLUMN IF NOT EXISTS faq_candidate BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS featured_question BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS related_resource_url TEXT,
  ADD COLUMN IF NOT EXISTS related_asset_id UUID;

ALTER TABLE posts DROP CONSTRAINT IF EXISTS posts_qa_status_check;
ALTER TABLE posts
  ADD CONSTRAINT posts_qa_status_check
  CHECK (qa_status IN ('open', 'answered', 'official_answer', 'needs_clarification', 'archived'));

ALTER TABLE comments
  ADD COLUMN IF NOT EXISTS is_official_answer BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS is_verified_answer BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS answer_marked_by UUID,
  ADD COLUMN IF NOT EXISTS answer_marked_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS community_answer_audit_log (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
  comment_id UUID REFERENCES comments(id) ON DELETE SET NULL,
  actor_user_id UUID NOT NULL,
  action VARCHAR(64) NOT NULL,
  previous_status VARCHAR(32),
  new_status VARCHAR(32),
  metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_posts_qa_status_created
  ON posts (qa_status, created_at DESC)
  WHERE is_hidden = false;

CREATE INDEX IF NOT EXISTS idx_posts_official_answer
  ON posts (official_answer_comment_id)
  WHERE official_answer_comment_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_comments_official_answers
  ON comments (post_id, is_official_answer, is_verified_answer)
  WHERE is_hidden = false;

CREATE INDEX IF NOT EXISTS idx_answer_audit_post_created
  ON community_answer_audit_log (post_id, created_at DESC);

COMMENT ON COLUMN posts.qa_status IS
  'Phase 6 Q&A lifecycle: open, answered, official_answer, needs_clarification, archived.';
COMMENT ON COLUMN posts.official_answer_comment_id IS
  'Comment selected by a Circle responder/platform admin as the official answer.';
COMMENT ON TABLE community_answer_audit_log IS
  'Append-only audit log for Q&A status changes and official/verified answer markings.';
