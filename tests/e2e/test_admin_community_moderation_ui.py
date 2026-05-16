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


def _open_admin(playwright_session, admin, path):
    ctx, page, errors = make_context(playwright_session, admin)
    page.goto(f"{BASE_URL}{path}", wait_until="domcontentloaded", timeout=15000)
    return ctx, page, errors


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
