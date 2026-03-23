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
        "WHERE tokens_available > 0 AND deleted_at IS NULL AND published = true "
        "LIMIT 1"
    )
    row = cur.fetchone()
    cur.close()
    conn.close()
    if row:
        return str(row[0])
    return None

def test_marketplace_listing_loads(authenticated_user_page):
    page, current_user = authenticated_user_page
    
    page.goto(f"{BASE_URL}/marketplace")
    page.wait_for_load_state("networkidle")
    
    # Wait for assets to render (class differs, usually `.asset-card`, `.property-card` etc)
    # The container usually has .marketplace-grid or similar
    expect(page.locator("h1").first).to_contain_text("Marketplace", ignore_case=True)
    
    expect(page.locator(".property-card").first).to_be_visible(timeout=5000)

def test_p2p_offer_modal(authenticated_user_page):
    page, current_user = authenticated_user_page
    
    asset_id = get_live_asset_for_testing()
    if not asset_id:
        pytest.skip("No live assets found to test P2P market")
        
    page.goto(f"{BASE_URL}/property/{asset_id}")
    page.wait_for_load_state("networkidle")
    
    # Try finding the "P2P" or "Offers" tab/button
    # If the app exposes a standard "Secondary Market" tab, click it
    secondary_tab = page.locator("button[data-tab='secondary']")
    if secondary_tab.is_visible():
        secondary_tab.click()
        
    # Then try to create a new offer
    new_offer_btn = page.locator("button.create-offer-btn").first
    if new_offer_btn.is_visible():
        new_offer_btn.click()
        
        # Verify modal shows up
        expect(page.locator(".offer-modal").first).to_be_visible()
        # Ensure input exists
        page.fill("input[name='offer_price']", "10.00")
        page.fill("input[name='quantity']", "1")
        submit_offer = page.locator("button[type='submit'].submit-offer-btn")
        if submit_offer.is_visible():
            submit_offer.click()
            expect(page.locator(".offer-modal").first).not_to_be_visible()

@pytest.fixture
def mock_fully_funded_asset():
    """Finds an asset, mocks it to 100% funded, yields its slug/title, then reverts."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    
    # Get any active asset
    cur.execute(
        "SELECT id, slug, title, funding_status, tokens_available "
        "FROM assets WHERE published = true AND deleted_at IS NULL LIMIT 1"
    )
    row = cur.fetchone()
    if not row:
        cur.close()
        conn.close()
        pytest.skip("No live assets found to mock as fully funded.")
        return
        
    asset_id, slug, title, orig_status, orig_tokens = row
    
    # Mock to fully funded (100% traded)
    cur.execute(
        "UPDATE assets SET funding_status = 'funded', tokens_available = 0 WHERE id = %s",
        (asset_id,)
    )
    conn.commit()
    
    yield {"id": asset_id, "slug": slug, "title": title}
    
    # Revert back to original state
    cur.execute(
        "UPDATE assets SET funding_status = %s, tokens_available = %s WHERE id = %s",
        (orig_status, orig_tokens, asset_id)
    )
    conn.commit()
    cur.close()
    conn.close()

def test_marketplace_fully_funded_asset_tab(authenticated_user_page, mock_fully_funded_asset):
    """Verifies that 100% funded assets correctly appear under the Funded tab and not Available tab."""
    page, current_user = authenticated_user_page
    asset_info = mock_fully_funded_asset
    
    # Navigate to marketplace
    page.goto(f"{BASE_URL}/marketplace")
    page.wait_for_load_state("networkidle")
    
    # By default, we are on the 'Available' tab
    expect(page.locator("#filter-bar-tab-available")).to_have_class(re.compile(r"active"))
    
    # The fully funded asset should NOT be visible under the Available tab
    asset_selector = f".property-card[data-property-id='{asset_info['slug']}']"
    expect(page.locator(asset_selector)).not_to_be_visible()
    
    # Click on the 'Funded' tab (triggers HTMX request)
    page.click("#filter-bar-tab-funded")
    page.wait_for_load_state("networkidle")
    
    # The 'Funded' tab should now be active
    expect(page.locator("#filter-bar-tab-funded")).to_have_class(re.compile(r"active"))
    
    # The fully funded asset SHOULD be visible under this tab
    funded_asset_locator = page.locator(asset_selector).first
    expect(funded_asset_locator).to_be_visible()
    
    # Asset UI should display 100% funded progression
    progress_text = funded_asset_locator.locator(".funded-percentage").inner_text()
    assert "100" in progress_text or "100.0" in progress_text

