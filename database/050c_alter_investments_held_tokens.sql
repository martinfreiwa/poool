-- Migration 050c: Add held_tokens to investments
-- Purpose: Track tokens blocked by open sell orders on the marketplace
-- Ref: Masterplan §4.3

-- Add the new column
ALTER TABLE investments
    ADD COLUMN IF NOT EXISTS held_tokens INTEGER NOT NULL DEFAULT 0;

-- Constraint: held tokens must be non-negative
ALTER TABLE investments
    ADD CONSTRAINT chk_held_tokens_non_negative
    CHECK (held_tokens >= 0);

-- Constraint: held tokens must never exceed owned tokens
ALTER TABLE investments
    ADD CONSTRAINT chk_held_tokens_lte_owned
    CHECK (held_tokens <= tokens_owned);

COMMENT ON COLUMN investments.held_tokens IS
    'Tokens blocked by open sell orders. Increased on order placement, decreased on cancel/fill.';
