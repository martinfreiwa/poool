-- Rollback for 141_villa_forecast_suggestions.sql.

DROP TABLE IF EXISTS villa_forecast_suggestions CASCADE;
DELETE FROM _schema_migrations WHERE filename = '141_villa_forecast_suggestions.sql';
