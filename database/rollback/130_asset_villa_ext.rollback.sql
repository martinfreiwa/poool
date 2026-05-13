-- Rollback for 130_asset_villa_ext.sql.
-- SAFE only before any row of villa_operations_log or villa_valuations references these columns.
-- Otherwise rename via ad-hoc incident script.

ALTER TABLE assets
  DROP COLUMN IF EXISTS tokenized_pct_bps,
  DROP COLUMN IF EXISTS tokens_payout_eligible,
  DROP COLUMN IF EXISTS tokens_owner_retained,
  DROP COLUMN IF EXISTS reserve_pct_bps,
  DROP COLUMN IF EXISTS payout_frequency,
  DROP COLUMN IF EXISTS payout_currency,
  DROP COLUMN IF EXISTS distribution_record_day,
  DROP COLUMN IF EXISTS withholding_tax_bps,
  DROP COLUMN IF EXISTS allow_developer_submission,
  DROP COLUMN IF EXISTS native_currency_code,
  DROP COLUMN IF EXISTS mgmt_fee_bps;

DELETE FROM _schema_migrations WHERE filename = '130_asset_villa_ext.sql';
