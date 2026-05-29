"""
POOOL E2E — Developer: Ranking
==============================
Covers /developer/ranking — the developer's leaderboard view.

Tests:
  • Page loads cleanly (no JS errors, no 500s).
  • The leaderboard widget (#lb-content-layer or one of its state layers) renders.
  • The "You are currently ranked #N." copy displays the actual rank when the
    user has a rank, OR the "Start investing to get ranked." copy when unranked.
    (M-1 regression guard — the label MUST NOT say "top tier of institutional
    traders" anymore.)
  • Mobile viewport: page renders, no JS errors.
"""

import os
import pytest
from playwright.sync_api import expect

from tests.e2e.conftest import (
    BASE_URL,
    _create_context_and_page,
    _teardown_context,
    attach_session_cookie,
    cleanup_test_user,
    create_e2e_user,
)

BASE_URL = os.environ.get("BASE_URL", BASE_URL)
TIMEOUT = 15_000

BANNED_COPY = "top tier of institutional traders"


@pytest.fixture(scope="function")
def ranking_dev(playwright_session, request):
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    user = create_e2e_user(
        email_prefix="e2e-rank",
        display_name="E2E Ranking Dev",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker, user

    _teardown_context(context, page, tracker, request)
    cleanup_test_user(user["user_id"])


@pytest.fixture(scope="function")
def ranking_dev_mobile(playwright_session, request):
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    user = create_e2e_user(
        email_prefix="e2e-rank-mob",
        display_name="E2E Ranking Mobile Dev",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker, user

    _teardown_context(context, page, tracker, request)
    cleanup_test_user(user["user_id"])


def _go_ranking(page, tracker):
    tracker.navigate_and_check(f"{BASE_URL}/developer/ranking", timeout=TIMEOUT)
    page.wait_for_load_state("domcontentloaded")
    # Wait until either the content layer is unhidden or an explicit state layer
    # (loading/error/empty) is showing — i.e. the JS finished its first paint.
    page.wait_for_function(
        """() => {
            const content = document.getElementById('lb-content-layer');
            const empty   = document.getElementById('lb-empty-layer');
            const err     = document.getElementById('lb-error-layer');
            const visible = (el) => el && getComputedStyle(el).display !== 'none';
            return visible(content) || visible(empty) || visible(err);
        }""",
        timeout=TIMEOUT,
    )


# ═══════════════════════════════════════════════════════════════════════════
# TESTS
# ═══════════════════════════════════════════════════════════════════════════

@pytest.mark.developer
@pytest.mark.smoke
def test_ranking_page_loads_clean(ranking_dev):
    """/developer/ranking renders for a developer user."""
    page, tracker, _ = ranking_dev
    _go_ranking(page, tracker)
    tracker.assert_page_loaded()
    expect(page.locator("body")).to_be_visible()
    tracker.assert_no_critical_errors()


@pytest.mark.developer
def test_leaderboard_widget_renders(ranking_dev):
    """
    The static skeleton is replaced by an interactive layer:
    either the content card grid, the empty state, or the error state.
    """
    page, tracker, _ = ranking_dev
    _go_ranking(page, tracker)

    # At least one of the three states is visible.
    content = page.locator("#lb-content-layer")
    empty   = page.locator("#lb-empty-layer")
    error   = page.locator("#lb-error-layer")

    def is_visible(loc):
        if loc.count() == 0:
            return False
        return loc.evaluate("el => getComputedStyle(el).display !== 'none'")

    assert any(is_visible(loc) for loc in (content, empty, error)), (
        "Leaderboard never rendered a content/empty/error layer."
    )


@pytest.mark.developer
def test_rank_copy_is_compliant(ranking_dev):
    """
    M-1 regression guard. The 'Your Standing' label MUST display either:
      "You are currently ranked #N."  (when ranked)
    or the unranked prompt — but must NEVER contain the legacy
    "top tier of institutional traders" copy.
    """
    page, tracker, _ = ranking_dev
    _go_ranking(page, tracker)

    label = page.locator("#lb-my-metric-label")
    # The label may live inside the content layer that is hidden if empty state
    # took over. If unranked + empty layer is visible, the legacy phrase still
    # MUST NOT appear anywhere on the page.
    body_text = page.locator("body").inner_text().lower()
    assert BANNED_COPY.lower() not in body_text, (
        f"Page still contains banned copy {BANNED_COPY!r} — M-1 regression!"
    )

    if label.count() and label.evaluate("el => getComputedStyle(el).display !== 'none'"):
        text = label.inner_text().strip()
        # Must match the new pattern (either ranked or unranked)
        assert (
            text.startswith("You are currently ranked #")
            or "start investing" in text.lower()
            or text == "Start investing to get ranked."
        ), f"Unexpected label copy: {text!r}"


@pytest.mark.developer
@pytest.mark.mobile
def test_ranking_mobile_loads(ranking_dev_mobile):
    """Mobile viewport: ranking page renders cleanly."""
    page, tracker, _ = ranking_dev_mobile
    _go_ranking(page, tracker)
    tracker.assert_page_loaded()
    expect(page.locator("body")).to_be_visible()
    tracker.assert_no_critical_errors()
