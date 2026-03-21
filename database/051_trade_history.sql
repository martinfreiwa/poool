-- Migration 051: Create trade_history table
-- Purpose: Immutable log of all executed trades (NEVER updated or deleted)
-- Ref: Masterplan §4.2 Mig051

CREATE TABLE trade_history (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id        UUID NOT NULL REFERENCES assets(id),
    buy_order_id    UUID NOT NULL REFERENCES market_orders(id),
    sell_order_id   UUID NOT NULL REFERENCES market_orders(id),
    buyer_user_id   UUID NOT NULL REFERENCES users(id),
    seller_user_id  UUID NOT NULL REFERENCES users(id),
    price_cents     BIGINT NOT NULL CHECK (price_cents > 0),
    quantity        INTEGER NOT NULL CHECK (quantity > 0),
    total_cents     BIGINT GENERATED ALWAYS AS (price_cents * quantity) STORED,
    fee_cents       BIGINT NOT NULL DEFAULT 0 CHECK (fee_cents >= 0),
    fee_bps         INTEGER NOT NULL DEFAULT 0,        -- Fee in basis points (for audit trail)

    -- On-chain settlement status
    on_chain_status VARCHAR(15) NOT NULL DEFAULT 'pending'
                    CHECK (on_chain_status IN ('pending', 'submitted', 'confirmed', 'failed')),
    on_chain_tx_hash VARCHAR(66),                       -- 0x + 64 hex chars
    on_chain_batch_id UUID,                             -- Reference to settlement batch

    executed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Invariant: buyer and seller must be different (no self-trades)
    CONSTRAINT chk_no_self_trade CHECK (buyer_user_id != seller_user_id)
);

-- Performance indexes
CREATE INDEX idx_trade_asset_time ON trade_history(asset_id, executed_at DESC);
CREATE INDEX idx_trade_buyer ON trade_history(buyer_user_id, executed_at DESC);
CREATE INDEX idx_trade_seller ON trade_history(seller_user_id, executed_at DESC);
CREATE INDEX idx_trade_onchain ON trade_history(on_chain_status)
    WHERE on_chain_status IN ('pending', 'submitted');
