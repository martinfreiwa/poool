-- 140 — Villa-Returns P1: daily FX rate snapshots.
--
-- Q1 lock-in: villas operate in IDR but UI also shows USD. FX rate is frozen
-- with each published log row to prevent silent revaluation of historical figures.
-- This table is the source the publish step pulls from to freeze the rate.
-- Populated by a small daily job (P4) hitting a public FX API.

CREATE TABLE IF NOT EXISTS fx_rates_daily (
    snapshot_date   DATE NOT NULL,
    base_currency   CHAR(3) NOT NULL,
    quote_currency  CHAR(3) NOT NULL,
    rate_bps        BIGINT  NOT NULL CHECK (rate_bps > 0),
    source          VARCHAR(80),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (snapshot_date, base_currency, quote_currency)
);

CREATE INDEX IF NOT EXISTS idx_fxrd_pair_date
    ON fx_rates_daily (base_currency, quote_currency, snapshot_date DESC);

COMMENT ON TABLE fx_rates_daily IS 'Daily FX snapshots. rate_bps stored as bps to avoid floats. For IDR→USD the rate is small; for high-precision use INTEGER bps with documented scale per pair.';
