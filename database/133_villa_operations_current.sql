-- 133 — Villa-Returns P1: trigger-maintained "current" view of villa_operations_log.
--
-- Read-side optimization: one row per (asset_id, period_year, period_month) pointing
-- at the latest published log row. Keeps investor-facing reads O(1).
-- Maintained by trigger on villa_operations_log: AFTER INSERT or AFTER UPDATE OF status.

CREATE TABLE IF NOT EXISTS villa_operations_current (
    asset_id                        UUID    NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    period_year                     INTEGER NOT NULL,
    period_month                    INTEGER NOT NULL,
    log_id                          BIGINT  NOT NULL REFERENCES villa_operations_log(id),
    distributable_idr_cents         BIGINT  NOT NULL,
    distributable_usd_cents         BIGINT  NOT NULL,
    net_rental_income_idr_cents     BIGINT  NOT NULL,
    net_rental_income_usd_cents     BIGINT  NOT NULL,
    occupancy_bps                   INTEGER NOT NULL,
    published_at                    TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (asset_id, period_year, period_month)
);

CREATE INDEX IF NOT EXISTS idx_voc_published_at
    ON villa_operations_current (published_at DESC);

-- AFTER trigger: when a log row enters status='published', upsert _current and supersede the prior row.
CREATE OR REPLACE FUNCTION fn_villa_operations_current_upsert()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'published' AND (TG_OP = 'INSERT' OR OLD.status IS DISTINCT FROM 'published') THEN
        INSERT INTO villa_operations_current (
            asset_id, period_year, period_month, log_id,
            distributable_idr_cents, distributable_usd_cents,
            net_rental_income_idr_cents, net_rental_income_usd_cents,
            occupancy_bps, published_at
        ) VALUES (
            NEW.asset_id, NEW.period_year, NEW.period_month, NEW.id,
            NEW.distributable_idr_cents, NEW.distributable_usd_cents,
            NEW.net_rental_income_idr_cents, NEW.net_rental_income_usd_cents,
            NEW.occupancy_bps, COALESCE(NEW.published_at, NOW())
        )
        ON CONFLICT (asset_id, period_year, period_month) DO UPDATE
        SET log_id                       = EXCLUDED.log_id,
            distributable_idr_cents      = EXCLUDED.distributable_idr_cents,
            distributable_usd_cents      = EXCLUDED.distributable_usd_cents,
            net_rental_income_idr_cents  = EXCLUDED.net_rental_income_idr_cents,
            net_rental_income_usd_cents  = EXCLUDED.net_rental_income_usd_cents,
            occupancy_bps                = EXCLUDED.occupancy_bps,
            published_at                 = EXCLUDED.published_at;

        IF NEW.supersedes_id IS NOT NULL THEN
            UPDATE villa_operations_log
               SET status = 'superseded'
             WHERE id = NEW.supersedes_id
               AND status = 'published';
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_villa_operations_current_upsert ON villa_operations_log;
CREATE TRIGGER trg_villa_operations_current_upsert
    AFTER INSERT OR UPDATE OF status ON villa_operations_log
    FOR EACH ROW EXECUTE FUNCTION fn_villa_operations_current_upsert();

COMMENT ON TABLE villa_operations_current IS 'Materialised view of the latest published row per (asset, period). Maintained by AFTER trigger on villa_operations_log. Investor-facing reads target this table.';
