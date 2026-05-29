#!/usr/bin/env bash
set -euo pipefail

COMPOSE_FILE="${COMPOSE_FILE:-compose.local-live.yml}"
PSQL_CMD="${PSQL_CMD:-}"
ALERT_ID="${1:-${ALERT_ID:-}}"
SINCE_INTERVAL="${SINCE_INTERVAL:-4 hours}"

if [[ -n "$ALERT_ID" && ! "$ALERT_ID" =~ ^[0-9a-fA-F-]{36}$ ]]; then
  printf 'ERROR: ALERT_ID must be a UUID.\n' >&2
  exit 1
fi

run_psql() {
  local sql="$1"
  if [[ -n "$PSQL_CMD" ]]; then
    # shellcheck disable=SC2086
    $PSQL_CMD -v ON_ERROR_STOP=1 -At -F '|' -c "$sql"
  else
    docker compose -f "$COMPOSE_FILE" exec -T postgres \
      psql -U poool -d poool_community -v ON_ERROR_STOP=1 -At -F '|' -c "$sql"
  fi
}

if [[ -n "$ALERT_ID" ]]; then
  filter_sql="n.alert_id = '${ALERT_ID}'::uuid"
else
  safe_since="${SINCE_INTERVAL//\'/}"
  filter_sql="n.created_at >= NOW() - INTERVAL '${safe_since}'"
fi

result="$(
  run_psql "
WITH rows AS (
    SELECT
        n.channel,
        n.status,
        n.attempts,
        n.provider_response_status,
        n.provider_response_at,
        n.payload::text AS payload_text,
        n.last_error,
        n.created_at,
        a.status AS alert_status,
        a.summary AS alert_summary
    FROM circle_ops_alert_notifications n
    JOIN circle_ops_alerts a ON a.id = n.alert_id
    WHERE n.channel IN ('slack', 'pagerduty')
      AND ${filter_sql}
)
SELECT
    COUNT(*) FILTER (WHERE channel = 'slack') AS slack_rows,
    COUNT(*) FILTER (
        WHERE channel = 'slack'
          AND status = 'enqueued'
          AND provider_response_status BETWEEN 200 AND 299
    ) AS slack_delivered,
    COUNT(*) FILTER (WHERE channel = 'pagerduty') AS pagerduty_rows,
    COUNT(*) FILTER (
        WHERE channel = 'pagerduty'
          AND status = 'enqueued'
          AND provider_response_status BETWEEN 200 AND 299
    ) AS pagerduty_delivered,
    COUNT(*) FILTER (WHERE status = 'failed') AS failed_rows,
    COUNT(*) FILTER (
        WHERE status NOT IN ('skipped')
          AND provider_response_status IS NULL
    ) AS missing_provider_status,
    COUNT(*) FILTER (
        WHERE payload_text ILIKE '%hooks.slack.com%'
           OR payload_text ILIKE '%routing_key%'
           OR payload_text ILIKE '%/services/T%'
    ) AS secret_leak_candidates,
    COALESCE(MAX(provider_response_at)::text, '') AS latest_provider_response_at,
    COALESCE(MAX(alert_summary), '') AS sample_summary
FROM rows;
"
)"

IFS='|' read -r slack_rows slack_delivered pagerduty_rows pagerduty_delivered failed_rows missing_provider_status secret_leak_candidates latest_provider_response_at sample_summary <<< "$result"

printf 'Community Circles provider receipt summary\n'
printf '  alert filter: %s\n' "${ALERT_ID:-last ${SINCE_INTERVAL}}"
printf '  sample summary: %s\n' "$sample_summary"
printf '  slack rows/delivered: %s/%s\n' "$slack_rows" "$slack_delivered"
printf '  pagerduty rows/delivered: %s/%s\n' "$pagerduty_rows" "$pagerduty_delivered"
printf '  failed rows: %s\n' "$failed_rows"
printf '  missing provider status: %s\n' "$missing_provider_status"
printf '  secret leak candidates: %s\n' "$secret_leak_candidates"
printf '  latest provider response: %s\n' "$latest_provider_response_at"

errors=0

if [[ "${slack_rows:-0}" -lt 1 || "${slack_delivered:-0}" -lt 1 ]]; then
  printf 'ERROR: Slack receipt is missing or not delivered.\n' >&2
  errors=$((errors + 1))
fi

if [[ "${pagerduty_rows:-0}" -lt 1 || "${pagerduty_delivered:-0}" -lt 1 ]]; then
  printf 'ERROR: PagerDuty receipt is missing or not delivered.\n' >&2
  errors=$((errors + 1))
fi

if [[ "${failed_rows:-0}" -gt 0 ]]; then
  printf 'ERROR: Provider notification has failed rows.\n' >&2
  errors=$((errors + 1))
fi

if [[ "${missing_provider_status:-0}" -gt 0 ]]; then
  printf 'ERROR: Provider notification is missing HTTP response status.\n' >&2
  errors=$((errors + 1))
fi

if [[ "${secret_leak_candidates:-0}" -gt 0 ]]; then
  printf 'ERROR: Provider notification payload appears to contain webhook or routing secrets.\n' >&2
  errors=$((errors + 1))
fi

if [[ "$errors" -gt 0 ]]; then
  exit 1
fi

printf 'Community Circles provider receipt check passed.\n'
