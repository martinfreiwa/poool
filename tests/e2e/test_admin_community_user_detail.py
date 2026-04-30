import os
import uuid

import psycopg2
import pytest
import requests
from playwright.sync_api import expect

from tests.e2e.conftest import cleanup_test_user, create_e2e_user


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
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


def _admin_session(admin, target_user_id):
    session = requests.Session()
    session.cookies.set("poool_session", admin["session_token"], domain="localhost", path="/")
    session.cookies.set("poool_session", admin["session_token"], path="/")
    page = session.get(f"{BASE_URL}/admin/community/user-detail?id={target_user_id}", timeout=10)
    assert page.status_code == 200, page.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin user detail page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def _seed_user_detail_fixture(user_id):
    unique = uuid.uuid4().hex[:10]
    display_name = f"E2E <img src=x onerror=alert('name')> {unique}"
    bio = f"Bio <img src=x onerror=alert('bio')> {unique}"
    mod_notes = f"Notes <script>alert('notes')</script> {unique}"
    post_content = f"Post <img src=x onerror=alert('post')> {unique}"
    badge_code = f"e2e-user-detail-{unique}"

    core_conn = _connect_core()
    core_cur = core_conn.cursor()
    conn = _connect_community()
    cur = conn.cursor()
    try:
        core_cur.execute(
            """
            UPDATE user_profiles
            SET display_name = %s
            WHERE user_id = %s
            """,
            (display_name, user_id),
        )
        core_conn.commit()
        cur.execute(
            """
            INSERT INTO community_profiles (user_id, bio, mod_notes, warning_count)
            VALUES (%s, %s, %s, 0)
            ON CONFLICT (user_id) DO UPDATE SET
                bio = EXCLUDED.bio,
                mod_notes = EXCLUDED.mod_notes,
                warning_count = 0,
                muted_until = NULL,
                is_shadowbanned = FALSE,
                is_community_banned = FALSE,
                ban_reason = NULL
            """,
            (user_id, bio, mod_notes),
        )
        cur.execute(
            """
            INSERT INTO posts (user_id, post_type, content, content_sanitized)
            VALUES (%s, 'general', %s, %s)
            RETURNING id
            """,
            (user_id, post_content, post_content),
        )
        post_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO badges (code, name, description, icon, display_order)
            VALUES (%s, %s, %s, %s, 999)
            RETURNING id
            """,
            (
                badge_code,
                f"Badge <img src=x onerror=alert('badge')> {unique}",
                f"Badge description <script>alert('badge')</script> {unique}",
                "<img onerror=x>",
            ),
        )
        badge_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO user_badges (user_id, badge_id)
            VALUES (%s, %s)
            ON CONFLICT DO NOTHING
            """,
            (user_id, badge_id),
        )
        conn.commit()
        return {
            "badge_code": badge_code,
            "badge_id": badge_id,
            "bio": bio,
            "display_name": display_name,
            "mod_notes": mod_notes,
            "post_content": post_content,
            "post_id": post_id,
        }
    finally:
        core_cur.close()
        core_conn.close()
        cur.close()
        conn.close()


def _cleanup_user_detail_fixture(*, user_id, post_id=None, badge_id=None):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            "DELETE FROM community_audit_logs WHERE target_user_id = %s OR actor_user_id = %s",
            (user_id, user_id),
        )
        cur.execute("DELETE FROM notifications WHERE user_id = %s OR actor_id = %s", (user_id, user_id))
        if badge_id:
            cur.execute("DELETE FROM user_badges WHERE badge_id = %s", (badge_id,))
            cur.execute("DELETE FROM badges WHERE id = %s", (badge_id,))
        if post_id:
            cur.execute("DELETE FROM posts WHERE id = %s", (post_id,))
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
            SELECT warning_count, muted_until IS NOT NULL, is_shadowbanned,
                   is_community_banned, ban_reason, mod_notes
            FROM community_profiles
            WHERE user_id = %s
            """,
            (user_id,),
        )
        return cur.fetchone()
    finally:
        cur.close()
        conn.close()


def _count_user_detail_audit(user_id, actor_id, actions):
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
              AND action = ANY(%s)
            """,
            (user_id, actor_id, list(actions)),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


@pytest.mark.admin
@pytest.mark.community
def test_admin_community_user_detail_api_permissions_csrf_and_audit():
    admin = create_e2e_user(email_prefix="e2e-user-detail-admin", roles=("admin",))
    target = create_e2e_user(email_prefix="e2e-user-detail-target")
    fixture = _seed_user_detail_fixture(target["user_id"])

    try:
        _restore_admin_role_permissions()
        session = _admin_session(admin, target["user_id"])

        _set_admin_role_permissions(())
        denied_detail = session.get(
            f"{BASE_URL}/api/admin/community/users/{target['user_id']}/detail",
            timeout=10,
        )
        assert denied_detail.status_code == 403

        _set_admin_role_permissions(("community.view",))
        allowed_detail = session.get(
            f"{BASE_URL}/api/admin/community/users/{target['user_id']}/detail",
            timeout=10,
        )
        assert allowed_detail.status_code == 200
        denied_warn = session.post(
            f"{BASE_URL}/api/admin/community/users/{target['user_id']}/warn",
            json={"reason": "View-only admins cannot warn users."},
            timeout=10,
        )
        assert denied_warn.status_code == 403

        _restore_admin_role_permissions()
        missing_csrf = requests.Session()
        missing_csrf.cookies.set("poool_session", admin["session_token"], domain="localhost", path="/")
        missing_csrf.cookies.set("poool_session", admin["session_token"], path="/")
        missing_csrf.cookies.set("csrf_token", session.cookies.get("csrf_token"), domain="localhost", path="/")
        missing_csrf.cookies.set("csrf_token", session.cookies.get("csrf_token"), path="/")
        missing_csrf_response = missing_csrf.post(
            f"{BASE_URL}/api/admin/community/users/{target['user_id']}/warn",
            json={"reason": "Missing CSRF should fail."},
            timeout=10,
        )
        assert missing_csrf_response.status_code == 403
        assert _fetch_profile(target["user_id"])[0] == 0

        missing_user = uuid.uuid4()
        assert session.post(
            f"{BASE_URL}/api/admin/community/users/{missing_user}/warn",
            json={"reason": "Missing user."},
            timeout=10,
        ).status_code == 404
        assert session.post(
            f"{BASE_URL}/api/admin/community/users/{target['user_id']}/ban",
            json={"is_banned": True, "reason": "  "},
            timeout=10,
        ).status_code == 400

        actions = [
            ("warn", {"reason": "API warning reason"}),
            ("mute", {"hours": 2}),
            ("shadowban", {"is_shadowbanned": True}),
            ("ban", {"is_banned": True, "reason": "API ban reason"}),
            ("mod-notes", {"notes": "API moderator notes"}),
        ]
        for endpoint, payload in actions:
            response = session.post(
                f"{BASE_URL}/api/admin/community/users/{target['user_id']}/{endpoint}",
                json=payload,
                timeout=10,
            )
            assert response.status_code == 200, response.text

        assert _fetch_profile(target["user_id"]) == (
            1,
            True,
            True,
            True,
            "API ban reason",
            "API moderator notes",
        )
        assert _count_user_detail_audit(
            target["user_id"],
            admin["user_id"],
            ["user.warn", "user.mute", "user.shadowban", "user.ban", "user.mod_notes.update"],
        ) == 5
    finally:
        _restore_admin_role_permissions()
        _cleanup_user_detail_fixture(
            user_id=target["user_id"],
            post_id=fixture["post_id"],
            badge_id=fixture["badge_id"],
        )
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(target["user_id"])


@pytest.mark.admin
@pytest.mark.community
def test_admin_community_user_detail_browser_safe_rendering_and_dialog(quality_page):
    page, tracker = quality_page
    admin = create_e2e_user(email_prefix="e2e-user-detail-admin", roles=("admin",))
    target = create_e2e_user(email_prefix="e2e-user-detail-target")
    fixture = _seed_user_detail_fixture(target["user_id"])

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
        page.goto(f"{BASE_URL}/admin/community/user-detail?id={target['user_id']}")

        expect(page.get_by_role("heading", name=fixture["display_name"])).to_be_visible()
        expect(page.get_by_text(fixture["bio"])).to_be_visible()
        expect(page.get_by_text(fixture["mod_notes"])).to_be_visible()
        expect(page.get_by_text(fixture["post_content"])).to_be_visible()
        expect(page.locator("img[src='x']")).to_have_count(0)
        expect(page.locator("script", has_text="alert")).to_have_count(0)

        page.get_by_role("button", name="Send Warning", exact=True).click()
        expect(page.get_by_role("dialog", name="Send warning")).to_be_visible()
        page.locator("#moderation-text-input").fill("Browser warning reason")
        page.get_by_role("button", name="Send warning", exact=True).click()
        expect(page.locator("#moderation-dialog")).to_be_hidden()
        expect(page.get_by_text("Warnings")).to_be_visible()
        expect(page.locator(".admin-user-detail-metric-value").filter(has_text="1")).to_be_visible()
        assert _fetch_profile(target["user_id"])[0] == 1
        assert not tracker.get_critical_errors()
    finally:
        _restore_admin_role_permissions()
        _cleanup_user_detail_fixture(
            user_id=target["user_id"],
            post_id=fixture["post_id"],
            badge_id=fixture["badge_id"],
        )
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(target["user_id"])
