"""
POOOL E2E Tests — Public Property Page
=======================================
Verifies that /p/:slug pages are accessible without authentication,
do not redirect to login, and render correctly with images intact.
"""

import pytest
from playwright.sync_api import expect

BASE_SLUGS = [
    "sunset-luxury-villa",
    "ocean-breeze-penthouse",
    "echo-beach-loft",
    "rice-terrace-retreat",
    "tropical-garden-villa",
    "cliffside-sunset-estate",
    "beachfront-bungalow",
    "modern-jungle-retreat",
]

FUNDED_SLUGS = {"beachfront-bungalow", "modern-jungle-retreat"}


@pytest.mark.smoke
def test_public_property_no_auth_redirect(quality_page):
    """Unauthenticated visit to /p/:slug must NOT redirect to login."""
    page, tracker = quality_page
    slug = "sunset-luxury-villa"

    tracker.navigate_and_check(f"http://localhost:8888/p/{slug}")

    # Must stay on the property page — not redirected to login
    assert "/auth/login" not in page.url, (
        f"Redirected to login from /p/{slug} — auth guard still firing"
    )
    assert f"/p/{slug}" in page.url, (
        f"Unexpected URL after navigating to /p/{slug}: {page.url}"
    )


@pytest.mark.smoke
def test_public_property_content_renders(quality_page):
    """Property title, price card and sign-up CTA are visible."""
    page, tracker = quality_page
    tracker.navigate_and_check("http://localhost:8888/p/sunset-luxury-villa")

    # Heading
    expect(page.locator("h1#property-title")).to_be_visible(timeout=8000)

    # Price card
    expect(page.locator("#property-price-card")).to_be_visible()

    # Sign up CTA (not Add to cart — public page)
    cta = page.locator("#property-price-card .add-to-cart-btn")
    expect(cta).to_be_visible()
    cta_text = cta.inner_text().lower()
    assert "sign up" in cta_text or "sold out" in cta_text, (
        f"Unexpected CTA text on public page: '{cta_text}'"
    )


@pytest.mark.smoke
def test_public_property_images_load(quality_page):
    """No broken images on the public property page."""
    page, tracker = quality_page
    tracker.navigate_and_check("http://localhost:8888/p/sunset-luxury-villa")

    # Give images time to load
    page.wait_for_load_state("networkidle", timeout=10000)
    tracker.assert_no_broken_images()


@pytest.mark.parametrize("slug", BASE_SLUGS)
def test_all_landing_card_slugs_resolve(quality_page, slug):
    """Every landing page card slug returns a 200 and stays off login."""
    page, tracker = quality_page
    response = tracker.navigate_and_check(f"http://localhost:8888/p/{slug}")

    assert response is not None and response.status == 200, (
        f"/p/{slug} returned HTTP {response.status if response else 'no response'}"
    )
    assert "/auth/login" not in page.url, (
        f"/p/{slug} redirected to login"
    )


def test_fully_funded_property_shows_sold_out(quality_page):
    """Fully-funded cards show 'sold out' CTA, not 'sign up to invest'."""
    page, tracker = quality_page
    tracker.navigate_and_check("http://localhost:8888/p/beachfront-bungalow")

    cta = page.locator("#property-price-card .add-to-cart-btn")
    expect(cta).to_be_visible(timeout=8000)
    assert "sold out" in cta.inner_text().lower(), (
        "Fully-funded property should show 'sold out', not invest CTA"
    )


def test_breadcrumb_links_back_to_landing(quality_page):
    """Breadcrumb 'Properties' link points to landing-v2 marketplace section."""
    page, tracker = quality_page
    tracker.navigate_and_check("http://localhost:8888/p/sunset-luxury-villa")

    breadcrumb = page.locator("#property-breadcrumbs a").first
    expect(breadcrumb).to_be_visible(timeout=5000)
    href = breadcrumb.get_attribute("href") or ""
    assert "marketplace" in href or "landing" in href, (
        f"Breadcrumb href unexpected: {href}"
    )


def test_public_property_full_health(quality_page):
    """Full quality check: no JS errors, no HTTP failures, no broken images.
    401s on /api/me and /api/user/legal-status are expected for unauthenticated visits."""
    page, tracker = quality_page
    tracker.navigate_and_check("http://localhost:8888/p/echo-beach-loft")
    page.wait_for_load_state("networkidle", timeout=10000)
    tracker.assert_page_loaded()
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures(ignore_status=[401, 404])
    tracker.assert_no_broken_images()
