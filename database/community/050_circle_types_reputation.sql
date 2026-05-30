-- 050_circle_types_reputation.sql
-- Phase 3: first-class Circle types, visibility gates, role vocabulary,
-- and admin/system-granted reputation flairs.

BEGIN;

ALTER TABLE circles
  ADD COLUMN IF NOT EXISTS circle_type VARCHAR(32) NOT NULL DEFAULT 'social',
  ADD COLUMN IF NOT EXISTS visibility VARCHAR(32) NOT NULL DEFAULT 'public',
  ADD COLUMN IF NOT EXISTS join_policy VARCHAR(32) NOT NULL DEFAULT 'open',
  ADD COLUMN IF NOT EXISTS is_official BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS kyc_required BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS private_investor_club BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS allow_cross_post BOOLEAN NOT NULL DEFAULT TRUE;

UPDATE circles
SET visibility = CASE WHEN is_public THEN 'public' ELSE 'private' END
WHERE visibility IS NULL
   OR visibility NOT IN ('public', 'private', 'hidden');

UPDATE circles
SET circle_type = 'asset'
WHERE token_gate_asset_id IS NOT NULL
  AND circle_type = 'social';

UPDATE circles
SET join_policy = CASE
  WHEN token_gate_asset_id IS NOT NULL THEN 'holder_only'
  WHEN kyc_required THEN 'kyc_required'
  WHEN is_public THEN 'open'
  ELSE 'request'
END
WHERE join_policy IS NULL
   OR join_policy NOT IN ('open', 'request', 'invite_only', 'holder_only', 'kyc_required');

UPDATE circles
SET private_investor_club = TRUE,
    allow_cross_post = FALSE
WHERE circle_type = 'private_investor';

UPDATE circles
SET is_official = TRUE
WHERE circle_type = 'official';

ALTER TABLE circles DROP CONSTRAINT IF EXISTS circles_circle_type_check;
ALTER TABLE circles
  ADD CONSTRAINT circles_circle_type_check
  CHECK (circle_type IN ('social', 'asset', 'topic', 'expert', 'private_investor', 'official'));

ALTER TABLE circles DROP CONSTRAINT IF EXISTS circles_visibility_check;
ALTER TABLE circles
  ADD CONSTRAINT circles_visibility_check
  CHECK (visibility IN ('public', 'private', 'hidden'));

ALTER TABLE circles DROP CONSTRAINT IF EXISTS circles_join_policy_check;
ALTER TABLE circles
  ADD CONSTRAINT circles_join_policy_check
  CHECK (join_policy IN ('open', 'request', 'invite_only', 'holder_only', 'kyc_required'));

ALTER TABLE circle_members DROP CONSTRAINT IF EXISTS circle_members_role_check;
ALTER TABLE circle_members
  ADD CONSTRAINT circle_members_role_check
  CHECK (role IN ('owner', 'admin', 'moderator', 'verified_expert', 'member'));

CREATE INDEX IF NOT EXISTS idx_circles_type_visibility
  ON circles (circle_type, visibility, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_circles_official
  ON circles (is_official, featured_at DESC, created_at DESC)
  WHERE is_official = TRUE;

CREATE INDEX IF NOT EXISTS idx_circles_kyc_required
  ON circles (kyc_required, created_at DESC)
  WHERE kyc_required = TRUE;

CREATE INDEX IF NOT EXISTS idx_circles_join_policy
  ON circles (join_policy, created_at DESC);

CREATE TABLE IF NOT EXISTS community_reputation_flair_grants (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL,
  flair_code VARCHAR(50) NOT NULL CHECK (flair_code IN (
    'verified_investor',
    'asset_holder',
    'helpful_contributor',
    'founder_member',
    'long_term_member',
    'ama_speaker',
    'official_poool',
    'real_estate_analyst',
    'commodity_expert'
  )),
  label VARCHAR(80) NOT NULL,
  granted_by UUID,
  source VARCHAR(32) NOT NULL DEFAULT 'system'
    CHECK (source IN ('system', 'admin', 'asset', 'kyc', 'event')),
  scope_circle_id UUID REFERENCES circles(id) ON DELETE CASCADE,
  scope_asset_id UUID,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  revoked_at TIMESTAMPTZ,
  CONSTRAINT community_reputation_flair_label_nonempty CHECK (length(trim(label)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_reputation_flairs_user_active
  ON community_reputation_flair_grants (user_id, flair_code)
  WHERE is_active = TRUE;

CREATE INDEX IF NOT EXISTS idx_reputation_flairs_scope_circle
  ON community_reputation_flair_grants (scope_circle_id, flair_code)
  WHERE is_active = TRUE AND scope_circle_id IS NOT NULL;

COMMENT ON TABLE community_reputation_flair_grants IS
  'Admin/system-granted reputation flairs. These are participation and verification signals only, never investment advice or performance claims.';
COMMENT ON COLUMN community_reputation_flair_grants.flair_code IS
  'Allowlisted reputation signal such as verified_investor, asset_holder, official_poool, or domain expert.';
COMMENT ON COLUMN community_reputation_flair_grants.metadata IS
  'Audit context for the grant; do not store raw KYC documents or sensitive PII here.';

COMMIT;
