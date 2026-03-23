import pytest
import os
import psycopg2
from playwright.sync_api import expect

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
ADMIN_EMAIL = os.environ.get("ADMIN_EMAIL", "test@poool.app")
ADMIN_PASSWORD = os.environ.get("ADMIN_PASSWORD", "TestPass123!")

@pytest.fixture(scope="function")
def user_id():
    """Fetches a real user ID from the database for testing."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    cur.execute("SELECT id FROM users LIMIT 1")
    uid = cur.fetchone()[0]
    cur.close()
    conn.close()
    return uid

@pytest.mark.admin
def test_admin_full_workflow(admin_page, user_id):
    """
    Verifies the full admin workflow:
    1. Login as admin (done via fixture)
    2. Navigate to User Details
    3. Verify Sidebar and UI components are rendered (JS execution)
    4. Test Profile Edit Modal layout (No overlaps)
    5. Perform successful profile update
    """
    page, tracker = admin_page
    
    # 2. Go to User Details (Already authenticated by fixture)
    page.goto(f"{BASE_URL}/admin/user-details.html?id={user_id}")
    
    # Verify User Content and JS-rendered Sidebar
    expect(page.locator("#user-content")).to_be_visible(timeout=10000)
    expect(page.locator(".admin-sidebar")).to_be_visible()

    # 3. Test Profile Modal UI Layout
    page.click("#btn-edit-profile")
    expect(page.locator("#edit-profile-modal")).to_be_visible()
    
    # Layout verification: Check save button visibility and input alignment
    save_btn = page.locator("#edit-profile-submit")
    expect(save_btn).to_be_visible()
    
    fn_box = page.locator("#edit-first-name").bounding_box()
    ln_box = page.locator("#edit-last-name").bounding_box()
    
    # Ensure they are not strictly overlapping (same top-left)
    assert fn_box['x'] != ln_box['x'] or fn_box['y'] != ln_box['y'], "Input fields are overlapping!"
    
    # 4. Perform an actual update
    new_first = "Playwright"
    new_last = "E2E"
    page.fill("#edit-first-name", new_first)
    page.fill("#edit-last-name", new_last)
    page.click("#edit-profile-submit")
    
    # Modal should close and UI update
    expect(page.locator("#edit-profile-modal")).not_to_be_visible()
    expect(page.locator("#user-fullname")).to_contain_text(f"{new_first} {new_last}")

    page.close()
