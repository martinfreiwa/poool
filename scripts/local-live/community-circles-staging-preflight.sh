#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ENV_FILE="${1:-.env.local-live}"
REQUIRE_EXTERNAL_ALERTS="${REQUIRE_EXTERNAL_ALERTS:-1}"

cd "$ROOT_DIR"

if [[ "$REQUIRE_EXTERNAL_ALERTS" == "1" ]]; then
  export POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS=1
fi

scripts/local-live/validate-env.sh "$ENV_FILE"

(
  cd backend
  cargo test circle_ops_alert_webhook_tests --lib
)

python3 -m pytest \
  tests/test_community_circles_phase8_static.py \
  tests/test_community_circles_phase9_static.py \
  tests/test_community_tab_contract_static.py \
  tests/test_community_circles_staging_preflight_static.py \
  -q

cat <<'EOF'
Community Circles staging provider preflight passed locally.

Staging execution checklist:
1. Start the staging/local-live stack with the same env file.
2. Seed one synthetic critical provider alert with community-circles-seed-provider-alert.sh.
3. Confirm Slack delivery in the configured channel.
4. Confirm PagerDuty event creation and capture the incident/event id.
5. Run community-circles-provider-receipt-check.sh against the seeded alert id.
   It validates provider_response_status, provider_response_at, attempts, and no raw secret values.
6. Resolve the alert in the platform-admin UI and record the audit log id.
EOF
