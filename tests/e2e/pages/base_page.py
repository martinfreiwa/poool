"""
Page Object Model (POM) — Base Page
=====================================
All page objects inherit from BasePage.
This enforces consistent quality checks across every page.
"""

from playwright.sync_api import Page, expect
import os

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")


class BasePage:
    """Base page object with built-in quality assertions."""

    # Override in subclasses
    PATH = "/"
    TITLE_CONTAINS = ""

    def __init__(self, page: Page, tracker=None):
        self.page = page
        self.tracker = tracker

    @property
    def url(self):
        return f"{BASE_URL}{self.PATH}"

    def navigate(self):
        """Navigate to this page and verify basic load."""
        if self.tracker:
            self.tracker.navigate_and_check(self.url)
        else:
            self.page.goto(self.url, wait_until="domcontentloaded")
        return self

    def verify_loaded(self):
        """Verify the page loaded correctly."""
        expect(self.page.locator("body")).to_be_visible()
        if self.TITLE_CONTAINS:
            import re
            expect(self.page).to_have_title(
                re.compile(self.TITLE_CONTAINS, re.I)
            )
        return self

    def take_screenshot(self, name: str):
        """Save a named screenshot."""
        from conftest import take_named_screenshot
        return take_named_screenshot(self.page, name)

    def full_health_check(self):
        """Run all quality checks on current page state."""
        if self.tracker:
            self.tracker.full_health_check()
        return self
