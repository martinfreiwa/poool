-- Rollback for 142_developer_asset_links.sql.

DROP TRIGGER IF EXISTS trg_developer_asset_links_guard ON developer_asset_links;
DROP FUNCTION IF EXISTS fn_developer_asset_links_guard();
DROP TABLE IF EXISTS developer_asset_links CASCADE;
DELETE FROM _schema_migrations WHERE filename = '142_developer_asset_links.sql';
