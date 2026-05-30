"""
Admin community moderation UI tests.

Covers the three admin community pages that were missing dedicated UI
tests as of 2026-05-16:

  /admin/community/posts                    — post moderation table
  /admin/community/circles                  — circle admin browser
  /admin/community/verified-owner-requests  — verified-owner approval queue

Each test mints a fresh admin (admin + super_admin roles), drives the
page, and where applicable verifies the DB side-effect of the action
(post.is_hidden flips). Backend correctness for the underlying admin
APIs is covered by the bash + Rust tests already in tree; this file is
about UI wiring + JS-no-error coverage.

Run:
    pytest tests/e2e/test_admin_community_moderation_ui.py -v
"""

import uuid

import psycopg2
import pytest
from playwright.sync_api import expect

from community_helpers import (
    BASE_URL,
    COMMUNITY_DB_URL,
    mint_admin,
    mint_user,
    make_context,
    cleanup_user,
    seed_circle,
    seed_post,
)


# ─── Fixtures ──────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def admin_and_target_post():
    """Admin user + an ordinary user whose post the admin will moderate."""
    admin = mint_admin(prefix="e2e-mod-admin", display_name="Mod Admin")
    target = mint_user(prefix="e2e-mod-target", display_name="Mod Target")
    pid = seed_post(target["user_id"],
                    content="Admin UI moderation target post",
                    post_type="general")
    yield admin, target, pid
    cleanup_user(admin["user_id"])
    cleanup_user(target["user_id"])


@pytest.fixture(scope="function")
def lone_admin():
    admin = mint_admin(prefix="e2e-lone-admin", display_name="Solo Admin")
    yield admin
    cleanup_user(admin["user_id"])


@pytest.fixture(scope="function")
def admin_circle_ops_alert():
    admin = mint_admin(prefix="e2e-alert-admin", display_name="Alert Admin")
    circle = seed_circle(
        admin["user_id"],
        name=f"Admin Ops Circle {uuid.uuid4().hex[:8]}",
    )
    alert = _seed_circle_ops_alert(
        circle["id"],
        summary=f"E2E admin ops alert {uuid.uuid4().hex[:10]}",
    )
    yield admin, circle, alert
    cleanup_user(admin["user_id"])


# ─── Helpers ───────────────────────────────────────────────────────────

def _post_hidden(post_id):
    """Read posts.is_hidden directly from the community DB."""
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute("SELECT is_hidden FROM posts WHERE id = %s", (str(post_id),))
        row = cur.fetchone()
        return bool(row and row[0])
    finally:
        conn.close()


def _seed_circle_ops_alert(
    circle_id,
    summary="E2E Circle ops alert",
    alert_type="report_backlog",
    severity="critical",
):
    """Insert a Circle ops alert for admin UI action coverage."""
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute(
            """
            INSERT INTO circle_ops_alerts (
                circle_id,
                alert_type,
                severity,
                status,
                summary,
                details
            )
            VALUES (%s, %s, %s, 'open', %s, '{}'::jsonb)
            RETURNING id
            """,
            (str(circle_id), alert_type, severity, summary),
        )
        alert_id = str(cur.fetchone()[0])
        conn.commit()
        return {
            "id": alert_id,
            "summary": summary,
            "alert_type": alert_type,
            "severity": severity,
        }
    finally:
        conn.close()


def _fetch_circle_ops_alert_state(alert_id):
    """Read durable admin action state for a Circle ops alert."""
    conn = psycopg2.connect(COMMUNITY_DB_URL)
    try:
        cur = conn.cursor()
        cur.execute(
            """
            SELECT
                status,
                assigned_to_user_id::text,
                escalation_level,
                snoozed_until,
                on_call_notified_at,
                workflow_state,
                workflow_note,
                workflow_updated_by::text,
                resolved_at,
                details->>'last_platform_action',
                details->>'last_platform_action_note'
            FROM circle_ops_alerts
            WHERE id = %s
            """,
            (str(alert_id),),
        )
        row = cur.fetchone()
        assert row, f"Circle ops alert {alert_id} should exist"
        return {
            "status": row[0],
            "assigned_to_user_id": row[1],
            "escalation_level": row[2],
            "snoozed_until": row[3],
            "on_call_notified_at": row[4],
            "workflow_state": row[5],
            "workflow_note": row[6],
            "workflow_updated_by": row[7],
            "resolved_at": row[8],
            "last_platform_action": row[9],
            "last_platform_action_note": row[10],
        }
    finally:
        conn.close()


def _open_admin(playwright_session, admin, path):
    ctx, page, errors = make_context(playwright_session, admin)
    page.goto(f"{BASE_URL}{path}", wait_until="domcontentloaded", timeout=15000)
    return ctx, page, errors


def _wait_for_alert_row(page, summary):
    page.wait_for_function(
        "() => {"
        "  const loading = document.getElementById('circle-alerts-loading');"
        "  return !loading || loading.hidden;"
        "}",
        timeout=10000,
    )
    row = page.locator("#circle-alerts-table-body tr").filter(has_text=summary).first
    expect(row).to_be_visible(timeout=10000)
    return row


def _run_alert_action(
    page,
    alert_id,
    summary,
    button_name,
    note,
    assignee=None,
    snooze_minutes=None,
    workflow_state=None,
):
    row = _wait_for_alert_row(page, summary)
    row.get_by_role("button", name=button_name).click()
    modal = page.locator("#circle-alert-action-modal")
    page.wait_for_function(
        "() => document"
        ".getElementById('circle-alert-action-modal')"
        "?.classList.contains('active')",
        timeout=5000,
    )
    page.locator("#circle-alert-action-note").fill(note)
    if assignee is not None:
        page.locator("#circle-alert-action-assignee").fill(str(assignee))
    if snooze_minutes is not None:
        page.locator("#circle-alert-action-snooze-minutes").fill(str(snooze_minutes))
    if workflow_state is not None:
        page.locator("#circle-alert-action-workflow-state").select_option(workflow_state)

    with page.expect_response(
        lambda r: f"/api/admin/community/ops-alerts/{alert_id}/action" in r.url
        and r.request.method == "POST",
        timeout=10000,
    ) as info:
        page.locator("#circle-alert-action-confirm").click()
    assert info.value.ok, f"Alert action POST failed: {info.value.status}"
    page.wait_for_function(
        "() => !document"
        ".getElementById('circle-alert-action-modal')"
        "?.classList.contains('active')",
        timeout=5000,
    )
    page.wait_for_function(
        "() => {"
        "  const loading = document.getElementById('circle-alerts-loading');"
        "  return !loading || loading.hidden;"
        "}",
        timeout=10000,
    )


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
@pytest.mark.admin
def test_admin_posts_page_loads_and_lists_seeded_post(
    playwright_session, admin_and_target_post
):
    """`/admin/community/posts` should render the seeded post in #posts-table."""
    admin, _target, pid = admin_and_target_post
    ctx, page, errors = _open_admin(playwright_session, admin, "/admin/community/posts")
    try:
        tbody = page.locator("#posts-table")
        expect(tbody).to_be_visible(timeout=10000)
        # Wait for the loadPosts() fetch to populate.
        page.wait_for_function(
            "() => { const tb = document.getElementById('posts-table');"
            "  return tb && !tb.textContent.includes('Loading posts'); }",
            timeout=10000,
        )
        expect(tbody).to_contain_text("moderation target post", timeout=5000)
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
@pytest.mark.admin
def test_admin_hide_post_flips_is_hidden_in_db(
    playwright_session, admin_and_target_post
):
    """Clicking Hide for a seeded post must POST /hide and flip is_hidden=TRUE."""
    admin, _target, pid = admin_and_target_post
    assert _post_hidden(pid) is False, "Pre-condition: seeded post not hidden"

    ctx, page, errors = _open_admin(playwright_session, admin, "/admin/community/posts")
    # The Hide handler uses window.confirm — auto-accept.
    page.on("dialog", lambda d: d.accept())

    try:
        page.wait_for_function(
            "() => { const tb = document.getElementById('posts-table');"
            "  return tb && !tb.textContent.includes('Loading posts'); }",
            timeout=10000,
        )
        # Find the row for our seeded post (matched by content snippet),
        # then click its Hide button.
        row = page.locator("#posts-table tr").filter(
            has_text="moderation target post"
        ).first
        expect(row).to_be_visible()
        with page.expect_response(
            lambda r: f"/api/admin/community/posts/{pid}/hide" in r.url
            and r.request.method == "POST",
            timeout=10000,
        ) as info:
            row.locator("button", has_text="Hide").first.click()
        assert info.value.status in (200, 204), f"Hide POST: {info.value.status}"

        # DB side-effect.
        # Allow the backend a moment to commit + the page to re-render.
        page.wait_for_timeout(400)
        assert _post_hidden(pid) is True, "Post should be hidden after admin action"

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
@pytest.mark.admin
def test_admin_verified_owner_requests_page_loads(playwright_session, lone_admin):
    """`/admin/community/verified-owner-requests` mounts + table container present."""
    ctx, page, errors = _open_admin(
        playwright_session, lone_admin, "/admin/community/verified-owner-requests"
    )
    try:
        # Filter dropdown + refresh button + table tbody.
        expect(page.locator("#vor-status-filter")).to_be_visible(timeout=10000)
        expect(page.locator("#vor-refresh-btn")).to_be_visible()
        expect(page.locator("#vor-table")).to_be_attached()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
@pytest.mark.admin
def test_admin_circles_page_loads_with_stats(playwright_session, lone_admin):
    """`/admin/community/circles` mounts the stats block + filter form."""
    ctx, page, errors = _open_admin(
        playwright_session, lone_admin, "/admin/community/circles"
    )
    try:
        # Stat block populated (loadCircles fetches counts on mount).
        for sel in ["#circles-total-count", "#circles-avg-members", "#circles-total-xp"]:
            expect(page.locator(sel)).to_be_visible(timeout=10000)

        # Filter form + search input.
        expect(page.locator("#circles-search")).to_be_visible()
        expect(page.locator("#circles-visibility")).to_be_visible()

        # Wait for the initial fetch — total-count should leave "0" if any
        # circle exists, but we don't assert on the value (DB may be empty).
        page.wait_for_timeout(800)

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
@pytest.mark.admin
def test_admin_circle_ops_alert_workflow_action_updates_db(
    playwright_session, admin_circle_ops_alert
):
    """Platform admins can move an active Circle ops alert through workflow UI."""
    admin, _circle, alert = admin_circle_ops_alert
    ctx, page, errors = _open_admin(
        playwright_session, admin, "/admin/community/circles"
    )
    try:
        _wait_for_alert_row(page, alert["summary"])
        _run_alert_action(
            page,
            alert["id"],
            alert["summary"],
            "Workflow",
            "Waiting on policy owner for E2E workflow triage.",
            workflow_state="waiting_on_policy",
        )

        state = _fetch_circle_ops_alert_state(alert["id"])
        assert state["status"] == "acknowledged"
        assert state["workflow_state"] == "waiting_on_policy"
        assert state["workflow_note"] == "Waiting on policy owner for E2E workflow triage."
        assert state["workflow_updated_by"] == str(admin["user_id"])
        assert state["last_platform_action"] == "set_workflow_state"

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
@pytest.mark.admin
def test_admin_circle_ops_alert_operational_actions_update_db(
    playwright_session, admin_circle_ops_alert
):
    """Admin Ops actions should persist assignment, escalation, snooze, on-call, and resolution."""
    admin, _circle, alert = admin_circle_ops_alert
    ctx, page, errors = _open_admin(
        playwright_session, admin, "/admin/community/circles"
    )
    try:
        _wait_for_alert_row(page, alert["summary"])

        _run_alert_action(
            page,
            alert["id"],
            alert["summary"],
            "Assign",
            "Assigning to platform admin during E2E triage.",
            assignee=admin["user_id"],
        )
        assigned = _fetch_circle_ops_alert_state(alert["id"])
        assert assigned["assigned_to_user_id"] == str(admin["user_id"])
        assert assigned["last_platform_action"] == "assign"

        _run_alert_action(
            page,
            alert["id"],
            alert["summary"],
            "Escalate",
            "Escalating report backlog for E2E triage.",
        )
        escalated = _fetch_circle_ops_alert_state(alert["id"])
        assert escalated["escalation_level"] == 1
        assert escalated["last_platform_action"] == "escalate"

        _run_alert_action(
            page,
            alert["id"],
            alert["summary"],
            "Snooze",
            "Snoozing during E2E maintenance window.",
            snooze_minutes=30,
        )
        snoozed = _fetch_circle_ops_alert_state(alert["id"])
        assert snoozed["snoozed_until"] is not None
        assert snoozed["last_platform_action"] == "snooze"

        _run_alert_action(
            page,
            alert["id"],
            alert["summary"],
            "On-call",
            "Manual on-call notification recorded from E2E.",
        )
        notified = _fetch_circle_ops_alert_state(alert["id"])
        assert notified["on_call_notified_at"] is not None
        assert notified["last_platform_action"] == "mark_on_call_notified"

        _run_alert_action(
            page,
            alert["id"],
            alert["summary"],
            "Resolve",
            "Resolved after E2E operational checks.",
        )
        resolved = _fetch_circle_ops_alert_state(alert["id"])
        assert resolved["status"] == "resolved"
        assert resolved["resolved_at"] is not None
        assert resolved["snoozed_until"] is None
        assert resolved["last_platform_action"] == "resolve"

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
