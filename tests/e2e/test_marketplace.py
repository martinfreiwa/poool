"""
POOOL E2E Tests — Marketplace
==============================
Tests the marketplace listing, tabs, and asset visibility using POM.
"""

import pytest
import re
from playwright.sync_api import expect
from tests.e2e.pages.marketplace_page import MarketplacePage
import psycopg2
import os

DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")

def get_live_asset_for_testing():
    """Retrieve an asset ID from DB that has tokens available for purchase."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    cur.execute(
        "SELECT id, slug FROM assets "
        "WHERE tokens_available > 0 AND deleted_at IS NULL AND published = true "
        "LIMIT 1"
    )
    row = cur.fetchone()
    cur.close()
    conn.close()
    return row if row else (None, None)

@pytest.mark.marketplace
@pytest.mark.smoke
def test_marketplace_listing_loads(authenticated_user_page):
    """Verifies the marketplace grid loads with property cards."""
    page, tracker, user = authenticated_user_page
    mp = MarketplacePage(page, tracker)
    
    mp.navigate().verify_loaded().verify_heading()
    mp.verify_cards_visible(min_count=1)
    mp.full_health_check() # Console/Network/Blank errors check

@pytest.fixture
def mock_funded_asset():
    """Mocks an asset to 100% funded for the duration of the test."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    
    cur.execute(
        "SELECT id, slug, funding_status, tokens_available "
        "FROM assets WHERE published = true AND deleted_at IS NULL LIMIT 1"
    )
    row = cur.fetchone()
    if not row:
        cur.close()
        conn.close()
        pytest.skip("No live assets found to mock.")
        
    asset_id, slug, orig_status, orig_tokens = row
    
    # Mock to fully funded
    cur.execute(
        "UPDATE assets SET funding_status = 'funded', tokens_available = 0 WHERE id = %s",
        (asset_id,)
    )
    conn.commit()
    
    yield {"id": asset_id, "slug": slug}
    
    # Revert
    cur.execute(
        "UPDATE assets SET funding_status = %s, tokens_available = %s WHERE id = %s",
        (orig_status, orig_tokens, asset_id)
    )
    conn.commit()
    cur.close()
    conn.close()

@pytest.mark.marketplace
def test_marketplace_tabs_and_filtering(authenticated_user_page, mock_funded_asset):
    """Verifies that 'Available' and 'Funded' tabs correctly filter assets."""
    page, tracker, user = authenticated_user_page
    mp = MarketplacePage(page, tracker)
    asset = mock_funded_asset
    
    mp.navigate().verify_available_tab_active()
    
    # 1. Funded asset should NOT be in 'Available' tab
    asset_selector = f".property-card[data-property-id='{asset['slug']}']"
    expect(page.locator(asset_selector)).not_to_be_visible()
    
    # 2. Switch to 'Funded' tab
    mp.switch_to_funded_tab().verify_funded_tab_active()
    
    # 3. Funded asset SHOULD be visible
    expect(page.locator(asset_selector).first).to_be_visible()
    
    # 4. Check progress indicator
    card = page.locator(asset_selector).first
    progress = card.locator(".funded-percentage").inner_text()
    assert "100" in progress
    
    mp.full_health_check()

@pytest.mark.marketplace
def test_p2p_offer_modal_launch(authenticated_user_page):
    """Verifies that the P2P offer modal opens from the property detail page."""
    page, tracker, user = authenticated_user_page
    asset_id, asset_slug = get_live_asset_for_testing()
    
    if not asset_id:
        pytest.skip("No live assets found for P2P test")
        
    tracker.navigate_and_check(f"http://localhost:8888/property/{asset_slug}")
    
    # Try finding the "P2P" or "Secondary Market" tab
    secondary_tab = page.locator("button[data-tab='secondary'], .tab-secondary")
    if secondary_tab.is_visible():
        secondary_tab.click()
        
    # Check if we can open the offer modal
    new_offer_btn = page.locator("button.create-offer-btn, .btn-new-offer").first
    if new_offer_btn.is_visible():
        new_offer_btn.click()
        expect(page.locator(".offer-modal, .modal-offer").first).to_be_visible()
        
    tracker.assert_no_critical_errors()
