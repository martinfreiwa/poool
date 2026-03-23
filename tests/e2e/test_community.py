import re
from playwright.sync_api import expect
import os

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")

def test_community_feed_load(authenticated_user_page):
    page, current_user = authenticated_user_page
    
    # Navigate to the community feed
    page.goto(f"{BASE_URL}/community/feed")
    page.wait_for_load_state("networkidle")
    
    # Assert that the main community wrapper is visible
    expect(page.locator(".community-wrapper").first).to_be_visible(timeout=10000)
    
    # Attempt to create a post if the text area exists
    post_textarea = page.locator("textarea[name='content']").first
    if post_textarea.is_visible():
        post_textarea.fill("Hello from Playwright E2E!")
        post_button = page.locator("button[type='submit']").first
        if post_button.is_visible():
            post_button.click()
            # Wait for success UI indication, often a toast or the input clears out
            expect(post_textarea).to_have_value("")

def test_community_announcements(authenticated_user_page):
    page, current_user = authenticated_user_page
    
    # Navigate to the community announcements
    page.goto(f"{BASE_URL}/community/announcements")
    page.wait_for_load_state("networkidle")
    
    # Should see the list of announcements
    expect(page.locator("h1")).to_contain_text("Announcements", ignore_case=True)
