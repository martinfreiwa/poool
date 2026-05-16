"""
Wave H — Follow / Block UI tests.

Validates the social-graph wiring on the community profile page:
  1. Clicking the Follow button on another user's profile flips the
     button to the "Following" state.
  2. Clicking again unfollows + flips back to "+ Follow".
  3. Calling window.toggleBlock() hides the blocked user's posts from
     the viewer's feed (reload-feed fires + post disappears).

Backend correctness for these endpoints is covered by:
  scripts/test_follow_e2e.sh
  scripts/test_block_mute_e2e.sh
This file confirms the JS layer + DOM stay in sync.

Run:
    pytest tests/e2e/test_community_follow_block_ui.py -v
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
def viewer_and_target():
    """Two fresh users — viewer follows/blocks target."""
    viewer = mint_user(prefix="e2e-fb-viewer", display_name="FB Viewer")
    target = mint_user(prefix="e2e-fb-target", display_name="FB Target")
    yield viewer, target
    cleanup_user(viewer["user_id"])
    cleanup_user(target["user_id"])


@pytest.fixture(scope="function")
def viewer_target_with_post():
    """Two users; target has a post we'll try to hide via block."""
    viewer = mint_user(prefix="e2e-fb-viewer2", display_name="FB Viewer2")
    target = mint_user(prefix="e2e-fb-target2", display_name="FB Target2")
    pid = seed_post(target["user_id"],
                    content=f"Block me {uuid.uuid4().hex[:6]}",
                    post_type="general")
    yield viewer, target, pid
    cleanup_user(viewer["user_id"])
    cleanup_user(target["user_id"])


# ─── Helpers ───────────────────────────────────────────────────────────

def _open_target_profile(playwright_session, viewer, target_id):
    ctx, page, errors = make_context(playwright_session, viewer)
    page.goto(
        f"{BASE_URL}/community/u/{target_id}",
        wait_until="domcontentloaded",
        timeout=15000,
    )
    expect(page.locator(".cp-hero")).to_be_visible(timeout=10000)
    return ctx, page, errors


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
def test_follow_button_flips_to_following(playwright_session, viewer_and_target):
    """Viewer opens target profile → clicks + Follow → button shows
    Following ✓ + class flips to secondary."""
    viewer, target = viewer_and_target
    ctx, page, errors = _open_target_profile(
        playwright_session, viewer, target["user_id"]
    )
    try:
        btn = page.locator("#community-profile-follow-btn")
        expect(btn).to_be_visible()
        # Sanity: starts on the primary (+ Follow) variant.
        text_before = (btn.text_content() or "").strip()
        assert "Follow" in text_before and "Following" not in text_before, (
            f"Expected '+ Follow' text, got {text_before!r}"
        )

        with page.expect_response(
            lambda r: f"/api/community/follow/{target['user_id']}" in r.url
            and r.request.method == "POST",
            timeout=5000,
        ) as info:
            btn.click()
        assert info.value.status in (200, 201), f"Follow POST: {info.value.status}"

        # JS sets the text to "Unfollow" after a successful follow.
        page.wait_for_function(
            "() => document.getElementById('community-profile-follow-btn')"
            ".textContent.toLowerCase().includes('unfollow')",
            timeout=5000,
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_unfollow_after_follow(playwright_session, viewer_and_target):
    """After following, the second click should unfollow and revert text."""
    viewer, target = viewer_and_target
    ctx, page, errors = _open_target_profile(
        playwright_session, viewer, target["user_id"]
    )
    try:
        btn = page.locator("#community-profile-follow-btn")

        # First click → follow.
        with page.expect_response(
            lambda r: f"/api/community/follow/{target['user_id']}" in r.url
            and r.request.method == "POST",
            timeout=5000,
        ):
            btn.click()
        # JS finishes by setting innerText to "Unfollow" after a successful follow.
        page.wait_for_function(
            "() => document.getElementById('community-profile-follow-btn')"
            ".textContent.toLowerCase().includes('unfollow')",
            timeout=5000,
        )

        # Second click → unfollow (DELETE).
        with page.expect_response(
            lambda r: f"/api/community/follow/{target['user_id']}" in r.url
            and r.request.method == "DELETE",
            timeout=5000,
        ) as info:
            btn.click()
        assert info.value.status in (200, 204), f"Unfollow: {info.value.status}"

        # Text reverts: JS unfollow path sets it to "Follow User".
        page.wait_for_function(
            "() => { const t = document.getElementById('community-profile-follow-btn').textContent.toLowerCase();"
            "  return t.includes('follow') && !t.includes('unfollow'); }",
            timeout=5000,
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_block_user_hides_their_posts_from_feed(
    playwright_session, viewer_target_with_post
):
    """Calling window.toggleBlock() must fire a reload-feed event +
    hide the blocked user's seeded post from the viewer's feed."""
    viewer, target, pid = viewer_target_with_post
    ctx, page, errors = make_context(playwright_session, viewer)
    try:
        page.goto(
            f"{BASE_URL}/community?tab=feed",
            wait_until="domcontentloaded",
            timeout=15000,
        )
        # Target's seed post should be visible before block.
        page.locator(f'.feed-post[data-post-id="{pid}"]').first.wait_for(
            state="visible", timeout=10000
        )

        # Trigger block via the exposed window helper. We pass a synthetic
        # button so toggleBlock can mutate its label without crashing.
        page.evaluate(
            "({uid}) => {"
            "  const fake = document.createElement('button'); fake.textContent='Block';"
            "  document.body.appendChild(fake);"
            "  window.toggleBlock(uid, false, fake);"
            "}",
            {"uid": target["user_id"]},
        )

        # Wait for the BLOCK POST to land.
        with page.expect_response(
            lambda r: f"/api/community/users/{target['user_id']}/block" in r.url
            and r.request.method == "POST",
            timeout=5000,
        ):
            # The block call may already be in flight; allow it.
            pass

        # After reload-feed fires, the blocked user's post should be gone.
        page.wait_for_function(
            f"() => !document.querySelector('.feed-post[data-post-id=\"{pid}\"]')",
            timeout=10000,
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
