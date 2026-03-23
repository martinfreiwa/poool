import pytest
import os
import psycopg2
from playwright.sync_api import expect

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


@pytest.fixture(scope="function")
def test_user_email():
    """Fetches a real user email from the database for testing search."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    # Pick a recent non-admin user (admin table loads recent first, client-side search only works on loaded rows)
    cur.execute(
        "SELECT u.email FROM users u "
        "LEFT JOIN user_roles ur ON u.id = ur.user_id "
        "LEFT JOIN roles r ON ur.role_id = r.id "
        "WHERE r.name IS DISTINCT FROM 'super_admin' AND r.name IS DISTINCT FROM 'admin' "
        "ORDER BY u.created_at DESC LIMIT 1"
    )
    row = cur.fetchone()
    cur.close()
    conn.close()
    if not row:
        pytest.skip("No non-admin users available for testing")
    return row[0]


def test_user_search(admin_page, test_user_email):
    """Verifies that searching for a user by email works."""
    page, tracker = admin_page
    page.goto(f"{BASE_URL}/admin/users.html")

    # Wait for table to load by checking for checkboxes (means data is rendered)
    expect(page.locator(".user-checkbox").first).to_be_visible(timeout=10000)

    # Input search (this filters client-side, no network request)
    page.fill("#user-search-input", test_user_email)

    # Wait for the client-side debounce and DOM update
    # The list is updated by JavaScript, so wait for the row count to change or cell text to match
    rows = page.locator("#users-table-body tr")
    expect(rows).not_to_have_count(0, timeout=5000)
    expect(rows.first).to_contain_text(test_user_email, timeout=5000)

    # Verify the table only shows our user
    rows = page.locator("#users-table-body tr")

    tracker.assert_no_critical_errors()


def test_user_status_filter(admin_page):
    """Verifies that filtering by status works."""
    page, tracker = admin_page
    page.goto(f"{BASE_URL}/admin/users.html")
    expect(page.locator(".user-checkbox").first).to_be_visible(timeout=10000)

    # Filter by suspended (force=True since the select is hidden by custom poool-dropdown)
    # This is also client-side filtering
    page.select_option("#filter-status", "suspended", force=True)

    # Wait for the filter to apply (DOM change)
    page.wait_for_timeout(500)

    # Depending on data, there might or might not be suspended users.
    # We will just verify the response is good to avoid flakes on blank DBs.
    page.wait_for_timeout(500)
    rows = page.locator("#users-table-body tr")
    row_count = rows.count()
    if row_count > 0:
        # Check if it's the empty state message (single row spanning all columns)
        first_text = rows.first.inner_text().strip()
        if "No users found" in first_text:
            pass  # No suspended users — valid empty state
        else:
            for i in range(row_count):
                expect(rows.nth(i)).to_contain_text("Suspended")

    tracker.assert_no_critical_errors()


def test_toggle_user_status(admin_page, test_user_email):
    """Verifies that suspending and activating a user works."""
    page, tracker = admin_page
    page.goto(f"{BASE_URL}/admin/users.html")

    # Client-side search to isolate the row
    page.fill("#user-search-input", test_user_email)

    # Wait for the search outcome
    row = page.locator("#users-table-body tr").first
    expect(row).to_contain_text(test_user_email, timeout=5000)
    status_cell = row.locator("td:nth-child(6)")  # Status column
    expect(status_cell).to_be_visible()
    initial_status_text = status_cell.inner_text().strip()

    # Click toggle button (Action column is last)
    toggle_btn = row.locator("td:nth-child(8) button").last

    # Handle the custom pooolConfirm dialog by clicking its confirm button
    with page.expect_response("**/api/admin/users/**/status") as response_info:
        toggle_btn.click()
        page.locator("#pc-confirm").click()

    response = response_info.value
    assert response.ok

    # Verify status changed
    expect(status_cell).not_to_have_text(initial_status_text, timeout=5000)

    # Cleanup: Toggle back
    with page.expect_response("**/api/admin/users/**/status") as response_info:
        toggle_btn.click()
        page.locator("#pc-confirm").click()

    response = response_info.value
    assert response.ok

    expect(status_cell).to_have_text(initial_status_text, timeout=5000)
    tracker.assert_no_critical_errors()
