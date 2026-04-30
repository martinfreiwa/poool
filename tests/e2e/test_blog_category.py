"""
POOOL E2E Tests - Blog Category Page
====================================
Verifies the public blog category route renders category-specific metadata,
rejects unsafe slugs, and keeps the mobile navigation accessible.
"""

from pathlib import Path

from playwright.sync_api import expect

from tests.e2e.conftest import BASE_URL


REPO_ROOT = Path(__file__).resolve().parents[2]


def assert_nav_open(nav, expected: bool):
    classes = (nav.get_attribute("class") or "").split()
    assert ("open" in classes) is expected


def test_blog_category_browser_metadata_and_health(quality_page):
    """Category pages should render category-specific SEO and breadcrumb data."""
    page, tracker = quality_page

    response = tracker.navigate_and_check(
        f"{BASE_URL}/blog/category/investment-guides"
    )

    assert response is not None and response.status == 200
    assert "/blog/category/investment-guides" in page.url
    assert "Investment Guides Articles" in page.title()

    expect(page.locator("h1")).to_contain_text("The POOOL Blog")
    expect(page.locator(".blog-category-pill--active")).to_contain_text(
        "Investment Guides"
    )

    canonical = page.locator("link[rel='canonical']").get_attribute("href") or ""
    og_url = page.locator("meta[property='og:url']").get_attribute("content") or ""
    assert canonical.endswith("/blog/category/investment-guides")
    assert og_url.endswith("/blog/category/investment-guides")

    breadcrumb_json = page.locator("script[type='application/ld+json']").last.inner_text()
    assert '"Investment Guides"' in breadcrumb_json
    assert "/blog/category/investment-guides" in breadcrumb_json

    tracker.assert_page_loaded()
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures()


def test_blog_category_unsafe_slug_returns_404(quality_page):
    """Unsafe category slugs must 404 instead of dropping the category filter."""
    page, tracker = quality_page

    response = page.goto(
        f"{BASE_URL}/blog/category/%3Cscript%3E",
        wait_until="domcontentloaded",
        timeout=15000,
    )

    assert response is not None and response.status == 404
    expect(page.locator("body")).to_contain_text("Category not found")
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures(ignore_status=[404])


def test_blog_category_mobile_menu_keyboard_state(mobile_page):
    """The mobile menu should expose and update expanded state for keyboard users."""
    page, tracker = mobile_page

    tracker.navigate_and_check(f"{BASE_URL}/blog/category/investment-guides")

    toggle = page.locator(".blog-header__mobile-toggle")
    nav = page.locator("#blog-primary-nav")

    expect(toggle).to_be_visible()
    expect(toggle).to_have_attribute("aria-controls", "blog-primary-nav")
    expect(toggle).to_have_attribute("aria-expanded", "false")

    toggle.click()
    expect(toggle).to_have_attribute("aria-expanded", "true")
    assert_nav_open(nav, True)

    page.keyboard.press("Escape")
    expect(toggle).to_have_attribute("aria-expanded", "false")
    assert_nav_open(nav, False)

    toggle.focus()
    page.keyboard.press("Enter")
    expect(toggle).to_have_attribute("aria-expanded", "true")

    page.locator(".blog-main").click(position={"x": 10, "y": 10})
    expect(toggle).to_have_attribute("aria-expanded", "false")

    tracker.assert_page_loaded()
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures()


def test_blog_category_pagination_template_contract():
    """Multi-page category pagination must stay on the selected category route."""
    template = (REPO_ROOT / "frontend/platform/blog/index.html").read_text(
        encoding="utf-8"
    )

    assert "/blog/category/{{ active_category }}?page={{ page - 1 }}" in template
    assert "/blog/category/{{ active_category }}?page={{ page + 1 }}" in template
    assert "&category={{ active_category }}" not in template
