import pytest
from playwright.sync_api import expect

from tests.e2e.conftest import (
    BASE_URL,
    _create_context_and_page,
    _teardown_context,
    attach_session_cookie,
    cleanup_test_user,
    create_e2e_user,
    get_db_connection,
)


def cleanup_support_tickets(user_id):
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            DELETE FROM support_ticket_replies
            WHERE ticket_id IN (
                SELECT id FROM support_tickets WHERE user_id = %s
            )
            """,
            (str(user_id),),
        )
        cur.execute("DELETE FROM support_tickets WHERE user_id = %s", (str(user_id),))
        conn.commit()
    finally:
        cur.close()
        conn.close()


@pytest.fixture(scope="function")
def developer_support_page(playwright_session, request):
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    user = create_e2e_user(
        email_prefix="e2e-dev-support",
        display_name="E2E Developer Support",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker, user

    _teardown_context(context, page, tracker, request)
    cleanup_support_tickets(user["user_id"])
    cleanup_test_user(user["user_id"])


def get_ticket_id(user_id, subject):
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT id::text
            FROM support_tickets
            WHERE user_id = %s AND subject = %s
            ORDER BY created_at DESC
            LIMIT 1
            """,
            (str(user_id), subject),
        )
        row = cur.fetchone()
        return row[0] if row else None
    finally:
        cur.close()
        conn.close()


def mark_ticket_resolved(ticket_id):
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            "UPDATE support_tickets SET status = 'resolved' WHERE id = %s",
            (ticket_id,),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()


def set_hidden_select_value(page, selector, value):
    page.locator(selector).evaluate(
        """(el, value) => {
            el.value = value;
            el.dispatchEvent(new Event('change', { bubbles: true }));
        }""",
        value,
    )


def test_developer_support_create_reply_reopen(developer_support_page):
    page, tracker, user = developer_support_page
    subject = f"E2E Developer Support {user['unique_id']}"

    tracker.navigate_and_check(f"{BASE_URL}/developer/support")
    expect(page.locator("#support-form")).to_be_visible()
    expect(page.locator("#submit-ticket-btn")).to_be_visible()

    set_hidden_select_value(page, "#ticket-category", "technical")
    set_hidden_select_value(page, "#ticket-priority", "high")
    page.fill("#ticket-subject", subject)
    page.fill(
        "#ticket-message",
        "This is an authenticated developer support E2E ticket with enough detail.",
    )

    with page.expect_response("**/api/support/tickets") as submit_response:
        page.click("#submit-ticket-btn")
    assert submit_response.value.status == 200
    expect(page.locator("#tickets-list")).to_contain_text(subject, timeout=10000)

    ticket_id = get_ticket_id(user["user_id"], subject)
    assert ticket_id

    ticket_card = page.locator(f'.support-ticket-card[data-ticket-id="{ticket_id}"]')
    toggle = ticket_card.locator("[data-ticket-toggle]")
    expect(toggle).to_have_attribute("aria-expanded", "false")
    toggle.press("Enter")
    expect(toggle).to_have_attribute("aria-expanded", "true")

    ticket_card.locator(".ticket-reply-input").fill("This is a developer support reply.")
    with page.expect_response(f"**/api/support/tickets/{ticket_id}/reply") as reply_response:
        ticket_card.locator(".ticket-reply-btn").click()
    assert reply_response.value.status == 200

    mark_ticket_resolved(ticket_id)
    page.reload(wait_until="domcontentloaded")
    expect(page.locator(f'.support-ticket-card[data-ticket-id="{ticket_id}"]')).to_be_visible(timeout=10000)
    page.locator(f'.support-ticket-card[data-ticket-id="{ticket_id}"] [data-ticket-toggle]').click()
    page.evaluate("window.pooolConfirm = async () => true")

    with page.expect_response(f"**/api/support/tickets/{ticket_id}/reopen") as reopen_response:
        page.locator(f'.support-ticket-card[data-ticket-id="{ticket_id}"] .ticket-reopen-btn').click()
    assert reopen_response.value.status == 200

    tracker.assert_page_loaded()
    tracker.assert_no_critical_errors()


def test_developer_support_ticket_list_error_state(developer_support_page):
    page, tracker, _user = developer_support_page
    page.route(
        "**/api/support/tickets",
        lambda route: route.fulfill(
            status=500,
            content_type="application/json",
            body='{"error":"Forced support list failure"}',
        ),
    )

    tracker.navigate_and_check(f"{BASE_URL}/developer/support")
    expect(page.locator("#tickets-list [role='alert']")).to_contain_text(
        "Forced support list failure"
    )
    expect(page.locator("#support-retry-btn")).to_be_visible()
    tracker.assert_page_loaded()


def test_developer_support_rate_limit_response(developer_support_page):
    page, tracker, user = developer_support_page
    tracker.navigate_and_check(f"{BASE_URL}/developer/support")

    statuses = []
    # Support ticket creation uses the shared auth limiter in production, but
    # local/test backends may run with the limiter disabled. The workflow guard
    # verifies burst requests never degrade into server errors and accepts 429
    # when the limiter is active.
    for idx in range(12):
        response = page.evaluate(
            """
            async ({ subject, idx }) => {
                const formData = new FormData();
                formData.append("subject", `${subject} ${idx}`);
                formData.append("message", "This support ticket is long enough for validation.");
                formData.append("priority", "normal");
                formData.append("category", "general");
                formData.append("context", "{}");
                const csrf = document.cookie
                    .split("; ")
                    .find((part) => part.startsWith("csrf_token="))
                    ?.split("=")[1] || "";
                const response = await fetch("/api/support/tickets", {
                    method: "POST",
                    headers: {"X-CSRF-Token": decodeURIComponent(csrf)},
                    body: formData
                });
                return response.status;
            }
            """,
            {"subject": f"E2E Rate Limited {user['unique_id']}", "idx": idx},
        )
        statuses.append(response)

    assert all(status in (200, 429) for status in statuses), statuses
    if 429 in statuses:
        first_limited = statuses.index(429)
        assert all(status == 429 for status in statuses[first_limited:]), statuses
    tracker.assert_page_loaded()
