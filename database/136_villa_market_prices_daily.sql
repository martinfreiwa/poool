-- 136 — Villa-Returns P1: daily NAV + market price snapshots per asset.
--
-- Powers Share Price Performance +3M / +6M / +12M (PDF §6) and the NAV vs Market
-- price chart on property.html (PDF §8 — never merged into a single series).
-- Populated by the nightly job (P4) at 00:30 UTC; immediate row written on
-- valuation publish or distributable change.

CREATE TABLE IF NOT EXISTS villa_market_prices_daily (
    asset_id            UUID    NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    snapshot_date       DATE    NOT NULL,
    nav_token_idr_cents BIGINT  NOT NULL,
    nav_token_usd_cents BIGINT  NOT NULL,
    market_token_idr_cents BIGINT,
    market_token_usd_cents BIGINT,
    annual_yield_bps    INTEGER,
    trade_count         INTEGER NOT NULL DEFAULT 0 CHECK (trade_count >= 0),
    volume_tokens       BIGINT  NOT NULL DEFAULT 0 CHECK (volume_tokens >= 0),
    fx_rate_idr_to_usd_bps INTEGER NOT NULL DEFAULT 1 CHECK (fx_rate_idr_to_usd_bps > 0),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (asset_id, snapshot_date)
);

CREATE INDEX IF NOT EXISTS idx_vmpd_asset_date
    ON villa_market_prices_daily (asset_id, snapshot_date DESC);

COMMENT ON TABLE villa_market_prices_daily IS 'Daily NAV + Market token price snapshots. PDF §8: NAV and Market are always shown as two separate UI series, never merged.';
