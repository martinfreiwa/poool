"""
Page Object: Marketplace Page
===============================
Encapsulates all locators and actions for /marketplace.
"""

from playwright.sync_api import Page, expect
from tests.e2e.pages.base_page import BasePage


class MarketplacePage(BasePage):
    PATH = "/marketplace"
    TITLE_CONTAINS = "Marketplace"

    # ── Locators ──
    @property
    def heading(self):
        return self.page.locator("h1").first

    @property
    def available_tab(self):
        return self.page.locator("#filter-bar-tab-available")

    @property
    def funded_tab(self):
        return self.page.locator("#filter-bar-tab-funded")

    @property
    def property_cards(self):
        return self.page.locator(".property-card")

    @property
    def search_input(self):
        return self.page.locator("#filter-bar-search-input")

    @property
    def location_filter(self):
        return self.page.locator("#filter-bar-location-select")

    @property
    def investment_filter(self):
        return self.page.locator("#filter-bar-investment-select")

    @property
    def property_type_filter(self):
        return self.page.locator("#filter-bar-property-select")

    @property
    def no_results(self):
        return self.page.locator(".marketplace-no-results")

    # ── Actions ──
    def switch_to_funded_tab(self):
        """Click the Funded tab."""
        self.funded_tab.click()
        return self

    def switch_to_available_tab(self):
        """Click the Available tab."""
        self.available_tab.click()
        return self

    def click_first_asset(self):
        """Click on the first property card."""
        self.property_cards.first.click()
        return self

    def search(self, query: str):
        """Type a search query."""
        self.search_input.fill(query)
        return self

    def clear_search(self):
        self.page.locator("#filter-bar-clear-btn").click()
        return self

    def filter_property_type(self, value: str):
        self.property_type_filter.evaluate(
            "(select, value) => { select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); }",
            value,
        )
        return self

    def filter_investment_type(self, value: str):
        self.investment_filter.evaluate(
            "(select, value) => { select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); }",
            value,
        )
        return self

    # ── Assertions ──
    def verify_cards_visible(self, min_count=1):
        """Verify at least N property cards are rendered."""
        expect(self.property_cards.first).to_be_visible(timeout=10000)
        count = self.property_cards.count()
        assert count >= min_count, f"Expected >= {min_count} cards, got {count}"
        return self

    def verify_card_contract(self, card=None):
        """Verify card attributes used by filtering and accessible navigation."""
        import re

        card = card or self.property_cards.first
        expect(card).to_have_attribute("data-property-id", re.compile(r".+"))
        expect(card).to_have_attribute("data-location", re.compile(r".+"))
        expect(card).to_have_attribute("data-asset-type", re.compile(r".+"))
        expect(card).to_have_attribute("data-duration", re.compile(r".*"))
        expect(card.locator(".property-title-link")).to_have_attribute("href", re.compile(r"^/property/.+"))
        assert card.locator(".ds-progress__fill").count() > 0
        return self

    def verify_available_tab_active(self):
        """Verify the Available tab is active."""
        import re
        expect(self.available_tab).to_have_class(re.compile(r"active", re.I))
        return self

    def verify_funded_tab_active(self):
        """Verify the Funded tab is active."""
        import re
        expect(self.funded_tab).to_have_class(re.compile(r"active", re.I))
        return self

    def verify_heading(self):
        """Verify the page heading (can be 'Marketplace' or 'Properties')."""
        import re
        expect(self.heading).to_have_text(re.compile(r"Marketplace|Properties", re.I))
        return self
