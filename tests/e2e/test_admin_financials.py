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

@pytest.fixture(scope="session")
def test_user_id():
    """Fetches a real user ID from the database for testing balances."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    cur.execute("SELECT id FROM users WHERE email != %s LIMIT 1", (ADMIN_EMAIL,))
    uid = cur.fetchone()[0]
    cur.close()
    conn.close()
    return uid

def test_admin_deposits_list(admin_page):
    """Verifies that the deposits list loads successfully."""
    admin_page.goto(f"{BASE_URL}/admin/deposits.html")
    
    # Wait for the table to load at least one row or empty state
    # Checking for .admin-table or rows
    expect(admin_page.locator(".admin-table").first).to_be_visible(timeout=10000)
    
    # The statistics should load
    expect(admin_page.locator("#stat-pending")).not_to_have_text("—", timeout=10000)

def test_admin_adjust_balance(admin_page, test_user_id):
    """Verifies that a superadmin can adjust a user's balance."""
    admin_page.goto(f"{BASE_URL}/admin/user-details.html?id={test_user_id}")
    
    # Wait for user info to load
    expect(admin_page.locator("#user-fullname")).not_to_have_text("—", timeout=10000)
    
    # Click on Adjust Balance button
    admin_page.click("#btn-edit-balance")
    
    # Wait for modal to open
    expect(admin_page.locator("#edit-balance-modal")).to_be_visible(timeout=5000)
    
    # Fill in the form
    admin_page.fill("#edit-balance-amount", "100.00")
    admin_page.fill("#edit-balance-reason", "E2E Test Bonus")
    
    # Get current balance to compare
    initial_balance_text = admin_page.locator("#user-cash-balance").inner_text().replace("$", "").replace(",", "")
    initial_balance = float(initial_balance_text)
    
    # Save the adjustment
    admin_page.click("#edit-balance-submit")
    
    # Wait for modal to close (hidden)
    expect(admin_page.locator("#edit-balance-modal")).not_to_be_visible(timeout=10000)
    
    # Verify the balance increased on the screen
    admin_page.wait_for_timeout(2000) # Give time for UI update
    
    # The UI should update the balance. Wait for networkidle
    new_balance_text = admin_page.locator("#user-cash-balance").inner_text().replace("$", "").replace(",", "")
    new_balance = float(new_balance_text)
    
    # Assert
    assert new_balance == initial_balance + 100.0
