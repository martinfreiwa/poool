-- Migration 050b: Add held_balance_cents to wallets
-- Purpose: Track funds blocked by open buy orders on the marketplace
-- Ref: Masterplan §4.3

-- Add the new column
ALTER TABLE wallets
    ADD COLUMN IF NOT EXISTS held_balance_cents BIGINT NOT NULL DEFAULT 0;

-- Constraint: held balance must be non-negative
ALTER TABLE wallets
    ADD CONSTRAINT chk_held_balance_non_negative
    CHECK (held_balance_cents >= 0);

-- Constraint: held balance must never exceed total balance
ALTER TABLE wallets
    ADD CONSTRAINT chk_held_lte_balance
    CHECK (held_balance_cents <= balance_cents);

COMMENT ON COLUMN wallets.held_balance_cents IS
    'Amount blocked by open buy orders. Increased on order placement, decreased on cancel/fill.';
