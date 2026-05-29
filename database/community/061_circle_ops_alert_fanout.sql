-- Phase 9 follow-up: durable notification fan-out foundation for Circle ops alerts.
--
-- Circle data lives in the community database while transactional email
-- delivery lives in the core database. This table is the bridge: Circle
-- alert actions enqueue intent here, and a worker copies ready rows into the
-- core transactional_email_outbox with retry/backoff and audit visibility.

CREATE TABLE IF NOT EXISTS circle_ops_alert_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    alert_id UUID NOT NULL REFERENCES circle_ops_alerts(id) ON DELETE CASCADE,
    channel TEXT NOT NULL DEFAULT 'email'
        CHECK (channel IN ('email')),
    trigger_action TEXT NOT NULL
        CHECK (trigger_action IN ('auto_critical', 'escalate', 'mark_on_call_notified')),
    target_user_id UUID,
    recipient_role TEXT NOT NULL DEFAULT 'assigned_operator'
        CHECK (recipient_role IN ('assigned_operator', 'platform_admin_fallback')),
    status TEXT NOT NULL DEFAULT 'queued'
        CHECK (status IN ('queued', 'sending', 'enqueued', 'skipped', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    enqueued_email_outbox_id UUID,
    last_error TEXT,
    payload JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_circle_ops_alert_notifications_ready
    ON circle_ops_alert_notifications (status, next_attempt_at, created_at)
    WHERE status IN ('queued', 'failed');

CREATE INDEX IF NOT EXISTS idx_circle_ops_alert_notifications_alert
    ON circle_ops_alert_notifications (alert_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_circle_ops_alert_notifications_target
    ON circle_ops_alert_notifications (target_user_id, status, created_at DESC)
    WHERE target_user_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_circle_ops_alert_notifications_active_unique
    ON circle_ops_alert_notifications (alert_id, channel, trigger_action, COALESCE(target_user_id, '00000000-0000-0000-0000-000000000000'::UUID))
    WHERE status IN ('queued', 'sending');

COMMENT ON TABLE circle_ops_alert_notifications IS
    'Durable Community DB outbox for Circle ops alert fan-out. Worker copies ready rows into core transactional_email_outbox.';
COMMENT ON COLUMN circle_ops_alert_notifications.target_user_id IS
    'Core users.id target for email fan-out; NULL means fall back to the canonical platform admin inbox.';
COMMENT ON COLUMN circle_ops_alert_notifications.enqueued_email_outbox_id IS
    'Core transactional_email_outbox.id once the worker has bridged the notification into the core email delivery queue.';
