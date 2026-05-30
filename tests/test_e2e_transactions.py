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
from pathlib import Path

sys.path.append(str(Path(__file__).resolve().parents[1]))
from tests.e2e.conftest import cleanup_test_user, create_e2e_user

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DB_DSN", "dbname=poool user=martin host=localhost")
TEST_EMAIL = os.environ.get("TEST_EMAIL")

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

def get_session(user=None):
    if user:
        session = requests.Session()
        session.cookies.set("poool_session", str(user["session_token"]))
        return session

    if not TEST_EMAIL:
        return None

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
    
    user = create_e2e_user(
        email_prefix="e2e-transactions-script",
        display_name="E2E Transactions Script",
        roles=("investor",),
    )
    session = get_session(user)
    if not session:
        results.check("Authentication", False, "Missing valid session for user")
        cleanup_test_user(user["user_id"])
        return results

    try:
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
    finally:
        cleanup_test_user(user["user_id"])

    return results

if __name__ == "__main__":
    res = run_transactions_test()
    if not res.report():
        sys.exit(1)
    sys.exit(0)
