-- ═══════════════════════════════════════════════════════════════════════
-- 045: Circles rework — multi-join + slug + moderator role + featured + bans
-- ═══════════════════════════════════════════════════════════════════════
--
-- Closes the MyCircle rework gaps (2026-05-16 user spec):
--   1. App-level "already in a circle, leave first" check is dropped in
--      Rust. UNIQUE(circle_id, user_id) stays — still prevents duplicate
--      join of the SAME circle, which is correct.
--   2. `community_profiles.circle_id` is repurposed as the user's
--      ACTIVE/PRIMARY circle (UI context). On first join it's set; on
--      subsequent joins it's NOT overwritten unless user picks a new
--      primary explicitly. Subsequent unjoins clear it if it pointed to
--      the leaving circle.
--   3. Adds `slug` for pretty URLs (e.g. /community/circle/coffee-pros/settings).
--      Slug is unique among non-terminated circles.
--   4. Adds 'moderator' to the role CHECK so circle owners can promote
--      members to mods with kick/ban/approve/post-moderate rights.
--   5. Adds is_featured flag — admin editorial pinning for the
--      Discovery > Featured section.
--   6. Adds banned_at / banned_by / ban_reason columns on circle_members
--      so a "removed" row can persist as a tombstone for ban enforcement
--      (UNIQUE(circle_id, user_id) means we soft-flag instead of delete).
--      Actually simpler: separate circle_bans table to keep ban history
--      independent of membership lifecycle.

BEGIN;

-- ── 1. Extend role enum to include 'moderator' ───────────────────────
ALTER TABLE circle_members DROP CONSTRAINT IF EXISTS circle_members_role_check;
ALTER TABLE circle_members
    ADD CONSTRAINT circle_members_role_check
    CHECK (role IN ('owner', 'admin', 'moderator', 'member'));

-- ── 2. Slug for pretty URLs ──────────────────────────────────────────
ALTER TABLE circles
    ADD COLUMN IF NOT EXISTS slug VARCHAR(60);

-- Backfill: lowercase name, strip non-alphanumeric, collapse dashes.
-- For collisions append `-<6-char-hex>` suffix derived from circle id.
UPDATE circles
SET slug = LOWER(
    REGEXP_REPLACE(
        REGEXP_REPLACE(name, '[^a-zA-Z0-9]+', '-', 'g'),
        '^-+|-+$', '', 'g'
    )
)
WHERE slug IS NULL;

-- Disambiguate duplicates by appending part of the UUID
UPDATE circles c1 SET slug = c1.slug || '-' || SUBSTR(REPLACE(c1.id::text, '-', ''), 1, 6)
WHERE c1.id IN (
    SELECT c2.id FROM circles c2
    WHERE EXISTS (
        SELECT 1 FROM circles c3
        WHERE c3.slug = c2.slug AND c3.id <> c2.id
    )
);

-- Enforce uniqueness + presence going forward.
ALTER TABLE circles ALTER COLUMN slug SET NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS circles_slug_unique ON circles(LOWER(slug));

-- ── 3. Editorial featured flag ───────────────────────────────────────
ALTER TABLE circles
    ADD COLUMN IF NOT EXISTS is_featured BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS featured_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_circles_featured ON circles(is_featured, featured_at DESC)
    WHERE is_featured = TRUE;

-- ── 4. Trending-helper: posts-per-7d column, refreshed by background job
ALTER TABLE circles
    ADD COLUMN IF NOT EXISTS recent_post_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS recent_post_count_updated_at TIMESTAMPTZ;

-- ── 5. Ban registry (separate from membership) ───────────────────────
CREATE TABLE IF NOT EXISTS circle_bans (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circle_id    UUID NOT NULL REFERENCES circles(id) ON DELETE CASCADE,
    banned_user_id UUID NOT NULL,
    banned_by    UUID NOT NULL,
    reason       TEXT,
    banned_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at   TIMESTAMPTZ,  -- NULL = permanent
    UNIQUE (circle_id, banned_user_id)
);

CREATE INDEX IF NOT EXISTS idx_circle_bans_user ON circle_bans(banned_user_id);
CREATE INDEX IF NOT EXISTS idx_circle_bans_circle ON circle_bans(circle_id);

-- ── 6. Search index for discover/search ──────────────────────────────
-- Trigram index for ILIKE name + description search at low latency.
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE INDEX IF NOT EXISTS idx_circles_name_trgm
    ON circles USING gin(LOWER(name) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_circles_description_trgm
    ON circles USING gin(LOWER(description) gin_trgm_ops)
    WHERE description IS NOT NULL;

COMMIT;
