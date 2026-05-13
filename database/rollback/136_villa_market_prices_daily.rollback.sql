-- Rollback for 136_villa_market_prices_daily.sql.

DROP TABLE IF EXISTS villa_market_prices_daily CASCADE;
DELETE FROM _schema_migrations WHERE filename = '136_villa_market_prices_daily.sql';
