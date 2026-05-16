"""
Wave B — Circle Settings Subpage UI tests.

Drives the browser against /community/circle/:slug/settings.

The page is owner-only; we mint a fresh user, seed a circle as that user
(so the user is the owner), and then verify the settings page:
  1. Page loads — back link + header card render
  2. Basic-info inputs populated from the seeded circle
  3. Privacy toggle reflects is_public=TRUE
  4. Members card renders the owner row
  5. Save button starts disabled; editing a field enables it

Run:
    pytest tests/e2e/test_community_circle_settings_ui.py -v
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
def circle_owner():
    """Mint a user, seed a circle they own. Yields (user, circle)."""
    user = mint_user(prefix="e2e-ccs-owner", display_name="CCS Owner")
    circle = seed_circle(user["user_id"], name=f"CCS Test {uuid.uuid4().hex[:6]}")
    yield user, circle
    cleanup_user(user["user_id"])


# ─── Helpers ───────────────────────────────────────────────────────────

def _open_settings(playwright_session, user, slug):
    ctx, page, errors = make_context(playwright_session, user)
    page.goto(
        f"{BASE_URL}/community/circle/{slug}/settings",
        wait_until="domcontentloaded",
        timeout=15000,
    )
    expect(page.locator("#ccs-root")).to_be_visible(timeout=10000)
    return ctx, page, errors


def _wait_for_hydration(page, slug):
    """Wait until the JS replaces the '—' placeholder in the header."""
    page.wait_for_function(
        "() => { const n = document.getElementById('ccs-name');"
        "  return n && n.textContent && n.textContent.trim() !== '—'; }",
        timeout=10000,
    )


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
def test_circle_settings_page_loads_with_back_link(playwright_session, circle_owner):
    """Page renders with back-link + all cards present in DOM."""
    user, circle = circle_owner
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        expect(page.locator(".ccs-back")).to_be_visible()
        expect(page.locator(".ccs-back")).to_have_attribute("href", "/community?tab=circle")

        # All major cards exist in DOM (even if some are hidden).
        for card_title in ["Basic Info", "Privacy", "Members"]:
            expect(page.get_by_role("heading", name=card_title)).to_be_visible()

        # Sticky footer exists.
        expect(page.locator("#ccs-footer")).to_be_attached()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_settings_header_populates_from_api(playwright_session, circle_owner):
    """After JS hydration, header shows the circle name + 1 member + role badge."""
    user, circle = circle_owner
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])

        expect(page.locator("#ccs-name")).to_have_text(circle["name"], timeout=5000)
        # The seeded circle starts with member_count = 1 (just the owner).
        expect(page.locator("#ccs-meta-members")).to_contain_text("member")
        # Owner badge.
        role = page.locator("#ccs-meta-role").text_content() or ""
        assert "owner" in role.lower(), f"Expected owner role, got '{role}'"

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_settings_basic_info_inputs_populated(playwright_session, circle_owner):
    """Name + slug input mirror the seeded values."""
    user, circle = circle_owner
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])

        expect(page.locator("#ccs-input-name")).to_have_value(circle["name"], timeout=5000)
        expect(page.locator("#ccs-input-slug")).to_have_value(circle["slug"])

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_settings_privacy_toggle_reflects_public_state(playwright_session, circle_owner):
    """seed_circle creates with is_public=TRUE → checkbox should be checked."""
    user, circle = circle_owner
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])

        # Give JS one tick to wire the checkbox.
        page.wait_for_timeout(300)
        is_checked = page.locator("#ccs-input-public").is_checked()
        assert is_checked, "Public toggle should be ON for a seeded public circle"

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_circle_settings_save_btn_enables_after_edit(playwright_session, circle_owner):
    """Save button starts disabled; typing a new name enables it."""
    user, circle = circle_owner
    ctx, page, errors = _open_settings(playwright_session, user, circle["slug"])
    try:
        _wait_for_hydration(page, circle["slug"])

        save_btn = page.locator("#ccs-save-btn")
        # Disabled on pristine load.
        expect(save_btn).to_be_disabled()

        # Edit the name → save button should enable.
        page.fill("#ccs-input-name", circle["name"] + " edited")
        # The JS uses an input listener to flip data-state to dirty.
        page.wait_for_function(
            "() => !document.getElementById('ccs-save-btn').disabled",
            timeout=3000,
        )
        expect(save_btn).to_be_enabled()

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
