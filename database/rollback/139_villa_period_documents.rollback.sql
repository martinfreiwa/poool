-- Rollback for 139_villa_period_documents.sql.

DROP TABLE IF EXISTS villa_period_documents CASCADE;
DELETE FROM _schema_migrations WHERE filename = '139_villa_period_documents.sql';
