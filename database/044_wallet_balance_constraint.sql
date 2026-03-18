-- Add CHECK constraint to prevent negative wallet balances.
-- This is a safety net: application logic already validates before deduction,
-- but this DB-level guard protects against any code path that bypasses the check.

-- First, verify no wallets currently have negative balances
-- (if any do, this migration would fail, and you'd need to fix them first)
ALTER TABLE wallets ADD CONSTRAINT wallet_balance_non_negative CHECK (balance_cents >= 0);
