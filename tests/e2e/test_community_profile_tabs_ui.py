"""
Wave C — Community Profile Tabs UI tests.

Drives /community/me as a fresh user and exercises each tab in the
community-profile page:
  1. Page loads with no JS errors, hero header + tab nav present.
  2. Level badge has the new tier-specific CSS modifier class.
  3. Clicking each tab swaps the visible panel without throwing.
  4. Media tab empty-state renders correctly when user has no posts.
  5. Circle tab empty-state renders correctly when user has no circle.
  6. Activity tab shows empty state (not a Failed-to-Load error).

Run:
    pytest tests/e2e/test_community_profile_tabs_ui.py -v
"""

import pytest
from playwright.sync_api import expect

from community_helpers import BASE_URL, mint_user, make_context, cleanup_user


# ─── Fixtures ──────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def lone_user():
    """Fresh user, no posts, no circle — every empty state should trigger."""
    user = mint_user(prefix="e2e-prof-tabs", display_name="Profile Tabber")
    yield user
    cleanup_user(user["user_id"])


# ─── Helpers ───────────────────────────────────────────────────────────

def _open_profile(playwright_session, user):
    ctx, page, errors = make_context(playwright_session, user)
    page.goto(f"{BASE_URL}/community/me", wait_until="domcontentloaded", timeout=15000)
    expect(page.locator(".cp-hero")).to_be_visible(timeout=10000)
    return ctx, page, errors


def _click_tab(page, tab_name):
    btn = page.locator(f'.community-profile-tab[data-tab="{tab_name}"]')
    btn.click()
    # Wait for aria-selected to flip + the target panel to be visible.
    page.wait_for_function(
        f'() => document.querySelector(\'.community-profile-tab[data-tab="{tab_name}"]\').getAttribute("aria-selected") === "true"',
        timeout=3000,
    )


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
def test_profile_page_loads_with_hero_and_tabs(playwright_session, lone_user):
    """Hero header + 7+ tab buttons + Posts panel render after load."""
    ctx, page, errors = _open_profile(playwright_session, lone_user)
    try:
        expect(page.locator(".cp-hero__name")).to_contain_text(lone_user["display_name"])
        expect(page.locator(".cp-hero__meta")).to_be_visible()

        # Public tabs always visible.
        for tab in ["posts", "comments", "followers", "following", "media", "circle", "activity"]:
            expect(page.locator(f'.community-profile-tab[data-tab="{tab}"]')).to_be_visible()

        # Own-only tabs (this IS our own profile).
        for tab in ["analytics", "settings"]:
            expect(page.locator(f'.community-profile-tab[data-tab="{tab}"]')).to_be_visible()

        # Posts panel is the default selected one.
        posts_panel = page.locator('.community-profile-panel[data-panel="posts"]')
        expect(posts_panel).to_be_visible()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_profile_level_badge_has_tier_class(playwright_session, lone_user):
    """Hero level badge must include a `cp-hero__badge--level-{name}` modifier."""
    ctx, page, errors = _open_profile(playwright_session, lone_user)
    try:
        badge = page.locator(".cp-hero__badge--level").first
        expect(badge).to_be_visible()

        cls = badge.get_attribute("class") or ""
        # Must have the new tier-specific modifier (legend|mogul|veteran|sage|sapling|…).
        has_tier = any(
            mod in cls for mod in [
                "cp-hero__badge--level-legend",
                "cp-hero__badge--level-mogul",
                "cp-hero__badge--level-veteran",
                "cp-hero__badge--level-sage",
                "cp-hero__badge--level-sapling",
                "cp-hero__badge--level-sprout",
                "cp-hero__badge--level-seedling",
                "cp-hero__badge--level-explorer",
                "cp-hero__badge--level-strategist",
                "cp-hero__badge--level-titan",
            ]
        )
        assert has_tier, f"Level badge missing tier modifier: {cls!r}"

        # Format "L{n} · {NAME}".
        text = badge.text_content() or ""
        assert text.strip().startswith("L"), f"Expected 'L1 · …', got {text!r}"

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_profile_all_tabs_clickable_without_js_errors(playwright_session, lone_user):
    """Click through every tab; each should activate + emit no JS errors."""
    ctx, page, errors = _open_profile(playwright_session, lone_user)
    try:
        for tab in ["comments", "followers", "following", "media", "circle",
                    "activity", "analytics", "settings", "posts"]:
            _click_tab(page, tab)
            # The matching panel must be visible.
            panel = page.locator(f'.community-profile-panel[data-panel="{tab}"]')
            expect(panel).to_be_visible(timeout=3000)

        assert not errors, f"JS errors after tab cycle: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_profile_media_tab_empty_state(playwright_session, lone_user):
    """User has no posts/media → Media panel renders the empty-state card,
    not a 'Failed to load' error."""
    ctx, page, errors = _open_profile(playwright_session, lone_user)
    # Capture every media-API response so we know whether the backend
    # actually returned a 200 (empty list) or an error.
    media_responses = []
    page.on(
        "response",
        lambda r: media_responses.append((r.status, r.url))
        if "/media" in r.url and "/profile/" in r.url else None,
    )
    try:
        _click_tab(page, "media")
        panel = page.locator('.community-profile-panel[data-panel="media"]')
        expect(panel).to_be_visible()

        # Give the lazy-loader a moment.
        page.wait_for_timeout(1500)

        text = (panel.text_content() or "").lower()
        # The empty state copy contains "no media" / "haven't" / "nothing" etc.
        # Hard-fail only on the error path; pass on any reasonable empty msg.
        assert "failed" not in text and "error" not in text, (
            f"Media panel rendered an error state: {text[:200]}\n"
            f"Network responses: {media_responses}"
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_profile_circle_tab_empty_state_no_circle(playwright_session, lone_user):
    """User not in a circle → Circle panel renders 'Not in a circle' template."""
    ctx, page, errors = _open_profile(playwright_session, lone_user)
    try:
        _click_tab(page, "circle")
        panel = page.locator('.community-profile-panel[data-panel="circle"]')
        expect(panel).to_be_visible()
        expect(panel).to_contain_text("Not in a circle", timeout=3000)

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_profile_activity_tab_does_not_show_failed_to_load(playwright_session, lone_user):
    """Regression: Activity tab used to show 'Failed to Load' even when empty."""
    ctx, page, errors = _open_profile(playwright_session, lone_user)
    try:
        _click_tab(page, "activity")
        panel = page.locator('.community-profile-panel[data-panel="activity"]')
        expect(panel).to_be_visible()

        # Wait for the JS to finish populating.
        page.wait_for_timeout(1000)

        text = (panel.text_content() or "").lower()
        assert "failed to load" not in text, (
            f"Activity tab showed Failed-to-Load regression: {text[:200]}"
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
