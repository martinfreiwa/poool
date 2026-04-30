from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TEMPLATE = ROOT / "frontend/platform/affiliate-dashboard.html"
JS = ROOT / "frontend/platform/static/js/affiliate-dashboard.js"
ROUTES = ROOT / "backend/src/rewards/routes.rs"
SERVICE = ROOT / "backend/src/rewards/service.rs"
ADMIN_REWARDS = ROOT / "backend/src/admin/rewards.rs"
ADMIN_REWARDS_JS = ROOT / "frontend/platform/static/js/admin-rewards.js"
ADMIN_AFFILIATE_FINANCE_JS = ROOT / "frontend/platform/admin/js/admin-affiliate-finance.js"
PAYOUT_MIGRATION = ROOT / "database/089_affiliate_payout_requests.sql"


def test_affiliate_dashboard_required_ids_and_status_region_exist():
    html = TEMPLATE.read_text()

    for element_id in [
        "tier-name",
        "tier-rate",
        "affiliate-action-status",
        "subid-stats-body",
        "request-payout-btn",
        "postback-url-input",
    ]:
        assert f'id="{element_id}"' in html

    assert 'role="status"' in html
    assert 'aria-live="polite"' in html


def test_affiliate_dashboard_js_uses_safe_rendering_and_click_contract():
    js = JS.read_text()

    assert "dashboardData.clicks ?? dashboardData.referral_clicks ?? 0" in js
    assert "renderSubIDStats(tbody, data.stats)" in js
    assert "code.textContent = stat.sub_id || 'unknown'" in js
    assert "tbody.innerHTML = data.stats.map" not in js
    assert "${s.sub_id}" not in js


def test_affiliate_dashboard_post_actions_send_csrf_and_visible_status():
    js = JS.read_text()

    assert "function csrfHeaders" in js
    assert "fetch('/api/affiliate/payout/request', { method: 'POST', headers: csrfHeaders() })" in js
    assert "headers: csrfHeaders({ 'Content-Type': 'application/json' })" in js
    assert "alert(" not in js
    assert "setStatus(" in js


def test_affiliate_payout_request_is_durable_and_audited():
    routes = ROUTES.read_text()
    migration = PAYOUT_MIGRATION.read_text()

    assert "affiliate_payout_requests" in migration
    assert "idx_affiliate_payout_requests_open" in migration
    assert "INSERT INTO affiliate_payout_requests" in routes
    assert "affiliate.payout_requested" in routes
    assert "audit_logs" in routes
    assert "crate::common::currency::format_usd(payable)" in routes
    assert "payable as f64" not in routes


def test_affiliate_postback_is_validated_redacted_and_redirect_safe():
    routes = ROUTES.read_text()
    service = SERVICE.read_text()

    assert "service::validate_postback_url(&url).await?" in routes
    assert "tokio::net::lookup_host" in service
    assert "is_blocked_postback_ip" in service
    assert "url_encode(" in service
    assert "redact_url_query" in service
    assert "redirect(reqwest::redirect::Policy::none())" in service


def test_admin_payout_workflow_exposes_manual_requests_and_blocks_unready_release():
    admin_rewards = ADMIN_REWARDS.read_text()
    rewards_js = ADMIN_REWARDS_JS.read_text()
    finance_js = ADMIN_AFFILIATE_FINANCE_JS.read_text()

    assert "pr.amount_cents as payout_request_amount_cents" in admin_rewards
    assert "pr.status as payout_request_status" in admin_rewards
    assert "payout_blocked_reason" in admin_rewards
    assert "UPDATE affiliate_payout_requests" in admin_rewards
    assert "payout_batch_id = $2" in admin_rewards

    for js in [rewards_js, finance_js]:
        assert "payout_request_id" in js
        assert "payout_request_amount_cents" in js
        assert "payout_blocked_reason" in js
        assert "disabled title=" in js
