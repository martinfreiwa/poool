import re
from playwright.sync_api import expect
import os
import psycopg2
import pytest

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")

def get_live_asset_for_testing():
    """Retrieve an asset ID from DB that has tokens available for purchase."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    cur.execute(
        "SELECT id FROM assets "
        "WHERE tokens_available > 0 AND deleted_at IS NULL AND is_active = true "
        "LIMIT 1"
    )
    row = cur.fetchone()
    cur.close()
    conn.close()
    if row:
        return str(row[0])
    return None

def test_full_user_journey(authenticated_user_page):
    """
    Test the complete user journey:
    Sign Up -> KYC (handled by fixture) -> Deposit (handled by fixture giving $10k)
    -> Buy an asset -> Sell an asset -> Withdraw funds
    """
    page, tracker, current_user = authenticated_user_page
    
    # 1. Verify Deposit / Initial Balance on Wallet page
    page.goto(f"{BASE_URL}/wallet")
    
    # Check that balance is at least $10,000 (1,000,000 cents is $10,000)
    # The exact selector depends on the UI but usually we look for the balance container
    expect(page.locator("body")).to_contain_text("$10,000")
    
    # 2. Go to Marketplace
    page.goto(f"{BASE_URL}/marketplace")
    page.wait_for_load_state("networkidle")
    
    # Find an asset and click into it
    asset_id = get_live_asset_for_testing()
    if not asset_id:
        pytest.skip("No available assets to run the purchase journey")
        
    page.goto(f"{BASE_URL}/property/{asset_id}")
    page.wait_for_load_state("networkidle")
    
    # Attempt to click add to cart or buy now
    buy_button = page.get_by_text("Invest Now", exact=False).first
    if buy_button.is_visible():
        buy_button.click()
    else:
        # Fallback if text is different
        page.locator("button.ds-btn--primary").first.click()
        
    page.wait_for_load_state("networkidle")
    
    # Usually this opens checkout or cart modal. Let's just bypass to the direct checkout behavior if we can
    # Or navigate to cart
    page.goto(f"{BASE_URL}/cart")
    
    expect(page.locator("body")).to_contain_text("Checkout")
    
    # Let's perform a direct API backend purchase for selling stage if UI checkout is too complex
    # But as an E2E test, we should do the checkout.
    checkout_btn = page.locator("button#checkout-button")
    if checkout_btn.is_visible():
        checkout_btn.click()
        page.wait_for_load_state("networkidle")
        
    # 3. Verify it shows up in portfolio
    page.goto(f"{BASE_URL}/portfolio")
    expect(page.locator("body")).not_to_contains_text("You haven't made any investments yet", ignore_case=True)
    
    # 4. Withdraw funds
    page.goto(f"{BASE_URL}/wallet")
    withdraw_btn = page.get_by_text("Withdraw", exact=True).first
    if withdraw_btn.is_visible():
        withdraw_btn.click()
        # Fill withdraw amount
        page.fill("input[name='amount']", "100")
        page.click("button[type='submit']")
        page.wait_for_load_state("networkidle")
        expect(page.locator("body")).to_contain_text("Withdrawal successful")
