-- Migration 050: Create market_orders table
-- Purpose: All limit/market orders in the marketplace (open, filled, cancelled)
-- Ref: Masterplan §4.2 Mig050

CREATE TABLE market_orders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id),
    asset_id        UUID NOT NULL REFERENCES assets(id),
    side            VARCHAR(4) NOT NULL CHECK (side IN ('buy', 'sell')),
    order_type      VARCHAR(10) NOT NULL DEFAULT 'limit'
                    CHECK (order_type IN ('limit', 'market')),
    price_cents     BIGINT NOT NULL CHECK (price_cents > 0),
    quantity        INTEGER NOT NULL CHECK (quantity > 0),
    quantity_filled INTEGER NOT NULL DEFAULT 0 CHECK (quantity_filled >= 0),
    status          VARCHAR(20) NOT NULL DEFAULT 'open'
                    CHECK (status IN (
                        'open',              -- In orderbook, waiting for match
                        'partially_filled',  -- Partially executed
                        'filled',            -- Fully executed
                        'cancelled',         -- Cancelled by user
                        'admin_cancelled',   -- Cancelled by admin
                        'expired',           -- Expired (TTL)
                        'pending_review',    -- Large order awaiting admin approval
                        'rejected'           -- Large order rejected by admin
                    )),
    idempotency_key UUID UNIQUE,             -- Prevents double-submissions
    cancel_reason   TEXT,                     -- Cancellation reason (admin or system)
    expires_at      TIMESTAMPTZ,             -- Expiry time (default: created_at + 90 days)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraint: filled may never exceed quantity
    CONSTRAINT chk_filled_lte_qty CHECK (quantity_filled <= quantity)
);

-- Performance indexes
-- Partial index for active orders per asset (most common query)
CREATE INDEX idx_orders_asset_status ON market_orders(asset_id, status)
    WHERE status IN ('open', 'partially_filled');

-- User order history (most recent first)
CREATE INDEX idx_market_orders_user ON market_orders(user_id, created_at DESC);

-- Expiry worker needs to find expiring orders efficiently
CREATE INDEX idx_orders_expiry ON market_orders(expires_at)
    WHERE status = 'open' AND expires_at IS NOT NULL;

-- Admin approval queue
CREATE INDEX idx_orders_pending ON market_orders(status)
    WHERE status = 'pending_review';
