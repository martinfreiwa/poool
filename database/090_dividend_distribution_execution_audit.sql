-- Migration 090: Dividend distribution execution audit metadata
-- Records the admin who executes a wallet-crediting dividend distribution.

ALTER TABLE dividend_distributions
    ADD COLUMN IF NOT EXISTS distributed_by UUID REFERENCES users(id);

CREATE INDEX IF NOT EXISTS idx_dividend_dist_distributed_by
    ON dividend_distributions(distributed_by)
    WHERE distributed_by IS NOT NULL;
