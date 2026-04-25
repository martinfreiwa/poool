import os
import uuid

import psycopg2
import pytest
import requests
from playwright.sync_api import expect


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


def _create_admin_session():
    email = f"e2e-challenge-admin-{uuid.uuid4().hex[:10]}@poool.app"
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
            INSERT INTO user_sessions (user_id, session_token, remember_me, expires_at)
            VALUES (%s, %s, FALSE, NOW() + INTERVAL '1 hour')
            """,
            (user_id, session_token),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()

    session = requests.Session()
    session.cookies.set("poool_session", session_token, domain="localhost", path="/")
    session.cookies.set("poool_session", session_token, path="/")
    return session, user_id, session_token


def _cleanup(user_id, session_token, challenge_id=None):
    community = _connect_community()
    cur = community.cursor()
    try:
        if challenge_id:
            cur.execute("DELETE FROM community_audit_logs WHERE entity_id = %s", (challenge_id,))
            cur.execute("DELETE FROM challenge_progress WHERE challenge_id = %s", (challenge_id,))
            cur.execute("DELETE FROM challenges WHERE id = %s", (challenge_id,))
        community.commit()
    finally:
        cur.close()
        community.close()

    core = _connect_core()
    cur = core.cursor()
    try:
        cur.execute("DELETE FROM user_sessions WHERE session_token = %s", (session_token,))
        cur.execute("DELETE FROM user_roles WHERE user_id = %s", (user_id,))
        cur.execute("DELETE FROM users WHERE id = %s", (user_id,))
        core.commit()
    finally:
        cur.close()
        core.close()


def test_admin_community_challenges_create_toggle_and_audit():
    session, admin_id, session_token = _create_admin_session()
    challenge_id = None

    try:
        page = session.get(f"{BASE_URL}/admin/community/challenges", timeout=10)
        assert page.status_code == 200
        assert "New Challenge" in page.text
        assert 'role="dialog"' in page.text

        csrf_token = session.cookies.get("csrf_token")
        assert csrf_token
        session.headers.update({"X-CSRF-Token": csrf_token})

        listed = session.get(f"{BASE_URL}/api/admin/community/challenges", timeout=10)
        assert listed.status_code == 200
        assert isinstance(listed.json(), list)

        invalid = session.post(
            f"{BASE_URL}/api/admin/community/challenges",
            json={
                "title": "Invalid E2E Challenge",
                "description": "Should be rejected",
                "xp_reward": 1,
                "badge_reward": None,
                "requirement_type": "write_post",
                "requirement_value": 1,
                "frequency": "once",
            },
            timeout=10,
        )
        assert invalid.status_code == 400

        unique = uuid.uuid4().hex[:10]
        created = session.post(
            f"{BASE_URL}/api/admin/community/challenges",
            json={
                "title": f"E2E Challenge {unique}",
                "description": "Created by targeted E2E test.",
                "xp_reward": 42,
                "badge_reward": None,
                "requirement_type": "write_review",
                "requirement_value": 2,
                "frequency": "one_time",
            },
            timeout=10,
        )
        assert created.status_code == 200, created.text
        payload = created.json()
        challenge_id = payload["id"]
        assert payload["requirement_type"] == "write_review"
        assert payload["frequency"] == "one_time"

        community = _connect_community()
        cur = community.cursor()
        try:
            cur.execute(
                """
                SELECT title, requirement_type, requirement_value, frequency, xp_reward, is_active
                FROM challenges
                WHERE id = %s
                """,
                (challenge_id,),
            )
            row = cur.fetchone()
            assert row == (
                f"E2E Challenge {unique}",
                "write_review",
                2,
                "one_time",
                42,
                True,
            )

            cur.execute(
                """
                SELECT COUNT(*)
                FROM community_audit_logs
                WHERE actor_user_id = %s
                  AND action = 'challenge.create'
                  AND entity_type = 'challenge'
                  AND entity_id = %s
                """,
                (admin_id, challenge_id),
            )
            assert cur.fetchone()[0] == 1
        finally:
            cur.close()
            community.close()

        toggled = session.post(
            f"{BASE_URL}/api/admin/community/challenges/{challenge_id}/toggle",
            json={"is_active": False},
            timeout=10,
        )
        assert toggled.status_code == 200, toggled.text
        assert toggled.json()["challenge"]["is_active"] is False

        community = _connect_community()
        cur = community.cursor()
        try:
            cur.execute("SELECT is_active FROM challenges WHERE id = %s", (challenge_id,))
            assert cur.fetchone()[0] is False

            cur.execute(
                """
                SELECT COUNT(*)
                FROM community_audit_logs
                WHERE actor_user_id = %s
                  AND action = 'challenge.toggle'
                  AND entity_type = 'challenge'
                  AND entity_id = %s
                """,
                (admin_id, challenge_id),
            )
            assert cur.fetchone()[0] == 1
        finally:
            cur.close()
            community.close()

        stale = session.post(
            f"{BASE_URL}/api/admin/community/challenges/{uuid.uuid4()}/toggle",
            json={"is_active": True},
            timeout=10,
        )
        assert stale.status_code == 404
    finally:
        _cleanup(admin_id, session_token, challenge_id)


@pytest.mark.admin
def test_admin_community_challenges_modal_keyboard_accessibility(quality_page):
    page, _tracker = quality_page
    _session, admin_id, session_token = _create_admin_session()

    try:
        page.context.add_cookies(
            [
                {
                    "name": "poool_session",
                    "value": session_token,
                    "url": BASE_URL,
                }
            ]
        )

        page.goto(f"{BASE_URL}/admin/community/challenges")
        expect(page.locator("#new-challenge-btn")).to_be_visible()

        page.locator("#new-challenge-btn").click()
        modal = page.locator("#challenge-modal")
        expect(modal).to_be_visible()
        expect(modal).to_have_attribute("role", "dialog")
        expect(modal).to_have_attribute("aria-modal", "true")
        expect(page.locator("#challenge-title")).to_be_focused()

        page.locator("button[aria-label='Close challenge modal']").focus()
        page.keyboard.press("Shift+Tab")
        assert page.evaluate(
            """
            () => {
              const active = document.activeElement;
              return active.tagName === 'BUTTON' && active.textContent.trim() === 'Create Challenge';
            }
            """
        )

        for _ in range(12):
            page.keyboard.press("Tab")
            assert page.evaluate(
                """
                () => document
                  .getElementById('challenge-modal')
                  .contains(document.activeElement)
                """
            )

        page.keyboard.press("Escape")
        expect(modal).to_be_hidden()
        expect(page.locator("#new-challenge-btn")).to_be_focused()
    finally:
        _cleanup(admin_id, session_token)


@pytest.mark.admin
@pytest.mark.mobile
def test_admin_community_challenges_modal_mobile_viewport(mobile_page):
    page, _tracker = mobile_page
    _session, admin_id, session_token = _create_admin_session()

    try:
        page.context.add_cookies(
            [
                {
                    "name": "poool_session",
                    "value": session_token,
                    "url": BASE_URL,
                }
            ]
        )

        page.goto(f"{BASE_URL}/admin/community/challenges")
        expect(page.locator("#new-challenge-btn")).to_be_visible()
        assert page.evaluate("() => document.documentElement.scrollWidth <= window.innerWidth + 1")

        page.locator("#new-challenge-btn").click()
        panel = page.locator("#challenge-modal-panel")
        expect(panel).to_be_visible()
        expect(page.locator("#challenge-title")).to_be_focused()

        box = panel.bounding_box()
        assert box is not None
        assert box["x"] >= 0
        assert box["width"] <= page.viewport_size["width"]
    finally:
        _cleanup(admin_id, session_token)
