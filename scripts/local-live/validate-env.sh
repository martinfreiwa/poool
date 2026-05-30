#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${1:-.env.local-live}"
FORCE_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS="${POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS:-}"

if [[ -f "$ENV_FILE" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "$ENV_FILE"
  set +a
fi

if [[ -n "$FORCE_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS" ]]; then
  POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS="$FORCE_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS"
fi

errors=0

fail() {
  printf 'ERROR: %s\n' "$1" >&2
  errors=$((errors + 1))
}

require() {
  local key="$1"
  local value="${!key:-}"
  if [[ -z "$value" ]]; then
    fail "$key is required"
  fi
}

require_one_of() {
  local a="$1"
  local b="$2"
  if [[ -z "${!a:-}" && -z "${!b:-}" ]]; then
    fail "$a or $b is required"
  fi
}

reject_placeholder() {
  local key="$1"
  local value="${!key:-}"
  if [[ "$value" =~ replace|REPLACE|placeholder|PLACEHOLDER|not-for-production ]]; then
    fail "$key still looks like a placeholder"
  fi
}

validate_https_webhook_url() {
  local key="$1"
  local value="${!key:-}"
  if [[ -z "$value" ]]; then
    return 0
  fi

  if [[ "$value" =~ [[:cntrl:]] ]]; then
    fail "$key contains control characters"
  fi

  if [[ "$value" != https://* ]]; then
    fail "$key must use HTTPS for local-live/staging"
  fi

  if [[ "$value" == *"@"* || "$value" == *"#"* ]]; then
    fail "$key must not contain URL credentials or fragments"
  fi
}

require APP_ENV
require POOOL_ENV
require BASE_URL
require_one_of SESSION_SECRET JWT_SECRET
require_one_of TOTP_SECRET_ENCRYPTION_KEY ENCRYPTION_KEY
require GCS_BUCKET_NAME
require DIDIT_API_KEY
require DIDIT_WORKFLOW_ID
require DIDIT_WEBHOOK_SECRET
require SANITY_PROJECT_ID
require SANITY_DATASET
require SANITY_API_VERSION
require_one_of SANITY_READ_TOKEN SANITY_WRITE_TOKEN
require SENTRY_DSN
require STRIPE_SECRET_KEY
require STRIPE_PUBLISHABLE_KEY
require CHAIN_NETWORK
require CHAIN_ID

if [[ "${POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS:-}" == "1" ]]; then
  require POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL
  require POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY
fi

[[ "${APP_ENV:-}" == "staging" ]] || fail "APP_ENV must be staging for local-live"
[[ "${POOOL_ENV:-}" == "staging" ]] || fail "POOOL_ENV must be staging for local-live"
[[ "${PGBOUNCER_ENABLED:-true}" == "true" ]] || fail "PGBOUNCER_ENABLED must be true for local-live"

if [[ "${BASE_URL:-}" == *"platform.poool.app"* || "${BASE_URL:-}" == *"www.poool.app"* ]]; then
  fail "BASE_URL must not point at production"
fi

if [[ "${BASE_URL:-}" != https://* && "${LOCAL_LIVE_ALLOW_HTTP:-}" != "1" ]]; then
  fail "BASE_URL must be HTTPS for local-live unless LOCAL_LIVE_ALLOW_HTTP=1"
fi

if [[ "${REDIS_URL:-}" == redis://* ]]; then
  fail "REDIS_URL must not be plaintext redis:// in local-live; use unix:// or rediss://"
fi

if [[ -n "${POOOL_GCS_DOWNLOAD_FAKE_ROOT:-}" ]]; then
  case "${APP_ENV:-}" in
    development|dev|local) ;;
    *)
      fail "POOOL_GCS_DOWNLOAD_FAKE_ROOT is development-only and must not be set for local-live/staging"
      ;;
  esac
fi

validate_https_webhook_url POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL
validate_https_webhook_url POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL

if [[ "${STRIPE_SECRET_KEY:-}" != sk_test_* ]]; then
  fail "STRIPE_SECRET_KEY must be a Stripe test key"
fi

if [[ "${STRIPE_PUBLISHABLE_KEY:-}" != pk_test_* ]]; then
  fail "STRIPE_PUBLISHABLE_KEY must be a Stripe test publishable key"
fi

case "${CHAIN_ID:-}" in
  1|10|56|137|42161|8453)
    fail "CHAIN_ID appears to be a mainnet chain; local-live requires testnet/staging"
    ;;
esac

case "${CHAIN_NETWORK:-}" in
  mainnet|ethereum|polygon|base|optimism|arbitrum)
    fail "CHAIN_NETWORK appears to be mainnet; local-live requires testnet/staging"
    ;;
esac

for key in SESSION_SECRET JWT_SECRET TOTP_SECRET_ENCRYPTION_KEY ENCRYPTION_KEY DIDIT_API_KEY DIDIT_WEBHOOK_SECRET SANITY_READ_TOKEN SANITY_WRITE_TOKEN SENTRY_DSN; do
  reject_placeholder "$key"
done

for key in POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL; do
  if [[ -n "${!key:-}" ]]; then
    reject_placeholder "$key"
  fi
done

if [[ -n "${POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY:-}" && "${#POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY}" -lt 16 ]]; then
  fail "POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY looks too short"
fi

if [[ "$errors" -gt 0 ]]; then
  printf 'local-live env validation failed with %s error(s).\n' "$errors" >&2
  exit 1
fi

printf 'local-live env validation passed.\n'
