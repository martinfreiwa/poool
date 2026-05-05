-- Migration 121: Backfill primary-issuance settlement eligibility.
--
-- Migration 120 added `on_chain_status` etc. to `order_items`. The
-- columns default to NULL, which the worker treats as "not eligible."
-- For orders that completed BEFORE 120 was applied, we want to lift
-- the items that ARE actually eligible (buyer has a bound wallet, asset
-- has a deployed contract) into 'pending' so the worker picks them up.
--
-- Idempotent: only touches NULLs. Safe to re-run.

UPDATE orders
   SET settle_eligible_at = COALESCE(
       settle_eligible_at,
       COALESCE(completed_at, NOW())
   )
 WHERE status = 'completed'
   AND settle_eligible_at IS NULL;

UPDATE order_items oi
   SET on_chain_status = 'pending'
  FROM orders o, users u, assets a
 WHERE oi.order_id = o.id
   AND oi.on_chain_status IS NULL
   AND o.status = 'completed'
   AND o.user_id = u.id
   AND oi.asset_id = a.id
   AND u.chain_wallet_address IS NOT NULL
   AND u.chain_wallet_address <> ''
   AND a.chain_contract_address IS NOT NULL
   AND a.chain_contract_address <> '';
