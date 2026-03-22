-- Migration 058: Blockchain Integration — On-chain metadata for assets and users
-- Purpose: Link off-chain DB records to on-chain POOOLProperty1155 contract
-- Ref: Phase 7 — Smart Contracts (ERC-1155 on Polygon)

-- ═══════════════════════════════════════════════════════════════
-- 1. Assets: Add on-chain token ID and contract metadata
-- ═══════════════════════════════════════════════════════════════

ALTER TABLE assets ADD COLUMN IF NOT EXISTS chain_token_id VARCHAR(78);
    -- The uint256 tokenId on the POOOLProperty1155 contract
    -- Up to 78 decimal digits (uint256 max = 2^256 - 1)
    -- NULL means this asset hasn't been tokenized on-chain yet

ALTER TABLE assets ADD COLUMN IF NOT EXISTS chain_contract_address VARCHAR(42);
    -- The contract address (0x + 40 hex chars)
    -- NULL for assets not yet deployed on-chain

ALTER TABLE assets ADD COLUMN IF NOT EXISTS chain_network VARCHAR(20) DEFAULT 'polygon';
    -- 'polygon' for mainnet, 'polygon_amoy' for testnet

ALTER TABLE assets ADD COLUMN IF NOT EXISTS chain_tx_hash VARCHAR(66);
    -- The transaction hash of the createAsset() call

ALTER TABLE assets ADD COLUMN IF NOT EXISTS chain_metadata_uri VARCHAR(512);
    -- IPFS URI for the on-chain metadata JSON

CREATE INDEX IF NOT EXISTS idx_assets_chain_token ON assets(chain_token_id)
    WHERE chain_token_id IS NOT NULL;

-- ═══════════════════════════════════════════════════════════════
-- 2. Users: Add blockchain wallet address for on-chain settlements
-- ═══════════════════════════════════════════════════════════════

ALTER TABLE users ADD COLUMN IF NOT EXISTS chain_wallet_address VARCHAR(42);
    -- The user's whitelisted wallet address on Polygon
    -- NULL means user hasn't been whitelisted on-chain yet

ALTER TABLE users ADD COLUMN IF NOT EXISTS chain_whitelisted_at TIMESTAMPTZ;
    -- When the user was whitelisted on-chain

CREATE INDEX IF NOT EXISTS idx_users_chain_wallet ON users(chain_wallet_address)
    WHERE chain_wallet_address IS NOT NULL;

-- ═══════════════════════════════════════════════════════════════
-- 3. On-chain settlement batches — tracks each settleBatch() call
-- ═══════════════════════════════════════════════════════════════

CREATE TABLE IF NOT EXISTS chain_settlement_batches (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_size      INTEGER NOT NULL CHECK (batch_size > 0),
    tx_hash         VARCHAR(66),        -- Set after submission
    status          VARCHAR(15) NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'submitted', 'confirmed', 'failed')),
    gas_used        BIGINT,             -- Gas used (from receipt)
    gas_price_gwei  BIGINT,             -- Gas price in gwei
    block_number    BIGINT,             -- Block number (from receipt)
    error_message   TEXT,               -- Error details if failed
    submitted_at    TIMESTAMPTZ,
    confirmed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_chain_batches_status ON chain_settlement_batches(status)
    WHERE status IN ('pending', 'submitted');

-- ═══════════════════════════════════════════════════════════════
-- 4. Platform settings for blockchain config
-- ═══════════════════════════════════════════════════════════════

INSERT INTO platform_settings (key, value, value_type, description) VALUES
    ('chain_contract_address', '0xb61CCe33B546a5C7c36F0B58119e7F4B3D1D04e5', 'string', 'POOOLProperty1155 contract address on Polygon'),
    ('chain_network', 'polygon_amoy', 'string', 'Blockchain network: polygon or polygon_amoy'),
    ('chain_rpc_url', 'https://rpc-amoy.polygon.technology', 'string', 'Polygon RPC endpoint URL'),
    ('chain_settlement_enabled', 'false', 'boolean', 'Enable on-chain batch settlement of trades')
ON CONFLICT (key) DO NOTHING;
