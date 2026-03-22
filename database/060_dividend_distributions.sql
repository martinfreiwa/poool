-- Migration 060: Dividend Distributions & Payouts Extension
-- Purpose: Add dividend_distributions table and extend existing dividend_payouts
-- Ref: Phase 8A.5 / Phase 9 — Dividend System (§3.2.10 Masterplan)
--
-- EXISTING: dividend_payouts already exists (from earlier migration) with columns:
--   id, investment_id, user_id, asset_id, amount_cents, payout_type, status,
--   scheduled_at, paid_at, wallet_tx_id, created_at
--
-- NEW: dividend_distributions (master table for distribution cycles)
-- EXTEND: dividend_payouts gets new columns for anti-sniping and distribution linking
--
-- 🔴 FINANCIAL TABLE — all monetary values are BIGINT cents. No floats.
-- 🔴 All payout execution MUST happen inside a DB transaction.

-- ═══════════════════════════════════════════════════════════════
-- 1. Dividend Distributions (one per asset per period)
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS dividend_distributions (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id             UUID NOT NULL REFERENCES assets(id),
    period_start         DATE NOT NULL,
    period_end           DATE NOT NULL,
    total_amount_cents   BIGINT NOT NULL CHECK (total_amount_cents > 0),
    -- Snapshot metadata
    snapshot_block       BIGINT,                    -- On-chain block at snapshot time
    snapshot_at          TIMESTAMPTZ NOT NULL,       -- When the snapshot was taken
    total_tokens_snapshot INTEGER NOT NULL DEFAULT 0, -- Total tokens in circulation at snapshot
    eligible_holders     INTEGER NOT NULL DEFAULT 0, -- Number of eligible holders
    -- Anti-sniping: minimum holding period in days (0 = disabled)
    min_holding_days     INTEGER NOT NULL DEFAULT 0,
    -- Lifecycle
    status               VARCHAR(20) NOT NULL DEFAULT 'draft'
                         CHECK (status IN ('draft', 'calculated', 'approved', 'distributed', 'cancelled')),
    calculated_at        TIMESTAMPTZ,
    approved_by          UUID REFERENCES users(id),
    approved_at          TIMESTAMPTZ,
    distributed_at       TIMESTAMPTZ,
    cancelled_at         TIMESTAMPTZ,
    cancel_reason        TEXT,
    -- Audit
    created_by           UUID REFERENCES users(id),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Sanity: period_end must be after period_start
    CONSTRAINT chk_dividend_period CHECK (period_end > period_start),
    -- Prevent duplicate distributions for same asset+period
    CONSTRAINT uq_dividend_asset_period UNIQUE (asset_id, period_start, period_end)
);

CREATE INDEX IF NOT EXISTS idx_dividend_dist_asset ON dividend_distributions(asset_id);
CREATE INDEX IF NOT EXISTS idx_dividend_dist_status ON dividend_distributions(status)
    WHERE status IN ('draft', 'calculated', 'approved');

-- ═══════════════════════════════════════════════════════════════
-- 2. Extend existing dividend_payouts with distribution linkage
-- ═══════════════════════════════════════════════════════════════

-- Link payouts to a distribution cycle
ALTER TABLE dividend_payouts
    ADD COLUMN IF NOT EXISTS distribution_id UUID REFERENCES dividend_distributions(id);

-- Anti-sniping columns
ALTER TABLE dividend_payouts
    ADD COLUMN IF NOT EXISTS tokens_held INTEGER;

ALTER TABLE dividend_payouts
    ADD COLUMN IF NOT EXISTS percentage_bps INTEGER;

ALTER TABLE dividend_payouts
    ADD COLUMN IF NOT EXISTS first_acquired_at TIMESTAMPTZ;

ALTER TABLE dividend_payouts
    ADD COLUMN IF NOT EXISTS holding_days INTEGER NOT NULL DEFAULT 0;

ALTER TABLE dividend_payouts
    ADD COLUMN IF NOT EXISTS eligible BOOLEAN NOT NULL DEFAULT true;

-- Index for distribution-based lookups
CREATE INDEX IF NOT EXISTS idx_div_payouts_dist ON dividend_payouts(distribution_id)
    WHERE distribution_id IS NOT NULL;

-- ═══════════════════════════════════════════════════════════════
-- 3. Platform settings for dividends
-- ═══════════════════════════════════════════════════════════════

INSERT INTO platform_settings (key, value, value_type, description) VALUES
    ('dividend_default_holding_days', '7', 'number', 'Default minimum holding period for dividend eligibility (anti-sniping)'),
    ('dividend_auto_distribute', 'false', 'boolean', 'Auto-distribute dividends after admin approval (vs manual trigger)')
ON CONFLICT (key) DO NOTHING;
