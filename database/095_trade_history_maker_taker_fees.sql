-- Migration 095: Split fee tracking into maker / taker columns
--
-- Problem: `trade_history.fee_cents` is a single field. The original
-- settlement code charged only the seller (hardcoded as taker), and the
-- fee shown to the buyer was misleading. Fees should follow standard
-- exchange semantics:
--   - taker (crossed the spread) pays taker_fee_bps
--   - maker (resting order) pays maker_fee_bps (often 0 or negative rebate)
--
-- The legacy `fee_cents` and `fee_bps` columns are kept (filled with the
-- TAKER side's fee for backward compat) but the source of truth is now
-- the per-side breakdown.

ALTER TABLE trade_history
    ADD COLUMN IF NOT EXISTS taker_side VARCHAR(4)
        CHECK (taker_side IN ('buy', 'sell'));

ALTER TABLE trade_history
    ADD COLUMN IF NOT EXISTS buyer_fee_cents BIGINT NOT NULL DEFAULT 0
        CHECK (buyer_fee_cents >= 0);

ALTER TABLE trade_history
    ADD COLUMN IF NOT EXISTS seller_fee_cents BIGINT NOT NULL DEFAULT 0
        CHECK (seller_fee_cents >= 0);

ALTER TABLE trade_history
    ADD COLUMN IF NOT EXISTS buyer_fee_bps INTEGER NOT NULL DEFAULT 0;

ALTER TABLE trade_history
    ADD COLUMN IF NOT EXISTS seller_fee_bps INTEGER NOT NULL DEFAULT 0;

-- Backfill existing rows: legacy fee was charged to the seller (acting as
-- hardcoded taker). Mirror that into the new columns so historical
-- accounting reads correctly.
UPDATE trade_history
SET seller_fee_cents = fee_cents,
    seller_fee_bps   = fee_bps,
    taker_side       = 'sell'
WHERE taker_side IS NULL
  AND fee_cents > 0;

UPDATE trade_history
SET taker_side = 'sell'
WHERE taker_side IS NULL;

COMMENT ON COLUMN trade_history.buyer_fee_cents IS
    'Fee charged to the buyer side (taker or maker depending on taker_side).';
COMMENT ON COLUMN trade_history.seller_fee_cents IS
    'Fee charged to the seller side (taker or maker depending on taker_side).';
COMMENT ON COLUMN trade_history.taker_side IS
    'Which side crossed the spread. The other side is the maker.';
