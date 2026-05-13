-- 135 — Villa-Returns P1: forecast assumptions per asset per year (PDF §3, §6).
--
-- Q9 lock-in: per-asset, versioned annually. Drives Projected Return, Projected
-- Annualised Net Return, and 5-Year Total Return KPIs (PDF §6). Mutable rows
-- with audit_logs capturing changes (not append-only — overrides expected mid-year).

CREATE TABLE IF NOT EXISTS villa_forecast_assumptions (
    id                              BIGSERIAL PRIMARY KEY,
    asset_id                        UUID    NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    forecast_year                   INTEGER NOT NULL CHECK (forecast_year BETWEEN 2000 AND 2100),
    projected_occupancy_bps         INTEGER CHECK (projected_occupancy_bps IS NULL OR (projected_occupancy_bps BETWEEN 0 AND 10000)),
    projected_adr_idr_cents         BIGINT  CHECK (projected_adr_idr_cents IS NULL OR projected_adr_idr_cents >= 0),
    projected_rent_growth_bps       INTEGER,  -- can be negative
    projected_expense_inflation_bps INTEGER,
    projected_appreciation_bps      INTEGER,
    projected_exit_yield_bps        INTEGER CHECK (projected_exit_yield_bps IS NULL OR (projected_exit_yield_bps BETWEEN 0 AND 100000)),
    projected_annual_net_yield_bps  INTEGER,
    notes                           TEXT,

    finalized_by                    UUID REFERENCES users(id),
    finalized_at                    TIMESTAMPTZ,
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (asset_id, forecast_year)
);

CREATE INDEX IF NOT EXISTS idx_vfa_asset_year
    ON villa_forecast_assumptions (asset_id, forecast_year DESC);

COMMENT ON TABLE villa_forecast_assumptions IS 'Admin-finalised forecast inputs per asset per year. Developer suggestions live in villa_forecast_suggestions and are merged here by admin.';
