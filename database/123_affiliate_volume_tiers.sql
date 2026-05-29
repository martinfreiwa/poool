-- 123_affiliate_volume_tiers.sql
-- Phase 1 affiliate program: 8-tier ladder driven by qualified referral VOLUME
-- over a rolling 12-month lookback window (blueprint Point 7).
--
-- Replaces the prior count-based, lifetime-accumulating tier scheme that lived
-- in the AFFILIATE_TIERS const in src/rewards/service.rs.
--
-- WARNING: min_volume_cents thresholds below are PLACEHOLDERS. Final values
-- require legal/commercial sign-off (blueprint Point 7 marked these as
-- [LEGAL REVIEW REQUIRED]). Adjust before production launch.

BEGIN;

-- ─── 1. Dedicated affiliate tier table ────────────────────────────────────────
-- Separate from `tiers` (investor cashback), which already uses some of the
-- same names ("Plus", "Pro", "Elite", "Premium") with different semantics.
CREATE TABLE IF NOT EXISTS affiliate_tiers (
    id                   SERIAL PRIMARY KEY,
    name                 VARCHAR(32) NOT NULL UNIQUE,
    commission_rate_bps  INT         NOT NULL,           -- 50 = 0.50%
    min_volume_cents     BIGINT      NOT NULL DEFAULT 0, -- own qualified referral volume in last 12m
    sort_order           INT         NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed blueprint Phase 1 ladder. Volume thresholds are PLACEHOLDERS.
-- min_volume_cents are USD-cents (1 USD = 100 cents).
INSERT INTO affiliate_tiers (name, commission_rate_bps, min_volume_cents, sort_order) VALUES
    ('Access',     50,           0, 1),  -- entry tier, no minimum
    ('Plus',       75,     500000, 2),  -- TODO[legal-review]: ~$5,000
    ('Pro',       100,    1500000, 3),  -- TODO[legal-review]: ~$15,000
    ('Elite',     150,    5000000, 4),  -- TODO[legal-review]: ~$50,000
    ('Premium',   200,   15000000, 5),  -- TODO[legal-review]: ~$150,000
    ('Platinum',  275,   40000000, 6),  -- TODO[legal-review]: ~$400,000
    ('Signature', 350,  100000000, 7),  -- TODO[legal-review]: ~$1,000,000
    ('Sovereign', 450,  250000000, 8)   -- TODO[legal-review]: ~$2,500,000
ON CONFLICT (name) DO UPDATE SET
    commission_rate_bps = EXCLUDED.commission_rate_bps,
    min_volume_cents    = EXCLUDED.min_volume_cents,
    sort_order          = EXCLUDED.sort_order;

CREATE INDEX IF NOT EXISTS idx_affiliate_tiers_volume
    ON affiliate_tiers (min_volume_cents);

-- ─── 2. Track qualified-at timestamp on referrals ─────────────────────────────
-- The 12-month lookback needs an explicit timestamp marking when a referral
-- entered `qualified` status. updated_at is unreliable (any column update
-- bumps it).
ALTER TABLE affiliate_referrals
    ADD COLUMN IF NOT EXISTS qualified_at TIMESTAMPTZ;

-- Backfill: for rows already qualified or paid, treat the most recent
-- updated_at as the qualified_at timestamp. Best-effort only; new rows get
-- the timestamp set explicitly when status transitions to qualified.
UPDATE affiliate_referrals
   SET qualified_at = COALESCE(qualified_at, updated_at)
 WHERE status IN ('qualified', 'paid')
   AND qualified_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_affiliate_referrals_qualified_at
    ON affiliate_referrals (affiliate_id, qualified_at)
 WHERE status IN ('qualified', 'paid');

-- ─── 3. Drop stale assumptions ────────────────────────────────────────────────
-- The old worker treated `current_tier` defaults as the obsolete Bronze/Silver/
-- Gold/Diamond/Ambassador set. Reset any affiliate currently sitting on one of
-- those legacy tier names back to Access — the next worker tick will recompute
-- their tier from 12-month volume against the new thresholds.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM information_schema.columns
        WHERE table_schema = current_schema()
          AND table_name = 'affiliates'
          AND column_name = 'updated_at'
    ) THEN
        UPDATE affiliates
           SET current_tier        = 'Access',
               commission_rate_bps = 50,
               updated_at          = NOW()
         WHERE current_tier IN ('Bronze', 'Silver', 'Gold', 'Diamond', 'Ambassador');
    ELSE
        UPDATE affiliates
           SET current_tier        = 'Access',
               commission_rate_bps = 50
         WHERE current_tier IN ('Bronze', 'Silver', 'Gold', 'Diamond', 'Ambassador');
    END IF;
END $$;

COMMIT;
