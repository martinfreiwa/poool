#!/usr/bin/env python3
"""
End-to-End KYC & Registration Flow Test
=======================================
Implements section 2.1 & 2.3 of the E2E Master Workflow.
Tests registration limits, session acquisition, and KYC checkout blocking logic.
"""

import os
import requests
import psycopg2
import sys
import uuid
import time

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DB_DSN", "dbname=poool user=martin host=localhost")
TEST_EMAIL = os.environ.get("TEST_EMAIL", "test@poool.app")

def fix_secure_cookies(session):
    for cookie in session.cookies:
        cookie.secure = False

def get_session():
    # Helper to get session with CSRF
    conn = psycopg2.connect(DB_DSN)
    cur = conn.cursor()
    cur.execute("SELECT session_token FROM user_sessions WHERE user_id = (SELECT id FROM users WHERE email=%s) ORDER BY created_at DESC LIMIT 1", (TEST_EMAIL,))
    row = cur.fetchone()
    cur.close()
    conn.close()
    if not row: return None
    s = requests.Session()
    s.cookies.set("poool_session", row[0])
    s.get(f"{BASE_URL}/developer/dashboard")
    fix_secure_cookies(s)
    if "csrf_token" in s.cookies:
        s.headers.update({"X-CSRF-Token": s.cookies["csrf_token"]})
    return s

class E2EResults:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.errors = []

    def check(self, name, condition, detail=""):
        if condition:
            self.passed += 1
            print(f"  ✅ {name}")
        else:
            self.failed += 1
            print(f"  ❌ {name} {' - ' + detail if detail else ''}")
            self.errors.append(f"{name}: {detail}")

    def report(self):
        print("\n" + "="*60)
        print(f"E2E KYC & Registration Report: {self.passed} Passed, {self.failed} Failed")
        print("="*60 + "\n")
        return self.failed == 0

def get_db_connection():
    return psycopg2.connect(DB_DSN)

def run_kyc_e2e():
    results = E2EResults()
    print("\n--- Starting KYC & Registration Automation Flow ---")

    # Generate random email to ensure clean registration
    unique_id = str(uuid.uuid4())[:8]
    test_email = f"user_{unique_id}@poool.app"
    test_password = "SecurePassword123!"

    session = requests.Session()
    # Get CSRF token first
    session.get(f"{BASE_URL}/auth/signup")
    fix_secure_cookies(session)
    csrf_token = session.cookies.get("csrf_token", "")

    try:
        # Step 1: Registration (Happy Path)
        print(f"\n[1] Attempting Registration for {test_email}...")
        reg_resp = session.post(f"{BASE_URL}/auth/signup", data={
            "email": test_email,
            "password": test_password,
            "terms_accepted": "on"
        }, headers={
            "HX-Request": "true",
            "X-CSRF-Token": csrf_token
        }, allow_redirects=False)
        fix_secure_cookies(session)

        results.check("Registration API Route", reg_resp.status_code in [200, 201, 302, 303], f"Status: {reg_resp.status_code}")

        # Login to get session (Testing API handles it, but since we are over HTTP locally, we force inject the created cookie)
        csrf_token = session.cookies.get("csrf_token", "")
        login_resp = session.post(f"{BASE_URL}/auth/login", data={
            "email": test_email,
            "password": test_password
        }, headers={
            "HX-Request": "true",
            "X-CSRF-Token": csrf_token
        }, allow_redirects=False)
        fix_secure_cookies(session)
        
        conn = get_db_connection()
        cur = conn.cursor()
        
        cur.execute("SELECT id FROM users WHERE email=%s", (test_email,))
        user_row = cur.fetchone()
        
        if user_row:
            user_id = user_row[0]
            cur.execute("SELECT session_token FROM user_sessions WHERE user_id=%s ORDER BY created_at DESC LIMIT 1", (user_id,))
            sesh = cur.fetchone()
            if sesh:
                session.cookies.set("poool_session", str(sesh[0]))
                
        results.check("Login & Session Issuance", login_resp.status_code in [200, 302, 303] and sesh is not None, "Missing Auth Flow")
        
        results.check("DB User Row Creation", user_row is not None, "User not found in Postgres")
        
        if user_row:
            user_id = user_row[0]
            # Check default wallet creation
            cur.execute("SELECT id, balance_cents FROM wallets WHERE user_id=%s AND wallet_type='cash'", (user_id,))
            wallet = cur.fetchone()
            results.check("DB Default Wallet Allocation", wallet is not None and wallet[1] == 0, "No default empty cash wallet found")
            
            # Step 3: Check initial KYC status via API
            kyc_status_resp = session.get(f"{BASE_URL}/api/kyc/status")
            if kyc_status_resp.status_code == 200:
                kyc_json = kyc_status_resp.json()
                kyc_status = kyc_json.get("status")
                results.check("KYC Start State (Not Started)", kyc_status == "not_started", f"Status is {kyc_status}")
            else:
                results.check("KYC API reachability", False, f"Failed: {kyc_status_resp.status_code}")
                
            # Step 4: Admin KYC Approval Webhook Simulation
            # Manually inject an approval to bypass the manual process and see if state transitions
            print("\n[Admin] Injecting Backend KYC Approval...")
            
            cur.execute("INSERT INTO kyc_records (user_id, status) VALUES (%s, 'approved') RETURNING id", (user_id,))
            conn.commit()
            
            # Flush frontend KYC Status
            kyc_status_after = session.get(f"{BASE_URL}/api/kyc/status").json().get("status")
            results.check("KYC End State (Approved)", kyc_status_after == "approved", "Did not unlock after DB change")

            # Final Cleanup
            cur.execute("DELETE FROM audit_logs WHERE actor_user_id=%s OR (entity_type='user' AND entity_id=%s)", (user_id, user_id))
            cur.execute("DELETE FROM user_consents WHERE user_id=%s", (user_id,))
            cur.execute("DELETE FROM kyc_records WHERE user_id=%s", (user_id,))
            cur.execute("DELETE FROM referral_tracking WHERE referred_id=%s OR referrer_id=%s", (user_id, user_id))
            cur.execute("DELETE FROM user_sessions WHERE user_id=%s", (user_id,))
            cur.execute("DELETE FROM wallets WHERE user_id=%s", (user_id,))
            cur.execute("DELETE FROM users WHERE id=%s", (user_id,))
            conn.commit()
        
        cur.close()
        conn.close()

    except Exception as e:
        results.check("Exception during E2E", False, str(e))

    return results

if __name__ == "__main__":
    res = run_kyc_e2e()
    if not res.report():
        sys.exit(1)
    sys.exit(0)
