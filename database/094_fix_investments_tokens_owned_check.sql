-- Migration 094: Fix investments.tokens_owned CHECK constraint
--
-- Problem: The original constraint `CHECK (tokens_owned > 0)` causes the
-- on-chain/settlement transaction to FAIL the moment a seller's last token
-- is sold. The match event then sits in the Redis queue and retries
-- forever, blocking all subsequent settlements for that user/asset.
--
-- Fix: Allow zero. Empty investment rows are kept (they retain history) but
-- can be filtered by `tokens_owned > 0` at read time. We do NOT delete the
-- row on zero, because the row also carries `purchase_price_cents` /
-- `purchased_at` which may still be referenced by trade_history /
-- tax_reports / dividend_calculations.

ALTER TABLE investments
    DROP CONSTRAINT IF EXISTS investments_tokens_owned_check;

ALTER TABLE investments
    ADD CONSTRAINT investments_tokens_owned_check
    CHECK (tokens_owned >= 0);

-- Sanity: existing held_tokens constraint still applies (held <= owned).
-- After this migration: tokens_owned can be 0, held_tokens must also be 0
-- in that state (enforced by the existing chk_held_tokens_lte_owned check).

COMMENT ON CONSTRAINT investments_tokens_owned_check ON investments IS
    'Allow zero — empty positions are retained for audit / dividend history.
     Filter `tokens_owned > 0` at read time when listing active holdings.';
