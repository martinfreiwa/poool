import pytest
import os
import time
from playwright.sync_api import sync_playwright, expect

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
ADMIN_EMAIL = os.environ.get("ADMIN_EMAIL", "test@poool.app")
ADMIN_PASSWORD = os.environ.get("ADMIN_PASSWORD", "TestPass123!")

@pytest.fixture(scope="session")
def admin_page():
    """Returns an authenticated admin page."""
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(viewport={'width': 1280, 'height': 800})
        page = context.new_page()
        page.on("console", lambda msg: print(f"CONSOLE: {msg.text}"))
        
        # Login
        page.goto(f"{BASE_URL}/auth/login")
        page.fill("#email-input", ADMIN_EMAIL)
        page.fill("#password-input", ADMIN_PASSWORD)
        page.click("#login-button")
        
        # Wait for redirect
        page.wait_for_function("window.location.pathname !== '/auth/login'", timeout=10000)
        
        yield page
        browser.close()

def test_admin_platform_settings(admin_page):
    """Verifies that platform settings can be loaded and saved."""
    admin_page.goto(f"{BASE_URL}/admin/settings.html")
    
    # Wait for the general settings to load
    expect(admin_page.locator("input[x-model='settings.platform_name']")).to_be_visible(timeout=10000)
    
    # Check if we can change a setting
    # Just save without making changes to avoid polluting for now
    admin_page.click("button:has-text('Save General Settings')")
    
    # Wait for the success toast or button to not say 'Saving...'
    toast = admin_page.locator("div[x-show='toast.show']")
    expect(toast).to_be_visible(timeout=5000)
    expect(toast).to_contain_text("Settings saved successfully")

def test_admin_legal_settings(admin_page):
    """Verifies that legal settings tab works and versions can be updated."""
    admin_page.goto(f"{BASE_URL}/admin/settings.html")
    
    # Click Legal & Compliance tab
    admin_page.click("button.admin-tab:has-text('Legal & Compliance')")
    
    # Wait for the input to become visible
    terms_input = admin_page.locator("input[x-model='legalForm.legal_terms_version']")
    expect(terms_input).to_be_visible(timeout=10000)
    
    # Save the versions
    admin_page.click("button:has-text('Save Legal Versions')")
    
    # Wait for toast
    toast = admin_page.locator("div[x-show='toast.show']")
    expect(toast).to_be_visible(timeout=5000)
    expect(toast).to_contain_text("Legal versions updated")
