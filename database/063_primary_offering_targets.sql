-- ============================================================
-- POOOL Platform - Phase 16.3
-- Add Escrow and Primary Target thresholds to assets
-- ============================================================

ALTER TABLE assets 
ADD COLUMN min_funding_tokens INTEGER NOT NULL DEFAULT 0,
ADD COLUMN escrow_agent VARCHAR(50) DEFAULT 'unassigned';

-- Set a default minimum target of 100% for existing assets 
UPDATE assets SET min_funding_tokens = tokens_total;

-- Create an index for querying active primary escrow campaigns
CREATE INDEX IF NOT EXISTS idx_assets_funding_escrow 
ON assets(funding_status) 
WHERE funding_status IN ('funding_open', 'funding_in_progress');
