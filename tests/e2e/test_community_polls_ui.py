"""
Wave E — Polls UI tests.

Seeds a post + poll directly in the community DB, then validates the
feed-rendered poll:
  1. Poll options render with the labels we seeded.
  2. Clicking an option round-trips POST /poll/vote and re-renders the
     poll showing the user_voted state on that option.

Run:
    pytest tests/e2e/test_community_polls_ui.py -v
"""

import pytest
from playwright.sync_api import expect

from community_helpers import (
    BASE_URL,
    mint_user,
    make_context,
    cleanup_user,
    seed_poll,
)


# ─── Fixtures ──────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def voter_with_poll():
    """Mint a user; seed a poll on their own post (anyone can vote)."""
    user = mint_user(prefix="e2e-poll", display_name="Poll Voter")
    poll = seed_poll(
        user["user_id"],
        question="UI test — which sounds best?",
        options=["Alpha", "Bravo", "Charlie"],
    )
    yield user, poll
    cleanup_user(user["user_id"])


# ─── Helpers ───────────────────────────────────────────────────────────

def _open_feed_and_wait_for_poll(playwright_session, user, post_id):
    """Open feed, wait for the seeded post + its poll container to render."""
    ctx, page, errors = make_context(playwright_session, user)
    page.goto(f"{BASE_URL}/community?tab=feed", wait_until="domcontentloaded", timeout=15000)
    # Wait for the post card to render.
    page.locator(f'.feed-post[data-post-id="{post_id}"]').first.wait_for(
        state="visible", timeout=15000
    )
    # The poll container is `#poll-container-{post_id}` and is populated
    # async by loadPollForPost. Wait for at least one .poll-option child.
    page.locator(f"#poll-container-{post_id} .poll-option").first.wait_for(
        state="visible", timeout=10000
    )
    return ctx, page, errors


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
def test_poll_options_render_with_seeded_labels(playwright_session, voter_with_poll):
    """Seeded poll labels (Alpha/Bravo/Charlie) all show up in the post's poll."""
    user, poll = voter_with_poll
    ctx, page, errors = _open_feed_and_wait_for_poll(
        playwright_session, user, poll["post_id"]
    )
    try:
        container = page.locator(f'#poll-container-{poll["post_id"]}')
        for label in ["Alpha", "Bravo", "Charlie"]:
            expect(container).to_contain_text(label, timeout=5000)
        # 3 options means 3 .poll-option DOM nodes.
        assert container.locator(".poll-option").count() == 3

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_poll_vote_click_marks_option_voted(playwright_session, voter_with_poll):
    """Clicking option 'Bravo' fires POST /poll/vote and re-renders that
    option with `.voted` class + aria-pressed=true."""
    user, poll = voter_with_poll
    ctx, page, errors = _open_feed_and_wait_for_poll(
        playwright_session, user, poll["post_id"]
    )
    try:
        container = page.locator(f'#poll-container-{poll["post_id"]}')

        # Click the option containing "Bravo".
        bravo = container.locator(".poll-option").filter(has_text="Bravo").first
        expect(bravo).to_be_visible()

        # Wait for the POST round-trip atomically.
        with page.expect_response(
            lambda r: "/poll/vote" in r.url and r.request.method == "POST",
            timeout=10000,
        ) as res_info:
            bravo.click()
        assert res_info.value.status == 200, (
            f"Vote POST returned {res_info.value.status}"
        )

        # After the re-render, the Bravo option should have .voted + aria-pressed=true.
        post_id = poll["post_id"]
        js = (
            "() => { const opts = document.querySelectorAll("
            "'#poll-container-" + post_id + " .poll-option');"
            "  for (const o of opts) {"
            "    if (o.textContent.includes('Bravo') && o.classList.contains('voted')) return true;"
            "  } return false; }"
        )
        page.wait_for_function(js, timeout=5000)

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
