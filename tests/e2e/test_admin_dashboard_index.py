import os

import psycopg2
from playwright.sync_api import expect
from tests.e2e.conftest import take_named_screenshot

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def get_db_connection():
    return psycopg2.connect(DB_URL)


def _insert_malicious_audit_row():
    conn = get_db_connection()
    cur = conn.cursor()
    cur.execute(
        """
        INSERT INTO audit_logs (action, entity_type, entity_id)
        VALUES ('e2e.dashboard_xss', '<img src=x onerror=alert(1)>', NULL)
        RETURNING id
        """
    )
    row_id = cur.fetchone()[0]
    conn.commit()
    cur.close()
    conn.close()
    return row_id


def _delete_audit_row(row_id):
    conn = get_db_connection()
    cur = conn.cursor()
    cur.execute("DELETE FROM audit_logs WHERE id = %s", (row_id,))
    conn.commit()
    cur.close()
    conn.close()


def _insert_unread_notification():
    conn = get_db_connection()
    cur = conn.cursor()
    cur.execute(
        """
        INSERT INTO users (email, email_verified, status)
        VALUES ('e2e-dashboard-notification-' || gen_random_uuid() || '@poool.app', TRUE, 'active')
        RETURNING id
        """
    )
    user_id = cur.fetchone()[0]
    cur.execute(
        """
        INSERT INTO notifications (user_id, title, message, type, is_read)
        VALUES (%s, 'E2E dashboard notification', 'Unread dashboard badge coverage', 'system', FALSE)
        RETURNING id
        """,
        (user_id,),
    )
    notification_id = cur.fetchone()[0]
    cur.execute("SELECT COUNT(*) FROM notifications WHERE is_read = FALSE")
    unread_count = cur.fetchone()[0]
    conn.commit()
    cur.close()
    conn.close()
    return user_id, notification_id, unread_count


def _delete_notification_user(user_id, notification_id):
    conn = get_db_connection()
    cur = conn.cursor()
    cur.execute("DELETE FROM notifications WHERE id = %s", (notification_id,))
    cur.execute("DELETE FROM users WHERE id = %s", (user_id,))
    conn.commit()
    cur.close()
    conn.close()


def test_admin_dashboard_loads_safely_and_searches_server_side(admin_page):
    page, tracker = admin_page
    audit_row_id = _insert_malicious_audit_row()
    notification_user_id, notification_id, unread_count = _insert_unread_notification()
    try:
        with page.expect_response(lambda r: "/api/admin/stats/overview" in r.url and r.status == 200):
            tracker.navigate_and_check(f"{BASE_URL}/admin/")

        expect(page.locator("#kpi-total-users")).not_to_have_text("Unavailable")
        expected_badge = "99+" if unread_count > 99 else str(unread_count)
        expect(page.locator("#notification-count")).to_have_text(expected_badge)
        expect(page.locator("#notification-count")).to_be_visible()
        expect(page.locator("#activity-feed")).to_contain_text("Dashboard Xss")
        expect(page.locator("#activity-feed")).to_contain_text("<img src=x onerror=alert(1)>")
        assert page.locator("#activity-feed img").count() == 0
        assert page.locator("script[src^='https://']").count() == 0
        assert page.locator("[onclick]").count() == 0

        system = page.request.get(f"{BASE_URL}/api/admin/system")
        assert system.ok
        system_json = system.json()
        assert "psp_connected" in system_json
        assert "kyc_provider" in system_json
        assert "email_configured" in system_json
        assert system_json["api_healthy"] == (
            system_json["db_healthy"]
            and system_json["database"]["storage_available"]
            and system_json["database"]["table_stats_available"]
        )

        seen_urls = []
        page.on("request", lambda request: seen_urls.append(request.url))
        search = page.locator("#admin-global-search")
        search.focus()
        expect(search).to_be_focused()
        with page.expect_response(lambda r: "/api/admin/search" in r.url and r.status == 200):
            search.fill("e2e")
        expect(page.locator(".admin-global-search-results")).to_be_visible()
        search.press("ArrowDown")
        expect(page.locator(".admin-search-result-item").first).to_be_focused()
        page.keyboard.press("Escape")
        expect(search).to_be_focused()

        assert any("/api/admin/search" in url for url in seen_urls)
        assert not any("/api/admin/users" in url for url in seen_urls)
        assert not any("/api/admin/assets" in url for url in seen_urls)
        assert not any("/api/admin/orders" in url for url in seen_urls)
        assert not any("/api/admin/deposits" in url for url in seen_urls)

        page.locator("#admin-notification-button").focus()
        page.keyboard.press("Enter")
        expect(page).to_have_url(f"{BASE_URL}/admin/notifications.html")

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[404])
    finally:
        _delete_audit_row(audit_row_id)
        _delete_notification_user(notification_user_id, notification_id)


def test_admin_dashboard_stats_failure_shows_visible_retry(admin_page):
    page, tracker = admin_page

    page.route(
        "**/api/admin/stats/overview?**",
        lambda route: route.fulfill(
            status=503,
            content_type="application/json",
            body='{"error":"dashboard stats unavailable"}',
        ),
    )

    tracker.navigate_and_check(f"{BASE_URL}/admin/")

    expect(page.locator("#kpi-total-users")).to_have_text("Unavailable")
    expect(page.locator("#activity-feed [role='alert']")).to_contain_text("Dashboard stats unavailable")
    expect(page.locator("#recent-orders-table [role='alert']")).to_contain_text("Dashboard stats unavailable")
    expect(page.locator("#pending-deposits-table [role='alert']")).to_contain_text("Dashboard stats unavailable")
    expect(page.get_by_role("button", name="Retry").first).to_be_visible()

    tracker.assert_no_critical_errors()


def test_admin_dashboard_mobile_viewport_recheck(admin_mobile_page):
    page, tracker = admin_mobile_page

    with page.expect_response(lambda r: "/api/admin/stats/overview" in r.url and r.status == 200):
        tracker.navigate_and_check(f"{BASE_URL}/admin/")

    expect(page.locator(".admin-page-title")).to_be_visible()
    expect(page.locator("#admin-global-search")).to_be_visible()
    expect(page.locator(".admin-kpi-card").first).to_be_visible()

    layout = page.evaluate(
        """
        () => {
          const viewportWidth = window.innerWidth;
          const selectors = [
            ".admin-main",
            ".admin-topbar",
            ".admin-content",
            ".admin-kpi-grid",
            ".admin-section-grid",
          ];
          const clipped = selectors.flatMap((selector) => {
            return Array.from(document.querySelectorAll(selector)).map((el) => {
              const rect = el.getBoundingClientRect();
              return {
                selector,
                left: rect.left,
                right: rect.right,
                width: rect.width,
                visible: rect.width > 0 && rect.height > 0,
                overflows: rect.left < -1 || rect.right > viewportWidth + 1,
              };
            });
          }).filter((item) => !item.visible || item.overflows);
          return {
            viewportWidth,
            scrollWidth: document.documentElement.scrollWidth,
            clipped,
          };
        }
        """
    )
    assert not layout["clipped"], layout
    assert layout["scrollWidth"] <= layout["viewportWidth"] + 1, layout

    screenshot_path = take_named_screenshot(page, "admin_dashboard_mobile_recheck")
    assert screenshot_path.exists()
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures()
