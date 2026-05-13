-- 145 — Villa-Returns P2.7: link dividend_payouts to the source operations period.
--
-- Adds three nullable columns so existing rows (non-villa-returns payouts) are
-- unaffected. The partial unique index enforces idempotency for rental payouts
-- tied to a specific monthly period — re-running distribute on a published
-- row cannot double-pay an investor.

ALTER TABLE dividend_payouts
  ADD COLUMN IF NOT EXISTS source_villa_operations_log_id BIGINT REFERENCES villa_operations_log(id),
  ADD COLUMN IF NOT EXISTS period_year                    INTEGER,
  ADD COLUMN IF NOT EXISTS period_month                   INTEGER;

-- One rental payout per (asset, user, period). Other payout_types and
-- non-period payouts are unaffected by this partial index.
CREATE UNIQUE INDEX IF NOT EXISTS uq_dividend_payouts_villa_period
    ON dividend_payouts (asset_id, user_id, period_year, period_month)
    WHERE period_year IS NOT NULL AND payout_type = 'rental';

CREATE INDEX IF NOT EXISTS idx_dividend_payouts_source_log
    ON dividend_payouts (source_villa_operations_log_id)
    WHERE source_villa_operations_log_id IS NOT NULL;

COMMENT ON COLUMN dividend_payouts.source_villa_operations_log_id IS 'Villa-Returns P2.7: links a rental payout back to the villa_operations_log row that funded it.';
COMMENT ON COLUMN dividend_payouts.period_year  IS 'Villa-Returns P2.7: period the payout funds (matches villa_operations_log).';
COMMENT ON COLUMN dividend_payouts.period_month IS 'Villa-Returns P2.7: period the payout funds (matches villa_operations_log).';
