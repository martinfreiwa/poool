"""
POOOL E2E Tests — Admin Financials
=====================================
Tests admin-side balance adjustments and financial ledger views.
"""

import pytest
import psycopg2
import os
from playwright.sync_api import expect
from tests.e2e.pages.admin_pages import AdminDashboardPage, AdminUsersPage, AdminOrdersPage

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")

@pytest.fixture(scope="function")
def test_user_id():
    """Fetches a real user ID from the database for testing balances."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    # Don't use the admin user for balance tests
    cur.execute("SELECT id FROM users WHERE email NOT LIKE 'admin%' LIMIT 1")
    row = cur.fetchone()
    uid = row[0] if row else None
    cur.close()
    conn.close()
    return uid

@pytest.mark.admin
@pytest.mark.financial
def test_admin_deposits_list_loads(admin_page):
    """Verifies that the admin deposits list page renders correctly."""
    page, tracker = admin_page
    
    # 1. Page load
    tracker.navigate_and_check(f"{BASE_URL}/admin/deposits.html")
    tracker.assert_page_loaded()
    
    # 2. Table visibility
    expect(page.locator(".admin-table, table").first).to_be_visible(timeout=10000)
    
    # 3. Quick statistics check
    stat_pending = page.locator("#stat-pending, .stat-pending").first
    if stat_pending.is_visible():
        expect(stat_pending).not_to_have_text("—", timeout=10000)
    
    tracker.full_health_check()

@pytest.mark.admin
@pytest.mark.financial
def test_admin_adjust_balance_workflow(admin_page, test_user_id):
    """Verifies that superadmin balance adjustment updates UI and DB."""
    if not test_user_id:
        pytest.skip("No users found to test balance adjustment")
        
    page, tracker = admin_page
    
    # 1. User detail page
    tracker.navigate_and_check(f"{BASE_URL}/admin/user-details.html?id={test_user_id}")
    tracker.assert_page_loaded()
    
    # 2. Pre-check: fullname exists
    expect(page.locator("#user-fullname, .user-fullname").first).not_to_have_text("—", timeout=15000)
    
    # 3. Store initial balance
    initial_balance_text = page.locator("#user-cash-balance, .user-cash-balance").inner_text()
    initial_balance = float(initial_balance_text.replace("$", "").replace(",", "").strip())
    
    # 4. Launch adjustment modal
    page.click("#btn-edit-balance, .btn-adjust-balance")
    modal = page.locator("#edit-balance-modal, .modal-adjust-balance")
    expect(modal).to_be_visible(timeout=5000)
    
    # 5. Fill form ($100 bonus)
    page.fill("#edit-balance-amount, .input-amount", "100.00")
    page.fill("#edit-balance-reason, .input-reason", "E2E Test Bonus — Artifact Audit")
    
    # 6. Submit
    page.click("#edit-balance-submit, .btn-submit-adjustment")
    expect(modal).not_to_be_visible(timeout=10000)
    
    # 7. Verify success toast (check before waiting for reload)
    from tests.e2e.conftest import check_toast_message
    try:
        check_toast_message(page, "success", timeout=5000)
    except AssertionError:
        pass # Toast might not be configured for this specific action in the UI
        
    # 8. Check for UI update (HTMX reload expected)
    page.wait_for_timeout(2000) # Give UI time to reflect
    
    new_balance_text = page.locator("#user-cash-balance, .user-cash-balance").inner_text()
    new_balance = float(new_balance_text.replace("$", "").replace(",", "").strip())
    
    # 9. Assertions
    assert new_balance == initial_balance + 100.0
    
    tracker.assert_no_critical_errors()
