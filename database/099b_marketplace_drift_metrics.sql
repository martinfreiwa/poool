-- Migration 099: Persist marketplace Redis↔DB drift metrics.
--
-- The Redis sync worker already detects and fixes drift, but it logs only.
-- An admin dashboard / alerting needs persistent metrics so that:
--   - Drift trends over time are visible
--   - Sentry alerts can fire on `missing > N` for `T` consecutive cycles
--   - Operators can see at a glance whether the orderbook is healthy
--
-- One row per sync cycle. `metric_type` distinguishes
-- 'missing_in_redis' / 'stale_in_redis' / 'queue_depth' / etc.

CREATE TABLE IF NOT EXISTS marketplace_drift_metrics (
    id            BIGSERIAL PRIMARY KEY,
    metric_type   VARCHAR(32) NOT NULL,
    value         BIGINT NOT NULL,
    recorded_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_marketplace_drift_metrics_recorded_at
    ON marketplace_drift_metrics (metric_type, recorded_at DESC);

COMMENT ON TABLE marketplace_drift_metrics IS
    'Time-series of Redis↔PostgreSQL drift counts from the marketplace
     sync worker. Used by the admin dashboard and alerting.
     Retention: prune rows older than 30 days via an admin script.';
