from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_local_live_env_validator_hardens_circle_provider_rollout():
    validator = read("scripts/local-live/validate-env.sh")
    env_example = read(".env.local-live.example")
    compose = read("compose.local-live.yml")

    for token in [
        "POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS",
        "POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL",
        "POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY",
        "POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL",
        "FORCE_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS",
        "validate_https_webhook_url",
        "must use HTTPS for local-live/staging",
        "must not contain URL credentials or fragments",
        "POOOL_GCS_DOWNLOAD_FAKE_ROOT is development-only",
    ]:
        assert token in validator

    for token in [
        "POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS=0",
        "POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL=",
        "POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY=",
        "POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL=https://events.pagerduty.com/v2/enqueue",
        "POOOL_GCS_DOWNLOAD_FAKE_ROOT",
    ]:
        assert token in env_example

    for token in [
        "POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL: ${POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL:-}",
        "POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY: ${POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY:-}",
        "POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL: ${POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL:-}",
        "POOOL_CIRCLE_OPS_ALERT_FANOUT_SECS: ${POOOL_CIRCLE_OPS_ALERT_FANOUT_SECS:-}",
    ]:
        assert token in compose


def test_circle_staging_preflight_script_runs_relevant_gates():
    script = read("scripts/local-live/community-circles-staging-preflight.sh")
    seed_script = read("scripts/local-live/community-circles-seed-provider-alert.sh")
    receipt_script = read("scripts/local-live/community-circles-provider-receipt-check.sh")
    docs = read("docs/LOCAL_LIVE_PARITY.md")
    audit = read("docs/community-circles-post-implementation-audit.md")
    provider_runbook = read("docs/community-circles-provider-runbook.md")
    compliance_checklist = read("docs/community-circles-compliance-checklist.md")

    for token in [
        "scripts/local-live/validate-env.sh",
        "POOOL_CIRCLE_OPS_REQUIRE_EXTERNAL_ALERTS=1",
        "cargo test circle_ops_alert_webhook_tests --lib",
        "tests/test_community_circles_phase8_static.py",
        "tests/test_community_circles_phase9_static.py",
        "tests/test_community_tab_contract_static.py",
        "tests/test_community_circles_staging_preflight_static.py",
        "provider_response_status",
        "PagerDuty event creation",
    ]:
        assert token in script

    for token in [
        "Community Circles Provider Preflight",
        "community-circles-staging-preflight.sh",
        "community-circles-provider-receipt-check.sh",
        "rejects public HTTP webhooks",
        "POOOL_GCS_DOWNLOAD_FAKE_ROOT",
    ]:
        assert token in docs

    for token in [
        "live Slack/PagerDuty credential validation",
        "POOOL_CIRCLE_OPS_SLACK_WEBHOOK_URL",
        "POOOL_CIRCLE_OPS_PAGERDUTY_ROUTING_KEY",
        "POOOL_CIRCLE_OPS_PAGERDUTY_EVENTS_URL",
        "community-circles-provider-runbook.md",
        "community-circles-compliance-checklist.md",
    ]:
        assert token in audit

    for token in [
        "INSERT INTO circle_ops_alerts",
        "INSERT INTO circle_ops_alert_notifications",
        "'slack'",
        "'pagerduty'",
        "'synthetic', true",
        "PSQL_CMD",
    ]:
        assert token in seed_script

    for token in [
        "provider_response_status BETWEEN 200 AND 299",
        "secret_leak_candidates",
        "Slack receipt is missing or not delivered.",
        "PagerDuty receipt is missing or not delivered.",
        "Provider notification payload appears to contain webhook or routing secrets.",
        "PSQL_CMD",
    ]:
        assert token in receipt_script

    for token in [
        "Synthetic Provider Alert",
        "Receipt Check",
        "provider_response_status BETWEEN 200 AND 299",
        "PagerDuty event or incident ID",
        "Do not copy webhook URLs",
        "Production Gate",
    ]:
        assert token in provider_runbook

    for token in [
        "User opinion, not financial advice",
        "Verified Investor",
        "Verified Expert",
        "Private Circle mentions",
        "Holder-only resources",
        "Production launch is blocked",
    ]:
        assert token in compliance_checklist
