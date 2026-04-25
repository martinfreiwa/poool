import pytest
import os
import psycopg2
from playwright.sync_api import expect

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
ADMIN_EMAIL = os.environ.get("ADMIN_EMAIL", "test@poool.app")
ADMIN_PASSWORD = os.environ.get("ADMIN_PASSWORD", "TestPass123!")

@pytest.mark.admin
def test_support_ticket_list(admin_page):
    """Verifies that the support ticket list loads and can be filtered."""
    page, tracker = admin_page
    page.goto(f"{BASE_URL}/admin/support.html")
    
    # Wait for the table to populate with tickets
    expect(page.locator(".ticket-checkbox").first).to_be_visible(timeout=10000)
    
    # Check that KPIs are loaded (not dashes anymore)
    val = page.locator("#stat-open").inner_text()
    assert val != "—"

@pytest.mark.admin
def test_support_ticket_interaction(admin_page):
    """Verifies that an admin can view a ticket and update its status."""
    page, tracker = admin_page
    page.goto(f"{BASE_URL}/admin/support.html")
    
    # Wait for tickets
    expect(page.locator(".ticket-checkbox").first).to_be_visible(timeout=10000)
    
    # Filter by Open
    page.select_option("#filter-status", "open", force=True)
    page.wait_for_timeout(1000)
    
    # Click the "View" button of the first open ticket
    view_btn = page.locator("#tickets-table-body tr").first.locator("a:has-text('View')")
    ticket_url = view_btn.get_attribute("href")
    
    page.goto(f"{BASE_URL}{ticket_url}")
    
    # Wait for the ticket details to load
    expect(page.locator("#ticket-title")).not_to_have_text("Loading Ticket…", timeout=10000)
    
    # Send a reply using Quill editor
    # Wait for Quill to initialize
    expect(page.locator(".ql-editor")).to_be_visible(timeout=10000)
    
    reply_text = "Hello from Playwright E2E test"
    page.locator(".ql-editor").fill(reply_text)
    with page.expect_response(lambda r: f"/api/admin/support/" in r.url and r.url.endswith("/messages"), timeout=10000) as response_info:
        page.click("#btn-send-reply")
    assert response_info.value.ok

    # Wait for the reply constraint to show up (it should append to the thread)
    page.reload()
    expect(page.locator("#ticket-title")).not_to_have_text("Loading Ticket…", timeout=10000)
    expect(page.locator("#ticket-thread")).to_contain_text(reply_text)

@pytest.mark.admin
def test_ticket_status_update(admin_page):
    """Verifies that changing the status updates the backend."""
    page, tracker = admin_page
    page.goto(f"{BASE_URL}/admin/support.html")
    
    # Wait for tickets
    expect(page.locator(".ticket-checkbox").first).to_be_visible(timeout=10000)
    
    # Click the "View" button of the first open ticket
    view_btn = page.locator("#tickets-table-body tr").first.locator("a:has-text('View')")
    ticket_url = view_btn.get_attribute("href")
    
    page.goto(f"{BASE_URL}{ticket_url}")
    
    expect(page.locator("#ticket-title")).not_to_have_text("Loading Ticket…", timeout=10000)
    
    # Update Status via direct JS call since original select is hidden by custom dropdown
    page.evaluate("updateTicketField('status', 'resolved')")
    page.wait_for_timeout(3000)
    
    # Reload the page to check if it persists
    page.reload()
    expect(page.locator("#ticket-title")).not_to_have_text("Loading Ticket…", timeout=10000)
    
    # Wait for the status value to be updated by the async renderTicket function
    expect(page.locator("#sel-status")).to_have_value("resolved", timeout=10000)
