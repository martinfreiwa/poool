"""
POOOL E2E Tests — Settings
==========================
Tests profile updates and preference persistence.
"""

import pytest
from playwright.sync_api import expect
import os

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")

def set_settings_select(page, selector, value):
    """Set native settings selects even after PooolDropdown hides them."""
    select = page.locator(selector)
    if select.is_visible():
        select.select_option(value)
        return
    page.evaluate(
        """({ selector, value }) => {
            const select = document.querySelector(selector);
            if (!select) throw new Error(`Missing select: ${selector}`);
            select.value = value;
            select.dispatchEvent(new Event("change", { bubbles: true }));
        }""",
        {"selector": selector, "value": value},
    )

@pytest.mark.settings
@pytest.mark.smoke
def test_user_settings_profile_update(authenticated_user_page):
    """Verifies that first and last name updates persist."""
    page, tracker, user = authenticated_user_page
    
    # 1. Navigate to settings
    tracker.navigate_and_check(f"{BASE_URL}/settings")
    tracker.assert_page_loaded()
    # Wait for the client-side data fetch if applicable
    page.wait_for_timeout(1000)
    
    page.locator("#settings-content").wait_for(state="visible", timeout=15000)

    page.fill("#settings-first-name", "John")
    page.fill("#settings-last-name", "Doe")
    page.click("#form-core-profile button[type='submit']")
    
    # 5. Success check (Toast / Alert)
    from tests.e2e.conftest import check_toast_message
    check_toast_message(page, "success", timeout=5000)
    
    # 6. Reload and verify persistence
    page.reload()
    page.wait_for_timeout(1000)
    page.locator("#settings-content").wait_for(state="visible", timeout=15000)
    expect(page.locator("#settings-display-name")).to_contain_text("John Doe", ignore_case=True)
    
    tracker.full_health_check()

@pytest.mark.settings
def test_user_settings_preferences_update(authenticated_user_page):
    """Verifies that currency and other preferences persist."""
    page, tracker, user = authenticated_user_page
    
    tracker.navigate_and_check(f"{BASE_URL}/settings")
    page.wait_for_timeout(1000)
    
    page.locator("#settings-content").wait_for(state="visible", timeout=15000)
    page.click("a[href='#sec-preferences']")
    set_settings_select(page, "#settings-timezone", "Europe/London")
    page.click("#btn-save-preferences")
    from tests.e2e.conftest import check_toast_message
    check_toast_message(page, "success")
    
    # 4. Reload and verify
    page.reload()
    page.locator("#settings-content").wait_for(state="visible", timeout=15000)
    page.click("a[href='#sec-preferences']")
    expect(page.locator("#settings-timezone")).to_have_value("Europe/London")
    
    tracker.full_health_check()
