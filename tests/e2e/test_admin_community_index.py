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
    email = f"e2e-community-index-admin-{uuid.uuid4().hex[:10]}@poool.app"
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
            WHERE name IN ('admin', 'super_admin')
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


def _seed_community_overview(admin_id):
    unique = uuid.uuid4().hex[:10]
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO community_profiles (user_id, xp_total)
            VALUES (%s, 321)
            ON CONFLICT (user_id) DO UPDATE SET xp_total = EXCLUDED.xp_total
            """,
            (admin_id,),
        )
        cur.execute(
            """
            INSERT INTO posts (user_id, post_type, content, content_sanitized, is_pinned)
            VALUES (%s, 'announcement', %s, %s, TRUE)
            RETURNING id
            """,
            (
                admin_id,
                f"<img src=x onerror=window.__communityIndexXss=1>E2E Announcement {unique}",
                f"E2E Announcement {unique}",
            ),
        )
        announcement_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO announcement_categories (post_id, category)
            VALUES (%s, 'platform_update')
            """,
            (announcement_id,),
        )
        cur.execute(
            """
            INSERT INTO posts (user_id, post_type, content, content_sanitized)
            VALUES (%s, 'general', %s, %s)
            RETURNING id
            """,
            (
                admin_id,
                f"GENERAL_SHOULD_NOT_RENDER_{unique}",
                f"GENERAL_SHOULD_NOT_RENDER_{unique}",
            ),
        )
        general_post_id = cur.fetchone()[0]
        conn.commit()
        return {
            "unique": unique,
            "announcement_id": announcement_id,
            "general_post_id": general_post_id,
        }
    finally:
        cur.close()
        conn.close()


def _community_xp_sum():
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute("SELECT COALESCE(SUM(xp_total), 0) FROM community_profiles")
        return int(cur.fetchone()[0])
    finally:
        cur.close()
        conn.close()


def _cleanup(admin_id, session_token, seeded=None):
    seeded = seeded or {}
    conn = _connect_community()
    cur = conn.cursor()
    try:
        for post_id in (seeded.get("announcement_id"), seeded.get("general_post_id")):
            if post_id:
                cur.execute("DELETE FROM posts WHERE id = %s", (post_id,))
        cur.execute("DELETE FROM community_profiles WHERE user_id = %s", (admin_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()

    conn = _connect_core()
    cur = conn.cursor()
    try:
        cur.execute("DELETE FROM user_sessions WHERE session_token = %s", (session_token,))
        cur.execute("DELETE FROM user_roles WHERE user_id = %s", (admin_id,))
        cur.execute("DELETE FROM users WHERE id = %s", (admin_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()


def test_admin_community_index_loads_stats_and_announcements_safely(quality_page):
    page, tracker = quality_page
    admin_id, session_token = _create_admin_session()
    seeded = _seed_community_overview(admin_id)
    expected_xp = _community_xp_sum()
    page.context.add_cookies([{"name": "poool_session", "value": session_token, "url": BASE_URL}])

    try:
        stats_response = None
        announcements_response = None

        def capture_response(response):
            nonlocal stats_response, announcements_response
            if response.url.endswith("/api/admin/community/stats"):
                stats_response = response
            if response.url.endswith("/api/admin/community/announcements"):
                announcements_response = response

        page.on("response", capture_response)
        tracker.navigate_and_check(f"{BASE_URL}/admin/community/")
        expect(page.locator("#kpi-xp")).to_have_text(f"{expected_xp:,}", timeout=10_000)
        expect(page.locator("#recent-announcements-table")).to_contain_text(
            "platform_update",
            timeout=10_000,
        )
        expect(page.locator("#recent-announcements-table")).not_to_contain_text(
            f"GENERAL_SHOULD_NOT_RENDER_{seeded['unique']}"
        )

        assert stats_response is not None
        assert stats_response.status == 200
        stats = stats_response.json()
        assert stats["total_xp"] == expected_xp

        assert announcements_response is not None
        assert announcements_response.status == 200
        announcements = announcements_response.json()
        assert any(str(item["id"]) == str(seeded["announcement_id"]) for item in announcements)
        assert all(str(item["id"]) != str(seeded["general_post_id"]) for item in announcements)

        assert page.evaluate("window.__communityIndexXss === undefined")
        assert page.locator("#recent-announcements-table img[onerror]").count() == 0
        assert page.locator("#recent-announcements-table script").count() == 0

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[404])
    finally:
        _cleanup(admin_id, session_token, seeded)


def test_admin_community_index_visible_api_error_state(quality_page):
    page, _tracker = quality_page
    admin_id, session_token = _create_admin_session()
    seeded = _seed_community_overview(admin_id)
    page.context.add_cookies([{"name": "poool_session", "value": session_token, "url": BASE_URL}])

    try:
        page.route(
            "**/api/admin/community/stats",
            lambda route: route.fulfill(
                status=500,
                content_type="application/json",
                body='{"error":"forced failure"}',
            ),
        )
        page.goto(f"{BASE_URL}/admin/community/", wait_until="domcontentloaded")
        expect(page.locator("#community-overview-status")).to_contain_text(
            "Unable to load community overview data",
            timeout=10_000,
        )
        expect(page.locator("#community-overview-status").get_by_role("button", name="Retry")).to_be_visible()
        expect(page.locator("#recent-announcements-table")).to_contain_text(
            "platform_update",
            timeout=10_000,
        )
    finally:
        _cleanup(admin_id, session_token, seeded)


def test_admin_community_index_visible_announcements_error_state(quality_page):
    page, _tracker = quality_page
    admin_id, session_token = _create_admin_session()
    seeded = _seed_community_overview(admin_id)
    page.context.add_cookies([{"name": "poool_session", "value": session_token, "url": BASE_URL}])

    try:
        page.route(
            "**/api/admin/community/announcements",
            lambda route: route.fulfill(
                status=500,
                content_type="application/json",
                body='{"error":"forced failure"}',
            ),
        )
        page.goto(f"{BASE_URL}/admin/community/", wait_until="domcontentloaded")
        expect(page.locator("#recent-announcements-table")).to_contain_text(
            "Unable to load recent announcements",
            timeout=10_000,
        )
        expect(page.locator("#recent-announcements-table").get_by_role("button", name="Retry")).to_be_visible()
    finally:
        _cleanup(admin_id, session_token, seeded)


def test_admin_community_index_mobile_smoke(mobile_page):
    page, tracker = mobile_page
    admin_id, session_token = _create_admin_session()
    seeded = _seed_community_overview(admin_id)
    page.context.add_cookies([{"name": "poool_session", "value": session_token, "url": BASE_URL}])

    try:
        tracker.navigate_and_check(f"{BASE_URL}/admin/community/")
        expect(page.locator(".admin-page-title")).to_have_text("Community Overview")
        expect(page.locator("#recent-announcements-table")).to_contain_text("platform_update", timeout=10_000)
        overflow = page.evaluate(
            """
            () => document.documentElement.scrollWidth > document.documentElement.clientWidth + 2
            """
        )
        assert not overflow
        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[404])
    finally:
        _cleanup(admin_id, session_token, seeded)
