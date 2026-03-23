import re
from playwright.sync_api import expect
import os

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")


def test_community_feed_load(authenticated_user_page):
    page, tracker, current_user = authenticated_user_page

    # Community is a single page with tabs — default tab is Feed
    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    # Assert that the main community page wrapper is visible
    expect(page.locator(".community-page").first).to_be_visible(timeout=10000)

    # Verify the Feed tab is active by default
    feed_tab = page.locator("button.community-tab-btn.active", has_text="Feed")
    expect(feed_tab).to_be_visible(timeout=5000)

    # Attempt to create a post if the text area exists
    post_textarea = page.locator("textarea[name='content']").first
    if post_textarea.is_visible():
        post_textarea.fill("Hello from Playwright E2E!")
        post_button = page.locator("button[type='submit']").first
        if post_button.is_visible():
            post_button.click()
            # Wait for success UI indication
            expect(post_textarea).to_have_value("")

    tracker.assert_no_critical_errors()


def test_community_announcements(authenticated_user_page):
    page, tracker, current_user = authenticated_user_page

    # Navigate to community page
    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    # Click the "Announcements" tab
    announcements_tab = page.locator(
        "button.community-tab-btn", has_text=re.compile(r"Announcements", re.IGNORECASE)
    )
    expect(announcements_tab).to_be_visible(timeout=10000)
    announcements_tab.click()
    page.wait_for_timeout(500)

    # Verify the announcements panel is visible
    announcements_panel = page.locator("#community-announcements-tab")
    expect(announcements_panel).to_be_visible(timeout=5000)

    tracker.assert_no_critical_errors()
