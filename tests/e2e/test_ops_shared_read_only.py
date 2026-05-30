import requests
from playwright.sync_api import expect

from tests.e2e.conftest import BASE_URL, cleanup_test_user, create_e2e_user


def _browser_login(page, user):
    page.context.add_cookies(
        [{"name": "poool_session", "value": user["session_token"], "url": BASE_URL}]
    )


def test_ops_and_shared_read_only_surfaces(quality_page):
    page, tracker = quality_page
    investor = create_e2e_user(
        email_prefix="e2e-readonly-investor",
        display_name="Workflow Readonly Investor",
        cash_balance_cents=12_345,
    )
    admin = create_e2e_user(
        email_prefix="e2e-readonly-admin",
        display_name="Workflow Readonly Admin",
        roles=("admin", "super_admin"),
        cash_balance_cents=0,
    )

    try:
        health = requests.get(f"{BASE_URL}/health", timeout=10)
        assert health.status_code == 200
        assert "ok" in health.text.lower()

        anon_wallet = page.goto(f"{BASE_URL}/wallet", wait_until="domcontentloaded")
        assert anon_wallet is not None
        assert "/auth/login" in page.url

        _browser_login(page, investor)
        tracker.navigate_and_check(f"{BASE_URL}/wallet")
        expect(page.locator("body")).to_contain_text("Wallet")
        investor_balance = page.request.get(f"{BASE_URL}/api/wallet/balance")
        assert investor_balance.ok, investor_balance.text()
        assert isinstance(investor_balance.json()["cash_cents"], int)

        tracker.navigate_and_check(f"{BASE_URL}/settings")
        expect(page.locator("body")).to_contain_text("Settings")

        _browser_login(page, admin)
        for path, expected_text in (
            ("/admin/system", "System"),
            ("/admin/storage", "Storage"),
            ("/admin/audit-logs", "Audit"),
            ("/admin/reports", "Reports"),
        ):
            tracker.navigate_and_check(f"{BASE_URL}{path}")
            expect(page.locator("body")).to_contain_text(expected_text)

        for api_path, expected_keys in (
            ("/api/admin/system", {"api_healthy", "db_connected"}),
            ("/api/admin/storage", {"summary", "bucket"}),
            ("/api/admin/audit-logs", {"logs"}),
        ):
            response = page.request.get(f"{BASE_URL}{api_path}")
            assert response.ok, f"{api_path} returned {response.status}: {response.text()[:300]}"
            body = response.json()
            assert expected_keys.intersection(body.keys()), body

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[401, 403, 404])
    finally:
        cleanup_test_user(investor["user_id"])
        cleanup_test_user(admin["user_id"])
