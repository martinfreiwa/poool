-- 067_circle_ops_alert_workflow_states.sql
-- Phase 9 follow-up: human SLA workflow state for active Circle ops alerts.
-- Alert status tracks lifecycle; workflow_state tracks the operator's current
-- triage position before final resolution.

ALTER TABLE circle_ops_alerts
  ADD COLUMN IF NOT EXISTS workflow_state VARCHAR(40) NOT NULL DEFAULT 'triage',
  ADD COLUMN IF NOT EXISTS workflow_note TEXT,
  ADD COLUMN IF NOT EXISTS workflow_updated_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS workflow_updated_by UUID;

UPDATE circle_ops_alerts
   SET workflow_state = CASE
         WHEN status = 'resolved' THEN 'mitigated'
         WHEN status = 'acknowledged' THEN 'investigating'
         ELSE workflow_state
       END
 WHERE workflow_state = 'triage';

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'circle_ops_alerts_workflow_state_allowed'
  ) THEN
    ALTER TABLE circle_ops_alerts
      ADD CONSTRAINT circle_ops_alerts_workflow_state_allowed
      CHECK (workflow_state IN (
        'triage',
        'investigating',
        'waiting_on_moderator',
        'waiting_on_policy',
        'mitigated',
        'monitoring'
      ));
  END IF;
END
$$;

CREATE INDEX IF NOT EXISTS idx_circle_ops_alerts_workflow_active
  ON circle_ops_alerts (workflow_state, status, severity, created_at DESC)
  WHERE status IN ('open', 'acknowledged');

COMMENT ON COLUMN circle_ops_alerts.workflow_state IS
  'Human SLA workflow state for active triage: triage, investigating, waiting_on_moderator, waiting_on_policy, mitigated, or monitoring.';
COMMENT ON COLUMN circle_ops_alerts.workflow_note IS
  'Latest internal note explaining the current human workflow state.';
COMMENT ON COLUMN circle_ops_alerts.workflow_updated_at IS
  'Timestamp of the latest workflow-state transition.';
COMMENT ON COLUMN circle_ops_alerts.workflow_updated_by IS
  'Operator user ID that last changed workflow_state.';
