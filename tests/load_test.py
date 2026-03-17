#!/usr/bin/env python3
"""
POOOL Platform — Load Testing Script (Locust)
==============================================
Simulates realistic user behaviour on the POOOL platform.

Installation:
    pip install locust

Run (locally):
    locust -f tests/load_test.py --host=http://localhost:8888 --users=50 --spawn-rate=5

Run (headless, 2 min):
    locust -f tests/load_test.py --host=http://localhost:8888 \
           --users=100 --spawn-rate=10 --run-time=2m --headless \
           --html=tests/load_test_report.html

Targets:
  - GET /health            — heartbeat check
  - GET /en/               — landing page (unauthenticated)
  - POST /auth/login       — authentication (slow due to Argon2, small weight)
  - GET /marketplace       — investor page (authenticated)
  - GET /api/assets        — asset listing API
  - GET /api/me            — user info API
  - GET /api/cart          — cart data
  - GET /api/notifications — notifications
  - GET /wallet            — wallet page
  - GET /portfolio         — portfolio page
  - GET /rewards           — rewards page
"""

import random

from locust import HttpUser, between, task

# ─── Credentials ──────────────────────────────────────────────────────────────
# Adjust these to reflect real users in your staging DB.
# For load testing, use multiple accounts to avoid DB row contention.
TEST_USERS = [
    {"email": "test@poool.app", "password": "TestPass123!"},
]


class AnonymousUser(HttpUser):
    """
    Represents an unauthenticated visitor browsing public pages.
    These requests are the cheapest (no session lookup).
    """

    weight = 3  # 3x more anonymous than authenticated users
    wait_time = between(1, 3)

    @task(5)
    def landing_page_en(self):
        self.client.get("/en/", name="/en/ (landing)")

    @task(5)
    def landing_page_id(self):
        self.client.get("/id/", name="/id/ (landing)")

    @task(10)
    def health_check(self):
        self.client.get("/health", name="/health")

    @task(2)
    def login_page(self):
        self.client.get("/auth/login", name="/auth/login (page)")

    @task(1)
    def signup_page(self):
        self.client.get("/auth/signup", name="/auth/signup (page)")

    @task(2)
    def privacy_page(self):
        self.client.get("/platform/privacy-policy.html", name="/privacy-policy")

    @task(1)
    def try_protected_redirect(self):
        """Verify unauthenticated access is properly rejected (expect 303)."""
        with self.client.get(
            "/marketplace", allow_redirects=False, catch_response=True, name="/marketplace (unauth)"
        ) as r:
            if r.status_code in (302, 303):
                r.success()
            else:
                r.failure(f"Expected redirect, got {r.status_code}")


class AuthenticatedInvestor(HttpUser):
    """
    Represents a logged-in investor browsing the platform.
    Most production traffic will be this type.
    """

    weight = 7
    wait_time = between(1, 4)

    def on_start(self):
        """Log in before running tasks."""
        creds = random.choice(TEST_USERS)
        resp = self.client.post(
            "/auth/login",
            data={"email": creds["email"], "password": creds["password"]},
            allow_redirects=False,
            name="/auth/login (POST)",
            timeout=90,  # Argon2 is slow
        )
        if resp.status_code not in (200, 302, 303):
            # Gracefully continue even if login fails (user may be rate-limited)
            pass

    # ── Page loads ────────────────────────────────────────────────────────────

    @task(8)
    def marketplace(self):
        self.client.get("/marketplace", name="/marketplace")

    @task(4)
    def commodities_marketplace(self):
        self.client.get("/commodities-marketplace", name="/commodities-marketplace")

    @task(3)
    def wallet(self):
        self.client.get("/wallet", name="/wallet")

    @task(3)
    def portfolio(self):
        self.client.get("/portfolio", name="/portfolio")

    @task(2)
    def rewards(self):
        self.client.get("/rewards", name="/rewards")

    @task(2)
    def cart(self):
        self.client.get("/cart", name="/cart")

    @task(1)
    def settings(self):
        self.client.get("/settings", name="/settings")

    @task(1)
    def support(self):
        self.client.get("/support", name="/support")

    # ── API calls ─────────────────────────────────────────────────────────────

    @task(10)
    def api_assets(self):
        self.client.get("/api/assets", name="GET /api/assets")

    @task(8)
    def api_me(self):
        self.client.get("/api/me", name="GET /api/me")

    @task(5)
    def api_cart(self):
        self.client.get("/api/cart", name="GET /api/cart")

    @task(5)
    def api_notifications(self):
        self.client.get("/api/notifications", name="GET /api/notifications")

    @task(4)
    def api_portfolio(self):
        self.client.get("/api/portfolio", name="GET /api/portfolio")

    @task(3)
    def api_rewards(self):
        self.client.get("/api/rewards", name="GET /api/rewards")

    @task(3)
    def api_kyc_status(self):
        self.client.get("/api/kyc/status", name="GET /api/kyc/status")

    @task(2)
    def api_settings(self):
        self.client.get("/api/settings", name="GET /api/settings")

    @task(10)
    def health_check(self):
        self.client.get("/health", name="/health")
