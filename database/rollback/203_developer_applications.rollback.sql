-- Rollback for 203_developer_applications.sql.

DROP INDEX IF EXISTS idx_developer_applications_user;
DROP INDEX IF EXISTS idx_developer_applications_status_submitted;
DROP TABLE IF EXISTS developer_applications CASCADE;
DELETE FROM _schema_migrations WHERE filename = '203_developer_applications.sql';
