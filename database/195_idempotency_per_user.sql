-- ══════════════════════════════════════════════════════════════
-- 186_idempotency_per_user.sql
--
-- Security fix (H-2): idempotency keys must be unique per
-- (key, user_id), not globally per key. With a global unique
-- constraint:
--   - user B sending a key that happens to collide with user A's
--     in-flight request gets InProgress for 24h (DoS against B);
--   - the cached-response lookup leaks "this key is in use somewhere"
--     to anyone who can guess it (cross-account oracle).
--
-- This migration:
--   1. Drops the old unique constraint on `key` alone.
--   2. Adds a composite unique on (key, user_id).
--   3. Keeps the legacy index on `key` for the cached-response lookup
--      (it stays read-only useful even without uniqueness).
--
-- The Rust helper (`common::idempotency::try_reserve`) was already
-- doing the right thing on the read side; only the constraint shape
-- was wrong.
-- ══════════════════════════════════════════════════════════════

ALTER TABLE idempotency_keys
    DROP CONSTRAINT IF EXISTS idempotency_keys_key_key;

CREATE UNIQUE INDEX IF NOT EXISTS uq_idempotency_key_user
    ON idempotency_keys(key, user_id);
