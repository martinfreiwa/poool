-- Migration 122: Reset asset clones whose mintTo wallet has no recoverable key.
--
-- Background: at the time three assets were tokenized, the
-- `CHAIN_SETTLEMENT_ADDRESS` env var pointed at a throwaway wallet
-- (`0x94E1...`) whose private key was never persisted. The clones were
-- minted to that address — the `chain-settlement-private-key` secret
-- (which is a different wallet, `0x021F6B...`) cannot move the supply.
--
-- The defensive code change in this same release locks `mintTo` to the
-- key-derived address, so this can't happen again. This migration
-- resets the three orphaned assets so the operator can re-tokenize
-- them via the admin UI; the new clones will be minted to a recoverable
-- treasury.
--
-- Idempotent: WHERE filter only matches the three known orphan addresses.
-- Re-running on already-cleaned rows is a no-op.

-- 1. Null out the on-chain refs so admin UI shows "not yet tokenized"
--    and re-tokenization is allowed (`UPDATE ... WHERE chain_token_id IS NULL`
--    in the tokenize handler will then succeed).
UPDATE assets
   SET chain_token_id         = NULL,
       chain_contract_address = NULL,
       chain_tx_hash          = NULL,
       chain_metadata_uri     = NULL,
       updated_at             = NOW()
 WHERE LOWER(chain_contract_address) IN (
        '0x4b9fa2ee3f309b3678f7e0b58f31210e024363be',  -- Demo Villa 01
        '0xf92814f3538e5604bc2f18c5bf5bc5cd2dbcd978',  -- Villa Pillada Horadada
        '0x5b50417903e2f5c12fdf84bb8f2d5e89a33444e2'   -- Demo Apartment 01
    );

-- 2. Reset settlement state on those assets' order_items so the worker
--    treats them as "not yet eligible" until re-tokenization.
--    `mark_asset_eligible_after_tokenization` will lift them back to
--    'pending' automatically when the admin re-tokenizes each asset.
UPDATE order_items oi
   SET on_chain_status   = NULL,
       on_chain_batch_id = NULL,
       on_chain_tx_hash  = NULL
  FROM orders o, assets a
 WHERE oi.order_id  = o.id
   AND oi.asset_id  = a.id
   AND a.chain_contract_address IS NULL
   AND oi.on_chain_status IN ('pending', 'failed');

-- 3. Mark the failed primary-settlement batch records so they're not
--    surfaced as "current alerts" in the admin UI. (They're a
--    historical artifact of the orphan deploy.)
UPDATE chain_settlement_batches
   SET error_message = COALESCE(error_message, '') ||
                       ' [archived: orphan clone reset by migration 122]'
 WHERE batch_type = 'primary'
   AND status = 'failed'
   AND error_message LIKE '%balanceOf%';
