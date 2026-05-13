-- Rollback for 146. Removes the seed row + drops the column.
-- Safe pre-deploy; in prod the column may be referenced by app code reading
-- per-asset opt-in — rollback only on incident.

DELETE FROM platform_settings WHERE key = 'villa_returns.enabled';
ALTER TABLE assets DROP COLUMN IF EXISTS villa_returns_pilot;
DELETE FROM _schema_migrations WHERE filename = '146_villa_returns_feature_flag.sql';
