-- Rollback for 145. Safe only if no rows reference the new columns.
DROP INDEX IF EXISTS uq_dividend_payouts_villa_period;
DROP INDEX IF EXISTS idx_dividend_payouts_source_log;
ALTER TABLE dividend_payouts
  DROP COLUMN IF EXISTS source_villa_operations_log_id,
  DROP COLUMN IF EXISTS period_year,
  DROP COLUMN IF EXISTS period_month;
DELETE FROM _schema_migrations WHERE filename = '145_dividend_payouts_villa_period.sql';
