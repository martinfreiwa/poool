-- Rollback for 135_villa_forecast_assumptions.sql.

DROP TABLE IF EXISTS villa_forecast_assumptions CASCADE;
DELETE FROM _schema_migrations WHERE filename = '135_villa_forecast_assumptions.sql';
