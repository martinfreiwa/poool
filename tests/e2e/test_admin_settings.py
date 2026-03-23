import pytest
import os
import time
from playwright.sync_api import sync_playwright, expect

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
ADMIN_EMAIL = os.environ.get("ADMIN_EMAIL", "test@poool.app")
ADMIN_PASSWORD = os.environ.get("ADMIN_PASSWORD", "TestPass123!")

@pytest.mark.admin
def test_admin_platform_settings(admin_page):
    """Verifies that platform settings can be loaded and saved."""
    page, tracker = admin_page
    page.goto(f"{BASE_URL}/admin/settings.html")
    
    # Wait for the general settings to load
    expect(page.locator("input[x-model='settings.platform_name']")).to_be_visible(timeout=10000)
    
    # Check if we can change a setting
    # Just save without making changes to avoid polluting for now
    page.click("button:has-text('Save General Settings')")
    
    # Wait for the success toast or button to not say 'Saving...'
    toast = page.locator("div[x-show='toast.show']")
    expect(toast).to_be_visible(timeout=5000)
    expect(toast).to_contain_text("Settings saved successfully")

@pytest.mark.admin
def test_admin_legal_settings(admin_page):
    """Verifies that legal settings tab works and versions can be updated."""
    page, tracker = admin_page
    page.goto(f"{BASE_URL}/admin/settings.html")
    
    # Click Legal & Compliance tab
    page.click("button.admin-tab:has-text('Legal & Compliance')")
    
    # Wait for the input to become visible
    terms_input = page.locator("input[x-model='legalForm.legal_terms_version']")
    expect(terms_input).to_be_visible(timeout=10000)
    
    # Save the versions
    page.click("button:has-text('Save Legal Versions')")
    
    # Wait for toast
    toast = page.locator("div[x-show='toast.show']")
    expect(toast).to_be_visible(timeout=5000)
    expect(toast).to_contain_text("Legal versions updated")
