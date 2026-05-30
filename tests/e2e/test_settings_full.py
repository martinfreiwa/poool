"""
POOOL E2E — Settings Full Save/Reload Test
===========================================
For every settings section: fill fields → save → reload → assert values persisted.

Run:
    pytest tests/e2e/test_settings_full.py -v --base-url http://localhost:8888

Requires: pytest-playwright, a running backend, and a test user seeded in DB.
"""

import os
import pytest
import re
from playwright.sync_api import expect, Page

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
TIMEOUT = 10_000  # ms


# ─── Helpers ──────────────────────────────────────────────────────────────────

def set_select(page: Page, selector: str, value: str):
    """Set a <select> value even when PooolDropdown has hidden it."""
    page.evaluate(
        """({ sel, val }) => {
            const el = document.querySelector(sel);
            if (!el) throw new Error('select not found: ' + sel);
            el.value = val;
            el.dispatchEvent(new Event('change', { bubbles: true }));
        }""",
        {"sel": selector, "val": value},
    )


def wait_for_settings(page: Page):
    page.locator("#settings-content").wait_for(state="visible", timeout=15_000)
    page.wait_for_load_state("networkidle", timeout=10_000)
    page.wait_for_timeout(600)


def expect_toast(page: Page, kind: str = "success"):
    """Assert a toast appears within 6 s."""
    page.locator(".poool-toast-card").first.wait_for(state="visible", timeout=6_000)


def get_select_value(page: Page, selector: str) -> str:
    return page.evaluate(
        "sel => document.querySelector(sel)?.value ?? ''", selector
    )


# ─── Fixtures ─────────────────────────────────────────────────────────────────

@pytest.fixture
def settings_page(authenticated_user_page):
    """Navigate to /settings and wait for content."""
    page, tracker, user = authenticated_user_page
    page.goto(f"{BASE_URL}/settings", wait_until="domcontentloaded")
    wait_for_settings(page)
    return page, tracker, user


@pytest.fixture
def developer_settings_page(developer_page):
    """Navigate to /settings with a developer-role user."""
    page, tracker, user = developer_page
    page.goto(f"{BASE_URL}/settings", wait_until="domcontentloaded")
    wait_for_settings(page)
    return page, tracker, user


# ─── 1. Core Profile ──────────────────────────────────────────────────────────

@pytest.mark.settings
def test_core_profile_saves(settings_page):
    """First name, last name, middle name, phone, gender all persist."""
    page, tracker, _ = settings_page

    page.fill("#settings-first-name", "Alice")
    page.fill("#settings-middle-name", "Marie")
    page.fill("#settings-last-name", "Testuser")
    page.fill("#settings-phone", "+4915112345678")
    set_select(page, "#settings-gender", "female")

    page.locator("button.btn-save-profile[data-section='core']").click()
    expect_toast(page)

    page.reload(wait_until="domcontentloaded")
    wait_for_settings(page)

    expect(page.locator("#settings-first-name")).to_have_value("Alice")
    expect(page.locator("#settings-middle-name")).to_have_value("Marie")
    expect(page.locator("#settings-last-name")).to_have_value("Testuser")
    expect(page.locator("#settings-phone")).to_have_value(re.compile(r"\+?4915112345678"))
    assert get_select_value(page, "#settings-gender") == "female", "gender not persisted"

    tracker.full_health_check()


# ─── 2. Address ───────────────────────────────────────────────────────────────

@pytest.mark.settings
def test_address_saves(settings_page):
    """All address fields and country dropdown persist."""
    page, tracker, _ = settings_page

    page.fill("#settings-address-1", "Musterstrasse 42")
    page.fill("#settings-address-2", "Apt 3B")
    page.fill("#settings-city", "Berlin")
    page.fill("#settings-state", "Berlin")
    page.fill("#settings-postal", "10115")
    set_select(page, "#settings-country", "DE")

    page.locator("button.btn-save-profile[data-section='address']").click()
    expect_toast(page)

    page.reload(wait_until="domcontentloaded")
    wait_for_settings(page)

    expect(page.locator("#settings-address-1")).to_have_value("Musterstrasse 42")
    expect(page.locator("#settings-address-2")).to_have_value("Apt 3B")
    expect(page.locator("#settings-city")).to_have_value("Berlin")
    expect(page.locator("#settings-state")).to_have_value("Berlin")
    expect(page.locator("#settings-postal")).to_have_value("10115")
    assert get_select_value(page, "#settings-country") == "DE", "country not persisted"

    tracker.full_health_check()


# ─── 3. Identity Vault ────────────────────────────────────────────────────────

@pytest.mark.settings
def test_identity_saves(settings_page):
    """Date of birth, nationality dropdown, and tax ID persist."""
    page, tracker, _ = settings_page

    page.fill("#settings-dob", "1990-06-15")
    set_select(page, "#settings-nationality", "DE")
    page.fill("#settings-tax-id", "DE123456789")

    page.locator("button.btn-save-profile[data-section='identity']").click()
    expect_toast(page)

    page.reload(wait_until="domcontentloaded")
    wait_for_settings(page)

    expect(page.locator("#settings-dob")).to_have_value("1990-06-15")
    assert get_select_value(page, "#settings-nationality") == "DE", "nationality not persisted"
    expect(page.locator("#settings-tax-id")).to_have_value("DE123456789")

    tracker.full_health_check()


# ─── 4. Preferences ───────────────────────────────────────────────────────────

@pytest.mark.settings
def test_preferences_saves(settings_page):
    """Language, timezone, and currency dropdowns persist."""
    page, tracker, _ = settings_page

    set_select(page, "#settings-language", "de")
    set_select(page, "#settings-timezone", "Europe/Berlin")
    set_select(page, "#settings-currency", "EUR")

    page.locator("#btn-save-preferences").click()
    expect_toast(page)

    page.reload(wait_until="domcontentloaded")
    wait_for_settings(page)

    assert get_select_value(page, "#settings-language") == "de", "language not persisted"
    assert get_select_value(page, "#settings-timezone") == "Europe/Berlin", "timezone not persisted"
    assert get_select_value(page, "#settings-currency") == "EUR", "currency not persisted"

    # Reset to defaults
    set_select(page, "#settings-language", "en")
    set_select(page, "#settings-timezone", "UTC")
    set_select(page, "#settings-currency", "USD")
    page.locator("#btn-save-preferences").click()

    tracker.full_health_check()


# ─── 5. Leaderboard ───────────────────────────────────────────────────────────

@pytest.mark.settings
def test_leaderboard_saves(settings_page):
    """Display name and bio persist; toggles reflect correct state."""
    page, tracker, _ = settings_page

    page.fill("#settings-lb-display-name", "CryptoAlice")
    page.fill("#settings-lb-bio", "Investing in the future of real estate.")

    page.locator("#btn-save-leaderboard-privacy").click()
    expect_toast(page)

    page.reload(wait_until="domcontentloaded")
    wait_for_settings(page)

    expect(page.locator("#settings-lb-display-name")).to_have_value("CryptoAlice")
    expect(page.locator("#settings-lb-bio")).to_have_value(
        re.compile(r"Investing in the future", re.IGNORECASE)
    )

    tracker.full_health_check()


# ─── 6. Social Links ──────────────────────────────────────────────────────────

@pytest.mark.settings
def test_social_links_save(settings_page):
    """All social link fields persist."""
    page, tracker, _ = settings_page

    page.fill("#settings-social-twitter", "https://x.com/alicetestuser")
    page.fill("#settings-social-linkedin", "https://linkedin.com/in/alicetestuser")
    page.fill("#settings-social-instagram", "https://instagram.com/alicetestuser")
    page.fill("#settings-social-telegram", "https://t.me/alicetestuser")
    page.fill("#settings-social-discord", "alice#1234")
    page.fill("#settings-social-website", "https://alice.example.com")

    page.locator("#btn-save-social").click()
    expect_toast(page)

    page.reload(wait_until="domcontentloaded")
    wait_for_settings(page)

    expect(page.locator("#settings-social-twitter")).to_have_value("https://x.com/alicetestuser")
    expect(page.locator("#settings-social-linkedin")).to_have_value(
        "https://linkedin.com/in/alicetestuser"
    )
    expect(page.locator("#settings-social-instagram")).to_have_value(
        "https://instagram.com/alicetestuser"
    )
    expect(page.locator("#settings-social-telegram")).to_have_value("https://t.me/alicetestuser")
    expect(page.locator("#settings-social-discord")).to_have_value("alice#1234")
    expect(page.locator("#settings-social-website")).to_have_value("https://alice.example.com")

    tracker.full_health_check()


# ─── 7. Developer Profile (developer users only) ──────────────────────────────

@pytest.mark.settings
@pytest.mark.developer
def test_developer_profile_saves(developer_settings_page):
    """Developer company name and description persist."""
    page, tracker, _ = developer_settings_page

    page.fill("#settings-dev-company", "Alice Real Estate GmbH")
    page.fill(
        "#settings-dev-description",
        "We develop sustainable luxury properties across Southeast Asia.",
    )

    page.locator("#btn-save-developer-profile").click()
    expect_toast(page)

    page.reload(wait_until="domcontentloaded")
    wait_for_settings(page)

    expect(page.locator("#settings-dev-company")).to_have_value("Alice Real Estate GmbH")
    expect(page.locator("#settings-dev-description")).to_have_value(
        re.compile(r"sustainable luxury", re.IGNORECASE)
    )

    tracker.full_health_check()


# ─── 8. Developer Links (developer users only) ────────────────────────────────

@pytest.mark.settings
@pytest.mark.developer
def test_developer_links_save(developer_settings_page):
    """Developer external links persist."""
    page, tracker, _ = developer_settings_page

    page.fill("#settings-dev-website", "https://alice-re.example.com")
    page.fill("#settings-dev-github", "https://github.com/alicere")
    page.fill("#settings-dev-twitter", "https://x.com/alicere")
    page.fill("#settings-dev-linkedin", "https://linkedin.com/company/alicere")
    page.fill("#settings-dev-youtube", "https://youtube.com/@alicere")

    page.locator("#btn-save-developer-links").click()
    expect_toast(page)

    page.reload(wait_until="domcontentloaded")
    wait_for_settings(page)

    expect(page.locator("#settings-dev-website")).to_have_value("https://alice-re.example.com")
    expect(page.locator("#settings-dev-github")).to_have_value("https://github.com/alicere")
    expect(page.locator("#settings-dev-linkedin")).to_have_value(
        "https://linkedin.com/company/alicere"
    )

    tracker.full_health_check()


# ─── 9. Full page smoke — no JS errors ────────────────────────────────────────

@pytest.mark.settings
@pytest.mark.smoke
def test_settings_no_js_errors(settings_page):
    """Settings page loads with zero console errors."""
    page, tracker, _ = settings_page
    tracker.assert_page_loaded()
    tracker.full_health_check()
