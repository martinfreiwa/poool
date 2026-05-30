-- 064_circle_ops_alert_delivery_monitoring.sql
-- Phase 9 follow-up: monitor provider-level delivery state after Circle ops
-- alert fan-out rows have been bridged into the core transactional email
-- outbox.

ALTER TABLE circle_ops_alert_notifications
  ADD COLUMN IF NOT EXISTS email_outbox_status TEXT
    CHECK (
      email_outbox_status IS NULL
      OR email_outbox_status IN ('queued', 'sending', 'sent', 'failed', 'skipped', 'missing')
    ),
  ADD COLUMN IF NOT EXISTS email_outbox_attempts INTEGER
    CHECK (email_outbox_attempts IS NULL OR email_outbox_attempts >= 0),
  ADD COLUMN IF NOT EXISTS email_outbox_last_error TEXT,
  ADD COLUMN IF NOT EXISTS email_outbox_sent_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS delivery_checked_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS delivery_alerted_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_circle_ops_alert_notifications_delivery_monitor
    ON circle_ops_alert_notifications (status, delivery_checked_at, created_at)
    WHERE status = 'enqueued'
      AND enqueued_email_outbox_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_circle_ops_alert_notifications_delivery_unhealthy
    ON circle_ops_alert_notifications (email_outbox_status, email_outbox_attempts, created_at)
    WHERE status = 'enqueued'
      AND email_outbox_status IN ('queued', 'sending', 'failed', 'missing');

ALTER TABLE circle_ops_alerts
  DROP CONSTRAINT IF EXISTS circle_ops_alerts_alert_type_check;

ALTER TABLE circle_ops_alerts
  ADD CONSTRAINT circle_ops_alerts_alert_type_check
  CHECK (alert_type IN (
    'report_backlog',
    'spam_spike',
    'failed_worker',
    'posting_spike',
    'moderation_sla',
    'notification_delivery'
  ));

COMMENT ON COLUMN circle_ops_alert_notifications.email_outbox_status IS
  'Latest observed core transactional_email_outbox.status for the bridged Circle ops alert email.';
COMMENT ON COLUMN circle_ops_alert_notifications.delivery_checked_at IS
  'Timestamp of the latest provider/outbox delivery monitor pass for this notification.';
COMMENT ON COLUMN circle_ops_alert_notifications.delivery_alerted_at IS
  'Timestamp when a delivery monitoring alert was last raised for this notification.';
