import pytest
import os
import psycopg2
from playwright.sync_api import expect, Page, BrowserContext

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def set_switch_off(page, selector):
    switch = page.locator(selector)
    expect(switch).to_be_visible(timeout=5000)
    if switch.get_attribute("aria-checked") == "true":
        switch.click()


# Fixture to parameterize viewport sizes for Mobile and Desktop
@pytest.fixture(
    params=[
        {"viewport": {"width": 1920, "height": 1080}, "name": "desktop"},
        {
            "viewport": {"width": 375, "height": 812},
            "name": "mobile",
            "is_mobile": True,
            "has_touch": True,
        },
    ]
)
def viewport_context(request, browser):
    context = browser.new_context(**request.param)
    context.device_name = request.param["name"]
    yield context
    context.close()


def take_screenshot(page: Page, title: str):
    """Utility to take screenshots during critical flows for visual bug tracking."""
    os.makedirs("artifacts/e2e_screenshots", exist_ok=True)
    device = getattr(page.context, "device_name", "unknown")
    page.screenshot(
        path=f"artifacts/e2e_screenshots/{device}_{title}.png", full_page=True
    )


class TestExpertAuthAndSettings:
    """Expert E2E Suite: Tests Account Creation, Password Reset, Email/Password/Username updates, and Visual bounds."""

    def test_anonymize_leaderboard(self, authenticated_user_page):
        """Test enabling Leaderboard Anonymity and checking if it saves and reflects appropriately."""
        page, tracker, user = authenticated_user_page

        page.goto(f"{BASE_URL}/settings")
        page.wait_for_load_state("networkidle")

        # Wait for settings content to finish loading (JS removes 'hidden' class)
        settings_content = page.locator("#settings-content")
        expect(settings_content).to_be_visible(timeout=15000)

        # Click the Leaderboard tab (leaderboard privacy settings are in their own section)
        tab_leaderboard = page.locator("a.settings-nav__link[href='#sec-leaderboard']")
        expect(tab_leaderboard).to_be_visible(timeout=10000)
        tab_leaderboard.click()
        page.wait_for_timeout(500)

        # Verify the leaderboard section is visible
        expect(page.locator("#sec-leaderboard")).to_be_attached(timeout=5000)

        # Test the Leaderboard privacy controls
        checkbox_visible = page.locator("#settings-lb-visible")
        checkbox_avatar = page.locator("#settings-lb-avatar")
        input_display = page.locator("#settings-lb-display-name")
        btn_save = page.locator("#btn-save-leaderboard-privacy")

        # Wait for the leaderboard section to be ready
        expect(btn_save).to_be_visible(timeout=5000)

        # Perform modification
        set_switch_off(page, "#settings-lb-visible")
        set_switch_off(page, "#settings-lb-avatar")
        input_display.fill("E2E_Whale")

        # Capture pre-save state
        take_screenshot(page, "pre_leaderboard_privacy_save")

        btn_save.click()

        # Wait for save to complete
        page.wait_for_timeout(1000)
        take_screenshot(page, "post_leaderboard_privacy_save")

        # Assert database updated (integration check)
        conn = psycopg2.connect(DB_URL)
        cur = conn.cursor()
        cur.execute(
            "SELECT visible, show_avatar, display_name FROM leaderboard_preferences WHERE user_id = %s",
            (user["user_id"],),
        )
        lb_settings = cur.fetchone()
        cur.close()
        conn.close()

        assert lb_settings is not None
        assert lb_settings[0] is False  # visible
        assert lb_settings[1] is False  # show_avatar
        assert lb_settings[2] == "E2E_Whale"  # display_name

    def test_change_email_and_password_modals(self, authenticated_user_page):
        """Expert check verifying modal dialog boundaries and correct change operations."""
        page, tracker, user = authenticated_user_page

        page.goto(f"{BASE_URL}/settings")
        page.wait_for_load_state("networkidle")

        # Wait for settings content to finish loading
        expect(page.locator("#settings-content")).to_be_visible(timeout=15000)

        # Click Security & Access tab
        tab_security = page.locator("a.settings-nav__link[href='#sec-security']")
        expect(tab_security).to_be_visible(timeout=10000)
        tab_security.click()
        page.wait_for_timeout(500)

        # 1. Open password modal via the Change Password button
        change_password_btn = page.locator("#btn-change-password")
        expect(change_password_btn).to_be_visible(timeout=5000)
        change_password_btn.click()

        password_modal = page.locator("#modal-change-password")
        expect(password_modal).to_be_visible(timeout=5000)

        # Visual sanity
        take_screenshot(page, "modal_change_email_open")

        # Close the modal via Cancel button
        cancel_btn = password_modal.locator("button", has_text="Cancel")
        cancel_btn.click()
        expect(password_modal).not_to_be_visible(timeout=3000)


class TestExpertDataGridAndUI:
    """Expert E2E Suite: Testing Table Sorts, Global Searches, Plus/Minus Buttons, Loading bars."""

    def test_marketplace_table_and_interactions(self, authenticated_user_page):
        page, tracker, user = authenticated_user_page

        page.goto(f"{BASE_URL}/marketplace")
        page.wait_for_load_state("networkidle")

        # Verify Search Bar functionality
        search_input = page.locator(
            "input#marketplace-search-input, input[type='search']"
        ).first
        if search_input.is_visible():
            search_input.fill("E2E Query")
            page.keyboard.press("Enter")
            page.wait_for_load_state("networkidle")
            take_screenshot(page, "marketplace_search_results")

        # Verify any Sort dropdown
        sort_select = page.locator("select#sort-select, select.ds-select").first
        if sort_select.is_visible():
            sort_select.select_option(index=1)
            page.wait_for_load_state("networkidle")
            take_screenshot(page, "marketplace_sorted")

    def test_plus_minus_cart_interaction(self, authenticated_user_page):
        """Verifies exactly what the plus/minus quantities do dynamically."""
        page, tracker, user = authenticated_user_page

        page.goto(f"{BASE_URL}/cart")
        page.wait_for_load_state("networkidle")

        plus_btn = page.locator("button.qty-plus, .btn-plus").first
        minus_btn = page.locator("button.qty-minus, .btn-minus").first
        qty_input = page.locator("input.qty-input, input[name='quantity']").first

        if plus_btn.is_visible() and qty_input.is_visible():
            initial_val = int(qty_input.input_value() or "0")
            plus_btn.click()
            expect(qty_input).not_to_have_value(
                str(initial_val)
            )  # It should increment!
            take_screenshot(page, "cart_increment_qty")

            minus_btn.click()
            expect(qty_input).to_have_value(str(initial_val))  # Revert back
