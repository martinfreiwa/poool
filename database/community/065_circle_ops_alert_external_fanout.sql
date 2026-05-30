-- 065_circle_ops_alert_external_fanout.sql
-- Phase 9 follow-up: add Slack/PagerDuty delivery channels to the existing
-- Circle ops alert notification outbox. Rows remain durable and retryable;
-- webhook URLs/routing keys are provided by server-side environment variables.

ALTER TABLE circle_ops_alert_notifications
  DROP CONSTRAINT IF EXISTS circle_ops_alert_notifications_channel_check;

ALTER TABLE circle_ops_alert_notifications
  ADD CONSTRAINT circle_ops_alert_notifications_channel_check
  CHECK (channel IN ('email', 'slack', 'pagerduty'));

ALTER TABLE circle_ops_alert_notifications
  ADD COLUMN IF NOT EXISTS provider_response_status INTEGER
    CHECK (provider_response_status IS NULL OR provider_response_status BETWEEN 100 AND 599),
  ADD COLUMN IF NOT EXISTS provider_response_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_circle_ops_alert_notifications_external_ready
    ON circle_ops_alert_notifications (channel, status, next_attempt_at, created_at)
    WHERE channel IN ('slack', 'pagerduty')
      AND status IN ('queued', 'failed');

COMMENT ON COLUMN circle_ops_alert_notifications.channel IS
  'Fan-out channel for Circle ops alerts: email uses transactional_email_outbox; slack/pagerduty use server-side webhooks.';
COMMENT ON COLUMN circle_ops_alert_notifications.provider_response_status IS
  'Last HTTP response status observed from Slack or PagerDuty webhook delivery.';
COMMENT ON COLUMN circle_ops_alert_notifications.provider_response_at IS
  'Timestamp of the latest Slack/PagerDuty webhook response.';
