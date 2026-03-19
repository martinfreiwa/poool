-- ============================================================
-- Migration 048: Widen nationality column
-- The column was VARCHAR(3) (for ISO country codes) but the
-- nationality dropdown sends full country names like "Germany",
-- "Indonesia", etc. Same bug as location_country (migration 047).
-- ============================================================

ALTER TABLE user_profiles ALTER COLUMN nationality TYPE VARCHAR(100);
