-- ============================================================
-- Migration 047: Widen location_country column
-- The column was VARCHAR(3) (for ISO country codes) but the
-- developer application form allows free-text country names
-- like "Germany", "Indonesia", etc. This caused silent DB
-- errors on draft creation.
-- ============================================================

ALTER TABLE assets ALTER COLUMN location_country TYPE VARCHAR(100);
