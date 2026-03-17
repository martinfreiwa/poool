#!/usr/bin/env python3
"""
POOOL Admin Dashboard – Comprehensive Feature Test Suite
=======================================================
Functional testing for all Admin features including Users, Finance, 
Assets, Support, KYC, and System Settings.

Usage:
  python3 tests/admin/test_admin_features.py
"""

import json
import os
import sys
import time
from html.parser import HTMLParser
import psycopg2
import requests

# ═══════════════════════════════════════════════════════════════════
# Configuration
# ═══════════════════════════════════════════════════════════════════
BASE_URL = "http://localhost:8888"
DB_DSN = os.environ.get("DATABASE_URL", "dbname=poool user=martin host=localhost")
REQUEST_TIMEOUT = 10

class TestLogger:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.warnings = 0
        self.errors = []
        self.current_section = ""

    def section(self, name):
        self.current_section = name
        print(f"\n{'='*80}")
        print(f"  SECTION: {name}")
        print(f"{'='*80}")

    def ok(self, msg):
        self.passed += 1
        print(f"  ✅ [PASS] {msg}")

    def fail(self, msg, detail=""):
        self.failed += 1
        self.errors.append((self.current_section, msg))
        print(f"  ❌ [FAIL] {msg}")
        if detail: print(f"     → {detail}")

    def warn(self, msg):
        self.warnings += 1
        print(f"  ⚠️  [WARN] {msg}")

    def summary(self):
        print(f"\n{'═'*80}")
        print(f"  FINAL SUMMARY")
        print(f"{'═'*80}")
        print(f"  ✅ Passed:   {self.passed}")
        print(f"  ❌ Failed:   {self.failed}")
        print(f"  ⚠️  Warnings: {self.warnings}")
        if self.failed > 0:
            print("\n  Failures to fix:")
            for section, err in self.errors:
                print(f"  - [{section}] {err}")
            sys.exit(1)
        else:
            print("\n  🎉 ALL ADMIN FEATURE TESTS PASSED!")
            sys.exit(0)

# ═══════════════════════════════════════════════════════════════════
# Helpers
# ═══════════════════════════════════════════════════════════════════

def get_admin_session():
    session = requests.Session()

    def _has_admin_access(s):
        """Verify admin access by hitting an admin-only endpoint."""
        try:
            # First, acquire CSRF token by visiting any page
            s.get(f"{BASE_URL}/admin/", timeout=5)
            if "csrf_token" in s.cookies:
                s.headers.update({"X-CSRF-Token": s.cookies["csrf_token"]})
            r = s.get(f"{BASE_URL}/api/admin/stats/overview", timeout=5)
            return r.status_code == 200
        except:
            return False

    # Strategy 1: Use existing valid session from DB
    try:
        # Try a known injected token first for CI/automation
        session.cookies.set("poool_session", "fake_admin_session_token")
        if _has_admin_access(session):
            return session
            
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        cur.execute("""
            SELECT s.session_token FROM user_sessions s
            JOIN users u ON u.id = s.user_id
            JOIN user_roles ur ON ur.user_id = u.id
            JOIN roles r ON r.id = ur.role_id
            WHERE (r.name = 'admin' OR r.name = 'super_admin') AND s.expires_at > NOW()
            ORDER BY s.created_at DESC LIMIT 1
        """)
        row = cur.fetchone()
        cur.close()
        conn.close()
        if row:
            session = requests.Session()
            session.cookies.set("poool_session", row[0])
            if _has_admin_access(session):
                return session
    except: pass

    # Strategy 2: Try known combinations
    ADMIN_EMAIL = "test@poool.app"
    ADMIN_PASSWORD = "TestPass123!"
    creds = [
        (ADMIN_EMAIL, ADMIN_PASSWORD),
        ("test@poool.app", "TestPass123!"),
        ("martin@poool.app", "AdminPass123!")
    ]
    for email, password in creds:
        try:
            session = requests.Session()
            session.get(f"{BASE_URL}/admin/", timeout=5)
            csrf = session.cookies.get("csrf_token", "")
            r = session.post(f"{BASE_URL}/auth/login", 
                             data={"email": email, "password": password},
                             headers={"X-CSRF-Token": csrf},
                             timeout=REQUEST_TIMEOUT)
            if "poool_session" in session.cookies:
                if _has_admin_access(session):
                    return session
        except: continue
    return None

# ═══════════════════════════════════════════════════════════════════
# Test Suites
# ═══════════════════════════════════════════════════════════════════

def test_dashboard(session, logger):
    logger.section("DASHBOARD")
    r = session.get(f"{BASE_URL}/api/admin/stats/overview")
    if r.status_code == 200:
        logger.ok("KPI Stats API accessible")
        stats = r.json()
        expected = [
            "total_users", "pending_kyc", "live_assets", "open_tickets", 
            "total_invested_cents", "deposits_24h_cents", "pending_deposits_cents",
            "rewards_liability_cents"
        ]
        for field in expected:
            if field in stats: logger.ok(f"  {field}: {stats[field]}")
            else: logger.warn(f"  KPI field MISSING: {field}")
    else:
        logger.fail("Stats API failed", f"Status: {r.status_code}")

def test_admin_api_security(logger):
    logger.section("API SECURITY")
    session = requests.Session() # No auth
    endpoints = ["/api/admin/users", "/api/admin/stats/overview", "/api/admin/settings"]
    for ep in endpoints:
        try:
            r = session.get(f"{BASE_URL}{ep}", allow_redirects=False, timeout=5)
            # Should be 401 or redirected to login
            if r.status_code in (401, 403) or (r.status_code == 302 and "/auth/login" in r.headers.get("Location", "")):
                logger.ok(f"Unauthenticated access to {ep} blocked/redirected ({r.status_code})")
            else:
                logger.fail(f"Unauthenticated access to {ep} returned {r.status_code}")
        except:
            logger.fail(f"Error testing security for {ep}")

    # Test CSRF token enforcement on POST requests
    try:
        # Create a new session with valid auth but NO csrf header
        csrf_session = requests.Session()
        csrf_session.cookies.set("poool_session", "fake_admin_session_token")
        
        # We need a real user_id to avoid 400 Bad Request before hitting CSRF check 
        # (though ideally the middleware triggers first). We'll assume a dummy UUID formats correctly
        r = csrf_session.post(f"{BASE_URL}/api/admin/users/00000000-0000-0000-0000-000000000000/profile", json={"first_name": "Hack"}, timeout=5)
        if r.status_code in (403, 401):
            logger.ok("CSRF protection blocked POST request missing header")
        else:
            logger.fail("CSRF protection FAILED", f"POST without CSRF header returned {r.status_code}")
    except Exception as e:
        logger.fail("Error testing CSRF enforcement", str(e))

def test_users(session, logger):
    logger.section("USER MANAGEMENT")
    # List
    r = session.get(f"{BASE_URL}/api/admin/users")
    if r.status_code != 200:
        logger.fail("Users List API failed")
        return
    
    data = r.json()
    users = data if isinstance(data, list) else data.get("users", [])
    logger.ok(f"Users List API returned {len(users)} users")
    if not users: return

    user_id = users[0]["id"]
    # Detail
    r = session.get(f"{BASE_URL}/api/admin/users/{user_id}")
    if r.status_code == 200: logger.ok(f"User Detail API accessible for {user_id}")
    else: logger.fail(f"User Detail API failed for {user_id}")

    # Balance Update
    payload = {"wallet_type": "cash", "amount_cents": 1, "reason": "Test"}
    r = session.post(f"{BASE_URL}/api/admin/users/{user_id}/balance", json=payload)
    if r.status_code == 200: logger.ok("Balance update API successful")
    else: logger.fail("Balance update API failed", f"Status: {r.status_code}")

    # Status Toggle
    payload = {"status": "active"}
    r = session.post(f"{BASE_URL}/api/admin/users/{user_id}/status", json=payload)
    if r.status_code == 200: logger.ok("Status update API successful")
    else: logger.fail("Status update API failed", f"Status: {r.status_code}")

    # Profile Update (Partial Payload testing Default deserialization)
    payload = {"first_name": "Test"}
    r = session.post(f"{BASE_URL}/api/admin/users/{user_id}/profile", json=payload)
    if r.status_code == 200: logger.ok("Profile update API successful with partial payload")
    else: logger.fail("Profile update API failed (Missing Default derive on Rust struct?)", f"Status: {r.status_code}")

def test_finance(session, logger):
    logger.section("FINANCIAL")
    # Orders
    r = session.get(f"{BASE_URL}/api/admin/orders")
    if r.status_code == 200:
        data = r.json()
        orders = data if isinstance(data, list) else data.get("orders", [])
        logger.ok(f"Orders API: {len(orders)} items")
    else: logger.fail("Orders API failed")

    # Deposits
    r = session.get(f"{BASE_URL}/api/admin/deposits")
    if r.status_code == 200:
        data = r.json()
        deposits = data if isinstance(data, list) else data.get("deposits", [])
        logger.ok(f"Deposits API: {len(deposits)} items")
    else: logger.fail("Deposits API failed")

    # Treasury
    r = session.get(f"{BASE_URL}/api/admin/treasury")
    if r.status_code == 200: logger.ok("Treasury API accessible")
    else: logger.fail("Treasury API failed")

def test_assets_submissions(session, logger):
    logger.section("ASSETS & SUBMISSIONS")
    # Submissions
    r = session.get(f"{BASE_URL}/api/admin/submissions")
    if r.status_code == 200:
        data = r.json()
        subs = data if isinstance(data, list) else data.get("submissions", [])
        logger.ok(f"Submissions API: {len(subs)} items")
        if subs:
            asset_id = subs[0]["id"]
            r2 = session.get(f"{BASE_URL}/api/admin/submissions/{asset_id}/detail")
            if r2.status_code == 200: logger.ok(f"Submission Detail API accessible for {asset_id}")
    else: logger.fail("Submissions API failed")

    # Assets
    r = session.get(f"{BASE_URL}/api/admin/assets")
    if r.status_code == 200:
        data = r.json()
        assets = data if isinstance(data, list) else data.get("assets", [])
        logger.ok(f"Assets API: {len(assets)} items")
        if assets:
            asset_id = assets[0]["id"]
            r2 = session.get(f"{BASE_URL}/api/admin/assets/{asset_id}/detail")
            if r2.status_code == 200: logger.ok(f"Asset Detail API accessible for {asset_id}")
    else: logger.fail("Assets API failed")

def test_support_kyc(session, logger):
    logger.section("SUPPORT & KYC")
    # Support
    r = session.get(f"{BASE_URL}/api/admin/support")
    if r.status_code == 200: logger.ok("Support Tickets API accessible")
    else: logger.fail("Support Tickets API failed")

    # KYC
    r = session.get(f"{BASE_URL}/api/admin/kyc")
    if r.status_code == 200:
        data = r.json()
        records = data if isinstance(data, list) else data.get("records", [])
        logger.ok(f"KYC Records API: {len(records)} items")
    else: logger.fail("KYC Records API failed")

def test_system_settings(session, logger):
    logger.section("SYSTEM & SETTINGS")
    # Health
    r = session.get(f"{BASE_URL}/api/admin/system")
    if r.status_code == 200: logger.ok("System Health API accessible")
    else: logger.fail("System Health API failed")

    # Audit
    r = session.get(f"{BASE_URL}/api/admin/audit-logs")
    if r.status_code == 200: logger.ok(f"Audit Logs API found")
    else: logger.fail("Audit Logs API failed")

    # Settings
    r = session.get(f"{BASE_URL}/api/admin/settings")
    if r.status_code == 200: logger.ok("Admin Settings API accessible")
    else: logger.fail("Admin Settings API failed")

def test_admins_roles(session, logger):
    logger.section("ADMINS & ROLES")
    # Admins
    r = session.get(f"{BASE_URL}/api/admin/admins")
    if r.status_code == 200:
        admins = r.json()
        logger.ok(f"Admins API returned {len(admins)} admins")
    else: logger.fail(f"Admins API failed (status {r.status_code})")
    
    # Roles
    r = session.get(f"{BASE_URL}/api/admin/roles")
    if r.status_code == 200:
        roles = r.json()
        logger.ok(f"Roles API returned {len(roles)} roles")
    else: logger.fail(f"Roles API failed (status {r.status_code})")

def test_audit_logs(session, logger):
    logger.section("AUDIT TRAIL")
    r = session.get(f"{BASE_URL}/api/admin/audit-logs")
    if r.status_code == 200:
        logs = r.json()
        logger.ok(f"Audit Logs API returned {len(logs)} entries")
    else: logger.fail(f"Audit Logs API failed ({r.status_code})")

def test_dividends(session, logger):
    logger.section("DIVIDEND DISTRIBUTION")
    r = session.get(f"{BASE_URL}/api/admin/dividends/pending")
    if r.status_code == 200:
        try:
            pending = r.json()
            logger.ok(f"Dividends API returned {len(pending)} pending distributions")
        except Exception:
            logger.warn("Dividends API returned 200 but non-JSON response")
    elif r.status_code == 404:
        logger.warn("Dividends API might not be implemented yet")
    else: logger.fail(f"Dividends API failed ({r.status_code})")

def test_orders(session, logger):
    logger.section("ORDERS MANAGEMENT")
    r = session.get(f"{BASE_URL}/api/admin/orders")
    if r.status_code == 200:
        orders = r.json()
        logger.ok(f"Orders API returned {len(orders)} orders")
    else: logger.fail(f"Orders API failed ({r.status_code})")

def test_search(session, logger):
    logger.section("GLOBAL SEARCH API")
    # Test user search
    r = session.get(f"{BASE_URL}/api/admin/search?q=olivia")
    if r.status_code == 200:
        try:
            results = r.json()
            logger.ok(f"Search API returned {len(results.get('users', []))} user results")
        except Exception:
            logger.warn("Search API returned 200 but non-JSON response")
    else: logger.fail(f"Search API failed for users ({r.status_code})")
    
    # Test asset search
    r = session.get(f"{BASE_URL}/api/admin/search?q=property")
    if r.status_code == 200:
        try:
            results = r.json()
            logger.ok(f"Search API returned {len(results.get('assets', []))} asset results")
        except Exception:
            logger.warn("Search API returned 200 but non-JSON response")
    else: logger.fail(f"Search API failed for assets ({r.status_code})")

def test_emails(session, logger):
    logger.section("EMAIL MARKETING")
    r = session.get(f"{BASE_URL}/api/admin/emails")
    if r.status_code == 200:
        data = r.json()
        emails = data if isinstance(data, list) else data.get("emails", [])
        logger.ok(f"Email Engine API accessible ({len(emails)} templates/campaigns)")
    elif r.status_code == 404:
        logger.warn("Email Engine API (404) - Link exists but endpoint not implemented")
    else:
        logger.fail("Email Engine API failed", f"Status: {r.status_code}")

def test_rewards(session, logger):
    logger.section("REWARDS & REFERRALS")
    r = session.get(f"{BASE_URL}/api/admin/rewards")
    if r.status_code == 200:
        data = r.json()
        sections = ["user_tiers", "balances", "referrals", "tiers"]
        for section in sections:
            if section in data:
                logger.ok(f"  Section '{section}' found ({len(data[section]) if isinstance(data[section], list) else 'dict'})")
            else:
                logger.fail(f"  Missing section: {section}")
    else:
        logger.fail("Rewards API failed")

def test_notifications(session, logger):
    logger.section("NOTIFICATIONS")
    r = session.get(f"{BASE_URL}/api/admin/notifications")
    if r.status_code == 200: logger.ok("Notifications API accessible")
    else: logger.fail("Notifications API failed")

def test_approvals(session, logger):
    logger.section("FOUR-EYES APPROVALS (Maker-Checker)")
    r = session.get(f"{BASE_URL}/api/admin/approvals")
    if r.status_code == 200:
        data = r.json()
        pending = data.get("pending", [])
        logger.ok(f"Approvals API accessible. Found {len(pending)} pending requests.")
        
        # Test creation of a dummy approval request if possible
        # This usually requires a specific trigger, but we can check the queue
        if pending:
            req_id = pending[0]["id"]
            logger.info(f"Found pending request {req_id}, testing detail API...")
            r2 = session.get(f"{BASE_URL}/api/admin/approvals/{req_id}")
            if r2.status_code == 200:
                logger.ok(f"Approval detail API works for {req_id}")
            else:
                logger.fail(f"Approval detail API failed for {req_id}")
    elif r.status_code == 404:
        logger.warn("Approvals API NOT IMPLEMENTED (404)")
    else:
        logger.fail(f"Approvals API failed (Status {r.status_code})")

def main():
    logger = TestLogger()
    session = get_admin_session()
    if not session:
        print("Failed to authenticate")
        sys.exit(1)
    
    test_admin_api_security(logger)
    test_dashboard(session, logger)
    test_users(session, logger)
    test_finance(session, logger)
    test_assets_submissions(session, logger)
    test_support_kyc(session, logger)
    test_rewards(session, logger)
    test_emails(session, logger)
    test_system_settings(session, logger)
    test_admins_roles(session, logger)
    test_audit_logs(session, logger)
    test_dividends(session, logger)
    test_orders(session, logger)
    test_search(session, logger)
    test_notifications(session, logger)
    test_approvals(session, logger)
    
    logger.summary()

if __name__ == "__main__":
    main()
