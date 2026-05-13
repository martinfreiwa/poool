-- Rollback for 137_villa_capex_events.sql.
-- SAFE only pre-rows. Otherwise rename to villa_capex_events__rollback_<YYYYMMDD> first.

DROP TRIGGER IF EXISTS trg_villa_capex_events_guard ON villa_capex_events;
DROP FUNCTION IF EXISTS fn_villa_capex_events_guard();
DROP TABLE IF EXISTS villa_capex_events CASCADE;
DELETE FROM _schema_migrations WHERE filename = '137_villa_capex_events.sql';
