-- Rollback for 144 — restore NOW() defaults. Re-introduces the known same-tx
-- collision risk; only roll back for incident response.

ALTER TABLE villa_operations_log  ALTER COLUMN recorded_at SET DEFAULT NOW();
ALTER TABLE villa_valuations      ALTER COLUMN recorded_at SET DEFAULT NOW();
ALTER TABLE villa_capex_events    ALTER COLUMN recorded_at SET DEFAULT NOW();
DELETE FROM _schema_migrations WHERE filename = '144_villa_recorded_at_clock.sql';
