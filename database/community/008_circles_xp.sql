-- ═══════════════════════════════════════════════════════════════════════
-- Module 4: Circles & XP System
-- M4-DB.1 through M4-DB.4
-- ═══════════════════════════════════════════════════════════════════════

-- ────────────────────────────────────────────────────────────────────────
-- M4-DB.4: ALTER community_profiles to add XP + Circle fields
-- ────────────────────────────────────────────────────────────────────────
ALTER TABLE community_profiles
  ADD COLUMN IF NOT EXISTS circle_id UUID,
  ADD COLUMN IF NOT EXISTS xp_total INTEGER NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS level INTEGER NOT NULL DEFAULT 1,
  ADD COLUMN IF NOT EXISTS level_name VARCHAR(50) NOT NULL DEFAULT 'Seedling';

-- ────────────────────────────────────────────────────────────────────────
-- M4-DB.1: Circles System
-- ────────────────────────────────────────────────────────────────────────
CREATE TABLE circles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(100) NOT NULL,
    description     TEXT CHECK (char_length(description) <= 500),
    owner_id        UUID NOT NULL,          -- Creator / leader of the circle
    avatar_emoji    VARCHAR(10) DEFAULT '🟢',
    member_count    INTEGER NOT NULL DEFAULT 1,
    total_xp        BIGINT NOT NULL DEFAULT 0,
    level           INTEGER NOT NULL DEFAULT 1,
    level_name      VARCHAR(50) NOT NULL DEFAULT 'Sapling',
    is_public       BOOLEAN NOT NULL DEFAULT true,
    max_members     INTEGER NOT NULL DEFAULT 50,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_circles_owner ON circles(owner_id);
CREATE INDEX idx_circles_total_xp ON circles(total_xp DESC);

CREATE TABLE circle_members (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circle_id   UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL,
    role        VARCHAR(20) NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'member')),
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (circle_id, user_id)
);

CREATE INDEX idx_circle_members_user ON circle_members(user_id);
CREATE INDEX idx_circle_members_circle ON circle_members(circle_id);

-- Add FK from community_profiles.circle_id → circles.id
ALTER TABLE community_profiles
  ADD CONSTRAINT fk_cp_circle FOREIGN KEY (circle_id) REFERENCES circles(id) ON DELETE SET NULL;

-- ────────────────────────────────────────────────────────────────────────
-- M4-DB.3: Circle Invites
-- ────────────────────────────────────────────────────────────────────────
CREATE TABLE circle_invites (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circle_id   UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
    inviter_id  UUID NOT NULL,
    invitee_id  UUID NOT NULL,
    status      VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'declined', 'expired')),
    expires_at  TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '7 days'),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (circle_id, invitee_id, status)  -- One pending invite per circle per user
);

CREATE INDEX idx_circle_invites_invitee ON circle_invites(invitee_id, status);

-- ────────────────────────────────────────────────────────────────────────
-- M4-DB.2: XP Ledger (Append-Only)
-- ────────────────────────────────────────────────────────────────────────
CREATE TABLE xp_ledger (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL,
    amount      INTEGER NOT NULL CHECK (amount != 0),
    reason      VARCHAR(50) NOT NULL CHECK (reason IN (
        -- Content actions
        'post_created', 'comment_created', 'reaction_given', 'reaction_received',
        -- Social actions
        'follow_gained', 'profile_completed', 'first_post',
        -- Investment milestones
        'first_investment', 'investment_milestone_5', 'investment_milestone_10',
        'investment_milestone_25', 'investment_milestone_50',
        -- Circle actions
        'circle_created', 'circle_joined', 'circle_invite_accepted',
        -- Streak & engagement
        'daily_login', 'login_streak_7', 'login_streak_30',
        -- Badges
        'badge_earned',
        -- Admin adjustments
        'admin_grant', 'admin_revoke',
        -- Referral
        'referral_signup', 'referral_first_investment',
        -- Misc
        'onboarding_complete'
    )),
    description TEXT,
    metadata    JSONB,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_xp_ledger_user ON xp_ledger(user_id, created_at DESC);
CREATE INDEX idx_xp_ledger_created ON xp_ledger(created_at DESC);

-- ────────────────────────────────────────────────────────────────────────
-- XP Level Definitions (reference table)
-- ────────────────────────────────────────────────────────────────────────
CREATE TABLE xp_levels (
    level       INTEGER PRIMARY KEY,
    name        VARCHAR(50) NOT NULL,
    min_xp      INTEGER NOT NULL,
    icon        VARCHAR(10) NOT NULL DEFAULT '🌱'
);

INSERT INTO xp_levels (level, name, min_xp, icon) VALUES
(1,  'Seedling',       0,     '🌱'),
(2,  'Sprout',         100,   '🌿'),
(3,  'Sapling',        300,   '🌳'),
(4,  'Grower',         600,   '🪴'),
(5,  'Harvester',      1000,  '🌾'),
(6,  'Cultivator',     1500,  '🧑‍🌾'),
(7,  'Expert',         2500,  '📊'),
(8,  'Strategist',     4000,  '🎯'),
(9,  'Mogul',          6000,  '💎'),
(10, 'Legend',         10000, '👑');
