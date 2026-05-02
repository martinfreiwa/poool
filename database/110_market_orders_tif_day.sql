-- Migration 110: Add 'day' to market_orders.time_in_force allowed values.
--
-- Day orders auto-expire at session close. POOOL trades 24/7 (no formal
-- session), so 'day' is implemented as a 24-hour expiry from creation. The
-- existing expiry worker handles cleanup once expires_at < NOW().
--
-- This complements the FOK (fill-or-kill) implementation in service.rs
-- which uses pre-trade depth simulation + the IOC sweep path.
--
-- Idempotent: drop + add the constraint so re-runs are safe.

ALTER TABLE market_orders
    DROP CONSTRAINT IF EXISTS market_orders_time_in_force_check;

ALTER TABLE market_orders
    ADD CONSTRAINT market_orders_time_in_force_check
        CHECK (time_in_force IN ('gtc', 'ioc', 'fok', 'day'));

COMMENT ON COLUMN market_orders.time_in_force IS
    'Time-in-force: gtc=good till cancelled (90d), ioc=immediate or cancel,
     fok=fill or kill (all-or-nothing pre-checked at submit time),
     day=auto-expire 24h from creation (24/7 venue analogue of session close).';
