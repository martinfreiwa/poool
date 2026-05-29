-- Phase 9 follow-up: operational escalation metadata for Circle ops alerts.
-- This is intentionally metadata-only: external pager/email fan-out remains a
-- later integration, while assignment, escalation, snooze, and on-call-noted
-- state become durable and auditable now.

ALTER TABLE circle_ops_alerts
    ADD COLUMN IF NOT EXISTS assigned_to_user_id UUID,
    ADD COLUMN IF NOT EXISTS escalation_level INTEGER NOT NULL DEFAULT 0
        CHECK (escalation_level BETWEEN 0 AND 5),
    ADD COLUMN IF NOT EXISTS escalated_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS snoozed_until TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS escalation_note TEXT,
    ADD COLUMN IF NOT EXISTS on_call_notified_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_circle_ops_alerts_escalation_queue
    ON circle_ops_alerts (status, severity, escalation_level DESC, created_at DESC)
    WHERE status IN ('open', 'acknowledged');

CREATE INDEX IF NOT EXISTS idx_circle_ops_alerts_assigned_active
    ON circle_ops_alerts (assigned_to_user_id, status, created_at DESC)
    WHERE assigned_to_user_id IS NOT NULL
      AND status IN ('open', 'acknowledged');

CREATE INDEX IF NOT EXISTS idx_circle_ops_alerts_snoozed_active
    ON circle_ops_alerts (snoozed_until)
    WHERE snoozed_until IS NOT NULL
      AND status IN ('open', 'acknowledged');

COMMENT ON COLUMN circle_ops_alerts.assigned_to_user_id IS
    'Platform operator user assigned as current owner for follow-up.';
COMMENT ON COLUMN circle_ops_alerts.escalation_level IS
    'Bounded manual escalation level, 0-5, for Circle operations triage.';
COMMENT ON COLUMN circle_ops_alerts.escalated_at IS
    'Timestamp of the latest platform-admin escalation action.';
COMMENT ON COLUMN circle_ops_alerts.snoozed_until IS
    'Suppresses active triage priority until this timestamp without resolving the alert.';
COMMENT ON COLUMN circle_ops_alerts.escalation_note IS
    'Latest internal escalation context, kept out of member-facing Circle surfaces.';
COMMENT ON COLUMN circle_ops_alerts.on_call_notified_at IS
    'Timestamp marker that an on-call operator was manually notified; no external fan-out is performed by this migration.';
