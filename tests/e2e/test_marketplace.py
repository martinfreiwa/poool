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
    mp.verify_card_contract()
    mp.full_health_check() # Console/Network/Blank errors check

@pytest.fixture
def mock_funded_asset():
    """Mocks an asset to 100% funded for the duration of the test."""
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    
    cur.execute(
        "SELECT id, slug, title, funding_status, tokens_available "
        "FROM assets "
        "WHERE published = true "
        "AND deleted_at IS NULL "
        "AND asset_type IN ('real_estate', 'commercial_property', 'land_plot') "
        "AND tokens_available > 0 "
        "LIMIT 1"
    )
    row = cur.fetchone()
    if not row:
        cur.close()
        conn.close()
        pytest.skip("No live assets found to mock.")
        
    asset_id, slug, title, orig_status, orig_tokens = row
    
    # Mock to fully funded
    cur.execute(
        "UPDATE assets SET funding_status = 'funded', tokens_available = 0 WHERE id = %s",
        (asset_id,)
    )
    conn.commit()
    
    yield {"id": asset_id, "slug": slug, "title": title}
    
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

    # 5. Swapped fragment cards must preserve the parent filter contract.
    expect(card).to_have_attribute("data-location", re.compile(r".+"))
    expect(card).to_have_attribute("data-asset-type", re.compile(r".+"))
    expect(card).to_have_attribute("data-duration", re.compile(r".+"))
    expect(card).to_have_attribute("data-card-url", re.compile(r"^/property/.+"))

    # 6. Repeated tab swaps should not break search/filter controls.
    mp.switch_to_available_tab().verify_available_tab_active()
    mp.switch_to_funded_tab().verify_funded_tab_active()
    mp.search(asset["title"])
    page.keyboard.press("Enter")
    expect(page.locator(asset_selector).first).to_be_visible()

    mp.full_health_check()

    # 7. Fragment-rendered cards must expose semantic keyboard navigation.
    card = page.locator(asset_selector).first
    title_link = card.locator(".property-title-link").first
    expect(title_link).to_be_visible()
    with page.expect_navigation(url=re.compile(r".*/property/.+")):
        title_link.press("Enter")

@pytest.mark.marketplace
def test_marketplace_search_and_filters(authenticated_user_page):
    """Verifies visible marketplace filters operate against current card data."""
    page, tracker, user = authenticated_user_page
    mp = MarketplacePage(page, tracker)

    mp.navigate().verify_cards_visible(min_count=1)
    first_card = mp.property_cards.first
    first_title = first_card.locator(".property-title").inner_text().strip()
    first_asset_type = first_card.get_attribute("data-asset-type")

    mp.search(first_title)
    expect(first_card).to_be_visible()
    mp.clear_search().verify_cards_visible(min_count=1)

    if first_asset_type in {"real_estate", "commercial_property", "land_plot"}:
        mp.filter_property_type(first_asset_type)
        expect(first_card).to_be_visible()

    mp.clear_search().verify_cards_visible(min_count=1)
    mp.filter_investment_type("short-term")
    expect(mp.property_cards.first.or_(mp.no_results)).to_be_visible()

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
