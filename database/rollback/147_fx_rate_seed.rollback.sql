-- Rollback for 147. Removes the seed row (no other side effects).

DELETE FROM fx_rates_daily WHERE source = 'manual_dev_seed';
DELETE FROM _schema_migrations WHERE filename = '147_fx_rate_seed.sql';
