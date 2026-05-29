#!/usr/bin/env bash
set -euo pipefail

COMPOSE_FILE="${COMPOSE_FILE:-compose.local-live.yml}"
PSQL_CMD="${PSQL_CMD:-}"
SUMMARY="${SUMMARY:-STAGING PROVIDER TEST - Community Circles external alert fanout}"

run_psql() {
  local sql="$1"
  if [[ -n "$PSQL_CMD" ]]; then
    # shellcheck disable=SC2086
    $PSQL_CMD -v ON_ERROR_STOP=1 -At -c "$sql"
  else
    docker compose -f "$COMPOSE_FILE" exec -T postgres \
      psql -U poool -d poool_community -v ON_ERROR_STOP=1 -At -c "$sql"
  fi
}

escaped_summary="${SUMMARY//\'/\'\'}"

run_psql "
WITH inserted_alert AS (
    INSERT INTO circle_ops_alerts (
        circle_id,
        alert_type,
        severity,
        status,
        summary,
        details
    )
    VALUES (
        NULL,
        'moderation_sla',
        'critical',
        'open',
        '${escaped_summary}',
        jsonb_build_object(
            'synthetic', true,
            'purpose', 'staging_provider_receipt_check',
            'created_by', 'community-circles-seed-provider-alert.sh'
        )
    )
    RETURNING id, summary, severity, alert_type
),
inserted_notifications AS (
    INSERT INTO circle_ops_alert_notifications (
        alert_id,
        channel,
        trigger_action,
        target_user_id,
        recipient_role,
        payload
    )
    SELECT
        inserted_alert.id,
        channel,
        'auto_critical',
        NULL,
        'platform_admin_fallback',
        jsonb_build_object(
            'summary', inserted_alert.summary,
            'severity', inserted_alert.severity,
            'alert_type', inserted_alert.alert_type,
            'channel', channel,
            'circle_slug', 'platform',
            'trigger_action', 'auto_critical',
            'synthetic', true,
            'created_by', 'community-circles-seed-provider-alert.sh'
        )
    FROM inserted_alert
    CROSS JOIN (VALUES ('slack'), ('pagerduty')) AS channels(channel)
    RETURNING alert_id
)
SELECT DISTINCT alert_id FROM inserted_notifications;
"
