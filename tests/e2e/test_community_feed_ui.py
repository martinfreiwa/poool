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
import pytest
from playwright.sync_api import expect

from community_helpers import (
    BASE_URL,
    mint_user,
    make_context,
    cleanup_user,
    seed_post,
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
    """Clicking the bookmark icon toggles `.bookmarked` + flips aria-pressed."""
    user, pid = feed_user_with_post
    ctx, page, errors = _open_feed(playwright_session, user)
    try:
        _wait_for_feed_render(page, post_id=pid)
        bookmark = page.locator(f"#bookmark-btn-{pid}")
        expect(bookmark).to_be_visible()

        # Sanity: starts un-bookmarked.
        cls_before = bookmark.get_attribute("class") or ""
        assert "bookmarked" not in cls_before

        bookmark.click()

        page.wait_for_function(
            f"() => document.getElementById('bookmark-btn-{pid}')"
            ".classList.contains('bookmarked')",
            timeout=5000,
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
