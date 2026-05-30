-- 055_circle_manage_ops.sql
-- Phase 9: Circle Manage settings, moderation controls, analytics
-- snapshots, and ops-alert primitives. Settings remain server-side
-- authority; UI controls only render/edit this contract.

ALTER TABLE circles
  ADD COLUMN IF NOT EXISTS category VARCHAR(80),
  ADD COLUMN IF NOT EXISTS language VARCHAR(16) NOT NULL DEFAULT 'en',
  ADD COLUMN IF NOT EXISTS location_text VARCHAR(120),
  ADD COLUMN IF NOT EXISTS rules_text TEXT,
  ADD COLUMN IF NOT EXISTS investment_disclaimer TEXT,
  ADD COLUMN IF NOT EXISTS join_approval_required BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS auto_approve_verified_investors BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS member_questions JSONB NOT NULL DEFAULT '[]'::jsonb,
  ADD COLUMN IF NOT EXISTS media_uploads_enabled BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN IF NOT EXISTS polls_enabled BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN IF NOT EXISTS anonymous_posting_enabled BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS link_posting_enabled BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN IF NOT EXISTS first_post_approval_enabled BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS slow_mode_seconds INTEGER NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS blocked_words TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
  ADD COLUMN IF NOT EXISTS investment_risk_keywords TEXT[] NOT NULL DEFAULT ARRAY[
    'guaranteed return',
    'risk-free',
    'no risk',
    'sure profit'
  ]::TEXT[],
  ADD COLUMN IF NOT EXISTS analytics_enabled BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE circles DROP CONSTRAINT IF EXISTS circles_language_len_check;
ALTER TABLE circles
  ADD CONSTRAINT circles_language_len_check
  CHECK (length(trim(language)) BETWEEN 2 AND 16);

ALTER TABLE circles DROP CONSTRAINT IF EXISTS circles_slow_mode_seconds_check;
ALTER TABLE circles
  ADD CONSTRAINT circles_slow_mode_seconds_check
  CHECK (slow_mode_seconds BETWEEN 0 AND 86400);

ALTER TABLE circles DROP CONSTRAINT IF EXISTS circles_rules_len_check;
ALTER TABLE circles
  ADD CONSTRAINT circles_rules_len_check
  CHECK (rules_text IS NULL OR char_length(rules_text) <= 5000);

ALTER TABLE circles DROP CONSTRAINT IF EXISTS circles_disclaimer_len_check;
ALTER TABLE circles
  ADD CONSTRAINT circles_disclaimer_len_check
  CHECK (investment_disclaimer IS NULL OR char_length(investment_disclaimer) <= 2000);

CREATE INDEX IF NOT EXISTS idx_circles_manage_category
  ON circles (category, visibility, created_at DESC)
  WHERE category IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_circles_blocked_words_gin
  ON circles USING GIN (blocked_words);

CREATE TABLE IF NOT EXISTS circle_daily_analytics (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  circle_id UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
  snapshot_date DATE NOT NULL,
  member_count INTEGER NOT NULL DEFAULT 0,
  active_members INTEGER NOT NULL DEFAULT 0,
  posts_count INTEGER NOT NULL DEFAULT 0,
  comments_count INTEGER NOT NULL DEFAULT 0,
  qna_answer_rate_bps INTEGER NOT NULL DEFAULT 0,
  reported_content_count INTEGER NOT NULL DEFAULT 0,
  top_tags JSONB NOT NULL DEFAULT '[]'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (circle_id, snapshot_date)
);

CREATE INDEX IF NOT EXISTS idx_circle_daily_analytics_circle_date
  ON circle_daily_analytics (circle_id, snapshot_date DESC);

CREATE TABLE IF NOT EXISTS circle_ops_alerts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  circle_id UUID REFERENCES circles(id) ON DELETE CASCADE,
  alert_type VARCHAR(40) NOT NULL CHECK (alert_type IN (
    'report_backlog',
    'spam_spike',
    'failed_worker',
    'posting_spike',
    'moderation_sla'
  )),
  severity VARCHAR(16) NOT NULL DEFAULT 'info'
    CHECK (severity IN ('info', 'warning', 'critical')),
  status VARCHAR(20) NOT NULL DEFAULT 'open'
    CHECK (status IN ('open', 'acknowledged', 'resolved')),
  summary TEXT NOT NULL,
  details JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  resolved_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_circle_ops_alerts_open
  ON circle_ops_alerts (circle_id, severity, created_at DESC)
  WHERE status IN ('open', 'acknowledged');

COMMENT ON COLUMN circles.first_post_approval_enabled IS
  'Phase 9 moderation setting. New member first posts can be held for review before publication.';
COMMENT ON COLUMN circles.blocked_words IS
  'Phase 9 moderation setting. Server-side allowlisted update path; used by future pre-publish moderation.';
COMMENT ON COLUMN circles.investment_risk_keywords IS
  'Phase 9 compliance setting for terms requiring extra moderation attention in investment discussions.';
COMMENT ON TABLE circle_daily_analytics IS
  'Phase 9 per-Circle analytics snapshots for bounded dashboard reads.';
COMMENT ON TABLE circle_ops_alerts IS
  'Phase 9 operational alerts for report backlog, spam spikes, failed workers, and moderation SLA breaches.';
