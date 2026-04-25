import os

import psycopg2
from playwright.sync_api import expect

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


def test_admin_dashboard_loads_safely_and_searches_server_side(admin_page):
    page, tracker = admin_page
    audit_row_id = _insert_malicious_audit_row()
    try:
        with page.expect_response(lambda r: "/api/admin/stats/overview" in r.url and r.status == 200):
            tracker.navigate_and_check(f"{BASE_URL}/admin/")

        expect(page.locator("#kpi-total-users")).not_to_have_text("Unavailable")
        expect(page.locator("#activity-feed")).to_contain_text("Dashboard Xss")
        expect(page.locator("#activity-feed")).to_contain_text("<img src=x onerror=alert(1)>")
        assert page.locator("#activity-feed img").count() == 0

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
        with page.expect_response(lambda r: "/api/admin/search" in r.url and r.status == 200):
            search.fill("e2e")
        expect(page.locator(".admin-global-search-results")).to_be_visible()

        assert any("/api/admin/search" in url for url in seen_urls)
        assert not any("/api/admin/users" in url for url in seen_urls)
        assert not any("/api/admin/assets" in url for url in seen_urls)
        assert not any("/api/admin/orders" in url for url in seen_urls)
        assert not any("/api/admin/deposits" in url for url in seen_urls)

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures()
    finally:
        _delete_audit_row(audit_row_id)
