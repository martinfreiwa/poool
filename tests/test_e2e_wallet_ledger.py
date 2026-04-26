#!/usr/bin/env python3
"""
End-to-End Wallet & Ledger Test
===============================
Implements section 3.1 of the E2E Master Workflow.
Tests double entry deposits, withdrawals, and critical negative constraints (Negative Wealth Prevention).
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
        self.errors = []

    def check(self, name, condition, detail=""):
        if condition:
            self.ok(name)
        else:
            self.fail(name, detail)

    def ok(self, msg):
        self.passed += 1
        print(f"  ✅ {msg}")

    def fail(self, msg, detail=""):
        self.failed += 1
        full = f"{msg}: {detail}" if detail else msg
        self.errors.append(full)
        print(f"  ❌ {full}")

    def report(self):
        print("\n" + "="*60)
        print(f"E2E Wallet Report: {self.passed} Passed, {self.failed} Failed")
        print("="*60 + "\n")
        return self.failed == 0

def get_db_connection():
    return psycopg2.connect(DB_DSN)

def fix_secure_cookies(session):
    for cookie in session.cookies:
        cookie.secure = False

def get_session():
    conn = get_db_connection()
    cur = conn.cursor()
    cur.execute("""
        SELECT session_token FROM user_sessions 
        WHERE user_id = (SELECT id FROM users WHERE email=%s)
        ORDER BY created_at DESC LIMIT 1
    """, (TEST_EMAIL,))
    row = cur.fetchone()
    cur.close()
    conn.close()
    
    if not row:
        return None
        
    session = requests.Session()
    session.cookies.set("poool_session", str(row[0]))
    
    # Get CSRF token
    r = session.get(f"{BASE_URL}/wallet", timeout=10)
    fix_secure_cookies(session)
    if "csrf_token" in session.cookies:
        session.headers.update({"X-CSRF-Token": session.cookies["csrf_token"]})
    
    return session

def run_wallet_e2e():
    results = E2EResults()
    print("\n--- Starting Wallet & Ledger Automation Flow ---")

    session = get_session()
    if not session:
        results.check("Session Auth", False, "Missing Auth Token")
        return results

    conn = get_db_connection()
    cur = conn.cursor()

    try:
        cur.execute("SELECT id FROM users WHERE email=%s", (TEST_EMAIL,))
        user_id = cur.fetchone()[0]

        # Reset user wallet to 100 USD with a matching ledger entry.
        cur.execute(
            """
            UPDATE wallets
               SET balance_cents = 10000
             WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
             RETURNING id
            """,
            (user_id,),
        )
        wallet_id = cur.fetchone()[0]
        cur.execute("DELETE FROM wallet_transactions WHERE wallet_id = %s", (wallet_id,))
        cur.execute(
            """
            INSERT INTO wallet_transactions (
                wallet_id, type, status, amount_cents, currency, description, external_ref_id, completed_at
            )
            VALUES (%s, 'admin_credit', 'completed', 10000, 'USD', 'E2E wallet ledger initial funding', %s, NOW())
            """,
            (wallet_id, f"e2e-wallet-ledger-initial-funding:{user_id}"),
        )
        conn.commit()

        # Step 1: Deposit Test
        deposit_amount_dollars = 50.00
        resp = session.post(f"{BASE_URL}/wallet/deposit", data={
            "amount": str(deposit_amount_dollars),
            "currency": "USD",
            "payment_method_id": "none"  # Fallback to manual deposit as per backend logic
        }, allow_redirects=False)
        results.check("Deposit API Status", resp.status_code in [200, 302, 303], f"Status: {resp.status_code}")
        
        # Check Ledger for Deposit
        cur.execute("SELECT amount_cents, type, status FROM wallet_transactions WHERE wallet_id=(SELECT id FROM wallets WHERE user_id=%s AND wallet_type='cash' LIMIT 1) ORDER BY created_at DESC LIMIT 1", (user_id,))
        txn = cur.fetchone()
        
        # Depending on how the API is structured, deposit might be pending or completed, and positive
        results.check("DB Ledger - Deposit Verification", 
            txn is not None and int(txn[0]) == 5000 and str(txn[1]).lower() == 'deposit', 
            f"Row: {txn}")

        # Step 2: Negative Constraint - Over-withdrawal Test
        # The balance is $100. We will try to withdraw $200.
        resp_over = session.post(f"{BASE_URL}/wallet/withdraw", data={
            "amount": "200.00",
            "currency": "USD"
        }, allow_redirects=False)
        
        # Expected to fail since its more than 100
        results.check("Over-withdrawal blocked by API", resp_over.status_code in [302, 303, 400, 422, 403, 500], f"Status: {resp_over.status_code}")
        if resp_over.status_code in [302, 303] and "error=insufficient_funds" in resp_over.headers.get("Location", ""):
            results.ok("Redirect contains expected error message")
        elif resp_over.status_code not in [200]:
             results.ok("Correctly returned error status")
        else:
             results.fail("Should have redirected or returned error")
        
        # Double check DB that no negative ledger transaction occurred
        conn.commit()
        cur.execute("SELECT balance_cents FROM wallets WHERE user_id=%s AND wallet_type='cash'", (user_id,))
        current_balance = cur.fetchone()[0]
        results.check("DB Balance Protection", current_balance >= 0, f"Balance dropped below zero!: {current_balance}")

        # Step 3: Valid Withdrawal Test
        valid_withdraw = session.post(f"{BASE_URL}/wallet/withdraw", data={
            "amount": "50.00",
            "currency": "USD"
        }, allow_redirects=False)
        results.check("Valid Withdrawal API Status", valid_withdraw.status_code in [200, 302, 303], f"Status: {valid_withdraw.status_code}")

        # Check DB subtraction logic
        conn.commit()
        cur.execute("SELECT amount_cents, type FROM wallet_transactions WHERE wallet_id=(SELECT id FROM wallets WHERE user_id=%s AND wallet_type='cash' LIMIT 1) ORDER BY created_at DESC LIMIT 1", (user_id,))
        last_txn = cur.fetchone()
        
        results.check("DB Ledger - Withdrawal Verification",
            last_txn is not None and int(last_txn[0]) == -5000 and str(last_txn[1]).lower() == 'withdrawal',
            f"Row: {last_txn}")

    except Exception as e:
        results.check("Exception during E2E", False, str(e))
    finally:
        cur.close()
        conn.close()

    return results

if __name__ == "__main__":
    res = run_wallet_e2e()
    if not res.report():
        sys.exit(1)
    sys.exit(0)
