-- 053_circle_engagement_onboarding.sql
-- Phase 7: Circle-specific announcements/events, AMAs, challenges, and
-- onboarding progress. Global community surfaces remain global; Circle
-- engagement is scoped by circle_id and checked server-side.

ALTER TABLE circles
  ADD COLUMN IF NOT EXISTS announcement_comments_enabled BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN IF NOT EXISTS onboarding_enabled BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE amas
  ADD COLUMN IF NOT EXISTS circle_id UUID NULL REFERENCES circles(id) ON DELETE CASCADE,
  ADD COLUMN IF NOT EXISTS asset_id UUID NULL,
  ADD COLUMN IF NOT EXISTS rsvp_enabled BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_amas_circle_status_scheduled
  ON amas (circle_id, status, scheduled_at DESC)
  WHERE circle_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_amas_global_status_scheduled
  ON amas (status, scheduled_at DESC)
  WHERE circle_id IS NULL;

CREATE TABLE IF NOT EXISTS circle_event_rsvps (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  ama_id UUID NOT NULL REFERENCES amas(id) ON DELETE CASCADE,
  circle_id UUID NULL REFERENCES circles(id) ON DELETE CASCADE,
  user_id UUID NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'going'
    CHECK (status IN ('going', 'interested', 'declined')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (ama_id, user_id)
);

COMMENT ON TABLE circle_event_rsvps IS
  'Phase 7 RSVP storage for Circle AMAs/events. Notification fan-out remains feature-flagged until preferences/outbox are production-ready.';

ALTER TABLE challenges
  ADD COLUMN IF NOT EXISTS circle_id UUID NULL REFERENCES circles(id) ON DELETE CASCADE,
  ADD COLUMN IF NOT EXISTS challenge_scope VARCHAR(32) NOT NULL DEFAULT 'global',
  ADD COLUMN IF NOT EXISTS sort_order INTEGER NOT NULL DEFAULT 0;

UPDATE challenges
SET challenge_scope = 'global'
WHERE challenge_scope IS NULL
   OR challenge_scope NOT IN ('global', 'circle', 'asset');

ALTER TABLE challenges DROP CONSTRAINT IF EXISTS challenges_scope_check;
ALTER TABLE challenges
  ADD CONSTRAINT challenges_scope_check
  CHECK (challenge_scope IN ('global', 'circle', 'asset'));

CREATE INDEX IF NOT EXISTS idx_challenges_scope_active
  ON challenges (challenge_scope, circle_id, is_active, sort_order, created_at);

CREATE TABLE IF NOT EXISTS circle_challenge_progress (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  circle_id UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
  user_id UUID NOT NULL,
  challenge_id UUID NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
  current_value INTEGER NOT NULL DEFAULT 0,
  is_completed BOOLEAN NOT NULL DEFAULT FALSE,
  completed_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (circle_id, user_id, challenge_id)
);

CREATE INDEX IF NOT EXISTS idx_circle_challenge_progress_user
  ON circle_challenge_progress (user_id, circle_id);

CREATE INDEX IF NOT EXISTS idx_circle_challenge_progress_circle
  ON circle_challenge_progress (circle_id, is_completed);

CREATE TABLE IF NOT EXISTS circle_onboarding_progress (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  circle_id UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
  user_id UUID NOT NULL,
  rules_read BOOLEAN NOT NULL DEFAULT FALSE,
  introduced_self BOOLEAN NOT NULL DEFAULT FALSE,
  interests_selected BOOLEAN NOT NULL DEFAULT FALSE,
  ama_followed BOOLEAN NOT NULL DEFAULT FALSE,
  first_question_posted BOOLEAN NOT NULL DEFAULT FALSE,
  is_completed BOOLEAN NOT NULL DEFAULT FALSE,
  completed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (circle_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_circle_onboarding_user
  ON circle_onboarding_progress (user_id, circle_id);

CREATE INDEX IF NOT EXISTS idx_circle_onboarding_circle
  ON circle_onboarding_progress (circle_id, is_completed);

INSERT INTO challenges (
  title,
  description,
  xp_reward,
  requirement_type,
  requirement_value,
  frequency,
  challenge_scope,
  sort_order
)
SELECT title, description, xp_reward, requirement_type, requirement_value, frequency, 'circle', sort_order
FROM (VALUES
  ('Introduce yourself in this Circle', 'Post or confirm your introduction so members know what you want to discuss.', 25, 'circle_introduction', 1, 'one_time', 10),
  ('Ask your first due diligence question', 'Create a Circle question or due diligence post that helps investors evaluate assumptions.', 50, 'circle_due_diligence_question', 1, 'one_time', 20),
  ('Join the next Circle AMA', 'Follow or RSVP to a Circle AMA or expert session.', 25, 'circle_ama_join', 1, 'one_time', 30),
  ('Read the Circle guide', 'Confirm that you read the Circle rules, purpose, and investment-risk disclaimer.', 25, 'circle_guide_read', 1, 'one_time', 40),
  ('Share one market insight', 'Create a Circle market insight with relevant tags and a clear opinion disclaimer.', 50, 'circle_market_insight', 1, 'one_time', 50),
  ('Comment on 3 Circle posts', 'Contribute three comments inside this Circle.', 50, 'circle_comment', 3, 'one_time', 60)
) AS seed(title, description, xp_reward, requirement_type, requirement_value, frequency, sort_order)
WHERE NOT EXISTS (
  SELECT 1
  FROM challenges c
  WHERE c.title = seed.title
    AND c.challenge_scope = 'circle'
    AND c.circle_id IS NULL
);

COMMENT ON COLUMN challenges.challenge_scope IS
  'Phase 7 scope discriminator. Global challenge lists must only include challenge_scope=global.';

COMMENT ON TABLE circle_challenge_progress IS
  'Phase 7 per-Circle challenge progress. This prevents one Circle activity from completing the same challenge in another Circle.';

COMMENT ON TABLE circle_onboarding_progress IS
  'Phase 7 member onboarding checklist state for Circle welcome panels.';
