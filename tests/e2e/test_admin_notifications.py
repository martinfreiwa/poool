import os
import uuid

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


def admin_session(session_token, path="/admin/notifications"):
    session = requests.Session()
    session.cookies.set("poool_session", session_token, domain="localhost", path="/")
    session.cookies.set("poool_session", session_token, path="/")
    response = session.get(f"{BASE_URL}{path}", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin notifications page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def seed_notification(cur, user_id, marker):
    title = f"E2E Notifications {marker} Alpha"
    message = f"Searchable message {marker} <img src=x onerror=alert(1)>"
    cur.execute(
        """
        INSERT INTO notifications (user_id, title, message, type, is_read)
        VALUES (%s, %s, %s, 'system', FALSE)
        RETURNING id
        """,
        (user_id, title, message),
    )
    return cur.fetchone()[0], title, message


def cleanup_marker(marker):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            DELETE FROM notifications
            WHERE title LIKE %s OR message LIKE %s
            """,
            (f"%{marker}%", f"%{marker}%"),
        )
        cur.execute(
            """
            DELETE FROM audit_logs
            WHERE action = 'notification.broadcast'
              AND (
                new_state->>'title' LIKE %s
                OR metadata->>'title' LIKE %s
              )
            """,
            (f"%{marker}%", f"%{marker}%"),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()


def broadcast_audit_count(marker, actor_id):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT COUNT(*)
            FROM audit_logs
            WHERE actor_user_id = %s
              AND action = 'notification.broadcast'
              AND entity_type = 'notifications'
              AND new_state->>'title' LIKE %s
              AND (new_state->>'recipient_count')::bigint > 0
            """,
            (actor_id, f"%{marker}%"),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


@pytest.mark.admin
def test_admin_notifications_permissions_validation_csrf_audit_and_ui(quality_page):
    marker = f"e2e-notifications-{uuid.uuid4().hex[:10]}"
    admin = create_e2e_user(email_prefix="e2e-notifications-admin", roles=("admin",))
    denied_admin = create_e2e_user(email_prefix="e2e-notifications-denied", roles=("admin",))
    seeded_notification_id = None
    original_permissions = None

    conn = db_connect()
    cur = conn.cursor()
    try:
        original_permissions = snapshot_admin_permissions(cur)
        set_admin_permissions(cur, ())
        conn.commit()

        denied_session = requests.Session()
        denied_session.cookies.set("poool_session", denied_admin["session_token"])
        denied_page = denied_session.get(f"{BASE_URL}/admin/notifications", timeout=10)
        assert denied_page.url.endswith("/admin/")
        denied_api = denied_session.get(f"{BASE_URL}/api/admin/notifications", timeout=10)
        assert denied_api.status_code == 403

        cur.execute("BEGIN")
        allowed_permissions = sorted(
            set(original_permissions) | {"notifications.view", "notifications.send"}
        )
        set_admin_permissions(cur, allowed_permissions)
        seeded_notification_id, seeded_title, seeded_message = seed_notification(
            cur, admin["user_id"], marker
        )
        conn.commit()

        unauth_page = requests.get(f"{BASE_URL}/admin/notifications", timeout=10)
        assert unauth_page.status_code == 200 or unauth_page.status_code == 303
        assert unauth_page.url.endswith("/auth/login") or unauth_page.status_code == 303
        unauth_api = requests.get(f"{BASE_URL}/api/admin/notifications", timeout=10)
        assert unauth_api.status_code == 401

        session = admin_session(admin["session_token"])
        list_response = session.get(f"{BASE_URL}/api/admin/notifications", timeout=10)
        assert list_response.status_code == 200, list_response.text
        notifications = list_response.json()["notifications"]
        assert any(item["id"] == str(seeded_notification_id) for item in notifications)

        invalid_type = session.post(
            f"{BASE_URL}/api/admin/notifications/broadcast",
            json={"type": "bad", "title": f"{marker} invalid", "message": "Invalid type"},
            timeout=10,
        )
        assert invalid_type.status_code == 400
        empty_message = session.post(
            f"{BASE_URL}/api/admin/notifications/broadcast",
            json={"type": "system", "title": f"{marker} empty", "message": " "},
            timeout=10,
        )
        assert empty_message.status_code == 400

        no_csrf = requests.Session()
        no_csrf.cookies.set("poool_session", admin["session_token"])
        no_csrf.cookies.set("csrf_token", session.cookies.get("csrf_token"))
        csrf_denied = no_csrf.post(
            f"{BASE_URL}/api/admin/notifications/broadcast",
            json={"type": "system", "title": f"{marker} missing csrf", "message": "Denied"},
            timeout=10,
        )
        assert csrf_denied.status_code == 403

        broadcast_title = f"E2E Broadcast {marker}"
        broadcast = session.post(
            f"{BASE_URL}/api/admin/notifications/broadcast",
            json={
                "type": "system",
                "title": broadcast_title,
                "message": f"Broadcast message {marker}",
            },
            timeout=20,
        )
        assert broadcast.status_code == 200, broadcast.text
        recipient_count = broadcast.json()["count"]
        assert recipient_count > 0
        assert broadcast_audit_count(marker, admin["user_id"]) == 1

        page, tracker = quality_page
        page.context.add_cookies(
            [{"name": "poool_session", "value": admin["session_token"], "url": BASE_URL}]
        )
        with page.expect_response(
            lambda response: "/api/admin/notifications" in response.url and response.status == 200
        ):
            tracker.navigate_and_check(f"{BASE_URL}/admin/notifications")

        expect(page.locator("#broadcast-status")).to_have_attribute("role", "status")
        expect(page.locator("#notif-table-body")).to_contain_text(broadcast_title)
        expect(page.locator("#notif-table-body")).to_contain_text(f"Broadcast message {marker}")
        assert page.locator("#notif-table-body img[src='x']").count() == 0
        assert page.locator("script[src^='https://unpkg.com/htmx']").count() == 0

        search = page.locator("#notif-search")
        search.fill(marker)
        expect(page.locator("#notif-count-label")).to_contain_text("notifications")
        expect(page.locator("#notif-table-body")).to_contain_text(broadcast_title)

        page.eval_on_selector(
            "#filter-type",
            """el => {
                el.value = "system";
                el.dispatchEvent(new Event("change", { bubbles: true }));
            }""",
        )
        page.eval_on_selector(
            "#filter-read",
            """el => {
                el.value = "false";
                el.dispatchEvent(new Event("change", { bubbles: true }));
            }""",
        )
        expect(page.locator("#notif-table-body")).to_contain_text(broadcast_title)

        user_sort = page.get_by_role("button", name="User")
        user_sort.focus()
        expect(user_sort).to_be_focused()
        page.keyboard.press("Enter")
        expect(page.locator("th[data-sort='user_name']")).to_have_attribute("aria-sort", "ascending")

        page.locator("#notif-search").fill("")
        page.eval_on_selector(
            "#filter-read",
            """el => {
                el.value = "";
                el.dispatchEvent(new Event("change", { bubbles: true }));
            }""",
        )
        page.locator("#broadcast-title").fill(f"E2E Browser Broadcast {marker}")
        page.locator("#broadcast-message").fill(f"Browser message {marker}")
        with page.expect_response(
            lambda response: response.url.endswith("/api/admin/notifications/broadcast")
            and response.request.method == "POST",
            timeout=20_000,
        ) as broadcast_response:
            page.locator("#broadcast-send-btn").click()
        assert broadcast_response.value.status == 200
        expect(page.locator("#broadcast-status")).to_contain_text("Broadcast sent to")

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[404])
    finally:
        conn.rollback()
        cur.close()
        conn.close()
        cleanup_marker(marker)
        if original_permissions is not None:
            restore_admin_permissions(original_permissions)
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(denied_admin["user_id"])
