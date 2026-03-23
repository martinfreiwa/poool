import re
from playwright.sync_api import expect
import os

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")

def test_user_settings_profile_update(authenticated_user_page):
    page, current_user = authenticated_user_page
    
    # Go to settings
    page.goto(f"{BASE_URL}/settings")
    page.wait_for_load_state("networkidle")
    
    # Make sure we're on the right page
    expect(page.locator("h1").first).to_contain_text("Settings")
    
    # Update first and last name
    page.fill("#settings-first-name", "John")
    page.fill("#settings-last-name", "Doe")
    page.fill("#settings-phone", "+1234567890")
    
    # Save profile
    page.click("#btn-save-profile")
    
    # Expect a toast or success indicator (toast class might vary, we can wait for networkidle)
    page.wait_for_load_state("networkidle")
    
    # Reload and verify persistence
    page.reload()
    expect(page.locator("#settings-first-name")).to_have_value("John")
    expect(page.locator("#settings-last-name")).to_have_value("Doe")
    expect(page.locator("#settings-phone")).to_have_value("+1234567890")

def test_user_settings_preferences_update(authenticated_user_page):
    page, current_user = authenticated_user_page
    
    page.goto(f"{BASE_URL}/settings")
    page.wait_for_load_state("networkidle")
    
    # Navigate to preferences panel by clicking the tab
    page.click("#tab-preferences")
    
    # Wait for the panel to be visible (it controls opacity/display)
    expect(page.locator("#panel-preferences")).to_be_visible()
    
    # Update currency
    page.select_option("#settings-currency", "EUR")
    
    # Save preferences
    page.click("#btn-save-preferences")
    page.wait_for_load_state("networkidle")
    
    # Reload and verify
    page.reload()
    page.click("#tab-preferences")
    expect(page.locator("#panel-preferences")).to_be_visible()
    expect(page.locator("#settings-currency")).to_have_value("EUR")
