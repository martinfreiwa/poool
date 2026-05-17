-- Migration 069: Apply missing community schema columns
-- Fixes runtime SQL errors caused by missing columns in community_profiles and notifications
-- These columns were defined in database/community/*.sql migrations but never applied to main schema.
--
-- 2026-05-17 — production split the community schema into its own
-- `poool_community` database. On that DB this migration runs as-written.
-- On the legacy single-DB topology (and on the affiliate-integration CI
-- runner which only loads `database/*.sql`) the `community_profiles`
-- table simply isn't there, so we skip the table-touching blocks rather
-- than ERROR. The CREATE TABLE IF NOT EXISTS blocks below still run
-- because they're harmless when the table is already absent.
BEGIN;

-- 1. Add XP + Level + Circle columns to community_profiles (skip on DBs
-- that don't have the community schema — keeps single-DB CI green).
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = current_schema()
          AND table_name = 'community_profiles'
    ) THEN
        ALTER TABLE community_profiles
          ADD COLUMN IF NOT EXISTS circle_id UUID,
          ADD COLUMN IF NOT EXISTS xp_total INTEGER NOT NULL DEFAULT 0,
          ADD COLUMN IF NOT EXISTS level INTEGER NOT NULL DEFAULT 1,
          ADD COLUMN IF NOT EXISTS level_name VARCHAR(50) NOT NULL DEFAULT 'Seedling';
    END IF;
END $$;

-- 2. Add community-specific columns to notifications
ALTER TABLE notifications
  ADD COLUMN IF NOT EXISTS actor_id UUID,
  ADD COLUMN IF NOT EXISTS entity_id UUID,
  ADD COLUMN IF NOT EXISTS content TEXT,
  ADD COLUMN IF NOT EXISTS link_url TEXT;

-- Backfill content from message for existing notifications (message = existing column)
UPDATE notifications SET content = message WHERE content IS NULL;

-- 3. Create xp_levels reference table if not exists
CREATE TABLE IF NOT EXISTS xp_levels (
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
(10, 'Legend',         10000, '👑')
ON CONFLICT (level) DO NOTHING;

-- 4. Create xp_ledger table if not exists
CREATE TABLE IF NOT EXISTS xp_ledger (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL,
    amount      INTEGER NOT NULL,
    reason      VARCHAR(100) NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_xp_ledger_user_id ON xp_ledger(user_id);
CREATE INDEX IF NOT EXISTS idx_xp_ledger_user_reason_date ON xp_ledger(user_id, reason, created_at);

-- 5. Create circles table if not exists
CREATE TABLE IF NOT EXISTS circles (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name         VARCHAR(100) NOT NULL,
    description  TEXT,
    owner_id     UUID NOT NULL,
    is_private   BOOLEAN NOT NULL DEFAULT false,
    total_xp     BIGINT NOT NULL DEFAULT 0,
    member_count INTEGER NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 6. Create circle_members table if not exists
CREATE TABLE IF NOT EXISTS circle_members (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circle_id  UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
    user_id    UUID NOT NULL,
    role       VARCHAR(20) NOT NULL DEFAULT 'member',
    joined_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(circle_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_circle_members_user_id ON circle_members(user_id);
CREATE INDEX IF NOT EXISTS idx_circle_members_circle_id ON circle_members(circle_id);

-- 7. Add indexes for new notification columns
CREATE INDEX IF NOT EXISTS idx_notifications_actor_id ON notifications(actor_id) WHERE actor_id IS NOT NULL;

COMMIT;
