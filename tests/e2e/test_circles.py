import re
from playwright.sync_api import expect
import os

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")


def test_circles_list(authenticated_user_page):
    page, tracker, current_user = authenticated_user_page

    # Community is a single page with tabs — navigate there first
    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    # Click the "My Circle" tab
    circle_tab = page.locator(
        "button.community-tab-btn", has_text=re.compile(r"My Circle", re.IGNORECASE)
    )
    expect(circle_tab).to_be_visible(timeout=10000)
    circle_tab.click()
    page.wait_for_timeout(500)

    # Assert the circle tab panel is visible
    circle_panel = page.locator("#community-circle-tab")
    expect(circle_panel).to_be_visible(timeout=5000)

    # We should see either circle content or empty state
    tracker.assert_no_critical_errors()


def test_create_circle_modal(authenticated_user_page):
    page, tracker, current_user = authenticated_user_page

    page.goto(f"{BASE_URL}/community")
    page.wait_for_load_state("networkidle")

    # Click the "My Circle" tab
    circle_tab = page.locator(
        "button.community-tab-btn", has_text=re.compile(r"My Circle", re.IGNORECASE)
    )
    expect(circle_tab).to_be_visible(timeout=10000)
    circle_tab.click()
    page.wait_for_timeout(500)

    create_btn = page.locator(
        "button#btn-create-circle, button.btn-create-circle"
    ).first
    if create_btn.is_visible():
        create_btn.click()
        # Verify the modal opens
        expect(
            page.locator("#create-circle-modal, .create-circle-modal").first
        ).to_be_visible(timeout=5000)

        # Verify input fields
        expect(page.locator("input[name='name']").first).to_be_visible()
        expect(page.locator("textarea[name='description']").first).to_be_visible()

        # Test creation interaction
        page.fill("input[name='name']", "E2E Playwright Circle")
        page.fill(
            "textarea[name='description']", "This is an automated E2E test circle."
        )

        # The form should be submittable
        submit_btn = page.locator("button[type='submit']", has_text="Create").first
        submit_btn.click()

        page.wait_for_load_state("networkidle")
        # Should close the modal or show success
        expect(
            page.locator("#create-circle-modal, .create-circle-modal").first
        ).not_to_be_visible()
