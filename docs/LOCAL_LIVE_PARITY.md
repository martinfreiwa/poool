# POOOL Local-Live Parity

This guide defines two local profiles:

- `fast-dev`: quick development with `cargo run` on `http://localhost:8888`.
- `local-live`: production-like validation with Docker, PgBouncer, Postgres 16, Redis, staging envs, and readiness gates.

## Fast Dev

Use this when iterating on code quickly:

```bash
cp .env.fast-dev.example backend/.env
cd backend
cargo watch -x run
```

This mode intentionally allows development behavior. Redis is optional, GCS can fall back to local uploads, and the server runs directly without PgBouncer.

## Local Live

Use this before relying on a local test as deploy evidence:

```bash
cp .env.local-live.example .env.local-live
scripts/local-live/validate-env.sh .env.local-live
docker compose --env-file .env.local-live -f compose.local-live.yml up --build
scripts/local-live/smoke.sh
```

The local-live profile runs:

- Backend from the production `Dockerfile`.
- PgBouncer in strict mode with `pool_mode = session`.
- Postgres 16 with `poool` and `poool_community`.
- Redis via Unix socket, not plaintext TCP.
- `APP_ENV=staging` and `POOOL_ENV=staging`.
- `/live`, `/ready`, `/health`, and `/metrics`.

## Required Sandbox Services

Use staging/sandbox credentials only:

- GCS staging bucket.
- Local Google ADC mounted read-only from `${HOME}/.config/gcloud`.
- Didit sandbox/staging API key and webhook secret.
- Sanity staging dataset/tokens.
- Sentry staging DSN.
- Stripe test keys only.
- Chain testnet contracts only, such as Polygon Amoy.

Never use production databases, production object buckets, real payment keys, or mainnet signer keys in local-live.

## KYC Webhook Profile

Didit webhooks need an externally reachable HTTPS URL.

```bash
ngrok http 8080
```

Set `BASE_URL` in `.env.local-live` to the ngrok HTTPS URL and configure Didit webhook URL:

```text
{BASE_URL}/api/webhooks/kyc/didit
```

Then restart the stack and rerun:

```bash
scripts/local-live/validate-env.sh .env.local-live
scripts/local-live/smoke.sh
```

## Community Circles Provider Preflight

Before treating the Circles ops layer as staging-ready, run the dedicated
preflight with real staging Slack and PagerDuty credentials:

```bash
POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS=1 \
scripts/local-live/community-circles-staging-preflight.sh .env.local-live
```

This checks the local-live env, rejects public HTTP webhooks, rejects embedded
URL credentials/fragments, blocks `POOOL_GCS_DOWNLOAD_FAKE_ROOT` outside
development, runs the Circle webhook unit tests, and runs the focused Circles
static contracts. It does not send a real provider event; the final staging
step is to create one synthetic critical Circle ops alert and confirm Slack and
PagerDuty receipt.

Use the provider runbook for the staging event:

```bash
ALERT_ID="$(scripts/local-live/community-circles-seed-provider-alert.sh | tail -n 1)"
scripts/local-live/community-circles-provider-receipt-check.sh "$ALERT_ID"
```

Details and evidence requirements live in
`docs/community-circles-provider-runbook.md`. Legal/compliance launch criteria
live in `docs/community-circles-compliance-checklist.md`.

## Readiness Gates

Local-live is not green unless all of these pass:

- `/ready` returns `status=ok`.
- PgBouncer generated config contains `pool_mode = session`.
- Redis `PING` returns `PONG`.
- Core and community DBs both have `_schema_migrations`.
- The env validator rejects production domains, mainnet chain IDs, plaintext Redis, missing secrets, and Stripe live keys.
- The Community Circles provider preflight passes when external alerting is in scope.

## CI And Deploy Policy

CI and deploy should fail hard on:

- E2E test failure.
- Backend readiness failure.
- Missing readiness env components.
- Migration failure.
- Production-looking secrets in local-live files.
- Mainnet chain values in local-live.

Artifacts such as screenshots, reports, and traces should still upload with `if: always()`, but failing tests must keep the job red.

## Known Follow-Up

The repo still contains hardcoded `platform.poool.app` and `www.poool.app` references in public templates, email templates, and generated links. These must be replaced with relative URLs or a central base-URL helper before localhost browser testing can be treated as fully live-like.

Track that work with:

```bash
scripts/local-live/audit-prod-links.sh
```
