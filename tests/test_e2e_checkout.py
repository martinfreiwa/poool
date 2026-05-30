#!/usr/bin/env python3
"""
End-to-End Checkout Flow Test
=============================
Implements section 3.3 of the E2E Master Workflow: The Atomic Checkout Flow.
Focuses on tracking data transitions from Cart Addition to Ledger Deduction.
"""

import os
import requests
import psycopg2
import sys
from pathlib import Path
from decimal import Decimal, ROUND_CEILING

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from tests.e2e.conftest import cleanup_test_user, create_e2e_user

# Configuration matching the environment
BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DB_DSN", "dbname=poool user=martin")
TEST_EMAIL = os.environ.get("TEST_EMAIL")

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
        print(f"E2E Checkout Report: {self.passed} Passed, {self.failed} Failed")
        if self.errors:
            print("\nCritical Failures:")
            for err in self.errors:
                print(f"  - {err}")
        print("="*60 + "\n")
        return self.failed == 0

def get_db_connection():
    return psycopg2.connect(DB_DSN)

def fix_secure_cookies(session):
    for cookie in session.cookies:
        cookie.secure = False

def get_session(user=None):
    """Retrieve the robust poool_session cookie for standard E2E testing."""
    if user:
        session = requests.Session()
        session.cookies.set("poool_session", str(user["session_token"]))
        session.get(f"{BASE_URL}/cart")
        fix_secure_cookies(session)
        if "csrf_token" in session.cookies:
            session.headers.update({"X-CSRF-Token": session.cookies["csrf_token"]})
        return session

    if not TEST_EMAIL:
        print("⚠️ TEST_EMAIL not set; use a disposable E2E user instead.")
        return None

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
        print(f"⚠️ No active session found for {TEST_EMAIL} in DB.")
        return None
        
    session = requests.Session()
    session.cookies.set("poool_session", str(row[0]))
    
    # Get CSRF
    session.get(f"{BASE_URL}/cart")
    fix_secure_cookies(session)
    if "csrf_token" in session.cookies:
        session.headers.update({"X-CSRF-Token": session.cookies["csrf_token"]})
        
    return session

def run_checkout_e2e():
    results = E2EResults()
    print("\n--- Starting Atomic Checkout Automation Flow ---")

    disposable_user = create_e2e_user(
        email_prefix="e2e-checkout-script",
        cash_balance_cents=50_000_000,
        kyc_status="approved",
    )
    session = get_session(disposable_user)
    if not session:
        results.check("Session Auth Setup", False, "Missing Auth Token Data")
        return results

    conn = get_db_connection()
    cur = conn.cursor()
    asset_id = None
    original_tokens_available = None
    original_funding_status = None

    try:
        # Step 1: Pre-condition Checks (Get IDs and inject funds)
        user_id = disposable_user["user_id"]

        # Ensure the user has money and matching ledger state, then delete any stale cart items
        cur.execute(
            """
            UPDATE wallets
               SET balance_cents = 50000000
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
            VALUES (%s, 'admin_credit', 'completed', 50000000, 'USD', 'E2E checkout initial wallet funding', %s, NOW())
            """,
            (wallet_id, f"e2e-checkout-initial-funding:{user_id}"),
        )
        cur.execute("DELETE FROM cart_items WHERE user_id = %s", (user_id,))
        
        # Grab a mock asset that is funding_in_progress
        cur.execute("SELECT id, token_price_cents, tokens_total, tokens_available, funding_status FROM assets WHERE funding_status = 'funding_in_progress' AND tokens_available > 0 LIMIT 1")
        asset = cur.fetchone()
        
        if not asset:
            results.check("Asset Selection", False, "No active asset available for testing")
            return results
            
        asset_id, token_price_cents, tokens_total, tokens_available, original_funding_status = asset
        original_tokens_available = tokens_available
        conn.commit()

        qty_to_buy = 2
        investment_amount_dollars = (token_price_cents * qty_to_buy) / 100.0

        # Step 2: Add Asset to Cart (API) via Form submission
        resp = session.post(f"{BASE_URL}/cart/add", data={
            "property_id": str(asset_id),
            "investment_amount": str(investment_amount_dollars)
        })
        results.check("Add Item to Cart API", resp.status_code in [200, 302, 303], f"Status: {resp.status_code}")

        # Step 3: Verify Addition stored perfectly in database
        cur.execute("SELECT tokens_quantity FROM cart_items WHERE user_id=%s AND asset_id=%s", (user_id, asset_id))
        cart_row = cur.fetchone()
        results.check("DB Cart Allocation", cart_row is not None and cart_row[0] == qty_to_buy, f"Item missing or wrong quantity in DB: got {cart_row}")

        # Prep variables to check the checkout delta
        cur.execute("SELECT balance_cents FROM wallets WHERE user_id=%s AND wallet_type='cash'", (user_id,))
        initial_balance_cents = int(cur.fetchone()[0])

        expected_subtotal_cents = token_price_cents * qty_to_buy
        cur.execute("SELECT value FROM platform_settings WHERE key = 'platform_fee_percent'")
        fee_row = cur.fetchone()
        fee_pct = Decimal(str(fee_row[0])) if fee_row else Decimal("0")
        expected_fee_cents = int(
            (Decimal(expected_subtotal_cents) * fee_pct / Decimal("100"))
            .to_integral_value(rounding=ROUND_CEILING)
        )
        expected_total_cost_cents = expected_subtotal_cents + expected_fee_cents
        expected_balance_after_cents = initial_balance_cents - expected_total_cost_cents

        # Step 4: Execute Full Atomic Checkout POST
        checkout_resp = session.post(f"{BASE_URL}/checkout", data={
            "payment_method": "wallet",
            "payment_currency": "USD"
        })
        results.check("Execute Atomic Checkout API", checkout_resp.status_code in [200, 302, 303], f"Status: {checkout_resp.status_code}")

        # Step 5: Post-Checkout Database Assertions (The critical ACID checks)
        conn.commit() # Refresh snapshot
        
        # 5.1 Wallet Update check
        cur.execute("SELECT balance_cents FROM wallets WHERE user_id=%s AND wallet_type='cash'", (user_id,))
        new_balance_cents = int(cur.fetchone()[0])
        results.check("DB Wallet Deduction", new_balance_cents == expected_balance_after_cents, f"Expected {expected_balance_after_cents}, got {new_balance_cents}")

        # 5.2 Transaction Logging Pattern Check
        cur.execute("SELECT amount_cents, type, status FROM wallet_transactions WHERE wallet_id=(SELECT id FROM wallets WHERE user_id=%s AND wallet_type='cash' LIMIT 1) ORDER BY created_at DESC LIMIT 1", (user_id,))
        txn = cur.fetchone()
        
        # Order payment is logged as "purchase" type. The deduction includes
        # the platform fee because the wallet pays the grand total, not only
        # the asset subtotal.
        results.check("DB Wallet Ledger Accuracy", 
            txn is not None and int(txn[0]) == -expected_total_cost_cents and str(txn[1]).lower() == 'purchase', 
            f"Ledger Row mismatch: {txn}")

        # 5.3 Cart Flush Check
        cur.execute("SELECT count(*) FROM cart_items WHERE user_id=%s", (user_id,))
        cart_count = cur.fetchone()[0]
        results.check("DB Cart Flush", cart_count == 0, "Cart Items were not deleted")

        # 5.4 Ownership Record Generation
        cur.execute("SELECT tokens_owned FROM investments WHERE user_id=%s AND asset_id=%s", (user_id, asset_id))
        inv_row = cur.fetchone()
        results.check("DB Investment Ownership Validation", inv_row is not None and inv_row[0] >= qty_to_buy, "No investment token allocated")

    except Exception as e:
        results.check("E2E Test Exception", False, f"Exception occurred: {str(e)}")
    finally:
        if asset_id is not None and original_tokens_available is not None:
            try:
                cur.execute(
                    "UPDATE assets SET tokens_available = %s, funding_status = %s WHERE id = %s",
                    (original_tokens_available, original_funding_status, asset_id),
                )
                conn.commit()
            except Exception:
                conn.rollback()
        cur.close()
        conn.close()
        cleanup_test_user(disposable_user["user_id"])

    return results

if __name__ == "__main__":
    res = run_checkout_e2e()
    if not res.report():
        sys.exit(1)
    sys.exit(0)
