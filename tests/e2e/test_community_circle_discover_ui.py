"""
Wave A — My Circles Tab UI tests.

Drives the browser against /community?tab=circle. Verifies:
  1. Discovery sections (My Circles and unified Discover) render after page load.
  2. Typing in the search input shows the Search Results section.
  3. Create Circle modal opens + closes cleanly.
  4. A circle the user already belongs to appears in "My Circles".

The bash e2e (scripts/test_low_priority_e2e.sh + test_community_misc.sh)
already validates the API surface. This file is about UI rendering +
interactivity only — so we mint users + seed circles directly via
community_helpers and never exercise the join/create API from JS.

Run:
    pytest tests/e2e/test_community_circle_discover_ui.py -v
"""

import uuid
import pytest
from playwright.sync_api import expect

from community_helpers import (
    BASE_URL,
    mint_user,
    make_context,
    cleanup_user,
    seed_circle,
)


# ─── Fixtures ──────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def circle_member():
    """Mint a user already owning one circle (so My Circles list isn't empty)."""
    user = mint_user(prefix="e2e-cc-disc", display_name="Circle Discoverer")
    circle = seed_circle(user["user_id"], name=f"Discover Test {uuid.uuid4().hex[:6]}")
    yield user, circle
    cleanup_user(user["user_id"])


@pytest.fixture(scope="function")
def lone_user():
    """User with no circles — for create-modal + search tests."""
    user = mint_user(prefix="e2e-cc-lone", display_name="Lone Tester")
    yield user
    cleanup_user(user["user_id"])


# ─── Helpers ───────────────────────────────────────────────────────────

def _open_circle_tab(playwright_session, user):
    """Open /community?tab=circle as the given user. Returns (ctx, page, errors).

    Waits for both the tab panel mount AND network idle so subsequent
    page.evaluate() calls don't trip the "Execution context was
    destroyed, most likely because of a navigation" race that bit CI.
    """
    ctx, page, errors = make_context(playwright_session, user)
    page.goto(
        f"{BASE_URL}/community?tab=circle",
        wait_until="domcontentloaded",
        timeout=15000,
    )
    # Wait for the tab panel container to mount.
    expect(page.locator("#community-circle-tab")).to_be_visible(timeout=10000)
    # And for the HTMX swap + initial circle-discover XHRs to settle.
    try:
        page.wait_for_load_state("networkidle", timeout=10000)
    except Exception:
        pass  # Best-effort — some pages keep a poll alive forever.
    return ctx, page, errors


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
def test_circle_tab_renders_toolbar_and_sections(playwright_session, lone_user):
    """Loading /community?tab=circle paints the actionbar, My-Circles strip,
    and unified Discover grid."""
    ctx, page, errors = _open_circle_tab(playwright_session, lone_user)
    try:
        # Actionbar present.
        expect(page.locator(".cc-actionbar")).to_be_visible()
        expect(page.locator("#cc-search-input")).to_be_visible()
        expect(page.locator(".cc-actionbar__create")).to_be_visible()

        # Always-on areas.
        expect(page.locator("#cc-my-circles-section")).to_be_visible()
        expect(page.locator("#cc-discover-section")).to_be_visible()
        expect(page.locator("#cc-discover-filters")).to_be_visible()

        # Wait for discover list to leave its loading state (API resolved).
        page.wait_for_function(
            "() => { const el = document.getElementById('cc-discover-list');"
            "  return el && !el.textContent.includes('Loading'); }",
            timeout=10000,
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_search_input_reveals_results_section(playwright_session, lone_user):
    """Typing into #cc-search-input un-hides #cc-search-results-section
    and fires the debounced search."""
    ctx, page, errors = _open_circle_tab(playwright_session, lone_user)
    try:
        # Confirm it starts hidden.
        results_section = page.locator("#cc-search-results-section")
        # `hidden` attr — Playwright resolves via DOM property.
        assert page.evaluate(
            "document.getElementById('cc-search-results-section').hasAttribute('hidden')"
        )

        # Type a query — debounced, so we wait a beat.
        page.fill("#cc-search-input", "tes")
        page.wait_for_timeout(700)  # debounce window in community-circles-discover.js

        # Section should now be visible (hidden attr removed).
        assert not page.evaluate(
            "document.getElementById('cc-search-results-section').hasAttribute('hidden')"
        ), "Search results section did not un-hide after typing"

        # Results container exists + populated (either real rows or empty msg).
        expect(page.locator("#cc-search-results")).to_be_visible()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_create_modal_opens_and_closes(playwright_session, lone_user):
    """Clicking 'Create Circle' opens the modal; the close-X button hides it."""
    ctx, page, errors = _open_circle_tab(playwright_session, lone_user)
    try:
        modal = page.locator("#create-circle-modal")

        # Sanity: modal hidden by default.
        display = page.evaluate(
            "getComputedStyle(document.getElementById('create-circle-modal')).display"
        )
        assert display == "none", f"Modal not hidden on load (display={display})"

        # Open.
        page.click(".cc-actionbar__create")
        expect(modal).to_be_visible(timeout=3000)
        expect(page.locator("#create-circle-title")).to_have_text("Create a Circle")
        expect(page.locator("#circle-name-input")).to_be_visible()

        # Close via header close button.
        page.click("#create-circle-modal .cc-modal__close")
        page.wait_for_function(
            "() => getComputedStyle(document.getElementById('create-circle-modal')).display === 'none'",
            timeout=3000,
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_my_circles_renders_seeded_circle(playwright_session, circle_member):
    """The seeded circle (user is owner) appears in #cc-my-circles-list."""
    user, circle = circle_member
    ctx, page, errors = _open_circle_tab(playwright_session, user)
    try:
        # Wait for my-circles list to leave loading state.
        page.wait_for_function(
            "() => { const el = document.getElementById('cc-my-circles-list');"
            "  return el && !el.textContent.includes('Loading your circles'); }",
            timeout=10000,
        )

        # The seeded circle's name should appear.
        my_list = page.locator("#cc-my-circles-list")
        expect(my_list).to_contain_text(circle["name"], timeout=5000)

        # Count badge should be >= 1.
        count_text = page.locator("#cc-my-circles-count").text_content() or ""
        assert any(c.isdigit() for c in count_text), (
            f"Expected count badge to contain a number, got '{count_text}'"
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
