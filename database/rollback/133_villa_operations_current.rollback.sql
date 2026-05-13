-- Rollback for 133_villa_operations_current.sql.

DROP TRIGGER IF EXISTS trg_villa_operations_current_upsert ON villa_operations_log;
DROP FUNCTION IF EXISTS fn_villa_operations_current_upsert();
DROP TABLE IF EXISTS villa_operations_current CASCADE;
DELETE FROM _schema_migrations WHERE filename = '133_villa_operations_current.sql';
