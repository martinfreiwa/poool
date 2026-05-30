"""
Wave D — Community Feed UI tests.

Drives /community?tab=feed (default tab) and exercises:
  1. Feed loads + composer textarea + Post button visible.
  2. Creating a post via the composer shows it at the top of the feed.
  3. Seeded post is rendered as a `.feed-post` card.
  4. Clicking the fire-reaction toggles `.feed-reaction-btn.active`.
  5. Clicking the bookmark toggle adds `.bookmarked` class.

The bash e2e (scripts/test_feed_e2e.sh) covers backend correctness;
this validates the JS wiring stays alive.

Run:
    pytest tests/e2e/test_community_feed_ui.py -v
"""

import uuid
from urllib.parse import parse_qs, urlparse

import pytest
from playwright.sync_api import expect

from community_helpers import (
    BASE_URL,
    mint_user,
    make_context,
    cleanup_user,
    seed_post,
    comm_conn,
)


# ─── Fixtures ──────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def feed_user_with_post():
    """User + one seeded post they own (so reactions/bookmarks have a target)."""
    user = mint_user(prefix="e2e-feed", display_name="Feed Tester")
    pid = seed_post(
        user["user_id"],
        content=f"UI feed test post {uuid.uuid4().hex[:6]}",
        post_type="general",
    )
    yield user, pid
    cleanup_user(user["user_id"])


@pytest.fixture(scope="function")
def fresh_feed_user():
    """User with no posts — used for composer-create test so we can assert
    the new post appears at the top of an otherwise-empty feed."""
    user = mint_user(prefix="e2e-feed-fresh", display_name="Fresh Poster")
    yield user
    cleanup_user(user["user_id"])


# ─── Helpers ───────────────────────────────────────────────────────────

def _open_feed(playwright_session, user):
    ctx, page, errors = make_context(playwright_session, user)
    page.goto(f"{BASE_URL}/community?tab=feed", wait_until="domcontentloaded", timeout=15000)
    # Feed container must mount (HTMX swap completes).
    expect(page.locator("#community-feed-container")).to_be_visible(timeout=10000)
    return ctx, page, errors


def _wait_for_feed_render(page, post_id=None, timeout=10000):
    """Wait until at least one .feed-post is in the DOM (or specific post_id)."""
    sel = f'.feed-post[data-post-id="{post_id}"]' if post_id else ".feed-post"
    page.locator(sel).first.wait_for(state="visible", timeout=timeout)


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
def test_feed_loads_composer_and_filter_buttons(playwright_session, fresh_feed_user):
    """Feed page renders the composer + segmented controls."""
    ctx, page, errors = _open_feed(playwright_session, fresh_feed_user)
    try:
        expect(page.locator("#post-content-input")).to_be_visible()
        expect(page.locator("#submit-post-btn")).to_be_visible()
        expect(page.locator("#feed-btn-all")).to_be_visible()
        expect(page.locator("#sort-btn-fresh")).to_be_visible()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_feed_compose_creates_post(playwright_session, fresh_feed_user):
    """Typing into composer + clicking Post inserts a new .feed-post card."""
    ctx, page, errors = _open_feed(playwright_session, fresh_feed_user)
    # Capture the create response so we can debug if it 4xx-es.
    post_results = []
    page.on(
        "response",
        lambda r: post_results.append((r.status, r.url, r.request.method))
        if r.url.endswith("/api/community/posts") and r.request.method == "POST" else None,
    )
    try:
        content = f"UI compose test {uuid.uuid4().hex[:8]}"
        page.fill("#post-content-input", content)
        # Wait on the POST round-trip via page.expect_response so the click
        # + network result are atomic — avoids a race with feed refresh.
        with page.expect_response(
            lambda r: r.url.endswith("/api/community/posts") and r.request.method == "POST",
            timeout=10000,
        ) as res_info:
            page.click("#submit-post-btn")
        res = res_info.value
        assert res.status == 200 or res.status == 201, (
            f"Create-post POST failed: {res.status}. All POSTs: {post_results}"
        )

        # The new post should appear after the reload-feed event re-renders.
        # Don't rely on `.first` — global feed may have older sticky/pinned
        # posts above ours. Just assert the unique content shows somewhere.
        page.locator(".feed-post").first.wait_for(state="visible", timeout=10000)
        # `get_by_text` matches the unique suffix uuid bit we baked in.
        expect(page.get_by_text(content[:30], exact=False).first).to_be_visible(
            timeout=10000
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_feed_seeded_post_renders(playwright_session, feed_user_with_post):
    """A post seeded directly in DB shows up in the feed."""
    user, pid = feed_user_with_post
    ctx, page, errors = _open_feed(playwright_session, user)
    try:
        _wait_for_feed_render(page, post_id=pid)
        expect(page.locator(f'.feed-post[data-post-id="{pid}"]')).to_be_visible()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_feed_pagination_does_not_duplicate_filter_query_params(
    playwright_session,
    fresh_feed_user,
):
    """The infinite-scroll sentinel must not submit inherited filter params twice."""
    unique = uuid.uuid4().hex[:8]
    for idx in range(25):
        seed_post(
            fresh_feed_user["user_id"],
            content=f"pagination fixture {unique} #{idx:02d}",
            post_type="general",
        )

    ctx, page, errors = make_context(playwright_session, fresh_feed_user)
    try:
        page.goto(f"{BASE_URL}/community?tab=feed", wait_until="domcontentloaded", timeout=15000)
        expect(page.locator("#community-feed-container")).to_be_visible(timeout=10000)
        _wait_for_feed_render(page, timeout=10000)

        sentinel = page.locator(".community-feed-sentinel").first
        expect(sentinel).to_be_attached(timeout=10000)
        with page.expect_response(
            lambda r: "/community/partials/feed/list" in r.url and "page=2" in r.url,
            timeout=10000,
        ) as response_info:
            sentinel.scroll_into_view_if_needed(timeout=5000)

        response = response_info.value
        assert response.status == 200
        params = parse_qs(urlparse(response.url).query)
        assert params.get("feed_mode") == ["all"]
        assert params.get("sort_by") == ["fresh"]
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_feed_fire_reaction_toggles_active_class(playwright_session, feed_user_with_post):
    """Clicking the default reaction button flips `.active` class."""
    user, pid = feed_user_with_post
    ctx, page, errors = _open_feed(playwright_session, user)
    try:
        _wait_for_feed_render(page, post_id=pid)
        card = page.locator(f'.feed-post[data-post-id="{pid}"]')
        # First reaction button on the card is the toggle (fire by default).
        react_btn = card.locator(".feed-reaction-btn").first

        # Sanity: starts not active.
        cls_before = react_btn.get_attribute("class") or ""
        assert "active" not in cls_before, f"Reaction btn already active: {cls_before}"

        react_btn.click()

        # Wait for the click to round-trip (POST /reactions) + class flip.
        page.wait_for_function(
            f"() => document.querySelector('.feed-post[data-post-id=\"{pid}\"] .feed-reaction-btn')"
            ".classList.contains('active')",
            timeout=5000,
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_feed_bookmark_btn_toggles_state(playwright_session, feed_user_with_post):
    """Bookmark state persists into Saved and can be removed from Saved."""
    user, pid = feed_user_with_post
    ctx, page, errors = _open_feed(playwright_session, user)
    try:
        _wait_for_feed_render(page, post_id=pid)
        bookmark = page.locator(f"#bookmark-btn-{pid}")
        expect(bookmark).to_be_visible()

        # Sanity: starts un-bookmarked.
        cls_before = bookmark.get_attribute("class") or ""
        assert "bookmarked" not in cls_before

        with page.expect_response(
            lambda r: r.url.endswith(f"/api/community/posts/{pid}/bookmark")
            and r.request.method == "POST",
            timeout=10000,
        ) as bookmark_response:
            bookmark.click()
        assert bookmark_response.value.status == 200

        page.wait_for_function(
            f"() => document.getElementById('bookmark-btn-{pid}')"
            ".classList.contains('bookmarked')",
            timeout=5000,
        )
        expect(bookmark).to_have_attribute("aria-pressed", "true")

        status = page.request.get(f"{BASE_URL}/api/community/posts/{pid}/bookmark/status")
        assert status.status == 200
        assert status.json()["bookmarked"] is True

        page.goto(f"{BASE_URL}/community?tab=saved", wait_until="domcontentloaded", timeout=15000)
        saved_post = page.locator(f"#community-saved-feed-container #post-{pid}")
        expect(saved_post).to_be_visible(timeout=10000)

        saved_bookmark = page.locator(f"#community-saved-feed-container #bookmark-btn-{pid}")
        with page.expect_response(
            lambda r: r.url.endswith(f"/api/community/posts/{pid}/bookmark")
            and r.request.method == "POST",
            timeout=10000,
        ) as unbookmark_response:
            saved_bookmark.click()
        assert unbookmark_response.value.status == 200

        page.goto(f"{BASE_URL}/community?tab=saved", wait_until="domcontentloaded", timeout=15000)
        page.wait_for_function(
            f"""() => {{
                const container = document.getElementById('community-saved-feed-container');
                return container
                    && !container.textContent.includes('Loading saved posts')
                    && !document.getElementById('post-{pid}');
            }}""",
            timeout=10000,
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_feed_owner_edit_validation_and_delete_modal(playwright_session, feed_user_with_post):
    """Owners can edit via modal, blank edits are blocked, and delete is modal-based."""
    user, pid = feed_user_with_post
    ctx, page, errors = _open_feed(playwright_session, user)
    try:
        _wait_for_feed_render(page, post_id=pid)
        card = page.locator(f'.feed-post[data-post-id="{pid}"]')
        menu_toggle = card.locator(".feed-post-owner-menu__toggle")
        expect(menu_toggle).to_be_visible(timeout=5000)

        menu_toggle.click()
        edit_item = card.locator(".feed-post-owner-menu__item").filter(has_text="Edit")
        expect(edit_item).to_be_visible(timeout=5000)
        edit_item.click()
        expect(page.locator("#edit-post-modal")).to_be_visible(timeout=5000)

        page.fill("#edit-post-content", "   ")
        page.click("#submit-edit-post-btn")
        expect(page.locator("#edit-post-error")).to_contain_text(
            "Post content cannot be empty",
            timeout=5000,
        )

        edited_content = f"Edited owner post {uuid.uuid4().hex[:8]}"
        page.fill("#edit-post-content", edited_content)
        with page.expect_response(
            lambda r: r.url.endswith(f"/api/community/posts/{pid}")
            and r.request.method == "PUT",
            timeout=10000,
        ) as edit_response:
            page.click("#submit-edit-post-btn")
        assert edit_response.value.status == 200
        expect(page.get_by_text(edited_content, exact=False).first).to_be_visible(
            timeout=10000
        )

        conn = comm_conn()
        try:
            cur = conn.cursor()
            cur.execute("SELECT content FROM posts WHERE id = %s", (pid,))
            row = cur.fetchone()
            assert row is not None
            assert row[0] == edited_content
        finally:
            conn.close()

        card = page.locator(f'.feed-post[data-post-id="{pid}"]')
        card.locator(".feed-post-owner-menu__toggle").click()
        delete_item = card.locator(".feed-post-owner-menu__item").filter(has_text="Delete")
        expect(delete_item).to_be_visible(timeout=5000)
        delete_item.click()
        expect(page.locator("#delete-post-modal")).to_be_visible(timeout=5000)
        expect(page.locator("#delete-post-id")).to_have_value(pid)

        with page.expect_response(
            lambda r: r.url.endswith(f"/api/community/posts/{pid}")
            and r.request.method == "DELETE",
            timeout=10000,
        ) as delete_response:
            page.click("#delete-post-confirm-btn")
        assert delete_response.value.status == 200
        expect(page.locator(f'.feed-post[data-post-id="{pid}"]')).to_have_count(
            0,
            timeout=10000,
        )

        conn = comm_conn()
        try:
            cur = conn.cursor()
            cur.execute("SELECT COUNT(*) FROM posts WHERE id = %s", (pid,))
            assert cur.fetchone()[0] == 0
        finally:
            conn.close()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
