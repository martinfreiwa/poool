-- Migration 101: Time-in-force on market_orders.
--
-- Standard exchange TIF values:
--   GTC (good-till-cancelled): default — sit in book until filled, cancelled, or expired
--   IOC (immediate-or-cancel): match what's possible NOW; cancel any unfilled remainder
--   FOK (fill-or-kill):        all-or-nothing match NOW; cancel everything if not fully fillable
--
-- Currently every order is implicitly GTC (with a 90-day expiry). IOC enables
-- "fill what you can, drop the rest" — useful for sweeping the book without
-- leaving a trailing resting order. FOK is left for a later phase (requires
-- a pre-match liquidity check before any partial fill executes).

ALTER TABLE market_orders
    ADD COLUMN IF NOT EXISTS time_in_force VARCHAR(8) NOT NULL DEFAULT 'gtc'
        CHECK (time_in_force IN ('gtc', 'ioc', 'fok'));

COMMENT ON COLUMN market_orders.time_in_force IS
    'Time-in-force: gtc=good till cancelled (default), ioc=immediate or cancel
     (cancel unfilled remainder after first match cycle), fok=fill or kill
     (planned, not yet implemented).';
