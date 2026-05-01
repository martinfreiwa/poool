-- Migration 096: Persist fee-reserve rate on market_orders
--
-- When a buy order is placed, we hold (price * qty) + (taker_fee_bps share)
-- in `held_balance_cents` so settlement can charge the buyer fee atomically.
--
-- We need to remember WHICH fee_bps was used at creation time, because:
--   - Fee promotions may start/end between order creation and settlement
--   - On partial fills + cancel, we must release exactly what was held
--
-- Store the bps rate (not absolute cents) — exact cent reserve is
-- recomputed from `(price_cents * remaining_qty * bps) / 10000`.
ALTER TABLE market_orders
    ADD COLUMN IF NOT EXISTS fee_reserve_bps INTEGER NOT NULL DEFAULT 0
        CHECK (fee_reserve_bps >= 0 AND fee_reserve_bps <= 10000);

COMMENT ON COLUMN market_orders.fee_reserve_bps IS
    'Fee rate (basis points) reserved on hold at order creation. For buy
     orders this is the higher of taker/maker bps at the time of creation.
     Used to compute the exact hold release on cancel / partial fill.';
