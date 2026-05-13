-- Rollback for 132_villa_operations_log.sql.
-- SAFE only if no rows have been written. If data exists:
--   ALTER TABLE villa_operations_log RENAME TO villa_operations_log__rollback_<YYYYMMDD>;
-- Then run this rollback to drop dependents.

DROP TRIGGER IF EXISTS trg_villa_operations_log_guard ON villa_operations_log;
DROP FUNCTION IF EXISTS fn_villa_operations_log_guard();
DROP TABLE IF EXISTS villa_operations_log CASCADE;
DELETE FROM _schema_migrations WHERE filename = '132_villa_operations_log.sql';
