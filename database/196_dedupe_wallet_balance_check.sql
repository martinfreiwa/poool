-- ══════════════════════════════════════════════════════════════
-- 196_dedupe_wallet_balance_check.sql
--
-- Cleanup (M-4 follow-up): `wallets` carried TWO redundant CHECK
-- constraints both enforcing `balance_cents >= 0`:
--
--   1. `wallets_balance_cents_check` — Postgres auto-generated from
--      a `CHECK` inline on the column at table creation.
--   2. `wallet_balance_non_negative` — added later by migration 044
--      with an explicit, descriptive name.
--
-- Both fire on every UPDATE. Keeping the descriptively-named one and
-- dropping the auto-named duplicate has no semantic impact but reduces
-- per-row constraint overhead and removes ambiguity for anyone reading
-- `\d wallets`.
--
-- Defense-in-depth invariants currently enforced on `wallets`:
--   - balance_cents >= 0                  (wallet_balance_non_negative)
--   - held_balance_cents >= 0             (chk_held_balance_non_negative)
--   - held_balance_cents <= balance_cents (chk_held_lte_balance)
--
-- Transitively this guarantees spendable balance (balance - held) is
-- always non-negative, so application-level race conditions cannot
-- produce a negative spendable balance even if the row debit check is
-- bypassed.
-- ══════════════════════════════════════════════════════════════

ALTER TABLE wallets
    DROP CONSTRAINT IF EXISTS wallets_balance_cents_check;
