"""
Page Object: Admin Base Page
==============================
Shared logic for all admin pages (sidebar, header, navigation).
"""

from playwright.sync_api import Page, expect
from tests.e2e.pages.base_page import BasePage


class AdminBasePage(BasePage):
    """Base class for all admin page objects."""

    # ── Shared Admin Locators ──
    @property
    def sidebar(self):
        return self.page.locator(".admin-sidebar, #admin-sidebar, nav.sidebar")

    @property
    def sidebar_links(self):
        return self.sidebar.locator("a")

    @property
    def admin_header(self):
        return self.page.locator(".admin-header, header")

    @property
    def breadcrumb(self):
        return self.page.locator(".breadcrumb, nav[aria-label='breadcrumb']")

    @property
    def page_title(self):
        return self.page.locator("h1, .page-title").first

    @property
    def loading_spinner(self):
        return self.page.locator(".loading, .spinner, [data-loading]")

    # ── Shared Actions ──
    def navigate_via_sidebar(self, link_text: str):
        """Click a sidebar navigation link."""
        self.sidebar.get_by_text(link_text, exact=False).click()
        return self

    def wait_for_data_load(self, timeout=10000):
        """Wait for any loading spinner to disappear."""
        if self.loading_spinner.count() > 0:
            expect(self.loading_spinner.first).not_to_be_visible(timeout=timeout)
        return self

    # ── Shared Assertions ──
    def verify_admin_layout(self):
        """Verify the admin shell (sidebar + header) is rendered."""
        expect(self.page.locator("body")).to_be_visible()
        return self

    def verify_table_rendered(self, selector="table", min_rows=0):
        """Verify a data table is visible and has rows."""
        table = self.page.locator(selector).first
        expect(table).to_be_visible(timeout=10000)
        if min_rows > 0:
            rows = table.locator("tbody tr")
            assert rows.count() >= min_rows
        return self


class AdminDashboardPage(AdminBasePage):
    PATH = "/admin/"
    TITLE_CONTAINS = "Admin"


class AdminUsersPage(AdminBasePage):
    PATH = "/admin/users.html"
    TITLE_CONTAINS = "Users"

    @property
    def users_table(self):
        return self.page.locator("#users-table, table").first

    @property
    def search_input(self):
        return self.page.locator("#search-input, input[type='search']").first

    def search_user(self, query: str):
        self.search_input.fill(query)
        return self

    def verify_users_loaded(self, min_rows=1):
        return self.verify_table_rendered(min_rows=min_rows)


class AdminOrdersPage(AdminBasePage):
    PATH = "/admin/orders.html"
    TITLE_CONTAINS = "Orders"

    @property
    def orders_table(self):
        return self.page.locator("#orders-table, table").first

    def verify_orders_loaded(self, min_rows=0):
        return self.verify_table_rendered(min_rows=min_rows)


class AdminKYCPage(AdminBasePage):
    PATH = "/admin/kyc.html"
    TITLE_CONTAINS = "KYC"


class AdminDepositsPage(AdminBasePage):
    PATH = "/admin/deposits.html"
    TITLE_CONTAINS = "Deposits"


class AdminAssetsPage(AdminBasePage):
    PATH = "/admin/assets.html"
    TITLE_CONTAINS = "Assets"


class AdminRewardsPage(AdminBasePage):
    PATH = "/admin/rewards.html"
    TITLE_CONTAINS = "Rewards"


class AdminSupportPage(AdminBasePage):
    PATH = "/admin/support.html"
    TITLE_CONTAINS = "Support"


class AdminSettingsPage(AdminBasePage):
    PATH = "/admin/settings.html"
    TITLE_CONTAINS = "Settings"


class AdminAuditLogsPage(AdminBasePage):
    PATH = "/admin/audit-logs.html"
    TITLE_CONTAINS = "Audit"


class AdminSystemPage(AdminBasePage):
    PATH = "/admin/system.html"
    TITLE_CONTAINS = "System"
