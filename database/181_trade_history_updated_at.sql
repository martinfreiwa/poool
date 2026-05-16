-- 181: trade_history.updated_at column for bulk-retry audit
--
-- The admin bulk-retry endpoint
-- (`api_admin_marketplace_trades_bulk_retry_onchain` in
-- backend/src/admin/marketplace.rs:945) updates the column on every retry
-- so ops can see when a trade was last touched. The column was missing,
-- which broke the endpoint with `column "updated_at" of relation
-- "trade_history" does not exist`.
--
-- Backfill: NULL → executed_at (best available signal for "last change").

BEGIN;

ALTER TABLE trade_history
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ;

UPDATE trade_history SET updated_at = executed_at WHERE updated_at IS NULL;

ALTER TABLE trade_history
    ALTER COLUMN updated_at SET DEFAULT NOW(),
    ALTER COLUMN updated_at SET NOT NULL;

COMMIT;
