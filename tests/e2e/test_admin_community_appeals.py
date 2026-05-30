import os

import psycopg2
from playwright.sync_api import expect

from tests.e2e.conftest import BASE_URL, cleanup_test_user, create_e2e_user


COMMUNITY_DB_DSN = os.environ.get(
    "COMMUNITY_DB_DSN",
    os.environ.get("COMMUNITY_DATABASE_URL", "dbname=poool_community user=martin host=localhost"),
)


def _connect_community():
    return psycopg2.connect(COMMUNITY_DB_DSN)


def _seed_pending_appeal(user_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO community_profiles (user_id, is_community_banned, ban_reason)
            VALUES (%s, TRUE, 'Workflow appeal seed')
            ON CONFLICT (user_id) DO UPDATE SET
                is_community_banned = TRUE,
                ban_reason = EXCLUDED.ban_reason
            """,
            (user_id,),
        )
        cur.execute(
            """
            INSERT INTO ban_appeals (user_id, appeal_text)
            VALUES (%s, 'Workflow appeal review text for admin browser validation.')
            RETURNING id
            """,
            (user_id,),
        )
        appeal_id = cur.fetchone()[0]
        conn.commit()
        return appeal_id
    finally:
        cur.close()
        conn.close()


def _fetch_appeal_state(appeal_id, user_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT a.status, a.admin_notes, p.is_community_banned
            FROM ban_appeals a
            JOIN community_profiles p ON p.user_id = a.user_id
            WHERE a.id = %s AND a.user_id = %s
            """,
            (appeal_id, user_id),
        )
        return cur.fetchone()
    finally:
        cur.close()
        conn.close()


def _count_audit(action, appeal_id, actor_id, target_user_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT COUNT(*)
            FROM community_audit_logs
            WHERE action = %s
              AND entity_type = 'ban_appeal'
              AND entity_id = %s
              AND actor_user_id = %s
              AND target_user_id = %s
            """,
            (action, appeal_id, actor_id, target_user_id),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def _fetch_audit_rows(appeal_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT action, entity_type, entity_id, actor_user_id, target_user_id
            FROM community_audit_logs
            WHERE entity_id = %s
            ORDER BY created_at
            """,
            (appeal_id,),
        )
        return cur.fetchall()
    finally:
        cur.close()
        conn.close()


def _count_approval_notification(user_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT COUNT(*)
            FROM notifications
            WHERE user_id = %s
              AND content ILIKE %s
            """,
            (user_id, "%ban appeal approved%"),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def _cleanup_appeal_fixture(appeal_id, user_id, admin_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute("DELETE FROM notifications WHERE user_id = %s", (user_id,))
        cur.execute(
            "DELETE FROM community_audit_logs WHERE entity_id = %s OR target_user_id = %s OR actor_user_id = %s",
            (appeal_id, user_id, admin_id),
        )
        cur.execute("DELETE FROM ban_appeals WHERE id = %s", (appeal_id,))
        cur.execute("DELETE FROM community_profiles WHERE user_id = %s", (user_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()


def test_admin_community_appeal_approve_browser_audit_and_recovery(quality_page):
    page, tracker = quality_page
    admin = create_e2e_user(email_prefix="e2e-appeals-admin", roles=("admin", "super_admin"))
    target = create_e2e_user(
        email_prefix="e2e-appeals-target",
        display_name="Workflow Appeal Target",
    )
    appeal_id = _seed_pending_appeal(target["user_id"])

    try:
        page.context.add_cookies(
            [{"name": "poool_session", "value": admin["session_token"], "url": BASE_URL}]
        )
        tracker.navigate_and_check(f"{BASE_URL}/admin/community/appeals")

        row = page.locator("#appeals-table tr", has_text="Workflow appeal review text").first
        expect(row).to_be_visible(timeout=10_000)
        row.get_by_role("button", name="Review").click()

        modal = page.get_by_role("dialog", name="Review Appeal")
        expect(modal).to_be_visible()
        expect(page.locator("#appeal-modal-text")).to_contain_text("Workflow appeal review text")
        page.locator("#appeal-modal-notes").fill("Appeal approved by workflow test")

        with page.expect_response(
            lambda response: response.url.endswith(f"/api/admin/community/appeals/{appeal_id}/review")
            and response.request.method == "POST"
        ) as review_response:
            page.locator("#appeal-approve-btn").click()

        assert review_response.value.status == 200
        expect(modal).to_be_hidden(timeout=5_000)

        assert _fetch_appeal_state(appeal_id, target["user_id"]) == (
            "approved",
            "Appeal approved by workflow test",
            False,
        )
        assert _count_audit("appeal.approve", appeal_id, admin["user_id"], target["user_id"]) == 1, (
            f"Expected appeal.approve audit row, got {_fetch_audit_rows(appeal_id)!r}"
        )
        assert _count_approval_notification(target["user_id"]) == 1

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures()
    finally:
        _cleanup_appeal_fixture(appeal_id, target["user_id"], admin["user_id"])
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(target["user_id"])
