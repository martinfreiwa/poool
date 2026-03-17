#!/usr/bin/env python3
"""
POOOL Developer Dashboard – Comprehensive Test Suite
===================================================
Industry-standard testing for the developer dashboard, metrics, and assets management.

Run:  python3 tests/test_developer_dashboard.py
"""

import os
import sys
import psycopg2
import requests
from html.parser import HTMLParser

# Configuration
BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DATABASE_URL", "dbname=poool user=martin host=localhost")
DEV_EMAIL = os.environ.get("DEV_EMAIL", "test@poool.app") # Using same test email, will check role
DEV_PASSWORD = os.environ.get("DEV_PASSWORD", "TestPass123!")

class HTMLAnalyzer(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = []
        self.classes = []
        self.scripts = []
        self.links = []
        self.title = ""
        self._in_title = False

    def handle_starttag(self, tag, attrs):
        attr_dict = dict(attrs)
        if "id" in attr_dict: self.ids.append(attr_dict["id"])
        if "class" in attr_dict: self.classes.extend(attr_dict["class"].split())
        if tag == "script" and "src" in attr_dict: self.scripts.append(attr_dict["src"])
        if tag == "a" and "href" in attr_dict: self.links.append(attr_dict["href"])
        if tag == "title": self._in_title = True

    def handle_data(self, data):
        if self._in_title: self.title += data

    def handle_endtag(self, tag):
        if tag == "title": self._in_title = False

class TestResults:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.current_section = ""

    def section(self, name):
        self.current_section = name
        print(f"\n{'='*70}\n  {name}\n{'='*70}")

    def ok(self, msg):
        self.passed += 1
        print(f"  ✅ [PASS] {msg}")

    def warn(self, msg):
        print(f"  ⚠️ [WARN] {msg}")

    def fail(self, msg, detail=""):
        self.failed += 1
        print(f"  ❌ [FAIL] {msg}")
        if detail: print(f"     → {detail}")

def get_dev_session():
    session = requests.Session()
    # Strategy: Ensure user has developer role in DB
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        # Find a dev user or update test user to developer
        cur.execute("""
            INSERT INTO user_roles (user_id, role_id)
            SELECT u.id, r.id FROM users u, roles r
            WHERE u.email = %s AND r.name = 'developer'
            ON CONFLICT DO NOTHING
        """, (DEV_EMAIL,))
        conn.commit()
        
        cur.execute("""
            SELECT s.session_token FROM user_sessions s
            JOIN users u ON u.id = s.user_id
            WHERE u.email = %s AND s.expires_at > NOW()
            ORDER BY s.created_at DESC LIMIT 1
        """, (DEV_EMAIL,))
        row = cur.fetchone()
        cur.close()
        conn.close()
        
        if row:
            session.cookies.set("poool_session", row[0])
            r = session.get(f"{BASE_URL}/api/me")
            if r.status_code == 200 and r.json().get("role") == "developer":
                return session
    except Exception as e:
        print(f"Exception in get_dev_session: {e}")
    
    # Fallback to login
    print(f"Fallback to login for {DEV_EMAIL}")
    r = session.post(f"{BASE_URL}/auth/login", data={"email": DEV_EMAIL, "password": DEV_PASSWORD})
    if "poool_session" in session.cookies:
        return session
    return None

def test_developer_access(results):
    results.section("DEVELOPER ACCESS CONTROL")
    # Unauthenticated
    r = requests.get(f"{BASE_URL}/developer/dashboard", allow_redirects=False)
    if r.status_code in (302, 303, 401, 403):
        results.ok("Unauthenticated access blocked")
    else:
        results.fail("Unauthenticated access allowed", f"Status: {r.status_code}")

def test_developer_dashboard(session, results):
    results.section("DEVELOPER DASHBOARD COMPONENTS")
    r = session.get(f"{BASE_URL}/developer/dashboard")
    if r.status_code != 200:
        results.fail("Dashboard inaccessible", f"Status: {r.status_code}")
        return

    results.ok("Dashboard accessible (200 OK)")
    html = HTMLAnalyzer()
    html.feed(r.text)

    # Core Classes
    expected_classes = ["developer-dashboard-page", "developer-dashboard-sidebar"]
    for ecls in expected_classes:
        if ecls in html.classes: results.ok(f"Component .{ecls} present")
        else: results.fail(f"Component .{ecls} MISSING")

    # Core IDs
    expected_ids = ["metrics-section", "sales-chart-section", "dashboard-content-wrapper", "dashboard-main-content"]
    for eid in expected_ids:
        if eid in html.ids: results.ok(f"Component #{eid} present")
        else: results.fail(f"Component #{eid} MISSING")

    # Metrics Cards
    metrics = ["Total Assets", "Total Sales", "Total Investors", "New Investors", "Total Views"]
    for m in metrics:
        mid = f"metric-card-{m}"
        if mid in html.ids: results.ok(f"Metric Card: {m} found")
        else: results.warn(f"Metric Card: {m} MISSING (ID: '{mid}')")

    # CSS Integrity
    if "metric-number" in html.classes: results.ok("Metric numbers for animation found")
    else: results.fail("Class 'metric-number' MISSING", f"Found classes: {html.classes[:10]}...")

    # Chart & Table Headers
    if "sales-chart-card" in html.ids or "chart-container" in html.ids: results.ok("Sales chart element found")
    else: results.fail("Sales chart element MISSING")

    # Sidebar Links
    dev_links = ["/developer/dashboard", "/developer/assets"]
    for link in dev_links:
        if link in html.links: results.ok(f"Sidebar link to {link} found")
        else: results.fail(f"Sidebar link to {link} MISSING")

    # Check for profile switcher
    if any("profile-switcher" in str(c) for c in html.classes):
        results.ok("Profile switcher found")
    else:
        results.warn("Profile switcher MISSING (important for dev/investor toggle)")

def test_developer_apis(session, results):
    results.section("DEVELOPER API ENDPOINTS")
    
    # JSON API
    r = session.get(f"{BASE_URL}/api/developer/dashboard/stats")
    if r.status_code == 200:
        results.ok("/api/developer/dashboard/stats returns 200 OK")
        data = r.json()
        if "metrics" in data and "top_assets" in data:
            results.ok("API returns metrics and top_assets lists")
        else:
            results.fail("API JSON missing expected keys")
    else:
        results.fail(f"/api/developer/dashboard/stats failed with {r.status_code}")

    # HTMX Fragment Chart
    r = session.get(f"{BASE_URL}/developer/dashboard/fragments/chart?period=30d")
    if r.status_code == 200 and "sales-chart-card" in r.text:
        results.ok("/developer/dashboard/fragments/chart returns valid HTMX HTML")
    else:
        results.fail(f"/developer/dashboard/fragments/chart failed or missing chart element")

    # HTMX Fragment Assets
    r = session.get(f"{BASE_URL}/developer/dashboard/fragments/assets?period=all")
    if r.status_code == 200 and "developer-assets-wrapper" in r.text:
        results.ok("/developer/dashboard/fragments/assets returns valid HTMX HTML")
    else:
        results.fail(f"/developer/dashboard/fragments/assets failed or missing assets element")

def main():
    print("POOOL DEVELOPER DASHBOARD TEST SUITE")
    results = TestResults()
    
    test_developer_access(results)
    
    session = get_dev_session()
    if not session:
        print("❌ CRITICAL: Could not establish developer session.")
        sys.exit(1)
        
    test_developer_dashboard(session, results)
    test_developer_apis(session, results)
    
    print(f"\nSummary: {results.passed} passed, {results.failed} failed")
    sys.exit(0 if results.failed == 0 else 1)

if __name__ == "__main__":
    main()
