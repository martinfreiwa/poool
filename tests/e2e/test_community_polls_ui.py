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
    comm_conn,
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


def _csrf_headers(context):
    token = next(
        (cookie["value"] for cookie in context.cookies() if cookie["name"] == "csrf_token"),
        None,
    )
    assert token, "Expected csrf_token cookie after opening community page"
    return {"X-CSRF-Token": token}


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


@pytest.mark.community
def test_poll_single_choice_vote_replaces_previous_vote(playwright_session, voter_with_poll):
    """A single-choice poll moves the user's vote instead of accumulating two votes."""
    user, poll = voter_with_poll
    ctx, page, errors = _open_feed_and_wait_for_poll(
        playwright_session, user, poll["post_id"]
    )
    try:
        post_id = poll["post_id"]
        container = page.locator(f"#poll-container-{post_id}")
        alpha = container.locator(".poll-option").filter(has_text="Alpha").first
        charlie = container.locator(".poll-option").filter(has_text="Charlie").first

        with page.expect_response(
            lambda r: "/poll/vote" in r.url and r.request.method == "POST",
            timeout=10000,
        ) as first_vote:
            alpha.click()
        assert first_vote.value.status == 200
        page.wait_for_function(
            f"""() => Array.from(document.querySelectorAll('#poll-container-{post_id} .poll-option'))
                .some((option) => option.textContent.includes('Alpha') && option.classList.contains('voted'))""",
            timeout=5000,
        )

        with page.expect_response(
            lambda r: "/poll/vote" in r.url and r.request.method == "POST",
            timeout=10000,
        ) as second_vote:
            charlie.click()
        assert second_vote.value.status == 200
        page.wait_for_function(
            f"""() => {{
                const options = Array.from(document.querySelectorAll('#poll-container-{post_id} .poll-option'));
                const alpha = options.find((option) => option.textContent.includes('Alpha'));
                const charlie = options.find((option) => option.textContent.includes('Charlie'));
                return alpha && charlie && !alpha.classList.contains('voted') && charlie.classList.contains('voted');
            }}""",
            timeout=5000,
        )

        conn = comm_conn()
        try:
            cur = conn.cursor()
            cur.execute(
                "SELECT COUNT(*), COUNT(DISTINCT option_id) FROM poll_votes WHERE poll_id = %s AND user_id = %s",
                (poll["poll_id"], user["user_id"]),
            )
            assert cur.fetchone() == (1, 1)
        finally:
            conn.close()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_poll_expired_state_blocks_ui_and_api_votes(playwright_session, voter_with_poll):
    """Expired polls render disabled options and the vote endpoint rejects attempts."""
    user, poll = voter_with_poll
    conn = comm_conn()
    try:
        cur = conn.cursor()
        cur.execute(
            "UPDATE polls SET expires_at = NOW() - INTERVAL '1 hour' WHERE id = %s",
            (poll["poll_id"],),
        )
        conn.commit()
    finally:
        conn.close()

    ctx, page, errors = _open_feed_and_wait_for_poll(
        playwright_session, user, poll["post_id"]
    )
    try:
        post_id = poll["post_id"]
        container = page.locator(f"#poll-container-{post_id}")
        expect(container).to_contain_text("Poll ended", timeout=5000)
        assert container.locator(".poll-option:disabled").count() == 3

        alpha = container.locator(".poll-option").filter(has_text="Alpha").first
        alpha.click(force=True)
        page.wait_for_timeout(300)
        assert not alpha.evaluate("(el) => el.classList.contains('voted')")

        rejected = page.request.post(
            f"{BASE_URL}/api/community/posts/{post_id}/poll/vote",
            headers={**_csrf_headers(ctx), "Content-Type": "application/json"},
            data={"option_id": poll["option_ids"][0]},
        )
        assert rejected.status == 400
        assert "expired" in rejected.text().lower()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_poll_rejects_option_from_another_poll(playwright_session, voter_with_poll):
    """The vote endpoint refuses an option id that belongs to a different poll."""
    user, poll = voter_with_poll
    other_poll = seed_poll(
        user["user_id"],
        question="UI test — unrelated poll?",
        options=["Outside option", "Another outside option"],
    )
    ctx, page, errors = _open_feed_and_wait_for_poll(
        playwright_session, user, poll["post_id"]
    )
    try:
        rejected = page.request.post(
            f"{BASE_URL}/api/community/posts/{poll['post_id']}/poll/vote",
            headers={**_csrf_headers(ctx), "Content-Type": "application/json"},
            data={"option_id": other_poll["option_ids"][0]},
        )
        assert rejected.status == 400
        assert "invalid poll option" in rejected.text().lower()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
