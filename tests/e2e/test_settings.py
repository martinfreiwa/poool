"""
POOOL E2E Tests — Settings
==========================
Tests profile updates and preference persistence.
"""

import pytest
from playwright.sync_api import expect
import os

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")

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
    
    # 2. Click edit on Core Profile
    page.locator("#morph-core-profile .js-morph-edit").first.click()
    
    # 3. Fill profile form
    page.fill("#edit-first-name", "John")
    page.fill("#edit-last-name", "Doe")
    
    # 4. Save profile
    page.locator("#morph-core-profile .js-morph-save").click()
    
    # 5. Success check (Toast / Alert)
    from tests.e2e.conftest import check_toast_message
    check_toast_message(page, "success", timeout=5000)
    
    # 6. Reload and verify persistence
    page.reload()
    page.wait_for_timeout(1000)
    # The read view should show "John Doe"
    expect(page.locator("#read-name").first).to_have_text("John Doe", ignore_case=True)
    
    tracker.full_health_check()

@pytest.mark.settings
def test_user_settings_preferences_update(authenticated_user_page):
    """Verifies that currency and other preferences persist."""
    page, tracker, user = authenticated_user_page
    
    tracker.navigate_and_check(f"{BASE_URL}/settings")
    page.wait_for_timeout(1000)
    
    # 1. Switch to Preferences tab
    page.click("a[href='#section-preferences']")
    
    # 2. Select timezone (the select is visually hidden by a custom dropdown, use force=True)
    page.select_option("#edit-timezone", "Europe/London", force=True)
    
    # Trigger the change event because custom dropdowns often listen to native change events
    page.locator("#edit-timezone").evaluate("node => node.dispatchEvent(new Event('change'))")
    
    # 3. Save preferences
    page.click("#save-localization-btn")
    from tests.e2e.conftest import check_toast_message
    check_toast_message(page, "success")
    
    # 4. Reload and verify
    page.reload()
    page.wait_for_timeout(1000)
    page.click("a[href='#section-preferences']")
    expect(page.locator("#edit-timezone")).to_have_value("Europe/London")
    
    tracker.full_health_check()
