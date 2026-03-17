#!/usr/bin/env python3
"""
End-to-End Support Flow Test
============================
Tests the support dashboard and ticket submission APIs.
"""
import os
import requests
import psycopg2
import sys
import uuid

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
        print(f"E2E Support Test Report: {self.passed} Passed, {self.failed} Failed")
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
    
    # Get CSRF
    session.get(f"{BASE_URL}/support")
    fix_secure_cookies(session)
    if "csrf_token" in session.cookies:
        session.headers.update({"X-CSRF-Token": session.cookies["csrf_token"]})
        
    return session

def run_support_test():
    results = E2EResults()
    print("\n--- Testing Support Flow ---")
    
    session = get_session()
    if not session:
        results.check("Authentication", False, "Missing valid session for user")
        return results

    # 1. Test Support Page Rendering
    resp = session.get(f"{BASE_URL}/support")
    results.check("GET /support", resp.status_code == 200, f"Status: {resp.status_code}")
    
    html = resp.text.lower()
    results.check("Support Page Rendered", "support" in html or "ticket" in html, "Missing UI elements")

    # 2. Submit a Ticket via API
    # The endpoint consumes multipart/form-data. We can use requests files param to force it.
    ticket_subject = f"E2E Test Ticket {uuid.uuid4().hex[:6]}"
    
    # Needs multipart/form-data via requests' `files` parameter
    ticket_data = {
        "subject": (None, ticket_subject),
        "category": (None, "technical"),
        "priority": (None, "high"),
        "message": (None, "This is an automated E2E test ticket.")
    }
    
    submit_resp = session.post(f"{BASE_URL}/api/support/tickets", files=ticket_data)
    results.check("POST /api/support/tickets", submit_resp.status_code == 200, f"Status: {submit_resp.status_code}")

    # 3. Verify Ticket in DB
    conn = psycopg2.connect(DB_DSN)
    cur = conn.cursor()
    cur.execute("SELECT id, status FROM support_tickets WHERE subject = %s ORDER BY created_at DESC LIMIT 1", (ticket_subject,))
    ticket_row = cur.fetchone()

    results.check("DB Ticket Verification", ticket_row is not None and ticket_row[1] == 'open', f"Found: {ticket_row}")

    # 4. Clean up
    if ticket_row:
        cur.execute("DELETE FROM support_ticket_replies WHERE ticket_id = %s", (ticket_row[0],))
        cur.execute("DELETE FROM support_tickets WHERE id = %s", (ticket_row[0],))
        conn.commit()
    
    cur.close()
    conn.close()

    return results

if __name__ == "__main__":
    res = run_support_test()
    if not res.report():
        sys.exit(1)
    sys.exit(0)
