# POOOL Community Circles Provider Runbook

Date: 2026-05-21  
Scope: Slack/PagerDuty staging validation for Circle ops alerts.

## Purpose

This runbook closes the operational gap between local code validation and staging provider validation. It proves that Circle ops alerts can leave POOOL, arrive in Slack and PagerDuty, and leave a durable audit trail without exposing provider secrets.

## Preconditions

- Use staging or sandbox credentials only.
- `APP_ENV=staging` and `POOOL_ENV=staging`.
- `POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL` points at a staging Slack channel.
- `POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY` points at a non-production PagerDuty service.
- `POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL` is either unset or `https://events.pagerduty.com/v2/enqueue`.
- `POOOL_GCS_DOWNLOAD_FAKE_ROOT` is unset.
- The local-live/staging backend receives the provider env vars. In local-live this is wired through `compose.local-live.yml`.

## Preflight

Run the local preflight before sending a real provider event:

```bash
POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS=1 \
scripts/local-live/community-circles-staging-preflight.sh .env.local-live
```

This validates:

- required Slack/PagerDuty variables are present;
- provider webhook URLs use HTTPS;
- URL credentials and fragments are rejected;
- fake GCS roots are not enabled in staging/local-live;
- Circle webhook unit tests pass;
- focused Circles static contracts pass.

## Synthetic Provider Alert

After the local-live or staging stack is running with real staging provider variables, seed one synthetic alert:

```bash
ALERT_ID="$(scripts/local-live/community-circles-seed-provider-alert.sh | tail -n 1)"
printf 'Seeded Circle provider alert: %s\n' "$ALERT_ID"
```

For non-Docker staging databases, provide `PSQL_CMD`:

```bash
PSQL_CMD="psql $COMMUNITY_DATABASE_URL" \
scripts/local-live/community-circles-seed-provider-alert.sh
```

The seed inserts:

- one global critical `moderation_sla` alert;
- one queued Slack notification;
- one queued PagerDuty notification;
- payload metadata marked `synthetic=true`.

It does not insert provider secrets into the payload.

## Receipt Check

Wait for the `circle_ops_alert_fanout_worker` to process the queued notifications, then run:

```bash
scripts/local-live/community-circles-provider-receipt-check.sh "$ALERT_ID"
```

The receipt check fails if:

- Slack has no delivered external notification row;
- PagerDuty has no delivered external notification row;
- any provider row is failed;
- any provider row lacks `provider_response_status`;
- notification payload text appears to include webhook URLs or PagerDuty routing keys.

Expected database evidence:

- `circle_ops_alert_notifications.channel IN ('slack', 'pagerduty')`;
- `status = 'enqueued'`;
- `provider_response_status BETWEEN 200 AND 299`;
- `provider_response_at IS NOT NULL`;
- `attempts >= 1`.

## Manual Provider Evidence

Capture and attach the following to the release or audit ticket:

- Slack channel name and timestamp.
- PagerDuty event or incident ID.
- POOOL `circle_ops_alerts.id`.
- Two `circle_ops_alert_notifications.id` values, one Slack and one PagerDuty.
- `provider_response_status` values.
- Platform-admin audit log ID for resolving the synthetic alert.

Do not copy webhook URLs, routing keys, authorization headers, cookies, or full provider payloads into tickets.

## Resolve And Close Loop

Use the platform-admin Circle Ops Alerts UI to resolve the synthetic alert.

Then verify:

```sql
SELECT id, status, resolved_at, workflow_state
FROM circle_ops_alerts
WHERE id = '<ALERT_ID>';
```

Expected result:

- `status = 'resolved'`;
- `resolved_at IS NOT NULL`;
- a `platform.circle_ops_alert.resolve` audit entry exists.

## Rollback

If provider delivery fails:

1. Disable `POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL` and/or `POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY`.
2. Restart the backend.
3. Re-run the preflight and confirm missing required provider vars fail closed when `POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS=1`.
4. Keep email fan-out enabled as the fallback ops channel.
5. Do not declare Circles production-live until a new synthetic provider alert passes.

## Production Gate

Circles provider rollout is green only when:

- preflight passes;
- one synthetic alert reaches Slack;
- one synthetic alert reaches PagerDuty;
- receipt check passes;
- provider evidence is recorded without secrets;
- the synthetic alert is resolved through the platform-admin UI;
- legal/compliance sign-off is complete.
