"""POOOL E2E — Developer: /developer/submission-success

Coverage:
  * Page loads clean with ?title=Sample Asset Title → #submitted-asset-title
    becomes visible and displays the value in fancy quotes.
  * Page loads clean without ?title → #submitted-asset-title stays hidden.
  * "Go to Dashboard" CTA navigates to /developer/dashboard.

Run:
    pytest tests/e2e/test_developer_submission_success.py -v
    HEADED=1 pytest tests/e2e/test_developer_submission_success.py -v
"""
import os
import re
import pytest
from urllib.parse import quote
from playwright.sync_api import expect, Page

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
TIMEOUT = 15_000


def _goto(page: Page, tracker, query: str = ""):
    url = f"{BASE_URL}/developer/submission-success{query}"
    tracker.navigate_and_check(url, timeout=TIMEOUT, wait_until="domcontentloaded")
    page.wait_for_load_state("networkidle", timeout=TIMEOUT)


@pytest.mark.developer
@pytest.mark.smoke
def test_loads_clean(developer_page):
    """Page loads with no console errors, no failed network requests."""
    page, tracker, _ = developer_page
    _goto(page, tracker)

    expect(page).to_have_title(re.compile(r"Submission Success", re.IGNORECASE))
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures(ignore_status=[404])


@pytest.mark.developer
def test_title_param_echoes_into_dom(developer_page):
    """?title=Sample Asset Title → #submitted-asset-title is visible & quoted."""
    page, tracker, _ = developer_page
    title = "Sample Asset Title"
    _goto(page, tracker, f"?title={quote(title)}")

    el = page.locator("#submitted-asset-title")
    expect(el).to_be_visible(timeout=TIMEOUT)

    text = el.inner_text().strip()
    # Inline script wraps with U+201C / U+201D smart quotes — accept either smart
    # or straight quotes to keep test resilient.
    assert title in text, f"Expected title {title!r} in echo, got {text!r}"
    has_smart_quotes = "“" in text and "”" in text
    has_straight_quotes = '"' in text
    assert has_smart_quotes or has_straight_quotes, (
        f"Expected quotes around title, got {text!r}"
    )


@pytest.mark.developer
def test_no_title_param_keeps_element_hidden(developer_page):
    """Without ?title, the echo element stays hidden (data attribute or empty)."""
    page, tracker, _ = developer_page
    _goto(page, tracker)

    el = page.locator("#submitted-asset-title")
    # Element is in DOM but hidden via the `hidden` HTML attribute.
    expect(el).to_be_attached()
    expect(el).not_to_be_visible()


@pytest.mark.developer
def test_dashboard_button_navigates(developer_page):
    """The 'Go to Dashboard' button navigates to /developer/dashboard."""
    page, tracker, _ = developer_page
    _goto(page, tracker)

    btn = page.locator("button.submission-cta", has_text="Go to Dashboard")
    expect(btn).to_be_visible(timeout=TIMEOUT)

    btn.click()
    page.wait_for_url("**/developer/dashboard", timeout=TIMEOUT)
    assert "/developer/dashboard" in page.url


@pytest.mark.developer
@pytest.mark.mobile
def test_loads_on_mobile(developer_page):
    """Mobile viewport: success card renders cleanly."""
    page, tracker, _ = developer_page
    page.set_viewport_size({"width": 375, "height": 812})
    tracker.navigate_and_check(
        f"{BASE_URL}/developer/submission-success?title={quote('Mobile Test')}",
        timeout=TIMEOUT,
    )
    page.wait_for_load_state("networkidle", timeout=TIMEOUT)

    expect(page.locator("#submitted-asset-title")).to_be_visible(timeout=TIMEOUT)
    tracker.assert_no_critical_errors()
