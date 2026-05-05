-- Migration 120: Primary-issuance on-chain settlement.
--
-- Adds chain-tracking columns to `order_items` so the new primary-settlement
-- worker can lift completed primary purchases to the asset contract via
-- POOOLAssetToken.settleBatch(treasury → buyer transfers). Mirrors the
-- existing `trade_history.on_chain_*` columns used for P2P settlement.
--
-- Why on `order_items`, not `orders`: one order can hold multiple assets,
-- each targeting a different ERC-1155 contract. Chain settlement must be
-- granular enough that one item's revert doesn't poison the whole order.
--
-- Lifecycle:
--   NULL                          → not eligible / no buyer wallet bound
--   'pending'                     → eligible, awaiting next batch
--   'submitted'                   → tx broadcast, awaiting receipt
--   'confirmed'                   → on-chain transfer complete
--   'failed'                      → tx reverted or worker error
--
-- T+1 settlement delay is enforced via `orders.settle_eligible_at`
-- (default = order.completed_at + chain_primary_settle_delay_secs).
-- Bank-wire reversal window closes inside that delay so we never commit
-- a chain transfer that would have to be unwound.

ALTER TABLE order_items
    ADD COLUMN IF NOT EXISTS on_chain_status      VARCHAR(15)
        CHECK (on_chain_status IN ('pending', 'submitted', 'confirmed', 'failed')),
    ADD COLUMN IF NOT EXISTS on_chain_tx_hash     VARCHAR(66),
    ADD COLUMN IF NOT EXISTS on_chain_batch_id    UUID
        REFERENCES chain_settlement_batches(id),
    ADD COLUMN IF NOT EXISTS settle_attempt_count INTEGER NOT NULL DEFAULT 0;

ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS settle_eligible_at TIMESTAMPTZ;

-- Worker-fast-path index: only rows the worker is interested in.
CREATE INDEX IF NOT EXISTS idx_order_items_chain_pending
    ON order_items(order_id)
    WHERE on_chain_status = 'pending' AND on_chain_batch_id IS NULL;

-- Lookup by batch (admin UI: "show me what was in batch X").
CREATE INDEX IF NOT EXISTS idx_order_items_chain_batch
    ON order_items(on_chain_batch_id)
    WHERE on_chain_batch_id IS NOT NULL;

-- Distinguish primary-issuance batches from P2P batches in the shared
-- `chain_settlement_batches` table. Existing rows = P2P (default 'p2p').
ALTER TABLE chain_settlement_batches
    ADD COLUMN IF NOT EXISTS batch_type VARCHAR(20) NOT NULL DEFAULT 'p2p'
        CHECK (batch_type IN ('p2p', 'primary'));

-- Worker tunables. Read each cycle by run_primary_settlement_worker so
-- ops can adjust without redeploy.
INSERT INTO platform_settings (key, value, value_type, description) VALUES
    ('chain_primary_settlement_enabled',      'true',  'boolean',
        'Enable primary-issuance on-chain settlement worker'),
    ('chain_primary_settlement_interval_secs', '300',  'number',
        'Primary settlement worker cycle interval in seconds'),
    ('chain_primary_settle_delay_secs',       '86400', 'number',
        'Delay between order completion and chain settlement (T+1 = 86400)'),
    ('chain_primary_max_batch_size',          '50',    'number',
        'Maximum order_items settled in a single primary-issuance batch')
ON CONFLICT (key) DO NOTHING;
