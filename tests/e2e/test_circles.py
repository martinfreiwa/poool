import re
from playwright.sync_api import expect
import os

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")

def test_circles_list(authenticated_user_page):
    page, current_user = authenticated_user_page
    
    # Go to circles page
    page.goto(f"{BASE_URL}/community/circles")
    page.wait_for_load_state("networkidle")
    
    # Assert header and typical circle elements
    expect(page.locator("h1").first).to_contain_text("Circles", ignore_case=True)
    
    # If the user has a "Discover" tab, ensure it loads
    discover_tab = page.locator("button", has_text=re.compile("Discover", re.IGNORECASE))
    if discover_tab.is_visible():
        discover_tab.click()
        
    page.wait_for_load_state("networkidle")
    
    # We should see empty state or at least one circle card
    circle_cards = page.locator(".circle-card")
    empty_state = page.locator(".empty-state")
    
    # Wait for either to appear
    if circle_cards.count() > 0:
        expect(circle_cards.first).to_be_visible()
    else:
        expect(empty_state.first).to_be_visible()

def test_create_circle_modal(authenticated_user_page):
    page, current_user = authenticated_user_page
    
    page.goto(f"{BASE_URL}/community/circles")
    page.wait_for_load_state("networkidle")
    
    create_btn = page.locator("button#btn-create-circle, button.btn-create-circle").first
    if create_btn.is_visible():
        create_btn.click()
        # Verify the modal opens
        expect(page.locator("#create-circle-modal, .create-circle-modal").first).to_be_visible(timeout=5000)
        
        # Verify input fields
        expect(page.locator("input[name='name']").first).to_be_visible()
        expect(page.locator("textarea[name='description']").first).to_be_visible()
        
        # Test creation interaction
        page.fill("input[name='name']", "E2E Playwright Circle")
        page.fill("textarea[name='description']", "This is an automated E2E test circle.")
        
        # The form should be submittable (we won't actually trigger it if we want to avoid polluting the DB, but it's E2E so we should)
        submit_btn = page.locator("button[type='submit']", has_text="Create").first
        submit_btn.click()
        
        page.wait_for_load_state("networkidle")
        # Should close the modal or show success
        expect(page.locator("#create-circle-modal, .create-circle-modal").first).not_to_be_visible()
