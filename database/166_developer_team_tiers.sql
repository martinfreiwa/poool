-- 166_developer_team_tiers.sql
-- Team-level tier + commission rate on developer_teams.
--
-- Context: until this migration, ALL affiliate commissions used a single
-- `affiliates.commission_rate_bps` field on the payout user. For
-- team_business links this was the developer's PERSONAL tier rate — even
-- though conceptually a developer's TEAM has its own performance volume
-- that should drive a DIFFERENT rate.
--
-- This migration introduces:
--   * developer_teams.current_team_tier      — current tier name (mirrors affiliate_tiers.name)
--   * developer_teams.team_commission_rate_bps — rate currently paid on team_business links
--   * developer_teams.team_volume_12m_cents  — cached team-only qualified volume (last 12m)
--   * developer_teams.team_tier_updated_at   — last evaluation timestamp
--
-- The ladder definitions are reused from `affiliate_tiers` — same 8 tiers
-- (Access … Sovereign) and same volume thresholds, just evaluated against
-- TEAM-aggregated team_business-link volume instead of personal volume.
--
-- Backfill: computes initial tier per team from existing data, so the
-- new commission-compute branch (link_type='team_business' → team rate)
-- has correct values immediately after deploy.

BEGIN;

-- ─── 1. Add columns ───────────────────────────────────────────────────────
ALTER TABLE developer_teams
    ADD COLUMN IF NOT EXISTS current_team_tier        VARCHAR(32) NOT NULL DEFAULT 'Access',
    ADD COLUMN IF NOT EXISTS team_commission_rate_bps INT         NOT NULL DEFAULT 50,
    ADD COLUMN IF NOT EXISTS team_volume_12m_cents    BIGINT      NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS team_tier_updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Loose FK-by-convention: current_team_tier should match an affiliate_tiers.name.
-- Not enforced as FK to keep the worker write-path light and avoid lock-stepping
-- a team rate change with a tier-table edit, but documented here.
COMMENT ON COLUMN developer_teams.current_team_tier IS
    'Mirrors affiliate_tiers.name. Drives team_commission_rate_bps for team_business links.';
COMMENT ON COLUMN developer_teams.team_commission_rate_bps IS
    'Commission rate (basis points) paid to developer on every team_business-link conversion.';
COMMENT ON COLUMN developer_teams.team_volume_12m_cents IS
    'Cached sum of purchase_value_cents over qualified affiliate_referrals where link_type=team_business in last 12 months.';

-- ─── 2. Helper function: recompute one team's tier ─────────────────────────
-- Idempotent. Reads current team-business-link volume (rolling 12m), looks
-- up the highest matching ladder entry, writes back to developer_teams.
CREATE OR REPLACE FUNCTION recompute_team_tier(p_team_id UUID)
RETURNS TABLE(team_id UUID, old_tier TEXT, new_tier TEXT, old_bps INT, new_bps INT, volume_cents BIGINT)
LANGUAGE plpgsql AS $$
DECLARE
    v_old_tier TEXT;
    v_old_bps  INT;
    v_volume   BIGINT;
    v_new_tier TEXT;
    v_new_bps  INT;
BEGIN
    SELECT current_team_tier, team_commission_rate_bps
        INTO v_old_tier, v_old_bps
        FROM developer_teams WHERE id = p_team_id;

    -- Volume = sum of investments for qualified team-business referrals,
    -- rolling 12 months. Only count statuses that have actually monetised
    -- (qualified, paid, under_holdback) — pending/registered don't count.
    SELECT COALESCE(SUM(i.purchase_value_cents), 0)::BIGINT INTO v_volume
    FROM affiliate_referrals ar
    JOIN affiliate_links al ON al.id = ar.link_id
    JOIN affiliate_commissions ac ON ac.referral_id = ar.id
    JOIN investments i ON i.id = ac.source_order_id
    WHERE al.team_id = p_team_id
      AND al.link_type = 'team_business'
      AND ar.status IN ('qualified', 'paid', 'under_holdback')
      AND COALESCE(ar.qualified_at, ar.updated_at) >= NOW() - INTERVAL '12 months';

    -- Pick highest tier whose threshold is met.
    SELECT name, commission_rate_bps INTO v_new_tier, v_new_bps
    FROM affiliate_tiers
    WHERE min_volume_cents <= v_volume
    ORDER BY min_volume_cents DESC
    LIMIT 1;

    IF v_new_tier IS NULL THEN
        v_new_tier := 'Access';
        v_new_bps  := 50;
    END IF;

    UPDATE developer_teams
       SET current_team_tier        = v_new_tier,
           team_commission_rate_bps = v_new_bps,
           team_volume_12m_cents    = v_volume,
           team_tier_updated_at     = NOW()
     WHERE id = p_team_id;

    RETURN QUERY SELECT p_team_id, v_old_tier, v_new_tier, v_old_bps, v_new_bps, v_volume;
END $$;

COMMENT ON FUNCTION recompute_team_tier IS
    'Recompute and persist a single team''s tier from team_business-link volume (last 12m). Idempotent. Returns before/after.';

-- ─── 3. Backfill: compute initial tier for every existing team ─────────────
DO $$
DECLARE r RECORD;
BEGIN
    FOR r IN SELECT id FROM developer_teams WHERE status <> 'terminated'
    LOOP
        PERFORM recompute_team_tier(r.id);
    END LOOP;
END $$;

-- ─── 4. Audit log helper ──────────────────────────────────────────────────
-- We piggyback on the existing affiliate audit-log table if it exists, else
-- create a small one to track team tier transitions.
CREATE TABLE IF NOT EXISTS developer_team_tier_history (
    id           BIGSERIAL PRIMARY KEY,
    team_id      UUID NOT NULL REFERENCES developer_teams(id),
    old_tier     VARCHAR(32),
    new_tier     VARCHAR(32) NOT NULL,
    old_bps      INT,
    new_bps      INT         NOT NULL,
    volume_cents BIGINT      NOT NULL,
    changed_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_dtth_team_time
    ON developer_team_tier_history (team_id, changed_at DESC);

COMMENT ON TABLE developer_team_tier_history IS
    'Append-only log of every team-tier promotion or demotion. Insert from worker after recompute_team_tier.';

COMMIT;
