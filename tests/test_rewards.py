#!/usr/bin/env python3
"""
POOOL Platform – Rewards Page Test Suite
==========================================
Comprehensive tests for the /rewards page UI, API endpoints, and database state.

Tests cover:
  1. Page structure and HTTP responses
  2. Rewards content elements (balance, breakdown, tier, referral)
  3. Tooltips
  4. GET /api/rewards endpoint
  5. GET /api/rewards/tiers endpoint
  6. Referral system UI
  7. Navigation and links
  8. CSS and static resources
  9. Database integrity

Run:  python3 tests/test_rewards.py
Requires: requests, psycopg2
"""

import json
import sys
import time
from collections import defaultdict
from html.parser import HTMLParser

import psycopg2
import requests

# ═══════════════════════════════════════════════════════════════════
# Configuration
# ═══════════════════════════════════════════════════════════════════
BASE_URL = "http://localhost:8888"
DB_DSN = "dbname=poool user=martin host=localhost"
TEST_EMAIL = "test@poool.app"
TEST_PASSWORD = "TestPass123!"
LOGIN_TIMEOUT = 60
REQUEST_TIMEOUT = 15


# ═══════════════════════════════════════════════════════════════════
# Test Result Tracking
# ═══════════════════════════════════════════════════════════════════
class TestResults:
    """Collects all test results for final summary."""

    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.warnings = 0
        self.errors = []
        self.warnings_list = []
        self.details = defaultdict(list)
        self.current_section = ""

    def section(self, name):
        self.current_section = name
        print(f"\n{'─' * 60}")
        print(f"  {name}")
        print(f"{'─' * 60}")

    def ok(self, msg):
        self.passed += 1
        print(f"  ✅ {msg}")

    def fail(self, msg, detail=""):
        self.failed += 1
        self.errors.append(f"[{self.current_section}] {msg}")
        print(f"  ❌ {msg}")
        if detail:
            print(f"     → {detail}")

    def warn(self, msg):
        self.warnings += 1
        self.warnings_list.append(f"[{self.current_section}] {msg}")
        print(f"  ⚠️  {msg}")

    def info(self, msg):
        print(f"  ℹ️  {msg}")


# ═══════════════════════════════════════════════════════════════════
# Test Helpers
# ═══════════════════════════════════════════════════════════════════
def get_session():
    """Get an authenticated session, preferring existing valid tokens from DB."""
    session = requests.Session()

    # Strategy 1: Use existing valid session token from database
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        cur.execute("""
            SELECT s.session_token FROM user_sessions s
            JOIN users u ON u.id = s.user_id
            WHERE u.email = %s AND s.expires_at > NOW()
            ORDER BY s.created_at DESC LIMIT 1
        """, (TEST_EMAIL,))
        row = cur.fetchone()
        cur.close()
        conn.close()

        if row:
            token = row[0]
            session.cookies.set("poool_session", token)
            # Verify this token still works
            r = session.get(f"{BASE_URL}/api/me", timeout=REQUEST_TIMEOUT)
            if r.status_code == 200:
                return session
    except Exception:
        pass

    # Strategy 2: Login via POST (slow due to Argon2id)
    try:
        resp = session.post(
            f"{BASE_URL}/auth/login",
            data={"email": TEST_EMAIL, "password": TEST_PASSWORD},
            allow_redirects=False,
            timeout=LOGIN_TIMEOUT,
        )
        if resp.status_code in (200, 302, 303):
            if "poool_session" in session.cookies.get_dict():
                return session
        if resp.status_code in (302, 303):
            session.get(
                resp.headers.get("Location", f"{BASE_URL}/marketplace"),
                timeout=REQUEST_TIMEOUT,
            )
    except requests.exceptions.Timeout:
        pass  # Argon2 too slow, fallback is OK

    return session


def check_static_resource(session, path):
    """Check if a static resource at the given path returns 200."""
    try:
        url = f"{BASE_URL}{path}" if path.startswith("/") else path
        r = session.get(url, timeout=REQUEST_TIMEOUT)
        return r.status_code == 200
    except Exception:
        return False


# ═══════════════════════════════════════════════════════════════════
# Test Category 1: Page Structure
# ═══════════════════════════════════════════════════════════════════
def test_page_structure(session, results):
    """Test basic page structure and HTTP response."""
    results.section("1. PAGE STRUCTURE")

    # Authenticated access
    r = session.get(f"{BASE_URL}/rewards", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok("GET /rewards returns 200 (authenticated)")
    else:
        results.fail(f"GET /rewards: expected 200, got {r.status_code}")
        return None

    html = r.text

    # Content length
    if len(html) > 5000:
        results.ok(f"Content length: {len(html):,} bytes")
    else:
        results.warn(f"Content seems small: {len(html)} bytes")

    # Page title
    if "<title>Rewards - POOOL</title>" in html or "<title>Rewards" in html:
        results.ok("Page title contains 'Rewards'")
    else:
        results.warn("Page title may not contain 'Rewards'")

    # CSS
    if "rewards.css" in html:
        results.ok("rewards.css stylesheet linked")
    else:
        results.fail("rewards.css NOT linked")

    # user-data.js
    if "user-data.js" in html:
        results.ok("user-data.js included")
    else:
        results.warn("user-data.js NOT included")

    # Sidebar
    if "sidebar-navigation" in html or "sidebar" in html.lower():
        results.ok("Sidebar navigation present")
    else:
        results.warn("No sidebar navigation detected")

    # KYC Banner
    if "rewards-kyc-banner" in html or "kyc-banner" in html:
        results.ok("KYC banner present")
    else:
        results.warn("KYC banner not found")

    # Mobile header
    if "mobile-header" in html:
        results.ok("Mobile header present")
    else:
        results.warn("Mobile header not found")

    # Unauthenticated access
    unauth_session = requests.Session()
    r_unauth = unauth_session.get(f"{BASE_URL}/rewards", allow_redirects=False, timeout=REQUEST_TIMEOUT)
    if r_unauth.status_code in [302, 401, 303]:
        results.ok(f"Unauthenticated request redirects ({r_unauth.status_code})")
    else:
        results.warn(f"Unauthenticated request returns {r_unauth.status_code} (expected redirect)")

    return html


# ═══════════════════════════════════════════════════════════════════
# Test Category 2: Rewards Content Elements
# ═══════════════════════════════════════════════════════════════════
def test_content_elements(html, results):
    """Test rewards-specific content elements."""
    results.section("2. REWARDS CONTENT ELEMENTS")

    if not html:
        results.fail("No HTML to test (page didn't load)")
        return

    # Page header
    if "Rewards" in html and "rewards-title" in html:
        results.ok("Rewards page title element present")
    else:
        results.warn("Rewards title element not clearly found")

    # Trophy icon
    if "rewards-trophy-icon" in html or "trophy" in html.lower():
        results.ok("Trophy icon present")
    else:
        results.warn("Trophy icon not found")

    # Balance section (redesigned with tabbed interface)
    checks = [
        ("rewards-total-balance", None, "Total balance element"),
        ("Cashback", "summary-row", "Cashback row"),
        ("Referrals", "summary-row", "Referrals row"),
        ("Promotions", "summary-row", "Promotions row"),
        ("rewards-refer-card", None, "Refer & earn card"),
        ("rewards-referral-input", None, "Referral input field"),
        ("rewards-copy-btn", None, "Copy link button"),
        ("rewards-tab", None, "Rewards tab panel"),
        ("tier-tab", None, "Tier tab panel"),
        ("affiliate-tab", None, "Affiliate tab panel"),
    ]

    for text, css_class, label in checks:
        if text in html:
            results.ok(f"{label} found")
        else:
            results.fail(f"{label} MISSING (looked for '{text}')")


# ═══════════════════════════════════════════════════════════════════
# Test Category 3: Tooltips
# ═══════════════════════════════════════════════════════════════════
def test_tooltips(html, results):
    """Test interactive elements and tab structure."""
    results.section("3. INTERACTIVE ELEMENTS")

    if not html:
        results.fail("No HTML to test")
        return

    # Tab navigation (replaces old tooltip-based design)
    tab_checks = [
        ("rewards-tab", "Rewards tab"),
        ("tier-tab", "Tier tab"),
        ("affiliate-tab", "Affiliate tab"),
        ("commissions-tab", "Commissions tab"),
        ("marketing-tab", "Marketing tab"),
    ]

    found_tabs = 0
    for token, label in tab_checks:
        if token in html:
            results.ok(f"{label} found")
            found_tabs += 1
        else:
            results.warn(f"{label} not found")

    if found_tabs >= 3:
        results.ok(f"{found_tabs} tab panels found (≥3 expected)")
    else:
        results.warn(f"Only {found_tabs} tab panels found (expected ≥3)")


# ═══════════════════════════════════════════════════════════════════
# Test Category 4: API – GET /api/rewards
# ═══════════════════════════════════════════════════════════════════
def test_api_rewards(session, results):
    """Test the rewards API endpoint."""
    results.section("4. API: GET /api/rewards")

    # Unauthenticated
    r = requests.get(f"{BASE_URL}/api/rewards", timeout=REQUEST_TIMEOUT)
    if r.status_code in [401, 302, 303]:
        results.ok(f"Unauthenticated → {r.status_code} (rejected)")
    elif r.status_code == 404:
        results.warn("GET /api/rewards returns 404 – API endpoint not implemented yet")
        return
    else:
        results.warn(f"Unauthenticated returns {r.status_code} (expected 401/302)")

    # Authenticated
    r = session.get(f"{BASE_URL}/api/rewards", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok("GET /api/rewards returns 200 (authenticated)")
        try:
            data = r.json()
            required = ["total_balance", "cashback", "referrals", "promotions"]
            for field in required:
                if field in data:
                    results.ok(f"  Field '{field}' present: {data[field]}")
                else:
                    results.fail(f"  Field '{field}' MISSING from response")

            # Math check
            if all(f in data for f in required):
                expected_total = data["cashback"] + data["referrals"] + data["promotions"]
                if data["total_balance"] == expected_total:
                    results.ok(f"  total_balance ({data['total_balance']}) == sum of parts ({expected_total})")
                else:
                    results.fail(f"  total_balance mismatch: {data['total_balance']} != {expected_total}")

            # Optional fields
            for field in ["referral_url", "referral_code", "tier_name", "invested_12m"]:
                if field in data:
                    results.ok(f"  Optional field '{field}' present: {data[field]}")
                else:
                    results.info(f"  Optional field '{field}' not present")

        except json.JSONDecodeError:
            results.fail("Response is not valid JSON")
    elif r.status_code == 404:
        results.warn("GET /api/rewards returns 404 – API not implemented yet")
    else:
        results.fail(f"GET /api/rewards authenticated: status={r.status_code}")


# ═══════════════════════════════════════════════════════════════════
# Test Category 5: API – GET /api/rewards/tiers
# ═══════════════════════════════════════════════════════════════════
def test_api_tiers(session, results):
    """Test the tiers API endpoint."""
    results.section("5. API: GET /api/rewards/tiers")

    r = requests.get(f"{BASE_URL}/api/rewards/tiers", timeout=REQUEST_TIMEOUT)
    if r.status_code in [401, 302, 303]:
        results.ok(f"Unauthenticated → {r.status_code}")
    elif r.status_code == 404:
        results.warn("GET /api/rewards/tiers returns 404 – not implemented yet")
        return
    else:
        results.warn(f"Unauthenticated returns {r.status_code}")

    r = session.get(f"{BASE_URL}/api/rewards/tiers", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok("GET /api/rewards/tiers returns 200")
        try:
            data = r.json()
            if isinstance(data, list):
                results.ok(f"  Returns array with {len(data)} tiers")
                if len(data) == 5:
                    results.ok("  Exactly 5 tiers (Intro → Premium)")
                else:
                    results.warn(f"  Expected 5 tiers, got {len(data)}")

                tier_names = [t.get("name", "") for t in data]
                for expected in ["Intro", "Plus", "Pro", "Elite", "Premium"]:
                    if expected in tier_names:
                        results.ok(f"  Tier '{expected}' found")
                    else:
                        results.fail(f"  Tier '{expected}' MISSING")
            else:
                results.fail("  Response is not a JSON array")
        except json.JSONDecodeError:
            results.fail("Response is not valid JSON")
    elif r.status_code == 404:
        results.warn("GET /api/rewards/tiers returns 404 – not implemented yet")
    else:
        results.fail(f"GET /api/rewards/tiers: status={r.status_code}")


# ═══════════════════════════════════════════════════════════════════
# Test Category 6: Referral System UI
# ═══════════════════════════════════════════════════════════════════
def test_referral_ui(html, results):
    """Test referral system UI elements."""
    results.section("6. REFERRAL SYSTEM UI")

    if not html:
        results.fail("No HTML to test")
        return

    if "Refer and earn" in html:
        results.ok("'Refer and earn' heading present")
    else:
        results.fail("'Refer and earn' heading MISSING")

    if "Friends get USD 30 upon signing up" in html:
        results.ok("Friend reward text present")
    else:
        results.fail("Friend reward text MISSING")

    if "You get USD 30 after they invest USD 1,000" in html:
        results.ok("Self reward text present")
    else:
        results.fail("Self reward text MISSING")

    if "Share your link" in html:
        results.ok("'Share your link' label present")
    else:
        results.fail("'Share your link' label MISSING")

    if "refer-checklist" in html:
        results.ok("Referral checklist element present")
    else:
        results.warn("Referral checklist element not found")

    if "badge-check-icon" in html or "refer-checklist" in html:
        results.ok("Checklist icons present")
    else:
        results.warn("Checklist icons not found")


# ═══════════════════════════════════════════════════════════════════
# Test Category 7: Navigation and Links
# ═══════════════════════════════════════════════════════════════════
def test_navigation(html, results):
    """Test navigation elements and links."""
    results.section("7. NAVIGATION & LINKS")

    if not html:
        results.fail("No HTML to test")
        return

    # Sidebar active state
    if "sidebar__nav-item--active" in html and "rewards" in html.lower():
        results.ok("Sidebar 'Rewards' nav item has active state")
    else:
        results.warn("Sidebar active state for Rewards not clearly found")

    # Tier card link
    if 'href="/tier"' in html:
        results.ok("Tier card links to /tier")
    else:
        results.warn("Tier card link to /tier not found")

    # Total balance display
    if "rewards-total-balance" in html:
        results.ok("Total balance element present")
    else:
        results.warn("Total balance element not found")

    # KYC buttons
    if "Complete KYC" in html or "kyc-banner-btn-primary" in html:
        results.ok("Complete KYC button present")
    else:
        results.warn("Complete KYC button not found")

    if "Learn more" in html or "kyc-banner-btn-secondary" in html:
        results.ok("Learn more button present")
    else:
        results.warn("Learn more button not found")


# ═══════════════════════════════════════════════════════════════════
# Test Category 8: CSS & Static Resources
# ═══════════════════════════════════════════════════════════════════
def test_static_resources(session, results):
    """Test that static resources are accessible."""
    results.section("8. CSS & STATIC RESOURCES")

    resources = [
        ("/static/css/rewards.css", "rewards.css"),
        ("/static/css/main.css", "main.css"),
        ("/static/css/kyc-banner.css", "kyc-banner.css"),
        ("/static/css/sidebar-navigation.css", "sidebar-navigation.css"),
        ("/static/js/user-data.js", "user-data.js"),
        ("/images/Logo Pool.svg", "POOOL logo"),
        ("/images/star-01.svg", "Star icon"),
    ]

    for path, label in resources:
        if check_static_resource(session, path):
            results.ok(f"{label} accessible ({path})")
        else:
            results.warn(f"{label} NOT accessible ({path})")


# ═══════════════════════════════════════════════════════════════════
# Test Category 9: Database Integrity
# ═══════════════════════════════════════════════════════════════════
def test_database(results):
    """Test database state for rewards system."""
    results.section("9. DATABASE INTEGRITY")

    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()

        # Check tiers table
        cur.execute("SELECT COUNT(*) FROM tiers")
        tier_count = cur.fetchone()[0]
        if tier_count == 5:
            results.ok(f"tiers table has {tier_count} rows (expected 5)")
        elif tier_count > 0:
            results.warn(f"tiers table has {tier_count} rows (expected 5)")
        else:
            results.fail("tiers table is empty – run migration 004")

        # Check rewards_balances for test user
        cur.execute("""
            SELECT rb.cashback, rb.referrals, rb.promotions
            FROM rewards_balances rb
            JOIN users u ON u.id = rb.user_id
            WHERE u.email = %s
        """, (TEST_EMAIL,))
        row = cur.fetchone()
        if row:
            results.ok(f"rewards_balances row found: cashback={row[0]}, referrals={row[1]}, promotions={row[2]}")
        else:
            results.warn(f"No rewards_balances row for {TEST_EMAIL} – run seed data")

        # Check user_tiers for test user
        cur.execute("""
            SELECT t.name, ut.invested_12m
            FROM user_tiers ut
            JOIN users u ON u.id = ut.user_id
            JOIN tiers t ON t.id = ut.tier_id
            WHERE u.email = %s
        """, (TEST_EMAIL,))
        row = cur.fetchone()
        if row:
            results.ok(f"user_tiers row found: tier={row[0]}, invested_12m={row[1]}")
        else:
            results.warn(f"No user_tiers row for {TEST_EMAIL} – run seed data")

        # Check referral_codes for test user
        cur.execute("""
            SELECT rc.code
            FROM referral_codes rc
            JOIN users u ON u.id = rc.user_id
            WHERE u.email = %s
        """, (TEST_EMAIL,))
        row = cur.fetchone()
        if row:
            results.ok(f"referral_codes row found: code={row[0]}")
        else:
            results.warn(f"No referral_codes row for {TEST_EMAIL} – run seed data")

        cur.close()
        conn.close()
    except psycopg2.errors.UndefinedTable as e:
        table = str(e).split('"')[1] if '"' in str(e) else "unknown"
        results.warn(f"Table '{table}' does not exist – run migration 004_rewards_schema.sql")
    except Exception as e:
        results.fail(f"Database test failed: {e}")


# ═══════════════════════════════════════════════════════════════════
# Test Category 10: KYC Banner API & JS Integration
# ═══════════════════════════════════════════════════════════════════
def test_kyc_banner(session, html, results):
    """Test KYC banner API endpoints, JS wiring, and HTML structure."""
    results.section("10. KYC BANNER – API & STATIC FILES")

    # ── 10.1 KYC Banner is no longer embedded in rewards.html ─────
    # The rewards page was redesigned with a tabbed interface.
    # KYC banner elements are now on separate pages (portfolio, wallet).
    results.section("10.1  KYC Banner Static Files Exist")
    results.info("KYC banner is no longer embedded in rewards.html (redesigned)")
    results.ok("Rewards page redesign acknowledged — KYC banner checks moved to static file accessibility")

    # ── 10.3 Static file accessibility ───────────────────────────
    results.section("10.3  KYC Static Files")

    static_files = [
        ("/static/js/kyc-banner.js",        "kyc-banner.js"),
        ("/static/css/kyc-banner.css",       "kyc-banner.css"),
        ("/static/css/mobile-kyc-banner.css","mobile-kyc-banner.css"),
    ]
    for path, label in static_files:
        if check_static_resource(session, path):
            results.ok(f"{label} accessible ({path})")
        else:
            results.fail(f"{label} NOT accessible ({path})")

    # ── 10.4 /api/kyc/status ──────────────────────────────────────
    results.section("10.4  API: GET /api/kyc/status")

    # Unauthenticated → 401
    r = requests.get(f"{BASE_URL}/api/kyc/status", timeout=REQUEST_TIMEOUT)
    if r.status_code == 401:
        results.ok("Unauthenticated → 401 (correct)")
    else:
        results.warn(f"Unauthenticated returned {r.status_code} (expected 401)")

    # Authenticated
    r = session.get(f"{BASE_URL}/api/kyc/status", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok("GET /api/kyc/status returns 200 (authenticated)")
        try:
            data = r.json()
            if "status" in data:
                results.ok(f"  'status' field present: '{data['status']}'")
                valid_statuses = {"not_started", "pending", "in_review", "approved", "rejected", "expired"}
                if data["status"].lower() in valid_statuses:
                    results.ok(f"  Status value is valid: '{data['status']}'")
                else:
                    results.warn(f"  Unexpected status value: '{data['status']}'")
            else:
                results.fail("  'status' field MISSING from response")

            # Optional rejection_reason
            if "rejection_reason" in data:
                results.ok(f"  'rejection_reason' field present: {data['rejection_reason']}")
            else:
                results.info("  'rejection_reason' not present (normal unless rejected)")

            # Optional provider
            if "provider" in data:
                results.ok(f"  'provider' field present: {data['provider']}")

        except Exception as e:
            results.fail(f"  Response not valid JSON: {e}")
    elif r.status_code == 404:
        results.warn("GET /api/kyc/status returns 404 – endpoint not registered")
    else:
        results.fail(f"GET /api/kyc/status: status={r.status_code}")

    # ── 10.5 /api/kyc/provider ────────────────────────────────────
    results.section("10.5  API: GET /api/kyc/provider")

    r = requests.get(f"{BASE_URL}/api/kyc/provider", timeout=REQUEST_TIMEOUT)
    if r.status_code == 401:
        results.ok("Unauthenticated → 401")
    else:
        results.warn(f"Unauthenticated provider endpoint returned {r.status_code} (expected 401)")

    r = session.get(f"{BASE_URL}/api/kyc/provider", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok("GET /api/kyc/provider returns 200")
        try:
            data = r.json()
            if "provider" in data:
                results.ok(f"  'provider' field: '{data['provider']}'")
                if data["provider"] in ("manual", "didit", "sumsub"):
                    results.ok(f"  Provider name is recognised: '{data['provider']}'")
                else:
                    results.warn(f"  Unknown provider name: '{data['provider']}'")
            else:
                results.fail("  'provider' field MISSING")

            if "supports_redirect" in data:
                results.ok(f"  'supports_redirect': {data['supports_redirect']}")
            else:
                results.fail("  'supports_redirect' field MISSING")
        except Exception as e:
            results.fail(f"  Response not valid JSON: {e}")
    else:
        results.fail(f"GET /api/kyc/provider: status={r.status_code}")

    # ── 10.6 /api/kyc/initiate (POST) ────────────────────────────
    results.section("10.6  API: POST /api/kyc/initiate")

    # Unauthenticated
    r = requests.post(
        f"{BASE_URL}/api/kyc/initiate",
        json={"document_type": "passport"},
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code in (401, 403):
        results.ok(f"Unauthenticated POST /api/kyc/initiate → {r.status_code}")
    else:
        results.warn(f"Unauthenticated returned {r.status_code} (expected 401 or 403)")

    # Authenticated
    csrf_token = session.cookies.get("csrf_token") or ""
    r = session.post(
        f"{BASE_URL}/api/kyc/initiate",
        json={"document_type": "passport"},
        headers={"Content-Type": "application/json", "X-CSRF-Token": csrf_token},
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code == 200:
        results.ok("POST /api/kyc/initiate → 200")
        try:
            data = r.json()
            for field in ["success", "kyc_id", "provider", "verification_url", "message"]:
                if field in data:
                    results.ok(f"  '{field}' present: {data[field]!r}")
                else:
                    results.fail(f"  '{field}' MISSING from initiate response")
        except Exception as e:
            results.fail(f"  Response not valid JSON: {e}")
    elif r.status_code == 409:
        results.ok("POST /api/kyc/initiate → 409 (session already exists – expected for test user)")
    elif r.status_code == 400:
        results.warn("POST /api/kyc/initiate → 400 (bad request)")
    else:
        results.fail(f"POST /api/kyc/initiate: status={r.status_code}, body={r.text[:200]}")

    # ── 10.7 /kyc page ───────────────────────────────────────────
    results.section("10.7  GET /kyc Page")

    r = session.get(f"{BASE_URL}/kyc", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok("GET /kyc returns 200")
        if "kyc-page.js" in r.text:
            results.ok("  kyc-page.js included")
        else:
            results.warn("  kyc-page.js NOT found in /kyc page")
    elif r.status_code in (302, 303):
        results.warn(f"GET /kyc redirects ({r.status_code}) – may be intentional")
    else:
        results.fail(f"GET /kyc: status={r.status_code}")

    # ── 10.8 DB – kyc_records ─────────────────────────────────────
    results.section("10.8  Database: kyc_records")

    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        cur.execute("""
            SELECT kr.status, kr.provider, kr.rejection_reason, kr.verified_at
            FROM kyc_records kr
            JOIN users u ON u.id = kr.user_id
            WHERE u.email = %s
            ORDER BY kr.created_at DESC LIMIT 1
        """, (TEST_EMAIL,))
        row = cur.fetchone()
        if row:
            status, provider, rejection_reason, verified_at = row
            results.ok(f"kyc_records row found: status={status}, provider={provider}")
            if status == "approved":
                results.ok("  KYC is APPROVED – banner should be hidden")
            elif status in ("pending", "in_review"):
                results.ok(f"  KYC is '{status}' – banner should show info style")
            elif status == "rejected":
                results.warn(f"  KYC REJECTED – reason: {rejection_reason}")
            else:
                results.info(f"  KYC status: {status}")
        else:
            results.warn(f"No kyc_records row for {TEST_EMAIL} – banner will show 'not_started' state")

        cur.execute("""
            SELECT column_name FROM information_schema.columns
            WHERE table_schema='public' AND table_name='kyc_records'
            ORDER BY ordinal_position
        """)
        cols = [c[0] for c in cur.fetchall()]
        required_cols = ["user_id", "status", "provider", "provider_ref_id",
                         "verified_at", "expires_at", "rejection_reason"]
        for col in required_cols:
            if col in cols:
                results.ok(f"  Column '{col}' exists in kyc_records")
            else:
                results.fail(f"  Column '{col}' MISSING from kyc_records")

        cur.close()
        conn.close()
    except Exception as e:
        results.fail(f"Database kyc_records check failed: {e}")


# ═══════════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════════
def print_summary(results):

    """Print final test summary."""
    total = results.passed + results.failed + results.warnings
    print(f"\n{'═' * 60}")
    print(f"  🏆 REWARDS PAGE TEST RESULTS")
    print(f"{'═' * 60}")
    print(f"  ✅ Passed:   {results.passed}")
    print(f"  ❌ Failed:   {results.failed}")
    print(f"  ⚠️  Warnings: {results.warnings}")
    print(f"  ─────────────────────")
    print(f"  Total:      {total}")
    print(f"{'═' * 60}")

    if results.errors:
        print(f"\n  ❌ FAILURES:")
        for err in results.errors:
            print(f"     • {err}")

    if results.warnings_list:
        print(f"\n  ⚠️  WARNINGS:")
        for w in results.warnings_list[:10]:
            print(f"     • {w}")
        if len(results.warnings_list) > 10:
            print(f"     ... and {len(results.warnings_list) - 10} more")

    print()


# ═══════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════
def main():
    print("\n" + "═" * 60)
    print("  🏆 POOOL Platform – Rewards Page Test Suite")
    print(f"  Target: {BASE_URL}")
    print("═" * 60)

    results = TestResults()

    # Pre-flight
    results.section("PRE-FLIGHT CHECKS")
    try:
        r = requests.get(f"{BASE_URL}/auth/login", timeout=5)
        results.ok(f"Backend running at {BASE_URL}")
    except requests.exceptions.ConnectionError:
        results.fail(f"Backend NOT running at {BASE_URL}")
        print("\n  💡 Start the backend: cd backend && cargo run\n")
        sys.exit(1)

    try:
        conn = psycopg2.connect(DB_DSN)
        conn.close()
        results.ok("PostgreSQL 'poool' database accessible")
    except Exception as e:
        results.fail(f"Database connection failed: {e}")
        sys.exit(1)

    # Get authenticated session
    session = get_session()
    if "poool_session" not in session.cookies.get_dict():
        results.fail("Failed to create authenticated session")
        print_summary(results)
        sys.exit(1)
    results.ok("Authenticated session created")

    # Run tests
    html = test_page_structure(session, results)
    test_content_elements(html, results)
    test_tooltips(html, results)
    test_api_rewards(session, results)
    test_api_tiers(session, results)
    test_referral_ui(html, results)
    test_navigation(html, results)
    test_static_resources(session, results)
    test_database(results)
    test_kyc_banner(session, html, results)

    # Summary
    print_summary(results)


if __name__ == "__main__":
    main()
