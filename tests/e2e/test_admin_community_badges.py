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


def _create_user(prefix, is_admin=False):
    email = f"{prefix}-{uuid.uuid4().hex[:10]}@poool.app"
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

        if is_admin:
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


def _cleanup_core_users(*users):
    conn = _connect_core()
    cur = conn.cursor()
    try:
        for user_id, session_token in users:
            cur.execute("DELETE FROM user_sessions WHERE session_token = %s", (session_token,))
            cur.execute("DELETE FROM user_roles WHERE user_id = %s", (user_id,))
            cur.execute("DELETE FROM users WHERE id = %s", (user_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _cleanup_badge(badge_id=None, badge_code=None, target_user_id=None):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        if badge_id:
            cur.execute("DELETE FROM user_badges WHERE badge_id = %s", (badge_id,))
            cur.execute("DELETE FROM community_audit_logs WHERE entity_id = %s", (badge_id,))
            cur.execute("DELETE FROM badges WHERE id = %s", (badge_id,))
        if badge_code:
            cur.execute("DELETE FROM badges WHERE code = %s", (badge_code,))
        if target_user_id:
            cur.execute(
                "DELETE FROM community_profiles WHERE user_id = %s",
                (target_user_id,),
            )
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _badge_id_for_code(code):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute("SELECT id FROM badges WHERE code = %s", (code,))
        row = cur.fetchone()
        return row[0] if row else None
    finally:
        cur.close()
        conn.close()


def _audit_actions_for_badge(badge_id, target_user_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT action, target_user_id, details->>'badge_code'
            FROM community_audit_logs
            WHERE entity_type = 'badge'
              AND entity_id = %s
            ORDER BY created_at ASC
            """,
            (badge_id,),
        )
        return cur.fetchall()
    finally:
        cur.close()
        conn.close()


def test_admin_community_badges_create_update_grant_revoke_and_audit(quality_page):
    page, tracker = quality_page
    admin_id, admin_session = _create_user("e2e-badge-admin", is_admin=True)
    target_id, target_session = _create_user("e2e-badge-target")
    unique = uuid.uuid4().hex[:10]
    badge_code = f"e2e_badge_{unique}"
    badge_name = f"E2E Badge {unique}"
    updated_name = f"E2E Badge Updated {unique}"
    badge_id = None

    try:
        page.context.add_cookies(
            [{"name": "poool_session", "value": admin_session, "url": BASE_URL}]
        )

        tracker.navigate_and_check(f"{BASE_URL}/admin/community/badges")
        expect(page).to_have_title("Badge Management | Admin | POOOL")
        expect(page.locator("#new-badge-btn")).to_be_visible()

        page.locator("#new-badge-btn").click()
        expect(page.locator("#badge-modal")).to_be_visible()
        expect(page.locator("#badge-code")).to_be_focused()
        page.locator("#badge-code").fill(badge_code)
        page.locator("#badge-name").fill(badge_name)
        page.locator("#badge-description").fill("Created by badge E2E recheck.")
        page.locator("#badge-icon").fill("*")
        page.locator("#badge-order").fill("7")
        with page.expect_response(
            lambda response: response.url.endswith("/api/admin/community/badges")
            and response.request.method == "POST"
        ) as create_response:
            page.locator("#badge-save-btn").click()
        assert create_response.value.status == 200
        expect(page.locator("#badges-grid")).to_contain_text(badge_code, timeout=10_000)
        expect(page.locator("#badges-grid")).to_contain_text(badge_name)

        badge_id = _badge_id_for_code(badge_code)
        assert badge_id is not None

        card = page.locator("#badges-grid > div", has_text=badge_code)
        card.get_by_role("button", name="Edit").click()
        expect(page.locator("#badge-modal")).to_be_visible()
        expect(page.locator("#badge-code")).to_be_disabled()
        page.locator("#badge-name").fill(updated_name)
        page.locator("#badge-description").fill("Updated by badge E2E recheck.")
        page.locator("#badge-order").fill("8")
        with page.expect_response(
            lambda response: f"/api/admin/community/badges/{badge_id}" in response.url
            and response.request.method == "PUT"
        ) as update_response:
            page.locator("#badge-save-btn").click()
        assert update_response.value.status == 200
        expect(page.locator("#badges-grid")).to_contain_text(updated_name, timeout=10_000)

        page.locator("#grant-user-id").fill(str(target_id))
        page.locator("#grant-badge-code").select_option(badge_code)
        with page.expect_response(
            lambda response: f"/api/admin/community/users/{target_id}/badge" in response.url
            and response.request.method == "POST"
        ) as grant_response:
            page.locator("#grant-badge-btn").click()
        assert grant_response.value.status == 200
        expect(page.locator("#grant-badge-status")).to_contain_text(
            "Badge granted successfully", timeout=10_000
        )
        expect(page.locator("#badges-grid")).to_contain_text(str(target_id), timeout=10_000)

        page.on("dialog", lambda dialog: dialog.accept())
        with page.expect_response(
            lambda response: f"/api/admin/community/users/{target_id}/badge/{badge_id}" in response.url
            and response.request.method == "DELETE"
        ) as revoke_response:
            page.locator("#badges-grid > div", has_text=badge_code).get_by_role(
                "button", name="Revoke"
            ).click()
        assert revoke_response.value.status == 200
        expect(page.locator("#badges-grid > div", has_text=badge_code)).not_to_contain_text(
            str(target_id), timeout=10_000
        )

        rows = _audit_actions_for_badge(badge_id, target_id)
        actions = [row[0] for row in rows]
        assert actions.count("badge.create") == 1
        assert actions.count("badge.update") == 1
        assert actions.count("badge.grant") == 1
        assert actions.count("badge.revoke") == 1

        target_rows = [row for row in rows if row[0] in {"badge.grant", "badge.revoke"}]
        assert len(target_rows) == 2
        assert all(row[1] == target_id for row in target_rows)
        assert all(row[2] == badge_code for row in target_rows)

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures()
    finally:
        _cleanup_badge(badge_id, badge_code, target_id)
        _cleanup_core_users((admin_id, admin_session), (target_id, target_session))
