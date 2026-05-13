-- 141 — Villa-Returns P1: developer-submitted forecast suggestions awaiting admin merge (Q4).
--
-- Per W10: developer suggests forecast values, admin reviews + finalises on A2 asset-tokenize.
-- Multiple suggestions per (asset, forecast_year) allowed across time; admin picks/merges
-- the active version into villa_forecast_assumptions.

CREATE TABLE IF NOT EXISTS villa_forecast_suggestions (
    id                              BIGSERIAL PRIMARY KEY,
    asset_id                        UUID    NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    forecast_year                   INTEGER NOT NULL CHECK (forecast_year BETWEEN 2000 AND 2100),
    projected_occupancy_bps         INTEGER CHECK (projected_occupancy_bps IS NULL OR (projected_occupancy_bps BETWEEN 0 AND 10000)),
    projected_adr_idr_cents         BIGINT  CHECK (projected_adr_idr_cents IS NULL OR projected_adr_idr_cents >= 0),
    projected_rent_growth_bps       INTEGER,
    projected_expense_inflation_bps INTEGER,
    projected_appreciation_bps      INTEGER,
    projected_exit_yield_bps        INTEGER CHECK (projected_exit_yield_bps IS NULL OR (projected_exit_yield_bps BETWEEN 0 AND 100000)),
    notes                           TEXT,

    status                          VARCHAR(20) NOT NULL DEFAULT 'submitted'
                                    CHECK (status IN ('submitted','accepted','discarded')),
    submitted_by                    UUID REFERENCES users(id),
    submitted_at                    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_by                    UUID REFERENCES users(id),
    processed_at                    TIMESTAMPTZ,
    processed_outcome_notes         TEXT,
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_vfs_asset_year_status
    ON villa_forecast_suggestions (asset_id, forecast_year DESC, status);

CREATE INDEX IF NOT EXISTS idx_vfs_pending
    ON villa_forecast_suggestions (status, submitted_at) WHERE status = 'submitted';

COMMENT ON TABLE villa_forecast_suggestions IS 'Developer forecast suggestions awaiting admin review. Admin merges accepted suggestions into villa_forecast_assumptions.';
