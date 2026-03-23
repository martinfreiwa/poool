"""
Page Object: Login Page
========================
Encapsulates all locators and actions for /auth/login.
Tests should NEVER use raw selectors — always use this page object.
"""

from playwright.sync_api import Page, expect
from tests.e2e.pages.base_page import BasePage


class LoginPage(BasePage):
    PATH = "/auth/login"
    TITLE_CONTAINS = "Login"

    # ── Locators (centralized — change once, fix everywhere) ──
    @property
    def email_input(self):
        return self.page.locator("#email-input")

    @property
    def password_input(self):
        return self.page.locator("#password-input")

    @property
    def login_button(self):
        return self.page.locator("#login-button")

    @property
    def google_button(self):
        return self.page.get_by_role("button", name="Google")

    @property
    def signup_link(self):
        return self.page.get_by_role("link", name="Sign up")

    @property
    def forgot_password_link(self):
        return self.page.get_by_role("link", name="Forgot")

    @property
    def error_message(self):
        return self.page.locator(".error-message, .alert-error, [role='alert']")

    # ── Actions ──
    def login(self, email: str, password: str):
        """Fill credentials and click login."""
        self.email_input.fill(email)
        self.password_input.fill(password)
        self.login_button.click()
        return self

    def login_and_wait(self, email: str, password: str, redirect_pattern="/**"):
        """Login and wait for redirect away from login page."""
        self.login(email, password)
        self.page.wait_for_url(
            lambda url: "/auth/login" not in url, timeout=10000
        )
        return self

    # ── Assertions ──
    def verify_form_visible(self):
        """Verify login form is fully rendered."""
        expect(self.email_input).to_be_visible()
        expect(self.password_input).to_be_visible()
        expect(self.login_button).to_be_visible()
        return self

    def verify_error_shown(self, text=None):
        """Verify an error message is displayed."""
        expect(self.error_message.first).to_be_visible()
        if text:
            expect(self.error_message.first).to_contain_text(text)
        return self

    def verify_redirect_to_dashboard(self):
        """Verify successful login redirected to dashboard."""
        expect(self.page).not_to_have_url(self.url)
        return self
