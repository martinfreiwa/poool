-- 058_circle_global_failed_worker_alert.sql
-- Phase 9 follow-up: one durable open global alert per failed Circle worker.

CREATE UNIQUE INDEX IF NOT EXISTS idx_circle_ops_alerts_global_open_unique
  ON circle_ops_alerts (alert_type)
  WHERE status = 'open' AND circle_id IS NULL;

COMMENT ON INDEX idx_circle_ops_alerts_global_open_unique IS
  'Prevents duplicate open global Circle ops alerts such as failed_worker.';
