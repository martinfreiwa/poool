-- Rollback for 134_villa_valuations.sql.
-- SAFE only pre-rows. Otherwise rename to villa_valuations__rollback_<YYYYMMDD> first.

DROP TRIGGER IF EXISTS trg_villa_valuations_guard ON villa_valuations;
DROP FUNCTION IF EXISTS fn_villa_valuations_guard();
DROP TABLE IF EXISTS villa_valuations CASCADE;
DELETE FROM _schema_migrations WHERE filename = '134_villa_valuations.sql';
