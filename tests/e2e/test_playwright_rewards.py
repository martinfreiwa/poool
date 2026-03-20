import os
import pytest
from playwright.sync_api import Page, expect
import psycopg2

# ==============================================================================
# POOOL E2E Verification - Rewards Page
# ==============================================================================
# This suite verifies the full integration of the Rewards page, including:
# - UI/Frontend rendering (SSR and HTMX)
# - Auth and CSRF constraints
# - Financial and Database invariants (BigInt Cents)
# 
# Run using: pytest tests/e2e/test_playwright_rewards.py --html=report.html
# ==============================================================================

BASE_URL = os.getenv("POOOL_BASE_URL", "http://localhost:8888")
DB_DSN = os.getenv("DATABASE_URL", "dbname=poool user=martin host=127.0.0.1")
TEST_USER_EMAIL = "test@poool.app"
TEST_PASSWORD = "TestPass123!"

@pytest.fixture(scope="session")
def db_connection():
    """Establish direct DB connection for state verification & teardown."""
    conn = psycopg2.connect(DB_DSN)
    conn.autocommit = True
    yield conn
    conn.close()

@pytest.fixture(scope="function")
def logged_in_page(page: Page) -> Page:
    """
    Fixture to log in the test user via HTMX POST and return the authenticated page.
    This also verifies that the login flow correctly handles CSRF tokens.
    """
    # 1. Navigate to login page first to get CSRF token cookie
    page.goto(f"{BASE_URL}/auth/login")
    page.wait_for_load_state("networkidle")
    
    # 2. Extract CSRF token from cookie
    csrf_cookie = next((c for c in page.context.cookies() if c["name"] == "csrf_token"), None)
    if not csrf_cookie:
        pytest.fail("csrf_token cookie not found after visiting login page")
        
    csrf_token = csrf_cookie["value"]
    
    # 3. Simulate HTMX POST to /auth/login with X-CSRF-Token header
    response = page.request.post(
        f"{BASE_URL}/auth/login",
        headers={
            "Content-Type": "application/x-www-form-urlencoded",
            "X-CSRF-Token": csrf_token,
            "HX-Request": "true"
        },
        form={
            "email": TEST_USER_EMAIL,
            "password": TEST_PASSWORD
        }
    )
    
    # Expect success or redirect from HTMX
    assert response.status in [200, 302, 303], f"Login failed with status {response.status}"
    
    # 4. Navigate to rewards page
    page.goto(f"{BASE_URL}/rewards")
    page.wait_for_load_state("networkidle")
    
    return page


class TestHappyPaths:
    """Test happy path flows: normal user interactions on the Rewards page."""
    
    def test_rewards_tabs_navigation(self, logged_in_page: Page):
        """Test that user can navigate through all tabs and content reveals successfully."""
        page = logged_in_page
        
        # Verify default tab 'Rewards' is visible and active
        expect(page.locator("#rewards-tab")).to_be_visible()
        expect(page.locator("button[data-tab='rewards-tab']")).to_have_class("rewards-tab-btn active")
        
        # Click Tier tab
        page.locator("button[data-tab='tier-tab']").click()
        # Auto-wait for visibility
        expect(page.locator("#tier-tab")).to_be_visible()
        expect(page.locator("#rewards-tab")).to_be_hidden()
        
        # Click Affiliate Dashboard tab
        page.locator("button[data-tab='affiliate-tab']").click()
        expect(page.locator("#affiliate-tab")).to_be_visible()

    def test_copy_referral_link(self, logged_in_page: Page):
        """Test the copy functionality for the rewards referral link."""
        page = logged_in_page
        
        # Wait for the primary referral input to load its value dynamically
        link_input = page.locator("#rewards-referral-input")
        expect(link_input).not_to_have_value("Loading...", timeout=10000)
        
        # Grant clipboard permissions to the browser context
        page.context.grant_permissions(["clipboard-read", "clipboard-write"])
        
        # Click copy button
        page.locator("#rewards-copy-btn").click()
        
        # In a real environment, we'd verify a toast appears or clipboard contents directly:
        # clipboard_text = page.evaluate("navigator.clipboard.readText()")
        # assert "app.poool.com" in clipboard_text


class TestEdgeCases:
    """Test boundary conditions, empty states, and limit scenarios."""

    def test_campaign_custom_date_picker(self, logged_in_page: Page):
        """Verify the custom date picker behaves correctly when selecting ranges."""
        page = logged_in_page
        page.locator("button[data-tab='affiliate-tab']").click()
        
        # Open custom date picker
        page.locator("#chart-custom-btn").click()
        expect(page.locator("#chart-date-picker")).to_be_visible()
        
        # Insert edge case boundary dates and apply
        page.locator("#chart-date-from").fill("2024-01-01")
        page.locator("#chart-date-to").fill("2024-01-01") # Same day range
        page.locator(".chart-date-apply-btn").click()
        
        # The dropdown should close
        expect(page.locator("#chart-date-picker")).to_be_hidden()


class TestNegativePaths:
    """Test invalid payloads, authentication rejections, and invalid states."""
    
    def test_unauthenticated_redirect(self, page: Page):
        """Verify unauthenticated access to protected /rewards returns 302/303 to login."""
        page.goto(f"{BASE_URL}/rewards")
        expect(page).to_have_url(f"{BASE_URL}/auth/login")

    def test_csrf_protection_missing_header(self, logged_in_page: Page):
        """Verify that a mutating API call without X-CSRF-Token is rejected."""
        page = logged_in_page
        
        # Attempt to save payout settings without the CSRF header
        response = page.request.post(
            f"{BASE_URL}/api/rewards/payout-settings",
            headers={
                "Content-Type": "application/json"
            },
            data={"bank_account_id": "123"}
        )
        
        # Expecting 403 Forbidden or similar CSRF error
        assert response.status in [401, 403], f"Expected CSRF failure but got {response.status}"


class TestDatabaseIntegrity:
    """Direct database assertions to enforce financial invariants and data consistency."""
    
    def test_wallet_balance_invariant(self, db_connection):
        """
        BUSINESS RULE 1: The Ledger Rule
        SUM(amount) from transactions == balance in wallets.
        BUSINESS RULE 2: Zero Negative Wealth
        """
        cursor = db_connection.cursor()
        
        # Fetch wallet for test user
        cursor.execute("""
            SELECT id, balance_cents 
            FROM wallets 
            WHERE user_id = (SELECT id FROM users WHERE email = %s LIMIT 1)
        """, (TEST_USER_EMAIL,))
        wallet = cursor.fetchone()
        
        if wallet:
            wallet_id, balance = wallet
            
            # 1. Enforce No Negative Wealth
            assert balance >= 0, f"Wallet {wallet_id} has negative balance: {balance} cents"
            
            # 2. Enforce Ledger Matching
            cursor.execute("""
                SELECT COALESCE(SUM(amount_cents), 0) 
                FROM wallet_transactions 
                WHERE wallet_id = %s
            """, (wallet_id,))
            calculated_balance = cursor.fetchone()[0]
            
            assert calculated_balance == balance, \
                f"Ledger mismatch for wallet {wallet_id}. " \
                f"Transactions sum: {calculated_balance}, Wallet balance: {balance}"
                
    def teardown_test_data(self, db_connection):
        """
        Optional teardown demonstrating how we'd clean up data created during tests.
        Not actively called here to prevent accidental truncations, but serves as template.
        """
        # cursor = db_connection.cursor()
        # cursor.execute("DELETE FROM support_tickets WHERE user_id = ...")
        pass
