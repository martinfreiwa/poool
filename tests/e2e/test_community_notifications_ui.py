"""
Wave G — Notifications inbox-bell UI tests.

Validates the topbar inbox-bell:
  1. Unread badge shows the seeded count + clicking the bell opens the panel.
  2. Mark-all-read fires POST + hides the badge.

Backend correctness covered by scripts/test_notifications_e2e.sh; this is
pure UI wiring.

Run:
    pytest tests/e2e/test_community_notifications_ui.py -v
"""

import pytest
from playwright.sync_api import expect

from community_helpers import (
    BASE_URL,
    mint_user,
    make_context,
    cleanup_user,
    seed_notification,
)


# ─── Fixtures ──────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def user_with_notifications():
    user = mint_user(prefix="e2e-notif", display_name="Notif Tester")
    ids = [
        seed_notification(user["user_id"], title=f"UI test notif #{i}",
                          message="Helper-seeded", ntype="system")
        for i in range(3)
    ]
    yield user, ids
    cleanup_user(user["user_id"])


# ─── Helpers ───────────────────────────────────────────────────────────

def _open_community(playwright_session, user):
    ctx, page, errors = make_context(playwright_session, user)
    page.goto(f"{BASE_URL}/community", wait_until="domcontentloaded", timeout=15000)
    expect(page.locator("#inbox-bell-btn")).to_be_visible(timeout=10000)
    return ctx, page, errors


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
def test_inbox_bell_badge_shows_unread_count(playwright_session, user_with_notifications):
    """After login, the bell badge should reveal a non-zero count
    (polled by inbox-bell.js)."""
    user, _ids = user_with_notifications
    ctx, page, errors = _open_community(playwright_session, user)
    try:
        badge = page.locator("#inbox-bell-badge")
        # The badge starts `hidden` until the unread-count poll resolves.
        # Wait for it to become visible.
        page.wait_for_function(
            "() => { const b = document.getElementById('inbox-bell-badge');"
            "  return b && !b.hasAttribute('hidden') && Number(b.textContent) > 0; }",
            timeout=10000,
        )
        count_text = badge.text_content() or ""
        assert int(count_text) >= 3, f"Expected ≥3 unread, got {count_text}"

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
def test_inbox_bell_dropdown_opens_and_mark_all_read(playwright_session, user_with_notifications):
    """Clicking the bell opens the panel; Mark all read fires POST and the
    badge becomes hidden again."""
    user, _ids = user_with_notifications
    ctx, page, errors = _open_community(playwright_session, user)
    try:
        # Wait until badge populates so we know the JS is alive.
        page.wait_for_function(
            "() => { const b = document.getElementById('inbox-bell-badge');"
            "  return b && !b.hasAttribute('hidden'); }",
            timeout=10000,
        )

        page.click("#inbox-bell-btn")
        # Panel injected on first click.
        panel = page.locator(".inbox-bell__panel")
        expect(panel).to_be_visible(timeout=5000)
        expect(page.locator("#inbox-mark-all")).to_be_visible()

        # Click mark-all-read and wait for POST.
        with page.expect_response(
            lambda r: r.url.endswith("/api/inbox/read-all") and r.request.method == "POST",
            timeout=5000,
        ) as info:
            page.click("#inbox-mark-all")
        assert info.value.status == 200, f"read-all POST: {info.value.status}"

        # Badge should disappear (hidden attr added back).
        page.wait_for_function(
            "() => document.getElementById('inbox-bell-badge').hasAttribute('hidden')",
            timeout=5000,
        )

        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
