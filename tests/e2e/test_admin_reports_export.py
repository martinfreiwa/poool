"""
POOOL E2E Tests - Admin Reports exports.
"""

import os

import psycopg2
import pytest
import requests
from playwright.sync_api import expect

from tests.e2e.conftest import cleanup_test_user, create_e2e_user


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def db_connect():
    return psycopg2.connect(DB_URL)


def snapshot_admin_permissions(cur):
    cur.execute(
        """
        SELECT permission
        FROM admin_permissions
        WHERE role_id = (SELECT id FROM roles WHERE name = 'admin')
        ORDER BY permission
        """
    )
    return [row[0] for row in cur.fetchall()]


def set_admin_permissions(cur, permissions):
    cur.execute(
        """
        DELETE FROM admin_permissions
        WHERE role_id = (SELECT id FROM roles WHERE name = 'admin')
        """
    )
    for permission in permissions:
        cur.execute(
            """
            INSERT INTO admin_permissions (role_id, permission)
            SELECT id, %s FROM roles WHERE name = 'admin'
            ON CONFLICT DO NOTHING
            """,
            (permission,),
        )


def restore_admin_permissions(permissions):
    conn = db_connect()
    cur = conn.cursor()
    try:
        set_admin_permissions(cur, permissions)
        conn.commit()
    finally:
        cur.close()
        conn.close()


def admin_session(session_token):
    session = requests.Session()
    session.cookies.set("poool_session", session_token, domain="localhost", path="/")
    session.cookies.set("poool_session", session_token, path="/")
    return session


def seed_wallet_transaction(cur, user_id, marker):
    cur.execute(
        """
        SELECT id
        FROM wallets
        WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
        LIMIT 1
        """,
        (user_id,),
    )
    wallet_id = cur.fetchone()[0]
    cur.execute(
        """
        INSERT INTO wallet_transactions (
            wallet_id, type, status, amount_cents, currency, description
        )
        VALUES (%s, 'deposit', 'completed', 12345, 'USD', %s)
        RETURNING id
        """,
        (wallet_id, marker),
    )
    return cur.fetchone()[0]


@pytest.mark.admin
@pytest.mark.financial
def test_admin_reports_permissions_validation_success_and_audit():
    admin = create_e2e_user(email_prefix="e2e-reports-admin", roles=("admin",))
    denied_admin = create_e2e_user(email_prefix="e2e-reports-denied", roles=("admin",))
    original_permissions = None
    tx_id = None

    conn = db_connect()
    cur = conn.cursor()
    try:
        original_permissions = snapshot_admin_permissions(cur)
        set_admin_permissions(cur, ())
        conn.commit()

        denied_session = admin_session(denied_admin["session_token"])
        denied_page = denied_session.get(f"{BASE_URL}/admin/reports", timeout=10)
        assert denied_page.url.endswith("/admin/")
        denied_api = denied_session.get(
            f"{BASE_URL}/api/admin/reports/user-growth", timeout=10
        )
        assert denied_api.status_code == 403

        cur.execute("BEGIN")
        set_admin_permissions(cur, ("reports.generate",))
        conn.commit()

        session = admin_session(admin["session_token"])
        financial_denied = session.get(
            f"{BASE_URL}/api/admin/reports/financial-summary", timeout=10
        )
        assert financial_denied.status_code == 403

        invalid_date = session.get(
            f"{BASE_URL}/api/admin/reports/user-growth?from=bad-date",
            timeout=10,
        )
        assert invalid_date.status_code == 400
        assert "YYYY-MM-DD" in invalid_date.text

        cur.execute("BEGIN")
        set_admin_permissions(cur, ("reports.generate", "treasury.read"))
        tx_id = seed_wallet_transaction(
            cur, admin["user_id"], "e2e admin reports export marker"
        )
        conn.commit()

        ok = session.get(
            f"{BASE_URL}/api/admin/reports/financial-summary?from=2020-01-01&to=2099-01-01",
            timeout=10,
        )
        assert ok.status_code == 200, ok.text
        payload = ok.json()
        assert payload["report_type"] == "financial-summary"
        assert isinstance(payload["rows"], list)
        assert payload["rows"], "Expected seeded financial report row"

        cur.execute(
            """
            SELECT new_state->>'report_type', (new_state->>'row_count')::int
            FROM audit_logs
            WHERE actor_user_id = %s
              AND action = 'report.exported'
              AND entity_type = 'admin_report'
            ORDER BY created_at DESC
            LIMIT 1
            """,
            (admin["user_id"],),
        )
        audit_row = cur.fetchone()
        assert audit_row is not None
        assert audit_row[0] == "financial-summary"
        assert audit_row[1] >= 1
    finally:
        if tx_id:
            cur.execute("DELETE FROM wallet_transactions WHERE id = %s", (tx_id,))
            cur.execute(
                "DELETE FROM audit_logs WHERE actor_user_id IN (%s, %s)",
                (admin["user_id"], denied_admin["user_id"]),
            )
            conn.commit()
        cur.close()
        conn.close()
        if original_permissions is not None:
            restore_admin_permissions(original_permissions)
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(denied_admin["user_id"])


@pytest.mark.admin
@pytest.mark.financial
def test_admin_reports_csv_preview_error_state_and_accessibility(admin_page, tmp_path):
    page, tracker = admin_page

    tracker.navigate_and_check(f"{BASE_URL}/admin/reports.html")
    tracker.assert_page_loaded()

    expect(page.locator("label[for='range-from']")).to_have_text("From")
    expect(page.locator("label[for='range-to']")).to_have_text("To")
    expect(page.locator("#reports-status")).to_be_hidden()
    expect(page.locator("#dl-btn-investor-pl")).to_contain_text("Download CSV")
    expect(page.locator("button[aria-label='Preview User Growth Report']")).to_be_visible()

    with page.expect_response(
        lambda response: "/api/admin/reports/user-growth" in response.url
        and response.status == 200,
        timeout=10_000,
    ):
        page.locator("button[aria-label='Preview User Growth Report']").click()
    expect(page.locator("#reports-status")).to_contain_text("Preview loaded", timeout=10_000)

    with page.expect_download(timeout=10_000) as download_info:
        page.locator("#dl-btn-users").click()
    download = download_info.value
    save_path = tmp_path / download.suggested_filename
    download.save_as(str(save_path))
    assert save_path.stat().st_size > 0
    assert save_path.read_text().startswith('"')

    page.route(
        "**/api/admin/reports/user-growth**",
        lambda route: route.fulfill(
            status=500,
            content_type="application/json",
            body='{"error":"Synthetic report failure"}',
        ),
    )
    page.locator("#dl-btn-users").click()
    expect(page.locator("#reports-status")).to_contain_text(
        "Synthetic report failure", timeout=10_000
    )

    tracker.assert_no_critical_errors()
