import os
import uuid

import psycopg2
from playwright.sync_api import expect


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
CORE_DB_DSN = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
COMMUNITY_DB_DSN = os.environ.get(
    "COMMUNITY_DB_DSN",
    os.environ.get("COMMUNITY_DATABASE_URL", "dbname=poool_community user=martin host=localhost"),
)


def _connect_core():
    return psycopg2.connect(CORE_DB_DSN)


def _connect_community():
    return psycopg2.connect(COMMUNITY_DB_DSN)


def _create_admin_session():
    email = f"e2e-ann-admin-{uuid.uuid4().hex[:10]}@poool.app"
    session_token = str(uuid.uuid4())

    conn = _connect_core()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO users (email, email_verified, status)
            VALUES (%s, TRUE, 'active')
            RETURNING id
            """,
            (email,),
        )
        user_id = cur.fetchone()[0]

        cur.execute(
            """
            INSERT INTO user_roles (user_id, role_id, is_active)
            SELECT %s, id, TRUE
            FROM roles
            WHERE name = 'admin'
            ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE
            """,
            (user_id,),
        )

        cur.execute(
            """
            INSERT INTO admin_permissions (role_id, permission)
            SELECT id, 'community.manage'
            FROM roles
            WHERE name = 'admin'
            ON CONFLICT DO NOTHING
            """
        )

        cur.execute(
            """
            INSERT INTO user_sessions (user_id, session_token, remember_me, expires_at)
            VALUES (%s, %s, FALSE, NOW() + INTERVAL '1 hour')
            """,
            (user_id, session_token),
        )
        conn.commit()
        return user_id, session_token
    finally:
        cur.close()
        conn.close()


def _find_announcement(user_id, marker):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT p.id, ac.category
            FROM posts p
            JOIN announcement_categories ac ON ac.post_id = p.id
            WHERE p.user_id = %s
              AND p.post_type = 'announcement'
              AND p.content LIKE %s
            ORDER BY p.created_at DESC
            LIMIT 1
            """,
            (user_id, f"%{marker}%"),
        )
        return cur.fetchone()
    finally:
        cur.close()
        conn.close()


def _audit_count(user_id, post_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT COUNT(*)
            FROM community_audit_logs
            WHERE actor_user_id = %s
              AND action = 'announcement.create'
              AND entity_type = 'announcement'
              AND entity_id = %s
            """,
            (user_id, post_id),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def _cleanup(user_id, session_token, marker):
    community = _connect_community()
    cur = community.cursor()
    try:
        cur.execute(
            """
            DELETE FROM community_audit_logs
            WHERE actor_user_id = %s
               OR target_user_id = %s
               OR entity_id IN (
                    SELECT id FROM posts WHERE user_id = %s OR content LIKE %s
               )
            """,
            (user_id, user_id, user_id, f"%{marker}%"),
        )
        cur.execute(
            "DELETE FROM posts WHERE user_id = %s OR content LIKE %s",
            (user_id, f"%{marker}%"),
        )
        cur.execute("DELETE FROM community_profiles WHERE user_id = %s", (user_id,))
        community.commit()
    finally:
        cur.close()
        community.close()

    core = _connect_core()
    cur = core.cursor()
    try:
        cur.execute("DELETE FROM audit_logs WHERE actor_user_id = %s OR entity_id = %s", (user_id, user_id))
        cur.execute("DELETE FROM user_sessions WHERE session_token = %s", (session_token,))
        cur.execute("DELETE FROM user_roles WHERE user_id = %s", (user_id,))
        cur.execute("DELETE FROM users WHERE id = %s", (user_id,))
        core.commit()
    finally:
        cur.close()
        core.close()


def test_admin_community_announcements_publish_csrf_and_audit(quality_page):
    page, tracker = quality_page
    admin_id, session_token = _create_admin_session()
    marker = f"E2E announcement {uuid.uuid4().hex[:10]}"

    try:
        page.context.add_cookies(
            [{"name": "poool_session", "value": session_token, "url": BASE_URL}]
        )

        tracker.navigate_and_check(f"{BASE_URL}/admin/community/announcements")
        expect(page.locator("h1")).to_contain_text("Manage Announcements")
        expect(page.locator("#open-create-modal")).to_be_visible()

        list_response = page.context.request.get(
            f"{BASE_URL}/api/admin/community/announcements"
        )
        assert list_response.status == 200, list_response.text()
        assert isinstance(list_response.json(), list)

        no_csrf_response = page.context.request.post(
            f"{BASE_URL}/api/admin/community/announcements",
            data={
                "content": f"<p>{marker} missing csrf</p>",
                "category": "platform_update",
                "is_pinned": False,
                "image_urls": None,
            },
        )
        assert no_csrf_response.status == 403

        page.locator("#open-create-modal").click()
        modal = page.locator("#create-modal")
        expect(modal).to_be_visible()
        expect(modal).to_have_attribute("role", "dialog")
        expect(modal).to_have_attribute("aria-modal", "true")

        page.locator("#ann-category").select_option("platform_update")
        page.wait_for_function(
            "window.Quill && document.querySelector('#editor-container .ql-editor')"
        )
        page.locator("#editor-container .ql-editor").fill(marker)

        with page.expect_response(
            lambda response: response.url.endswith("/api/admin/community/announcements")
            and response.request.method == "POST"
        ) as publish_response:
            page.locator("#create-form button[type='submit']").click()
        assert publish_response.value.status == 200, publish_response.value.text()

        expect(page.locator("#announcements-table")).to_contain_text(marker, timeout=10_000)

        row = _find_announcement(admin_id, marker)
        assert row is not None
        post_id, category = row
        assert category == "platform_update"
        assert _audit_count(admin_id, post_id) == 1

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[403])
    finally:
        _cleanup(admin_id, session_token, marker)
