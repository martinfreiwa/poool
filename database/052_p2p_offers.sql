-- Migration 052: Create p2p_offers table
-- Purpose: Peer-to-peer (OTC) direct offers between users
-- Ref: Masterplan §4.2 Mig052

CREATE TABLE p2p_offers (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id          UUID NOT NULL REFERENCES assets(id),
    maker_user_id     UUID NOT NULL REFERENCES users(id),
    taker_user_id     UUID NOT NULL REFERENCES users(id),
    side              VARCHAR(4) NOT NULL CHECK (side IN ('buy', 'sell')),
    price_cents       BIGINT NOT NULL CHECK (price_cents > 0),
    quantity          INTEGER NOT NULL CHECK (quantity > 0),
    message           TEXT,                            -- Optional message to taker
    status            VARCHAR(15) NOT NULL DEFAULT 'pending'
                      CHECK (status IN (
                          'pending',          -- Waiting for response
                          'accepted',         -- Accepted — will be executed
                          'declined',         -- Declined
                          'expired',          -- Not answered within TTL
                          'countered',        -- Counter-offer created
                          'cancelled',        -- Withdrawn by maker
                          'admin_cancelled'   -- Cancelled by admin
                      )),
    parent_offer_id   UUID REFERENCES p2p_offers(id),  -- Points to predecessor for counter-offers
    trade_id          UUID REFERENCES trade_history(id),-- Points to executed trade
    expires_at        TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '48 hours'),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraint: maker and taker must not be the same person
    CONSTRAINT chk_no_self_offer CHECK (maker_user_id != taker_user_id)
);

-- Pending offers for the taker (notification badge)
CREATE INDEX idx_p2p_taker ON p2p_offers(taker_user_id, status)
    WHERE status = 'pending';

-- All offers per asset (admin view)
CREATE INDEX idx_p2p_asset ON p2p_offers(asset_id, created_at DESC);

-- Expiry worker
CREATE INDEX idx_p2p_expiry ON p2p_offers(expires_at)
    WHERE status = 'pending';
