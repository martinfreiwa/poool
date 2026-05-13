-- Rollback for 140_fx_rates_daily.sql.

DROP TABLE IF EXISTS fx_rates_daily CASCADE;
DELETE FROM _schema_migrations WHERE filename = '140_fx_rates_daily.sql';
