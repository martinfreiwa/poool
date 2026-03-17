#!/usr/bin/env python3
"""
End-to-End Marketplace & Property Pages Test
============================================
Tests the marketplace listing and individual property detail pages.
"""
import os
import requests
import psycopg2
import sys

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DB_DSN", "dbname=poool user=martin host=localhost")
TEST_EMAIL = os.environ.get("TEST_EMAIL", "test@poool.app")

class E2EResults:
    def __init__(self):
        self.passed = 0
        self.failed = 0

    def check(self, name, condition, detail=""):
        if condition:
            self.passed += 1
            print(f"  ✅ {name}")
        else:
            self.failed += 1
            print(f"  ❌ {name} {' - ' + detail if detail else ''}")

    def report(self):
        print("\n" + "="*60)
        print(f"E2E Marketplace Report: {self.passed} Passed, {self.failed} Failed")
        print("="*60 + "\n")
        return self.failed == 0

def fix_secure_cookies(session):
    for cookie in session.cookies:
        cookie.secure = False

def get_session():
    conn = psycopg2.connect(DB_DSN)
    cur = conn.cursor()
    cur.execute("SELECT session_token FROM user_sessions WHERE user_id = (SELECT id FROM users WHERE email=%s) ORDER BY created_at DESC LIMIT 1", (TEST_EMAIL,))
    row = cur.fetchone()
    cur.close()
    conn.close()

    if not row:
        return None

    session = requests.Session()
    session.cookies.set("poool_session", str(row[0]))
    return session

def run_marketplace_test():
    results = E2EResults()
    print("\n--- Testing Marketplace Page ---")
    
    session = get_session()
    if not session:
        results.check("Authentication", False, "Missing valid session for user")
        return results

    # 1. Test Marketplace Listing
    resp = session.get(f"{BASE_URL}/marketplace")
    results.check("GET /marketplace", resp.status_code == 200, f"Status: {resp.status_code}")
    
    html = resp.text.lower()
    results.check("Marketplace Listing Rendered", "marketplace" in html, "Missing marketplace reference")

    # 2. Get the first property slug from DB
    conn = psycopg2.connect(DB_DSN)
    cur = conn.cursor()
    cur.execute("SELECT slug FROM assets WHERE published = true AND funding_status IN ('funding_open', 'funding_in_progress', 'funded') LIMIT 1")
    asset_row = cur.fetchone()
    cur.close()
    conn.close()

    if asset_row:
        slug = asset_row[0]
        # 3. Test Property Detail Page
        prop_url = f"{BASE_URL}/property/{slug}"
        prop_resp = session.get(prop_url)
        results.check(f"GET /property/{slug}", prop_resp.status_code == 200, f"Status: {prop_resp.status_code}")
        
        prop_html = prop_resp.text.lower()
        results.check("Property Detail Rendered", "financials" in prop_html or "investment" in prop_html, "Missing property details elements")
        results.check("Buy Action Available", "cart" in prop_html or "add to cart" in prop_html or "buy" in prop_html, "Missing buy button references")
    else:
        results.check("Asset Test Skipped", True, "No active assets in database")

    return results

if __name__ == "__main__":
    res = run_marketplace_test()
    if not res.report():
        sys.exit(1)
    sys.exit(0)
