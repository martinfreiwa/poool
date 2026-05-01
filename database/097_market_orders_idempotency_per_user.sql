-- Migration 097: Bind idempotency_key uniqueness to user_id
--
-- Problem: The original constraint `idempotency_key UUID UNIQUE` is global —
-- if a key leaks (logs, browser history, support ticket), an attacker can
-- block another user's order by submitting that key first. Idempotency is
-- a per-user concept; two different users using the same key by coincidence
-- should both succeed.
--
-- Fix: Drop the global unique constraint, replace with a composite unique
-- index on (user_id, idempotency_key). NULL keys remain allowed (legacy /
-- internal orders).

-- Drop the global unique constraint. Postgres auto-named the constraint
-- after the column when it was created via column-level UNIQUE.
ALTER TABLE market_orders
    DROP CONSTRAINT IF EXISTS market_orders_idempotency_key_key;

-- Composite unique index. Partial (WHERE NOT NULL) so multiple rows with
-- NULL idempotency_key are still allowed.
CREATE UNIQUE INDEX IF NOT EXISTS uq_market_orders_user_idempotency
    ON market_orders (user_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;

COMMENT ON INDEX uq_market_orders_user_idempotency IS
    'Idempotency is scoped per-user: same key from different users does not
     collide. A leaked key cannot be used to block another user.';
