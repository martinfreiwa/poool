"""
Wave F — Mobile viewport regression tests.

Loads each of the 5 key community pages at iPhone-13 width (375px) and
verifies:
  - The page actually renders (assert specific selector visible)
  - No horizontal scrollbar (regression for the topbar/sidebar overflow
    bugs that bit us during the multi-circle rework)
  - No JS errors

Pages covered:
  /community
  /community?tab=circle
  /community?tab=dms
  /community/me
  /community/me/edit

Run:
    pytest tests/e2e/test_community_mobile_ui.py -v
"""

import pytest
from playwright.sync_api import expect

from community_helpers import BASE_URL, mint_user, make_context, cleanup_user


# ─── Fixtures ──────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def mobile_user():
    user = mint_user(prefix="e2e-mobile", display_name="Mobile Tester")
    yield user
    cleanup_user(user["user_id"])


# ─── Helpers ───────────────────────────────────────────────────────────

def _open_mobile(playwright_session, user, path):
    ctx, page, errors = make_context(playwright_session, user, viewport="mobile")
    page.goto(f"{BASE_URL}{path}", wait_until="domcontentloaded", timeout=15000)
    return ctx, page, errors


def _assert_no_horizontal_overflow(page, tolerance=4):
    """Body shouldn't be wider than the viewport (tolerance for scrollbar).

    Skips fixed/transform-positioned off-canvas drawers (mobile burger menu,
    sliding panels) — they report wide bounding rects but do not contribute
    to document scrollWidth. Prints the top offenders for debugging.
    """
    viewport = page.viewport_size
    info = page.evaluate(
        """
        () => {
            const vw = window.innerWidth;
            const sw = document.documentElement.scrollWidth;
            const offenders = [];
            document.querySelectorAll('*').forEach(el => {
                const r = el.getBoundingClientRect();
                if (r.right > vw + 4) {
                    const cs = getComputedStyle(el);
                    // Skip the element itself if it's fixed.
                    if (cs.position === 'fixed') return;
                    // Walk up — if any ancestor is `position: fixed`, this
                    // element belongs to an off-canvas drawer; skip.
                    let p = el.parentElement;
                    let inFixed = false;
                    while (p) {
                        if (getComputedStyle(p).position === 'fixed') {
                            inFixed = true; break;
                        }
                        p = p.parentElement;
                    }
                    if (inFixed) return;
                    offenders.push({
                        tag: el.tagName,
                        id: el.id || null,
                        cls: (el.className && el.className.toString().slice(0, 80)) || null,
                        right: Math.round(r.right),
                        width: Math.round(r.width),
                        pos: cs.position,
                    });
                }
            });
            // Sort by right edge descending so the widest offender shows first.
            offenders.sort((a, b) => b.right - a.right);
            return {
                body: document.body.getBoundingClientRect().width,
                scrollWidth: sw,
                offenders: offenders.slice(0, 10),
            };
        }
        """
    )
    scroll_w = info["scrollWidth"]
    assert scroll_w <= viewport["width"] + tolerance, (
        f"Horizontal overflow: scrollWidth={scroll_w} > viewport={viewport['width']}"
        f" (body box={info['body']}, tolerance={tolerance})\n"
        f"Top non-fixed offenders: {info['offenders']}"
    )


# ─── Tests ─────────────────────────────────────────────────────────────

@pytest.mark.community
@pytest.mark.mobile
def test_mobile_community_main(playwright_session, mobile_user):
    """/community at 375px — composer must render + no JS errors."""
    ctx, page, errors = _open_mobile(playwright_session, mobile_user, "/community")
    try:
        # The composer textarea is the canonical proof the feed mounted.
        expect(page.locator("#post-content-input")).to_be_visible(timeout=10000)
        _assert_no_horizontal_overflow(page)
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
@pytest.mark.mobile
def test_mobile_community_circle_tab(playwright_session, mobile_user):
    """/community?tab=circle at 375px — toolbar + discovery rails."""
    ctx, page, errors = _open_mobile(
        playwright_session, mobile_user, "/community?tab=circle"
    )
    try:
        expect(page.locator("#community-circle-tab")).to_be_visible(timeout=10000)
        expect(page.locator(".cc-toolbar__title")).to_be_visible()
        _assert_no_horizontal_overflow(page)
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
@pytest.mark.mobile
def test_mobile_community_dms_tab(playwright_session, mobile_user):
    """/community?tab=dms at 375px — thread list rail mounts."""
    ctx, page, errors = _open_mobile(
        playwright_session, mobile_user, "/community?tab=dms"
    )
    try:
        expect(page.locator("#community-dm-thread-list")).to_be_visible(timeout=10000)
        _assert_no_horizontal_overflow(page)
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
@pytest.mark.mobile
def test_mobile_profile_page(playwright_session, mobile_user):
    """/community/me at 375px — hero header + tab nav render."""
    ctx, page, errors = _open_mobile(playwright_session, mobile_user, "/community/me")
    try:
        expect(page.locator(".cp-hero")).to_be_visible(timeout=10000)
        # All public tabs should still be present (horizontal scroll if needed).
        expect(page.locator('.community-profile-tab[data-tab="posts"]')).to_be_visible()
        _assert_no_horizontal_overflow(page)
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()


@pytest.mark.community
@pytest.mark.mobile
def test_mobile_profile_edit_page(playwright_session, mobile_user):
    """/community/me/edit at 375px — edit form mounts."""
    ctx, page, errors = _open_mobile(
        playwright_session, mobile_user, "/community/me/edit"
    )
    try:
        # Edit page should reach a known landmark. Multiple candidate selectors
        # because the page has been through several rewrites.
        page.wait_for_selector(
            "form, [id*='edit'], .ds-card, main",
            state="visible",
            timeout=10000,
        )
        _assert_no_horizontal_overflow(page)
        assert not errors, f"JS errors: {errors[:5]}"
    finally:
        ctx.close()
