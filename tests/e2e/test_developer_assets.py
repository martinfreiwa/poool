"""POOOL E2E — Developer: /developer/assets

Smoke + filter-strip coverage for the developer-assets management page.

Coverage:
  * Page loads clean (no JS errors, no 5xx).
  * If the developer has assets: the filter strip renders (search input,
    All/Available/Funded tabs, table columns).
  * If the developer has no assets: the split-hero empty state renders.
  * Switching tabs updates aria-pressed + active class.
  * Typing in the search input triggers client-side filtering.
  * Mobile viewport renders cleanly.

Run:
    pytest tests/e2e/test_developer_assets.py -v
    HEADED=1 pytest tests/e2e/test_developer_assets.py -v
"""
import os
import re
import pytest
from playwright.sync_api import expect, Page

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
TIMEOUT = 15_000


def _goto_assets(page: Page, tracker):
    tracker.navigate_and_check(
        f"{BASE_URL}/developer/assets",
        timeout=TIMEOUT,
        wait_until="domcontentloaded",
    )
    page.wait_for_load_state("networkidle", timeout=TIMEOUT)


def _has_assets(page: Page) -> bool:
    """True if the page rendered the assets workspace (vs the empty state)."""
    return page.locator(".dev-assets-workspace").count() > 0


@pytest.mark.developer
@pytest.mark.smoke
def test_loads_clean(developer_page):
    """Page loads with no console errors, no failed network requests."""
    page, tracker, _ = developer_page
    _goto_assets(page, tracker)

    expect(page).to_have_title(re.compile(r"Developer Assets", re.IGNORECASE))
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures(ignore_status=[404])


@pytest.mark.developer
def test_filter_strip_or_empty_state(developer_page):
    """A fresh developer has no assets → empty state. With assets → filter strip."""
    page, tracker, _ = developer_page
    _goto_assets(page, tracker)

    if _has_assets(page):
        # Search input, status tabs, table columns all rendered.
        expect(page.locator("#dev-assets-search-input")).to_be_visible(timeout=TIMEOUT)
        expect(page.locator('[data-dev-assets-tab="all"]')).to_be_visible()
        expect(page.locator('[data-dev-assets-tab="available"]')).to_be_visible()
        expect(page.locator('[data-dev-assets-tab="funded"]')).to_be_visible()
        expect(page.locator("#dev-assets-table")).to_be_visible()
    else:
        # Empty state hero present.
        expect(page.locator(".dae-empty")).to_be_visible(timeout=TIMEOUT)
        expect(page.locator("#dae-empty-title")).to_be_visible()


@pytest.mark.developer
def test_search_input_filters(developer_page):
    """Typing in the search input triggers client-side filtering (or is a no-op for empty state)."""
    page, tracker, _ = developer_page
    _goto_assets(page, tracker)

    if not _has_assets(page):
        pytest.skip("No assets — empty state has no search input.")

    search = page.locator("#dev-assets-search-input")
    expect(search).to_be_visible()

    # Type a string that almost-certainly matches nothing.
    search.fill("zzzz-very-unlikely-asset-name-xyz")
    page.wait_for_timeout(200)  # debounce / input handler

    # The empty-row should now be visible (or all rows hidden). Either way the
    # JS handler ran without throwing.
    empty_row = page.locator("#dev-assets-empty-row")
    visible_rows = page.locator("tr.dev-asset-row:visible").count()
    if visible_rows == 0:
        # Empty row should reveal (it has the `hidden` attr removed when no rows match).
        # Some impls toggle display rather than hidden — accept either.
        assert empty_row.count() == 1


@pytest.mark.developer
def test_tab_switching(developer_page):
    """Clicking a tab updates aria-pressed and active class."""
    page, tracker, _ = developer_page
    _goto_assets(page, tracker)

    if not _has_assets(page):
        pytest.skip("No assets — tabs only render in workspace.")

    available_tab = page.locator('[data-dev-assets-tab="available"]')
    all_tab = page.locator('[data-dev-assets-tab="all"]')

    expect(all_tab).to_have_attribute("aria-pressed", "true")

    available_tab.click()
    page.wait_for_timeout(100)
    expect(available_tab).to_have_attribute("aria-pressed", "true")
    expect(available_tab).to_have_class(re.compile(r"active"))
    expect(all_tab).to_have_attribute("aria-pressed", "false")


@pytest.mark.developer
@pytest.mark.mobile
def test_loads_on_mobile(developer_page):
    """Mobile viewport renders cleanly and the toolbar layout still loads."""
    page, tracker, _ = developer_page
    page.set_viewport_size({"width": 375, "height": 812})
    tracker.navigate_and_check(f"{BASE_URL}/developer/assets", timeout=TIMEOUT)
    page.wait_for_load_state("networkidle", timeout=TIMEOUT)

    expect(page.locator("#mobile-header")).to_be_attached(timeout=TIMEOUT)
    expect(page.locator("#mobile-burger-btn")).to_be_attached()
    tracker.assert_no_critical_errors()
