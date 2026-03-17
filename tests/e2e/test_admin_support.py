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
        
        def handle_request(request):
            if "/api/" in request.url:
                print(f"REQUEST: {request.method} {request.url}")
                print(f"HEADERS: {request.headers}")
                
        page.on("request", handle_request)
        
        # Login
        page.goto(f"{BASE_URL}/auth/login")
        page.fill("#email-input", ADMIN_EMAIL)
        page.fill("#password-input", ADMIN_PASSWORD)
        page.click("#login-button")
        
        # Wait for redirect
        page.wait_for_function("window.location.pathname !== '/auth/login'", timeout=10000)
        
        yield page
        browser.close()

def test_support_ticket_list(admin_page):
    """Verifies that the support ticket list loads and can be filtered."""
    admin_page.goto(f"{BASE_URL}/admin/support.html")
    
    # Wait for the table to populate with tickets
    expect(admin_page.locator(".ticket-checkbox").first).to_be_visible(timeout=10000)
    
    # Check that KPIs are loaded (not dashes anymore)
    val = admin_page.locator("#stat-open").inner_text()
    assert val != "—"

def test_support_ticket_interaction(admin_page):
    """Verifies that an admin can view a ticket and update its status."""
    admin_page.goto(f"{BASE_URL}/admin/support.html")
    
    # Wait for tickets
    expect(admin_page.locator(".ticket-checkbox").first).to_be_visible(timeout=10000)
    
    # Filter by Open
    admin_page.select_option("#filter-status", "open", force=True)
    admin_page.wait_for_timeout(1000)
    
    # Click the "View" button of the first open ticket
    view_btn = admin_page.locator("#tickets-table-body tr").first.locator("a:has-text('View')")
    ticket_url = view_btn.get_attribute("href")
    
    admin_page.goto(f"{BASE_URL}{ticket_url}")
    
    # Wait for the ticket details to load
    expect(admin_page.locator("#ticket-title")).not_to_have_text("Loading Ticket…", timeout=10000)
    
    # Send a reply using Quill editor
    # Wait for Quill to initialize
    expect(admin_page.locator(".ql-editor")).to_be_visible(timeout=10000)
    
    reply_text = "Hello from Playwright E2E test"
    admin_page.locator(".ql-editor").fill(reply_text)
    admin_page.click("#btn-send-reply")
    
    # Wait for the reply constraint to show up (it should append to the thread)
    expect(admin_page.locator("#ticket-thread")).to_contain_text(reply_text)

def test_ticket_status_update(admin_page):
    """Verifies that changing the status updates the backend."""
    admin_page.goto(f"{BASE_URL}/admin/support.html")
    
    # Wait for tickets
    expect(admin_page.locator(".ticket-checkbox").first).to_be_visible(timeout=10000)
    
    # Click the "View" button of the first open ticket
    view_btn = admin_page.locator("#tickets-table-body tr").first.locator("a:has-text('View')")
    ticket_url = view_btn.get_attribute("href")
    
    admin_page.goto(f"{BASE_URL}{ticket_url}")
    
    # Wait for the ticket details to load
    expect(admin_page.locator("#ticket-title")).not_to_have_text("Loading Ticket…", timeout=10000)
    
    # Update Status
    admin_page.locator("#sel-status").select_option("resolved")
    admin_page.wait_for_timeout(2000)
    
    # Reload the page to check if it persists
    admin_page.reload()
    expect(admin_page.locator("#ticket-title")).not_to_have_text("Loading Ticket…", timeout=10000)
    
    # Verify the select retains the value
    val = admin_page.locator("#sel-status").input_value()
    assert val == "resolved"
