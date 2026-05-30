"""
Authenticated browser checks for the affiliate promo page.
"""

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


STALE_COPY = [
    "48 hours",
    "Bronze",
    "Gold",
    "Diamond",
    "Ambassador",
]


@pytest.fixture(scope="function")
def authenticated_mobile_page(playwright_session, request):
    """Creates a fresh authenticated investor session in a mobile viewport."""
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    user = create_e2e_user(
        email_prefix="e2e-affiliate-promo-mobile",
        display_name="E2E Affiliate Promo Mobile",
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker

    _teardown_context(context, page, tracker, request)
    cleanup_test_user(user["user_id"])


def _set_range_value(page, selector, value):
    page.locator(selector).evaluate(
        """
        (el, nextValue) => {
          el.value = String(nextValue);
          el.dispatchEvent(new Event('input', { bubbles: true }));
        }
        """,
        value,
    )


def test_affiliate_promo_authenticated_desktop_regression(authenticated_user_page):
    page, tracker, _user = authenticated_user_page

    tracker.navigate_and_check(f"{BASE_URL}/affiliate")
    expect(page).to_have_url(f"{BASE_URL}/affiliate")
    expect(page.get_by_role("heading", name="Monetize your Audience with Institutional Assets")).to_be_visible()
    expect(page.get_by_role("heading", name="Who We Approve")).to_be_visible()
    expect(page.get_by_text("4.50%").first).to_be_visible()
    expect(page.get_by_role("cell", name="Sovereign")).to_be_visible()
    expect(page.get_by_text("~$2,500,000 qualified referral volume").first).to_be_visible()
    expect(page.get_by_text("1–3 business days").first).to_be_visible()

    body_text = page.locator("body").inner_text()
    for stale in STALE_COPY:
        assert stale not in body_text

    page.locator("#hero-view-tiers-btn").click()
    expect(page.locator("#who-we-want")).to_be_in_viewport()

    expect(page.get_by_label("Average Investment per Referral")).to_be_visible()
    expect(page.get_by_label("Qualified Referrals per Month")).to_be_visible()
    _set_range_value(page, "#calc-investment", 100000)
    _set_range_value(page, "#calc-referrals", 50)
    expect(page.locator("#calc-tier-badge")).to_have_text("Sovereign")
    expect(page.locator("#calc-tier-rate")).to_have_text("4.50%")
    expect(page.locator("#calc-monthly-earnings")).to_have_text("225,000")

    page.get_by_text("How does the commission structure work?").click()
    expect(page.get_by_text("There are no downlines")).to_be_visible()

    tracker.assert_basic_a11y()
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures()


@pytest.mark.mobile
def test_affiliate_promo_authenticated_mobile_layout(authenticated_mobile_page):
    page, tracker = authenticated_mobile_page

    tracker.navigate_and_check(f"{BASE_URL}/affiliate")
    expect(page.get_by_role("heading", name="Monetize your Audience with Institutional Assets")).to_be_visible()
    expect(page.locator("#hero-view-tiers-btn")).to_be_visible()
    expect(page.locator("#calculator")).to_be_visible()

    layout = page.evaluate(
        """
        () => {
          const viewportWidth = window.innerWidth;
          const selectors = [
            ".app-content",
            ".affiliate-promo-container",
            ".promo-hero",
            ".promo-stats-bar",
            ".tier-card",
            ".calc-card",
            ".promo-cta-section",
          ];
          const clipped = selectors.flatMap((selector) => {
            return Array.from(document.querySelectorAll(selector)).map((el) => {
              const rect = el.getBoundingClientRect();
              return {
                selector,
                left: rect.left,
                right: rect.right,
                width: rect.width,
                visible: rect.width > 0 && rect.height > 0,
                overflows: rect.left < -1 || rect.right > viewportWidth + 1,
              };
            });
          }).filter((item) => !item.visible || item.overflows);
          return {
            viewportWidth,
            scrollWidth: document.documentElement.scrollWidth,
            clipped,
          };
        }
        """
    )
    assert not layout["clipped"], layout
    assert layout["scrollWidth"] <= layout["viewportWidth"] + 1, layout

    _set_range_value(page, "#calc-referrals", 1)
    expect(page.locator("#calc-tier-badge")).to_have_text("Elite")

    tracker.assert_basic_a11y()
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures()
