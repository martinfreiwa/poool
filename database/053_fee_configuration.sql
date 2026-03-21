-- Migration 053: Create fee_configurations + fee_promotions tables
-- Purpose: 4-tier fee hierarchy: Promotion > Developer Deal > Asset > Platform Default
-- Ref: Masterplan §4.2 Mig053

-- Platform- and asset-specific fees
CREATE TABLE fee_configurations (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scope         VARCHAR(15) NOT NULL CHECK (scope IN ('platform', 'asset', 'developer')),
    asset_id      UUID REFERENCES assets(id),           -- NULL for scope='platform'
    developer_id  UUID REFERENCES users(id),             -- NULL except for scope='developer'
    taker_fee_bps INTEGER NOT NULL DEFAULT 500           -- 500 = 5.00%
                  CHECK (taker_fee_bps >= 0 AND taker_fee_bps <= 1000),
    maker_fee_bps INTEGER NOT NULL DEFAULT 0             -- 0 = 0.00%
                  CHECK (maker_fee_bps >= 0 AND maker_fee_bps <= 1000),
    is_active     BOOLEAN NOT NULL DEFAULT true,
    reason        TEXT,                                   -- Reason for override
    created_by    UUID REFERENCES users(id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Only one active entry per scope+reference
    CONSTRAINT uq_fee_scope UNIQUE (scope, asset_id, developer_id, is_active)
);

-- Time-bounded promotions (highest priority in fee lookup)
CREATE TABLE fee_promotions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name          VARCHAR(100) NOT NULL,
    scope         VARCHAR(15) NOT NULL CHECK (scope IN ('global', 'asset')),
    asset_id      UUID REFERENCES assets(id),            -- NULL for scope='global'
    taker_fee_bps INTEGER NOT NULL CHECK (taker_fee_bps >= 0),
    maker_fee_bps INTEGER NOT NULL CHECK (maker_fee_bps >= 0),
    starts_at     TIMESTAMPTZ NOT NULL,
    ends_at       TIMESTAMPTZ NOT NULL,
    is_active     BOOLEAN NOT NULL DEFAULT true,
    created_by    UUID REFERENCES users(id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_promo_dates CHECK (ends_at > starts_at)
);

-- Active promotions lookup
CREATE INDEX idx_promo_active ON fee_promotions(starts_at, ends_at)
    WHERE is_active = true;
