-- Migration 185: per-program affiliate config.
--
-- Phase-3 P1: pulls the hard-coded `INTERVAL '30 days'` (holdback) and
-- `90 days` (cookie / first-conversion window) literals out of SQL +
-- middleware and into a configurable `affiliate_programs` row.
--
-- For MVP: one row called 'default' holds the platform-wide values.
-- A future multi-program rollout (campaign-specific terms) can add rows
-- and reference them via a new `program_id` column on
-- `affiliate_referrals` / `affiliate_commissions`.
--
-- Idempotent. Safe to re-run.

CREATE TABLE IF NOT EXISTS affiliate_programs (
    id                              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Short stable handle used in code (e.g. `default`, `holiday-2026`).
    -- Lowercase letters / digits / hyphens only — enforced for slug-safe
    -- references in URLs and config files.
    program_key                     VARCHAR(40) NOT NULL UNIQUE
                                    CHECK (program_key ~ '^[a-z0-9-]+$'),
    display_name                    VARCHAR(120) NOT NULL,
    -- Conversion-window settings.
    -- Days between click and the first qualifying conversion before the
    -- attribution is dropped. Cookie max-age + middleware `?ref=`
    -- behavior should match this value when read by the click pipeline.
    attribution_window_days         INTEGER NOT NULL DEFAULT 90
                                    CHECK (attribution_window_days BETWEEN 1 AND 365),
    -- Holdback window — days from first commission before the row can be
    -- promoted from `under_holdback` to `qualified`. 30 days matches the
    -- platform's refund-protection contract.
    holdback_days                   INTEGER NOT NULL DEFAULT 30
                                    CHECK (holdback_days BETWEEN 0 AND 180),
    -- Volume window for tier progression (12 months default).
    tier_volume_window_days         INTEGER NOT NULL DEFAULT 365
                                    CHECK (tier_volume_window_days BETWEEN 30 AND 730),
    -- Payout configuration.
    min_payout_cents                BIGINT NOT NULL DEFAULT 5000
                                    CHECK (min_payout_cents >= 0),
    currency                        CHAR(3) NOT NULL DEFAULT 'EUR'
                                    CHECK (currency ~ '^[A-Z]{3}$'),
    -- Active/disabled. A disabled program continues to honor in-flight
    -- referrals but stops accepting new clicks.
    is_active                       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed the default program. ON CONFLICT to keep the migration re-runnable.
INSERT INTO affiliate_programs
    (program_key, display_name, attribution_window_days,
     holdback_days, tier_volume_window_days, min_payout_cents, currency)
VALUES
    ('default', 'POOOL Partner Syndicate', 90, 30, 365, 5000, 'EUR')
ON CONFLICT (program_key) DO NOTHING;

CREATE TRIGGER set_affiliate_programs_updated_at
    BEFORE UPDATE ON affiliate_programs
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

COMMENT ON TABLE affiliate_programs IS
  'Phase-3 P1 per-program config (holdback / attribution / tier-window / min-payout). One ''default'' row seeded; multi-program support is a future iteration.';
