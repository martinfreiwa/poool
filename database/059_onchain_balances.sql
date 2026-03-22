-- Migration 059: On-Chain Balances Cache
-- Purpose: Cache on-chain ERC-1155 token balances per user/asset from Polygon
-- Ref: Phase 8A.3 — Blockchain Sync (§3.2.11 Masterplan)
--
-- This table mirrors the on-chain state into PostgreSQL for:
-- 1. Fast portfolio reads (1ms vs 100-500ms blockchain reads)
-- 2. Dividend snapshot calculations
-- 3. Reconciliation between off-chain ledger and on-chain balances
--
-- 🔴 FINANCIAL TABLE — balance is BIGINT (token units), never floats.

CREATE TABLE IF NOT EXISTS onchain_balances (
    user_id           UUID NOT NULL REFERENCES users(id),
    asset_id          UUID NOT NULL REFERENCES assets(id),
    balance           BIGINT NOT NULL DEFAULT 0 CHECK (balance >= 0),
    last_synced_block BIGINT NOT NULL DEFAULT 0,
    last_synced_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, asset_id)
);

-- Index for dividend snapshot: "all holders of asset X with balance > 0"
CREATE INDEX IF NOT EXISTS idx_onchain_balances_asset
    ON onchain_balances(asset_id) WHERE balance > 0;

-- Index for portfolio view: "all assets held by user Y"
CREATE INDEX IF NOT EXISTS idx_onchain_balances_user
    ON onchain_balances(user_id) WHERE balance > 0;

-- Track the global indexer cursor (last processed block per contract)
CREATE TABLE IF NOT EXISTS chain_indexer_cursor (
    contract_address  VARCHAR(42) NOT NULL PRIMARY KEY,
    last_block        BIGINT NOT NULL DEFAULT 0,
    last_updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Platform settings for event indexer
INSERT INTO platform_settings (key, value, value_type, description) VALUES
    ('chain_indexer_enabled', 'false', 'boolean', 'Enable on-chain event indexer worker'),
    ('chain_indexer_poll_secs', '5', 'number', 'Polling interval in seconds for event indexer'),
    ('chain_indexer_confirmation_depth', '3', 'number', 'Number of blocks behind HEAD for re-org safety')
ON CONFLICT (key) DO NOTHING;
