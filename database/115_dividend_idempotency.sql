-- Migration 115: Idempotency-Key support for dividend distributions.
-- Prevents duplicate POST /api/admin/dividends/distributions creating
-- multiple drafts on accidental double-click or network retry.
--
-- A unique partial index on the key only enforces dedup for non-null values,
-- so old rows (created_by NULL) are not affected.

ALTER TABLE dividend_distributions
    ADD COLUMN IF NOT EXISTS idempotency_key TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS ux_dividend_distributions_idempotency
    ON dividend_distributions (idempotency_key)
    WHERE idempotency_key IS NOT NULL;

COMMENT ON COLUMN dividend_distributions.idempotency_key IS
    'Client-supplied idempotency key (UUID v4). When the same key is reused
     within the lifetime of an existing draft, the existing distribution is
     returned instead of inserting a duplicate row.';
