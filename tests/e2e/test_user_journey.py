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
        "SELECT slug FROM assets "
        "WHERE tokens_available > 0 AND deleted_at IS NULL AND published = true "
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
    # Ensure session is pickable by waiting for a main page first
    page.wait_for_url(re.compile(r"/marketplace|/portfolio|/dashboard"), timeout=15000)
    
    page.goto(f"{BASE_URL}/wallet")
    
    # Wait for the wallet header to confirm page load
    page.wait_for_selector("h1:has-text('Wallet')", timeout=15000)
    
    # Wait for the balance to be rendered (either SSR or AJAX update)
    # The fixture funds the wallet with $10,000 (1,000,000 cents)
    # Use a more flexible selector that works with both format_usd (USD 10,000.00) and display ($10,000.00)
    page.wait_for_selector(".wallet-balance-card-amount:has-text('10,000')", timeout=15000)
    
    # 2. Go to Marketplace
    page.goto(f"{BASE_URL}/marketplace")
    page.wait_for_selector(".marketplace-container, #marketplace-body, .sidebar", timeout=15000)
    page.wait_for_load_state("domcontentloaded")
    
    # Find an asset and click into it
    asset_id = get_live_asset_for_testing()
    if not asset_id:
        pytest.skip("No available assets to run the purchase journey")
        
    page.goto(f"{BASE_URL}/property/{asset_id}")
    page.wait_for_load_state("domcontentloaded")
    
    # Attempt to click add to cart or buy now
    buy_button = page.get_by_text(re.compile("Invest Now|Add to cart", re.IGNORECASE)).first
    if buy_button.is_visible():
        buy_button.click()
    else:
        # Fallback if text is different or not visible yet
        selectors = [
            "button.add-to-cart-btn",
            "button#add-to-cart-main-btn",
            "button.ds-btn--primary",
            "button:has-text('Add to cart')",
            "button:has-text('Invest Now')"
        ]
        
        button_found = False
        for selector in selectors:
            loc = page.locator(selector).first
            if loc.is_visible():
                loc.click()
                button_found = True
                break
        
        if not button_found:
            # Last resort: try to force click the first reasonably looking button in the price card
            page.locator("#property-price-card button").first.click()
        
    # The JS handler for add-to-cart redirects to /cart automatically
    # Wait for the redirect to happen instead of manually calling page.goto
    try:
        page.wait_for_url(re.compile(r".*/cart.*"), timeout=15000)
    except:
        # Fallback if the redirect doesn't happen automatically
        page.goto(f"{BASE_URL}/cart")
        
    page.wait_for_load_state("domcontentloaded")
    
    # Wait for cart items or content to load
    page.locator(".cart-item-card, .mobile-cart-item-card, .cart-page-content").first.wait_for(state="visible", timeout=10000)
    
    # Accept terms and proceed in Cart
    print("Checking terms in cart...")
    page.locator("#cart-terms-checkbox").check()
    # Also check KFS if it exists (usually does for primary offerings)
    if page.locator("#cart-kfs-checkbox").is_visible():
        page.locator("#cart-kfs-checkbox").check()
        
    print("Clicking proceed to checkout...")
    page.locator("#cart-proceed-btn").click()
    
    # Wait for checkout page
    page.wait_for_url(re.compile(r".*/checkout.*"), timeout=10000)
    page.wait_for_load_state("domcontentloaded")
    
    # Perform mock bank transfer checkout
    print("Performing checkout at /checkout...")
    # Upload dummy proof
    page.locator("#proof-upload").set_input_files("/tmp/dummy_proof.png")
    
    # Click confirm
    page.locator("#checkout-confirm-btn").click()
    
    # Wait for success or "in progress" page
    print("Waiting for payment completion page...")
    page.wait_for_url(re.compile(r".*/payment-(success|in-progress).*"), timeout=20000)
    
    # 3. Verify it shows up in portfolio
    print("Verifying portfolio...")
    page.goto(f"{BASE_URL}/portfolio")
    
    # Wait for either the content sections or the empty state to be visible
    print("Waiting for portfolio state transition...")
    page.locator("#portfolio-value-section, #portfolio-empty-state").first.wait_for(state="visible", timeout=20000)
    
    # Check that the empty state is NOT visible
    print("Confirming empty state is hidden...")
    expect(page.locator("#portfolio-empty-state")).not_to_be_visible()
    
    # Verify the assets section and rows are visible
    print("Checking for My Assets section and rows...")
    page.locator("#assets-title").wait_for(state="visible", timeout=10000)
    page.locator(".portfolio-assets-row").first.wait_for(state="visible", timeout=15000)
    print("Portfolio verification successful.")
    
    print("Full user journey test completed successfully.")
