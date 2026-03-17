#!/usr/bin/env python3
"""
End-to-End Transactions History Test
====================================
Tests the transactions history page for presence and access control.
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
        print(f"E2E Transactions Report: {self.passed} Passed, {self.failed} Failed")
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

def run_transactions_test():
    results = E2EResults()
    print("\n--- Testing Transactions Page ---")
    
    session = get_session()
    if not session:
        results.check("Authentication", False, "Missing valid session for user")
        return results

    # 1. Test Transactions Page Base Load
    resp = session.get(f"{BASE_URL}/transactions")
    results.check("GET /transactions", resp.status_code == 200, f"Status: {resp.status_code}")
    
    html = resp.text.lower()
    results.check("Transactions Title", "transactions" in html or "history" in html, "Missing transactions reference")
    results.check("Filter Tabs Rendering", "wallet" in html or "investments" in html, "Missing filter elements")

    # 2. Check Unauthenticated Redirect
    anon_session = requests.Session()
    anon_resp = anon_session.get(f"{BASE_URL}/transactions", allow_redirects=False)
    results.check("Unauthenticated Guard Redirect", anon_resp.status_code in [302, 303], f"Status: {anon_resp.status_code}")

    return results

if __name__ == "__main__":
    res = run_transactions_test()
    if not res.report():
        sys.exit(1)
    sys.exit(0)
