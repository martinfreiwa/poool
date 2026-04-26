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


def _admin_session(user):
    session = requests.Session()
    session.cookies.set("poool_session", user["session_token"], domain="localhost", path="/")
    session.cookies.set("poool_session", user["session_token"], path="/")
    page = session.get(f"{BASE_URL}/admin/community/comments", timeout=10)
    assert page.status_code == 200, page.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin comments page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def _seed_comment_fixture(author_id):
    unique = uuid.uuid4().hex[:10]
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO community_profiles (user_id)
            VALUES (%s)
            ON CONFLICT (user_id) DO NOTHING
            """,
            (author_id,),
        )
        cur.execute(
            """
            INSERT INTO posts (user_id, post_type, content, content_sanitized, comment_count)
            VALUES (%s, 'general', %s, %s, 2)
            RETURNING id
            """,
            (
                author_id,
                f"E2E comments post {unique}",
                f"E2E comments post {unique}",
            ),
        )
        post_id = cur.fetchone()[0]

        comments = {}
        for key in ("hide", "delete"):
            content = f"E2E {key} comment {unique}"
            cur.execute(
                """
                INSERT INTO comments (post_id, user_id, content, content_sanitized)
                VALUES (%s, %s, %s, %s)
                RETURNING id
                """,
                (post_id, author_id, content, content),
            )
            comments[key] = {"id": cur.fetchone()[0], "content": content}

        conn.commit()
        return {"post_id": post_id, "comments": comments, "unique": unique}
    finally:
        cur.close()
        conn.close()


def _cleanup_comment_fixture(*, post_id=None, comment_ids=(), user_ids=()):
    community = _connect_community()
    cur = community.cursor()
    try:
        for comment_id in comment_ids:
            cur.execute("DELETE FROM community_audit_logs WHERE entity_id = %s", (comment_id,))
            cur.execute("DELETE FROM comments WHERE id = %s", (comment_id,))
        if post_id:
            cur.execute("DELETE FROM posts WHERE id = %s", (post_id,))
        for user_id in user_ids:
            cur.execute("DELETE FROM community_profiles WHERE user_id = %s", (user_id,))
        community.commit()
    finally:
        cur.close()
        community.close()


def _comment_state(comment_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute("SELECT is_hidden, COALESCE(is_pinned, false) FROM comments WHERE id = %s", (comment_id,))
        return cur.fetchone()
    finally:
        cur.close()
        conn.close()


def _assert_comment_audit(comment_id, *, actor_id, action, target_user_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT COUNT(*)
            FROM community_audit_logs
            WHERE actor_user_id = %s
              AND action = %s
              AND entity_type = 'comment'
              AND entity_id = %s
              AND target_user_id = %s
            """,
            (actor_id, action, comment_id, target_user_id),
        )
        assert cur.fetchone()[0] == 1
    finally:
        cur.close()
        conn.close()


@pytest.mark.admin
@pytest.mark.community
def test_admin_community_comments_permissions_csrf_and_missing_rows():
    admin = create_e2e_user(email_prefix="e2e-comments-admin", roles=("admin",))
    author = create_e2e_user(email_prefix="e2e-comments-author")
    fixture = _seed_comment_fixture(author["user_id"])
    comment_id = fixture["comments"]["hide"]["id"]

    try:
        session = _admin_session(admin)

        _set_admin_role_permissions(())
        denied_list = session.get(f"{BASE_URL}/api/admin/community/comments?limit=1", timeout=10)
        assert denied_list.status_code == 403

        _set_admin_role_permissions(("community.view",))
        allowed_list = session.get(f"{BASE_URL}/api/admin/community/comments?limit=1", timeout=10)
        assert allowed_list.status_code == 200
        denied_hide = session.post(
            f"{BASE_URL}/api/admin/community/comments/{comment_id}/hide",
            json={"reason": "View-only admin cannot hide."},
            timeout=10,
        )
        assert denied_hide.status_code == 403

        _restore_admin_role_permissions()
        missing_csrf = requests.Session()
        missing_csrf.cookies.set("poool_session", admin["session_token"], domain="localhost", path="/")
        missing_csrf.cookies.set("poool_session", admin["session_token"], path="/")
        missing_csrf.cookies.set("csrf_token", session.cookies.get("csrf_token"), domain="localhost", path="/")
        missing_csrf.cookies.set("csrf_token", session.cookies.get("csrf_token"), path="/")
        missing_csrf_response = missing_csrf.post(
            f"{BASE_URL}/api/admin/community/comments/{comment_id}/hide",
            json={"reason": "Missing CSRF should fail."},
            timeout=10,
        )
        assert missing_csrf_response.status_code == 403
        assert _comment_state(comment_id) == (False, False)

        stale_id = uuid.uuid4()
        assert session.post(
            f"{BASE_URL}/api/admin/community/comments/{stale_id}/hide",
            json={"reason": "Missing comment."},
            timeout=10,
        ).status_code == 404
        assert session.delete(f"{BASE_URL}/api/admin/community/comments/{stale_id}", timeout=10).status_code == 404
        assert session.post(
            f"{BASE_URL}/api/admin/community/comments/{stale_id}/pin",
            json={"is_pinned": True},
            timeout=10,
        ).status_code == 404
    finally:
        _restore_admin_role_permissions()
        _cleanup_comment_fixture(
            post_id=fixture["post_id"],
            comment_ids=[item["id"] for item in fixture["comments"].values()],
            user_ids=(author["user_id"],),
        )
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(author["user_id"])


@pytest.mark.admin
@pytest.mark.community
def test_admin_community_comments_browser_moderation_and_audit(quality_page):
    page, tracker = quality_page
    admin = create_e2e_user(email_prefix="e2e-comments-admin", roles=("admin",))
    author = create_e2e_user(email_prefix="e2e-comments-author")
    fixture = _seed_comment_fixture(author["user_id"])
    session = _admin_session(admin)
    hide_comment = fixture["comments"]["hide"]
    delete_comment = fixture["comments"]["delete"]

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
        page.on("dialog", lambda dialog: dialog.accept())

        page.goto(f"{BASE_URL}/admin/community/comments")
        expect(page.locator("#comments-table")).to_be_visible()
        expect(page.get_by_text(hide_comment["content"])).to_be_visible()
        expect(page.get_by_text(delete_comment["content"])).to_be_visible()

        page.locator("#comments-search").fill(hide_comment["content"])
        expect(page.get_by_text(hide_comment["content"])).to_be_visible()
        expect(page.get_by_text(delete_comment["content"])).to_have_count(0)
        page.locator("#comments-search").fill("")
        expect(page.get_by_text(delete_comment["content"])).to_be_visible()

        page.locator("tr").filter(has_text=hide_comment["content"]).get_by_role("button", name="Hide").click()
        expect(page.locator("tr").filter(has_text=hide_comment["content"]).get_by_text("Hidden")).to_be_visible()
        assert _comment_state(hide_comment["id"]) == (True, False)
        _assert_comment_audit(
            hide_comment["id"],
            actor_id=admin["user_id"],
            action="comment.hide",
            target_user_id=author["user_id"],
        )

        pin_response = session.post(
            f"{BASE_URL}/api/admin/community/comments/{hide_comment['id']}/pin",
            json={"is_pinned": True},
            timeout=10,
        )
        assert pin_response.status_code == 200, pin_response.text
        assert _comment_state(hide_comment["id"]) == (True, True)
        _assert_comment_audit(
            hide_comment["id"],
            actor_id=admin["user_id"],
            action="comment.pin",
            target_user_id=author["user_id"],
        )

        page.locator("tr").filter(has_text=delete_comment["content"]).get_by_label("Delete comment permanently").click()
        expect(page.get_by_text(delete_comment["content"])).to_have_count(0)
        assert _comment_state(delete_comment["id"]) is None
        _assert_comment_audit(
            delete_comment["id"],
            actor_id=admin["user_id"],
            action="comment.delete",
            target_user_id=author["user_id"],
        )

        assert tracker.get_critical_errors() == []
    finally:
        _cleanup_comment_fixture(
            post_id=fixture["post_id"],
            comment_ids=[item["id"] for item in fixture["comments"].values()],
            user_ids=(author["user_id"],),
        )
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(author["user_id"])
