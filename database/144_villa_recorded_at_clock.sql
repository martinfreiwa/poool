-- 144 — Villa-Returns P2: tighten recorded_at default to clock_timestamp().
--
-- Smoke-test discovery: NOW() returns the transaction-start timestamp, so two
-- rows inserted in the same transaction share recorded_at. That breaks the
-- as-of time-travel invariant (plan §6.7 test #2): a correction that lands in
-- the same tx as the original cannot be distinguished by recorded_at alone.
-- clock_timestamp() returns true wall-clock time per call, so each INSERT
-- gets a strictly later value within a single transaction.
--
-- Read queries should also add `id DESC` as a tiebreaker for cases where the
-- clock resolution still ties (e.g. very fast bulk inserts). Both fixes
-- together make as-of reads deterministic.

ALTER TABLE villa_operations_log  ALTER COLUMN recorded_at SET DEFAULT clock_timestamp();
ALTER TABLE villa_valuations      ALTER COLUMN recorded_at SET DEFAULT clock_timestamp();
ALTER TABLE villa_capex_events    ALTER COLUMN recorded_at SET DEFAULT clock_timestamp();
