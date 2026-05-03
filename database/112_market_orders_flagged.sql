-- 112_market_orders_flagged.sql
-- Server-side anomaly flagging for marketplace orders.
--
-- Adds nullable `flagged_at` + `flag_reason` columns. Set by:
--   • backend matching engine when order trips a heuristic (large hold,
--     multi-order user, internal counterparty, stale > 7d)
--   • a future nightly job for retrospective re-flagging
--
-- Frontend uses these to render badges without re-deriving on every render.

ALTER TABLE market_orders
    ADD COLUMN IF NOT EXISTS flagged_at  TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS flag_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_market_orders_flagged
    ON market_orders (flagged_at)
    WHERE flagged_at IS NOT NULL;

COMMENT ON COLUMN market_orders.flagged_at  IS 'Set when order trips an anomaly heuristic. NULL = not flagged.';
COMMENT ON COLUMN market_orders.flag_reason IS 'Short label, e.g. "large_hold", "multi_order_user", "internal_counterparty", "stale_7d".';
