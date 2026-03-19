-- ============================================================
-- Migration 049: Widen bank_country column
-- The column was VARCHAR(3) (for ISO country codes) but the
-- bank country dropdown includes option value="OTHER" (5 chars)
-- which will crash when a user selects "Other country (SWIFT/BIC)".
-- ============================================================

ALTER TABLE payment_methods ALTER COLUMN bank_country TYPE VARCHAR(10);
