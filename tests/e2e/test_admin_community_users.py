import os
import uuid

import psycopg2
import pytest
import requests
from playwright.sync_api import expect

from tests.e2e.conftest import BASE_URL, cleanup_test_user, create_e2e_user


CORE_DB_DSN = os.environ.get("DB_DSN", "dbname=poool user=martin host=localhost")
COMMUNITY_DB_DSN = os.environ.get(
    "COMMUNITY_DB_DSN",
    os.environ.get("COMMUNITY_DATABASE_URL", "dbname=poool_community user=martin host=localhost"),
)


def _connect_core():
    return psycopg2.connect(CORE_DB_DSN)


def _connect_community():
    return psycopg2.connect(COMMUNITY_DB_DSN)


def _set_admin_role_permissions(permissions):
    conn = _connect_core()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            DELETE FROM admin_permissions
            WHERE role_id = (SELECT id FROM roles WHERE name = 'admin')
              AND permission IN ('all', 'community.view', 'community.manage')
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
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _restore_admin_role_permissions():
    _set_admin_role_permissions(("all", "community.view", "community.manage"))


def _admin_session(admin):
    session = requests.Session()
    session.cookies.set("poool_session", admin["session_token"], domain="localhost", path="/")
    session.cookies.set("poool_session", admin["session_token"], path="/")
    page = session.get(f"{BASE_URL}/admin/community/users", timeout=10)
    assert page.status_code == 200, page.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin community users page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def _seed_community_user(user_id):
    unique = uuid.uuid4().hex[:10]
    display_name = f"E2E <img src=x onerror=alert('users')> {unique}"
    conn = _connect_core()
    cur = conn.cursor()
    try:
        cur.execute(
            "UPDATE user_profiles SET display_name = %s WHERE user_id = %s",
            (display_name, user_id),
        )
        cur.execute(
            "UPDATE users SET avatar_url = %s WHERE id = %s",
            ("javascript:alert('avatar')", user_id),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()

    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO community_profiles (
                user_id, warning_count, post_count, is_community_banned, ban_reason, mod_notes
            )
            VALUES (%s, 2, 3, FALSE, NULL, %s)
            ON CONFLICT (user_id) DO UPDATE SET
                warning_count = 2,
                post_count = 3,
                is_community_banned = FALSE,
                ban_reason = NULL,
                mod_notes = EXCLUDED.mod_notes,
                muted_until = NULL,
                is_shadowbanned = FALSE,
                updated_at = NOW()
            """,
            (user_id, f"E2E users list notes {unique}"),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()
    return {"display_name": display_name}


def _cleanup_community_user(user_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            "DELETE FROM community_audit_logs WHERE target_user_id = %s OR actor_user_id = %s",
            (user_id, user_id),
        )
        cur.execute("DELETE FROM community_profiles WHERE user_id = %s", (user_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _fetch_profile(user_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT is_community_banned, ban_reason
            FROM community_profiles
            WHERE user_id = %s
            """,
            (user_id,),
        )
        return cur.fetchone()
    finally:
        cur.close()
        conn.close()


def _count_ban_audit(user_id, actor_id, action):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT COUNT(*)
            FROM community_audit_logs
            WHERE target_user_id = %s
              AND actor_user_id = %s
              AND entity_type = 'user'
              AND action = %s
              AND details ? 'previous_profile'
              AND details ? 'new_profile'
            """,
            (user_id, actor_id, action),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


@pytest.mark.admin
@pytest.mark.community
def test_admin_community_users_api_permissions_csrf_and_audit():
    admin = create_e2e_user(email_prefix="e2e-community-users-admin", roles=("admin",))
    target = create_e2e_user(email_prefix="e2e-community-users-target")
    _seed_community_user(target["user_id"])

    try:
        _restore_admin_role_permissions()
        session = _admin_session(admin)

        _set_admin_role_permissions(())
        denied_list = session.get(f"{BASE_URL}/api/admin/community/users", timeout=10)
        assert denied_list.status_code == 403

        _set_admin_role_permissions(("community.view",))
        allowed_list = session.get(f"{BASE_URL}/api/admin/community/users", timeout=10)
        assert allowed_list.status_code == 200
        assert any(
            user["user_id"] == str(target["user_id"])
            for user in allowed_list.json()
        )
        denied_ban = session.post(
            f"{BASE_URL}/api/admin/community/users/{target['user_id']}/ban",
            json={"is_banned": True, "reason": "View-only admins cannot ban users."},
            timeout=10,
        )
        assert denied_ban.status_code == 403
        assert _fetch_profile(target["user_id"]) == (False, None)

        _restore_admin_role_permissions()
        missing_csrf = requests.Session()
        missing_csrf.cookies.set("poool_session", admin["session_token"], domain="localhost", path="/")
        missing_csrf.cookies.set("poool_session", admin["session_token"], path="/")
        missing_csrf.cookies.set("csrf_token", session.cookies.get("csrf_token"), domain="localhost", path="/")
        missing_csrf.cookies.set("csrf_token", session.cookies.get("csrf_token"), path="/")
        missing_csrf_response = missing_csrf.post(
            f"{BASE_URL}/api/admin/community/users/{target['user_id']}/ban",
            json={"is_banned": True, "reason": "Missing CSRF should fail."},
            timeout=10,
        )
        assert missing_csrf_response.status_code == 403
        assert _fetch_profile(target["user_id"]) == (False, None)

        missing_user = uuid.uuid4()
        assert session.post(
            f"{BASE_URL}/api/admin/community/users/{missing_user}/ban",
            json={"is_banned": True, "reason": "Missing user should fail."},
            timeout=10,
        ).status_code == 404

        blank_reason = session.post(
            f"{BASE_URL}/api/admin/community/users/{target['user_id']}/ban",
            json={"is_banned": True, "reason": "   "},
            timeout=10,
        )
        assert blank_reason.status_code == 400
        assert _fetch_profile(target["user_id"]) == (False, None)

        ban_reason = "API ban reason"
        banned = session.post(
            f"{BASE_URL}/api/admin/community/users/{target['user_id']}/ban",
            json={"is_banned": True, "reason": ban_reason},
            timeout=10,
        )
        assert banned.status_code == 200, banned.text
        assert _fetch_profile(target["user_id"]) == (True, ban_reason)
        assert _count_ban_audit(target["user_id"], admin["user_id"], "user.ban") == 1

        unbanned = session.post(
            f"{BASE_URL}/api/admin/community/users/{target['user_id']}/ban",
            json={"is_banned": False, "reason": None},
            timeout=10,
        )
        assert unbanned.status_code == 200, unbanned.text
        assert _fetch_profile(target["user_id"]) == (False, None)
        assert _count_ban_audit(target["user_id"], admin["user_id"], "user.unban") == 1
    finally:
        _restore_admin_role_permissions()
        _cleanup_community_user(target["user_id"])
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(target["user_id"])


@pytest.mark.admin
@pytest.mark.community
def test_admin_community_users_browser_rendering_detail_link_and_dialog(quality_page):
    page, tracker = quality_page
    admin = create_e2e_user(email_prefix="e2e-community-users-admin", roles=("admin",))
    target = create_e2e_user(email_prefix="e2e-community-users-target")
    fixture = _seed_community_user(target["user_id"])

    try:
        _restore_admin_role_permissions()
        page.context.add_cookies(
            [
                {
                    "name": "poool_session",
                    "value": admin["session_token"],
                    "url": BASE_URL,
                }
            ]
        )
        tracker.navigate_and_check(f"{BASE_URL}/admin/community/users")

        row = page.locator("tbody#users-table tr", has_text=fixture["display_name"]).first
        expect(row).to_be_visible(timeout=10000)
        expect(row.get_by_text("3")).to_be_visible()
        expect(row.get_by_text("2 Warnings")).to_be_visible()
        expect(row.get_by_text("Active")).to_be_visible()
        expect(row.locator("img")).to_have_count(0)
        expect(page.locator("img[src^='javascript:']")).to_have_count(0)
        expect(page.locator("img[src='x']")).to_have_count(0)
        expect(page.locator("script", has_text="alert")).to_have_count(0)

        page.get_by_role("button", name="Refresh").click()
        expect(page.get_by_text("community users loaded")).to_be_visible(timeout=10000)

        with page.expect_popup() as popup_info:
            row.get_by_role("link", name="View").click(modifiers=["Meta"])
        detail_page = popup_info.value
        detail_page.wait_for_load_state("domcontentloaded")
        assert f"id={target['user_id']}" in detail_page.url
        detail_page.close()

        row.get_by_role("button", name="Ban").click()
        expect(page.get_by_role("dialog", name="Ban community user")).to_be_visible()
        expect(page.locator("#community-user-ban-reason")).to_be_focused()
        page.keyboard.press("Escape")
        expect(page.locator("#community-user-ban-dialog")).to_be_hidden()

        row.get_by_role("button", name="Ban").click()
        page.locator("#community-user-ban-reason").fill("Browser ban reason")
        page.get_by_role("button", name="Ban user").click()
        expect(page.get_by_text("banned.")).to_be_visible(timeout=10000)
        expect(page.locator("#community-user-ban-dialog")).to_be_hidden()
        assert _fetch_profile(target["user_id"]) == (True, "Browser ban reason")
        assert _count_ban_audit(target["user_id"], admin["user_id"], "user.ban") == 1
        assert not tracker.get_critical_errors()
    finally:
        _restore_admin_role_permissions()
        _cleanup_community_user(target["user_id"])
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(target["user_id"])
