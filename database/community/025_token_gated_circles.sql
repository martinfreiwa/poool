-- 025_token_gated_circles.sql — W3.1: Token-Gated Circles
-- Circle owners can require members to hold a minimum value of a specific asset.
-- The asset_id references the core DB assets table (cross-DB read-only lookup).

ALTER TABLE circles
  ADD COLUMN IF NOT EXISTS token_gate_asset_id UUID,             -- NULL = no gate
  ADD COLUMN IF NOT EXISTS token_gate_min_value_cents BIGINT DEFAULT 0,  -- Min holding in cents (e.g. 100000 = $1,000)
  ADD COLUMN IF NOT EXISTS token_gate_asset_name VARCHAR(200);   -- Denormalized for display (avoids cross-DB join)

-- Index for quick lookup of gated circles
CREATE INDEX IF NOT EXISTS idx_circles_token_gate ON circles(token_gate_asset_id) WHERE token_gate_asset_id IS NOT NULL;

COMMENT ON COLUMN circles.token_gate_asset_id IS 'If set, users must hold this asset to join. References core DB assets.id (cross-DB read-only).';
COMMENT ON COLUMN circles.token_gate_min_value_cents IS 'Minimum holding value in cents. E.g. 100000 = $1,000 worth of the asset.';
COMMENT ON COLUMN circles.token_gate_asset_name IS 'Denormalized asset name for UI display without cross-DB join.';
