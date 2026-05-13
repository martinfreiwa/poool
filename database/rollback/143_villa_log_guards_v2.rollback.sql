-- Rollback for 143: restores the original (looser) guard functions from 132/134/137.
-- WARNING: the original guards have a known hole (published data can be mutated
-- if no status flip occurs). Roll back only for incident response.

\i /Users/martin/Projects/poool/database/132_villa_operations_log.sql
\i /Users/martin/Projects/poool/database/134_villa_valuations.sql
\i /Users/martin/Projects/poool/database/137_villa_capex_events.sql

DELETE FROM _schema_migrations WHERE filename = '143_villa_log_guards_v2.sql';
