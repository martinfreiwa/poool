-- 182: Extend trade_history.on_chain_status CHECK to include retry states
--
-- Production code in `api_admin_marketplace_trades_bulk_retry_onchain`
-- (backend/src/admin/marketplace.rs) filters by `on_chain_status IN
-- ('failed', 'reverted', 'timeout')` when bulk-retrying terminal trades.
-- The CHECK constraint only allowed `pending|submitted|confirmed|failed|
-- cancelled` — meaning 'reverted' and 'timeout' were dead values that
-- could never be inserted, and the retry filter was a no-op for two of
-- its three branches.
--
-- This migration extends the CHECK to include all retry-eligible states
-- so the settlement worker can correctly classify failures.

BEGIN;

ALTER TABLE trade_history
    DROP CONSTRAINT IF EXISTS trade_history_on_chain_status_check;

ALTER TABLE trade_history
    ADD CONSTRAINT trade_history_on_chain_status_check
    CHECK (on_chain_status IN (
        'pending',
        'submitted',
        'confirmed',
        'failed',
        'reverted',
        'timeout',
        'cancelled'
    ));

COMMIT;
