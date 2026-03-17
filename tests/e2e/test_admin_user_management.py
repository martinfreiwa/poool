import pytest
import os
import psycopg2
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
        
        # Login
        page.goto(f"{BASE_URL}/auth/login")
        page.fill("#email-input", ADMIN_EMAIL)
        page.fill("#password-input", ADMIN_PASSWORD)
        page.click("#login-button")
        
        # Wait for redirect
        page.wait_for_function("window.location.pathname !== '/auth/login'", timeout=10000)
        
        yield page
        browser.close()

@pytest.fixture(scope="session")
def test_user_email():
    """Fetches a real user email from the database for testing search."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    cur.execute("SELECT email FROM users WHERE email != %s LIMIT 1", (ADMIN_EMAIL,))
    email = cur.fetchone()[0]
    cur.close()
    conn.close()
    return email

def test_user_search(admin_page, test_user_email):
    """Verifies that searching for a user by email works."""
    admin_page.goto(f"{BASE_URL}/admin/users.html")
    
    # Wait for table to load by checking for checkboxes (means data is rendered)
    expect(admin_page.locator(".user-checkbox").first).to_be_visible(timeout=10000)
    
    # Input search
    admin_page.fill("#user-search-input", test_user_email)
    
    # Debounce is 300ms, wait a bit
    admin_page.wait_for_timeout(1000)
    
    # Verify the table only shows our user
    rows = admin_page.locator("#users-table-body tr")
    count = rows.count()
    if count > 0:
        # Check if the email is present in the first row
        expect(rows.first).to_contain_text(test_user_email)
    else:
        pytest.fail(f"User with email {test_user_email} not found in search results")

def test_user_status_filter(admin_page):
    """Verifies that filtering by status works."""
    admin_page.goto(f"{BASE_URL}/admin/users.html")
    expect(admin_page.locator(".user-checkbox").first).to_be_visible(timeout=10000)
    
    # Filter by suspended (force=True since the select is hidden by custom poool-dropdown)
    admin_page.select_option("#filter-status", "suspended", force=True)
    admin_page.wait_for_timeout(1000)
    
    # Check if all visible users are suspended
    rows = admin_page.locator("#users-table-body tr")
    for i in range(rows.count()):
        # Check if the row contains "Suspended" badge
        expect(rows.nth(i)).to_contain_text("Suspended")

def test_toggle_user_status(admin_page, test_user_email):
    """Verifies that suspending and activating a user works."""
    admin_page.goto(f"{BASE_URL}/admin/users.html")
    admin_page.fill("#user-search-input", test_user_email)
    admin_page.wait_for_timeout(1000)
    
    # Get initial status
    row = admin_page.locator("#users-table-body tr").first
    status_cell = row.locator("td:nth-child(6)") # Status column
    initial_status = status_cell.inner_text().lower()
    
    # Click toggle button (Action column is last)
    toggle_btn = row.locator("td:nth-child(8) button").last
    
    # Handle the confirmation dialog
    admin_page.once("dialog", lambda dialog: dialog.accept())
    toggle_btn.click()
    
    # Wait for reload
    admin_page.wait_for_timeout(2000)
    
    # Verify status changed
    new_status = status_cell.inner_text().lower()
    assert initial_status != new_status
    
    # Cleanup: Toggle back
    admin_page.once("dialog", lambda dialog: dialog.accept())
    toggle_btn.click()
    admin_page.wait_for_timeout(2000)
    final_status = status_cell.inner_text().lower()
    assert final_status == initial_status
