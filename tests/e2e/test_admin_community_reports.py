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


def _admin_session(user):
    session = requests.Session()
    session.cookies.set("poool_session", user["session_token"], domain="localhost", path="/")
    session.cookies.set("poool_session", user["session_token"], path="/")
    page = session.get(f"{BASE_URL}/admin/community/reports", timeout=10)
    assert page.status_code == 200, page.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin reports page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


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


def _seed_report(author_id, reporter_id, *, reason=None, content=None):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        for user_id in (author_id, reporter_id):
            cur.execute(
                """
                INSERT INTO community_profiles (user_id)
                VALUES (%s)
                ON CONFLICT (user_id) DO NOTHING
                """,
                (user_id,),
            )
        cur.execute(
            """
            INSERT INTO posts (user_id, post_type, content, content_sanitized)
            VALUES (%s, 'general', %s, %s)
            RETURNING id
            """,
            (author_id, content or f"E2E reported post {uuid.uuid4().hex}", content),
        )
        post_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO content_reports (post_id, reporter_id, reason)
            VALUES (%s, %s, %s)
            RETURNING id
            """,
            (post_id, reporter_id, reason or f"E2E report reason {uuid.uuid4().hex}"),
        )
        report_id = cur.fetchone()[0]
        conn.commit()
        return {"post_id": post_id, "report_id": report_id}
    finally:
        cur.close()
        conn.close()


def _cleanup_report_fixture(*, post_id=None, report_id=None, user_ids=()):
    community = _connect_community()
    cur = community.cursor()
    try:
        if report_id:
            cur.execute("DELETE FROM community_audit_logs WHERE entity_id = %s", (report_id,))
            cur.execute("DELETE FROM content_reports WHERE id = %s", (report_id,))
        if post_id:
            cur.execute("DELETE FROM posts WHERE id = %s", (post_id,))
        for user_id in user_ids:
            cur.execute("DELETE FROM community_profiles WHERE user_id = %s", (user_id,))
        community.commit()
    finally:
        cur.close()
        community.close()


def _fetch_report(report_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            "SELECT status, admin_notes FROM content_reports WHERE id = %s",
            (report_id,),
        )
        return cur.fetchone()
    finally:
        cur.close()
        conn.close()


def _assert_audit_log(report_id, *, actor_id, action, target_user_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT COUNT(*)
            FROM community_audit_logs
            WHERE actor_user_id = %s
              AND action = %s
              AND entity_type = 'content_report'
              AND entity_id = %s
              AND target_user_id = %s
              AND details->>'admin_notes' IS NOT NULL
            """,
            (actor_id, action, report_id, target_user_id),
        )
        assert cur.fetchone()[0] == 1
    finally:
        cur.close()
        conn.close()


@pytest.mark.admin
@pytest.mark.community
def test_admin_community_reports_api_requires_permissions_and_csrf():
    admin = create_e2e_user(email_prefix="e2e-reports-admin", roles=("admin",))
    author = create_e2e_user(email_prefix="e2e-reports-author")
    reporter = create_e2e_user(email_prefix="e2e-reports-reporter")
    fixture = _seed_report(author["user_id"], reporter["user_id"])

    try:
        session = _admin_session(admin)

        _set_admin_role_permissions(())
        denied_list = session.get(f"{BASE_URL}/api/admin/community/reports", timeout=10)
        assert denied_list.status_code == 403

        _set_admin_role_permissions(("community.view",))
        allowed_list = session.get(f"{BASE_URL}/api/admin/community/reports", timeout=10)
        assert allowed_list.status_code == 200
        denied_action = session.post(
            f"{BASE_URL}/api/admin/community/reports/{fixture['report_id']}/action",
            json={"action": "dismiss_report", "admin_notes": "View-only admin cannot act."},
            timeout=10,
        )
        assert denied_action.status_code == 403

        _restore_admin_role_permissions()
        missing_csrf = requests.Session()
        missing_csrf.cookies.set("poool_session", admin["session_token"], domain="localhost", path="/")
        missing_csrf.cookies.set("poool_session", admin["session_token"], path="/")
        missing_csrf.cookies.set("csrf_token", session.cookies.get("csrf_token"), domain="localhost", path="/")
        response = missing_csrf.post(
            f"{BASE_URL}/api/admin/community/reports/{fixture['report_id']}/action",
            json={"action": "dismiss_report", "admin_notes": "Missing CSRF should fail."},
            timeout=10,
        )
        assert response.status_code == 403
        assert _fetch_report(fixture["report_id"])[0] == "pending"
    finally:
        _restore_admin_role_permissions()
        _cleanup_report_fixture(
            post_id=fixture["post_id"],
            report_id=fixture["report_id"],
            user_ids=(author["user_id"], reporter["user_id"]),
        )
        for user in (admin, author, reporter):
            cleanup_test_user(user["user_id"])


@pytest.mark.admin
@pytest.mark.community
def test_admin_community_report_actions_validate_notes_audit_and_conflicts():
    admin = create_e2e_user(email_prefix="e2e-reports-admin", roles=("admin",))
    author = create_e2e_user(email_prefix="e2e-reports-author")
    reporter = create_e2e_user(email_prefix="e2e-reports-reporter")
    session = _admin_session(admin)
    fixtures = []

    try:
        empty_notes = _seed_report(author["user_id"], reporter["user_id"], reason="empty-notes")
        fixtures.append(empty_notes)
        rejected = session.post(
            f"{BASE_URL}/api/admin/community/reports/{empty_notes['report_id']}/action",
            json={"action": "dismiss_report", "admin_notes": "   "},
            timeout=10,
        )
        assert rejected.status_code == 400
        assert "Admin notes are required" in rejected.text
        assert _fetch_report(empty_notes["report_id"])[0] == "pending"

        cases = [
            ("hide_post", "report.hide_post", "resolved"),
            ("warn_user", "report.warn_user", "resolved"),
            ("ban_user", "report.ban_user", "resolved"),
            ("dismiss_report", "report.dismiss", "dismissed"),
        ]
        for action, audit_action, expected_status in cases:
            fixture = _seed_report(author["user_id"], reporter["user_id"], reason=action)
            fixtures.append(fixture)
            notes = f"Moderation note for {action}"
            response = session.post(
                f"{BASE_URL}/api/admin/community/reports/{fixture['report_id']}/action",
                json={"action": action, "admin_notes": notes},
                timeout=10,
            )
            assert response.status_code == 200, response.text
            assert _fetch_report(fixture["report_id"]) == (expected_status, notes)
            _assert_audit_log(
                fixture["report_id"],
                actor_id=admin["user_id"],
                action=audit_action,
                target_user_id=author["user_id"],
            )

            if action == "hide_post":
                conn = _connect_community()
                cur = conn.cursor()
                try:
                    cur.execute(
                        "SELECT is_hidden, hidden_reason FROM posts WHERE id = %s",
                        (fixture["post_id"],),
                    )
                    assert cur.fetchone() == (True, f"Hidden after report: {notes}")
                finally:
                    cur.close()
                    conn.close()

                stale = session.post(
                    f"{BASE_URL}/api/admin/community/reports/{fixture['report_id']}/action",
                    json={"action": "hide_post", "admin_notes": "Second action should conflict."},
                    timeout=10,
                )
                assert stale.status_code == 409

        conn = _connect_community()
        cur = conn.cursor()
        try:
            cur.execute(
                """
                SELECT warning_count, is_community_banned, ban_reason
                FROM community_profiles
                WHERE user_id = %s
                """,
                (author["user_id"],),
            )
            warning_count, is_banned, ban_reason = cur.fetchone()
            assert warning_count >= 1
            assert is_banned is True
            assert ban_reason == "Moderation note for ban_user"
        finally:
            cur.close()
            conn.close()
    finally:
        for fixture in fixtures:
            _cleanup_report_fixture(
                post_id=fixture["post_id"],
                report_id=fixture["report_id"],
                user_ids=(),
            )
        _cleanup_report_fixture(user_ids=(author["user_id"], reporter["user_id"]))
        for user in (admin, author, reporter):
            cleanup_test_user(user["user_id"])


@pytest.mark.admin
@pytest.mark.community
def test_admin_community_reports_safe_rendering_and_accessible_modal(admin_page):
    page, tracker = admin_page
    marker = f"e2e-safe-render-{uuid.uuid4().hex[:8]}"
    author = create_e2e_user(
        email_prefix="e2e-reports-author",
        display_name=f"{marker} <img src=x onerror=window.__pooolXss=1>",
    )
    reporter = create_e2e_user(
        email_prefix="e2e-reports-reporter",
        display_name=f"Reporter <svg onload=window.__pooolXss=2> {marker}",
    )
    fixture = _seed_report(
        author["user_id"],
        reporter["user_id"],
        reason=f"{marker} <script>window.__pooolXss=3</script>",
        content=f"Post body {marker} <img src=x onerror=window.__pooolXss=4>",
    )

    try:
        tracker.navigate_and_check(f"{BASE_URL}/admin/community/reports")
        row = page.locator("#reports-table tr", has_text=marker).first
        expect(row).to_be_visible(timeout=10000)
        assert page.evaluate("window.__pooolXss") is None
        assert page.locator("script[src*='cdn.jsdelivr.net']").count() == 0
        assert page.locator("[onclick]").count() == 0

        hide_button = row.get_by_role("button", name="Hide Post report")
        hide_button.click()
        modal = page.get_by_role("dialog", name="Hide Reported Post")
        expect(modal).to_be_visible()
        expect(page.locator("#modal-notes")).to_be_focused()

        page.keyboard.press("Escape")
        expect(modal).to_be_hidden()
        expect(hide_button).to_be_focused()
        tracker.assert_no_critical_errors()
    finally:
        _cleanup_report_fixture(
            post_id=fixture["post_id"],
            report_id=fixture["report_id"],
            user_ids=(author["user_id"], reporter["user_id"]),
        )
        cleanup_test_user(author["user_id"])
        cleanup_test_user(reporter["user_id"])
