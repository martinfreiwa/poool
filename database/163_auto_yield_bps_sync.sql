-- 163 — Auto-sync assets.annual_yield_bps from trailing 12-month distributable
--       after each villa operations publish.
--
-- Extends fn_villa_operations_current_upsert (first created in migration 133) to
-- also write back assets.annual_yield_bps using the same formula that
-- villa_performance.rs uses for the live-performance endpoint:
--
--   pool_value_idr  = valuation_idr_cents × tokenized_pct_bps / 10 000
--   annual_yield_bps = SUM(distributable_idr_cents, last 12 months) × 10 000 / pool_value_idr
--
-- Guard: only runs when a published/superseded villa_valuations row exists
-- (pool_value_idr > 0), so it never zeroes-out a manually-set yield on an
-- asset that has no valuation on record yet.

CREATE OR REPLACE FUNCTION fn_villa_operations_current_upsert()
RETURNS TRIGGER AS $$
DECLARE
    v_last_12m    NUMERIC;
    v_pool_value  NUMERIC;
    v_new_yield   INTEGER;
    v_occupancy   INTEGER;
BEGIN
    IF NEW.status = 'published' AND (TG_OP = 'INSERT' OR OLD.status IS DISTINCT FROM 'published') THEN

        -- ── 1. Upsert villa_operations_current (unchanged from migration 133) ──────
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

        -- Supersede the previous published row if this is a correction
        IF NEW.supersedes_id IS NOT NULL THEN
            UPDATE villa_operations_log
               SET status = 'superseded'
             WHERE id = NEW.supersedes_id
               AND status = 'published';
        END IF;

        -- ── 2. Auto-sync assets.annual_yield_bps from trailing 12-month sum ────────
        --
        -- Trailing 12-month distributable (includes the row just upserted above).
        SELECT COALESCE(SUM(distributable_idr_cents), 0)::NUMERIC
          INTO v_last_12m
          FROM (
              SELECT distributable_idr_cents
                FROM villa_operations_current
               WHERE asset_id = NEW.asset_id
               ORDER BY period_year DESC, period_month DESC
               LIMIT 12
          ) last12;

        -- Pool value in IDR cents: latest valuation × tokenized fraction.
        -- Mirrors villa_performance.rs:
        --   pool_value_idr = valuation_idr_cents * tokenized_pct_bps / 10_000
        SELECT (vv.valuation_idr_cents::NUMERIC * a.tokenized_pct_bps / 10000)
          INTO v_pool_value
          FROM assets a
          JOIN villa_valuations vv ON vv.asset_id = a.id
         WHERE a.id = NEW.asset_id
           AND vv.status IN ('published', 'superseded')
         ORDER BY vv.valuation_date DESC, vv.id DESC
         LIMIT 1;

        -- ── 3. Auto-sync assets.occupancy_rate_bps from latest published period ───
        -- Always update — occupancy is a simple fact, no valuation dependency.
        SELECT occupancy_bps INTO v_occupancy
          FROM villa_operations_current
         WHERE asset_id = NEW.asset_id
         ORDER BY period_year DESC, period_month DESC
         LIMIT 1;

        IF v_occupancy IS NOT NULL THEN
            UPDATE assets SET occupancy_rate_bps = v_occupancy WHERE id = NEW.asset_id;
        END IF;

        -- Only update yield when both data sources are valid:
        --   • v_pool_value IS NULL  → no valuation on record  → leave existing value alone
        --   • v_pool_value = 0      → tokenized_pct_bps = 0   → leave existing value alone
        --   • v_last_12m = 0        → no distributions yet    → write 0 (genuine 0% yield)
        IF v_pool_value IS NOT NULL AND v_pool_value > 0 THEN
            v_new_yield := LEAST(
                FLOOR((v_last_12m * 10000) / v_pool_value)::INTEGER,
                50000  -- hard cap at 500 % to guard against data entry errors
            );
            UPDATE assets
               SET annual_yield_bps = v_new_yield
             WHERE id = NEW.asset_id;
        END IF;

    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Re-register the trigger (same name/condition as migration 133; DROP+CREATE is idempotent).
DROP TRIGGER IF EXISTS trg_villa_operations_current_upsert ON villa_operations_log;
CREATE TRIGGER trg_villa_operations_current_upsert
    AFTER INSERT OR UPDATE OF status ON villa_operations_log
    FOR EACH ROW EXECUTE FUNCTION fn_villa_operations_current_upsert();

COMMENT ON FUNCTION fn_villa_operations_current_upsert() IS
    'Maintains villa_operations_current on publish (migration 133), then auto-syncs '
    'assets.annual_yield_bps from trailing 12-month distributable (migration 163). '
    'Formula matches villa_performance.rs. No-op when no villa_valuations row exists.';
