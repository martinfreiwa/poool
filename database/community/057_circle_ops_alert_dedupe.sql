-- 057_circle_ops_alert_dedupe.sql
-- Phase 9 follow-up: keep scheduled Circle ops workers idempotent.

CREATE UNIQUE INDEX IF NOT EXISTS idx_circle_ops_alerts_open_unique
  ON circle_ops_alerts (circle_id, alert_type)
  WHERE status = 'open' AND circle_id IS NOT NULL;

COMMENT ON INDEX idx_circle_ops_alerts_open_unique IS
  'Prevents duplicate open Circle ops alerts while allowing acknowledged/resolved history.';
