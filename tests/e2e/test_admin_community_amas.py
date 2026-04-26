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
    email = f"e2e-ama-admin-{uuid.uuid4().hex[:10]}@poool.app"
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
        return user_id, session_token
    finally:
        cur.close()
        conn.close()


def _cleanup_admin_session(user_id, session_token):
    conn = _connect_core()
    cur = conn.cursor()
    try:
        cur.execute("DELETE FROM user_sessions WHERE session_token = %s", (session_token,))
        cur.execute("DELETE FROM user_roles WHERE user_id = %s", (user_id,))
        cur.execute("DELETE FROM users WHERE id = %s", (user_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _insert_question(ama_id, question):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO ama_questions (ama_id, user_id, question)
            VALUES (%s, gen_random_uuid(), %s)
            RETURNING id
            """,
            (ama_id, question),
        )
        question_id = cur.fetchone()[0]
        conn.commit()
        return question_id
    finally:
        cur.close()
        conn.close()


def _find_ama_by_title(title):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute("SELECT id FROM amas WHERE title = %s", (title,))
        row = cur.fetchone()
        return row[0] if row else None
    finally:
        cur.close()
        conn.close()


def _audit_actions_for(ama_id, question_id):
    conn = _connect_community()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT action
            FROM community_audit_logs
            WHERE entity_id IN (%s, %s)
               OR details->>'ama_id' = %s
            """,
            (ama_id, question_id, str(ama_id)),
        )
        return {row[0] for row in cur.fetchall()}
    finally:
        cur.close()
        conn.close()


def _cleanup_ama(ama_id, question_id=None):
    if not ama_id:
        return
    conn = _connect_community()
    cur = conn.cursor()
    try:
        if question_id:
            cur.execute(
                "DELETE FROM community_audit_logs WHERE entity_id IN (%s, %s) OR details->>'ama_id' = %s",
                (ama_id, question_id, str(ama_id)),
            )
        else:
            cur.execute("DELETE FROM community_audit_logs WHERE entity_id = %s", (ama_id,))
        cur.execute("DELETE FROM amas WHERE id = %s", (ama_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _expect_modal_within_viewport(page, selector):
    box = page.locator(selector).bounding_box()
    viewport = page.viewport_size
    assert box is not None
    assert viewport is not None
    assert box["x"] >= 0
    assert box["y"] >= 0
    assert box["x"] + box["width"] <= viewport["width"]
    assert box["y"] + box["height"] <= viewport["height"]


def test_admin_community_amas_create_moderate_and_audit(quality_page):
    user_id, session_token = _create_admin_session()
    page, tracker = quality_page
    page.context.add_cookies([{"name": "poool_session", "value": session_token, "url": BASE_URL}])
    unique = uuid.uuid4().hex[:10]
    title = f"E2E AMA {unique}"
    question = f"What risk controls should investors review for AMA {unique}?"
    answer = f"Review custody, crop exposure, insurance, and reporting cadence for {unique}."
    ama_id = None
    question_id = None

    try:
        tracker.navigate_and_check(f"{BASE_URL}/admin/community/amas")
        expect(page).to_have_title("Expert AMAs | Admin | POOOL")
        expect(page.locator("#open-ama-modal-btn")).to_be_visible()

        page.locator("#open-ama-modal-btn").click()
        expect(page.locator("#ama-modal")).to_be_visible()
        expect(page.locator("#ama-title")).to_be_focused()
        page.locator("#close-ama-modal-btn").focus()
        page.keyboard.press("Shift+Tab")
        expect(page.locator("#create-ama-btn")).to_be_focused()
        page.keyboard.press("Tab")
        expect(page.locator("#close-ama-modal-btn")).to_be_focused()
        page.keyboard.press("Escape")
        expect(page.locator("#ama-modal")).not_to_be_visible()
        expect(page.locator("#open-ama-modal-btn")).to_be_focused()

        page.locator("#open-ama-modal-btn").click()
        expect(page.locator("#ama-title")).to_be_focused()
        page.locator("#ama-title").fill(title)
        page.locator("#ama-description").fill("Created by targeted admin AMA E2E test.")
        page.locator("#ama-expert-name").fill("E2E Expert")
        page.locator("#ama-expert-title").fill("Test Moderator")

        with page.expect_response("**/api/admin/community/amas") as created_response:
            page.locator("#create-ama-btn").click()
        assert created_response.value.status == 200
        expect(page.get_by_text(title)).to_be_visible(timeout=10_000)

        ama_id = _find_ama_by_title(title)
        assert ama_id is not None

        question_id = _insert_question(ama_id, question)
        row = page.locator("tr", has_text=title)
        row.get_by_role("button", name="Questions").click()
        expect(page.locator("#ama-detail-panel")).to_be_visible()
        expect(page.get_by_text(question)).to_be_visible(timeout=10_000)

        answer_button = page.get_by_role("button", name="Answer")
        answer_button.click()
        expect(page.locator("#answer-modal")).to_be_visible()
        expect(page.locator("#answer-text")).to_be_focused()
        page.locator("#close-answer-modal-btn").focus()
        page.keyboard.press("Shift+Tab")
        expect(page.locator("#submit-answer-btn")).to_be_focused()
        page.keyboard.press("Tab")
        expect(page.locator("#close-answer-modal-btn")).to_be_focused()
        page.keyboard.press("Escape")
        expect(page.locator("#answer-modal")).not_to_be_visible()
        expect(answer_button).to_be_focused()

        answer_button.click()
        expect(page.locator("#answer-text")).to_be_focused()
        page.locator("#answer-text").fill(answer)
        with page.expect_response("**/answer") as answer_response:
            page.locator("#submit-answer-btn").click()
        assert answer_response.value.status == 200
        expect(page.locator("#ama-questions-table")).to_contain_text("Answered", timeout=10_000)

        with page.expect_response("**/feature") as feature_response:
            page.get_by_role("button", name="Feature").click()
        assert feature_response.value.status == 200
        expect(page.locator("#ama-questions-table")).to_contain_text("Featured", timeout=10_000)

        with page.expect_response("**/status") as status_response:
            row.locator("select[data-action='change-status']").select_option("live")
        assert status_response.value.status == 200
        expect(page.locator("tr", has_text=title)).to_contain_text("LIVE", timeout=10_000)

        actions = _audit_actions_for(ama_id, question_id)
        assert {"ama.create", "ama.status_update", "ama.answer_question", "ama.question_feature"} <= actions

        tracker.assert_no_critical_errors()
    finally:
        _cleanup_ama(ama_id, question_id)
        _cleanup_admin_session(user_id, session_token)


def test_admin_community_amas_mobile_modal_smoke(admin_mobile_page):
    page, tracker = admin_mobile_page

    tracker.navigate_and_check(f"{BASE_URL}/admin/community/amas")
    expect(page).to_have_title("Expert AMAs | Admin | POOOL")
    expect(page.locator("#open-ama-modal-btn")).to_be_visible()
    expect(page.locator("#amas-table")).to_be_visible()

    page.locator("#open-ama-modal-btn").click()
    expect(page.locator("#ama-modal")).to_be_visible()
    expect(page.locator("#ama-title")).to_be_focused()
    _expect_modal_within_viewport(page, "#ama-modal > div")
    page.keyboard.press("Escape")
    expect(page.locator("#ama-modal")).not_to_be_visible()
    expect(page.locator("#open-ama-modal-btn")).to_be_focused()

    tracker.assert_no_critical_errors()
