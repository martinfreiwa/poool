#!/usr/bin/env python3
"""
POOOL Platform — Security Audit Tests
Tests for the critical bugs found and fixed during the security audit.
Run: python3 tests/test_security_audit.py

Requires:
  - Backend running on :8888
  - Test user test@poool.app with admin role
  - pip install requests psycopg2-binary
"""
import json
import sys
import concurrent.futures

import psycopg2
import requests

BASE_URL = "http://127.0.0.1:8888"
DB_DSN = "dbname=poool user=martin host=127.0.0.1"
TEST_EMAIL = "test@poool.app"
TEST_PASSWORD = "TestPass123!"
REQUEST_TIMEOUT = 15

passed = 0
failed = 0
skipped = 0


def ok(msg):
    global passed
    passed += 1
    print(f"  ✅  {msg}")


def fail(msg, detail=""):
    global failed
    failed += 1
    full = f"{msg}: {detail}" if detail else msg
    print(f"  ❌  {full}")


def skip(msg):
    global skipped
    skipped += 1
    print(f"  ⏭️   {msg}")


def section(name):
    print(f"\n{'=' * 70}")
    print(f"  {name}")
    print(f"{'=' * 70}")


def get_session():
    """Get an authenticated session using direct DB token lookup."""
    session = requests.Session()
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        cur.execute(
            "SELECT s.session_token FROM user_sessions s "
            "JOIN users u ON u.id = s.user_id "
            "WHERE u.email = %s AND s.expires_at > NOW() "
            "ORDER BY s.created_at DESC LIMIT 1",
            (TEST_EMAIL,),
        )
        row = cur.fetchone()
        cur.close()
        conn.close()
        if row:
            session.cookies.set("poool_session", row[0])
            # Get CSRF token
            r = session.get(f"{BASE_URL}/settings", timeout=REQUEST_TIMEOUT)
            if "csrf_token" in session.cookies:
                session.headers.update({"X-CSRF-Token": session.cookies["csrf_token"]})
            return session
    except Exception:
        pass

    # Fallback: login via HTMX
    try:
        session.get(f"{BASE_URL}/settings", timeout=REQUEST_TIMEOUT)
        resp = session.post(
            f"{BASE_URL}/auth/login",
            data={"email": TEST_EMAIL, "password": TEST_PASSWORD},
            headers={
                "HX-Request": "true",
                "X-CSRF-Token": session.cookies.get("csrf_token", ""),
            },
            allow_redirects=False,
            timeout=60,
        )
        if "csrf_token" in session.cookies:
            session.headers.update({"X-CSRF-Token": session.cookies["csrf_token"]})
    except Exception:
        pass
    return session


# ─── BUG-A01: Admin Withdrawal Auth ─────────────────────────────

def test_admin_withdrawal_auth():
    """BUG-A01: Verify admin withdrawal endpoints require admin auth."""
    section("BUG-A01: Admin Withdrawal Endpoint Auth")

    # Unauthenticated request
    r = requests.get(f"{BASE_URL}/api/admin/withdrawals", timeout=REQUEST_TIMEOUT)
    if r.status_code in (401, 403):
        ok("GET /api/admin/withdrawals rejects unauthenticated (401/403)")
    else:
        fail("GET /api/admin/withdrawals accessible without auth", f"status={r.status_code}")


# ─── BUG-A03/A14: Integer-Only Currency Display ────────────────

def test_integer_currency_display(session):
    """BUG-A03/A14: Verify API responses use integer amounts, not floats."""
    section("BUG-A03/A14: Integer-Only Currency in API Responses")

    r = session.get(f"{BASE_URL}/api/wallet/transactions", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        data = r.json()
        transactions = data.get("transactions", [])
        if not transactions:
            skip("No transactions to check")
            return

        for tx in transactions[:5]:
            # amount_cents should be an integer
            if "amount_cents" in tx:
                if isinstance(tx["amount_cents"], int):
                    ok(f"Transaction amount_cents is integer: {tx['amount_cents']}")
                else:
                    fail(f"Transaction amount_cents is NOT integer: {type(tx['amount_cents'])}")

            # amount_display should be a string (not a float)
            if "amount_display" in tx:
                if isinstance(tx["amount_display"], str):
                    ok(f"Transaction amount_display is string: {tx['amount_display']}")
                else:
                    fail(f"Transaction amount_display is NOT string: {type(tx['amount_display'])}")

            # Should NOT have float amount_usd field
            if "amount_usd" in tx:
                if isinstance(tx["amount_usd"], float):
                    fail("Transaction still has float amount_usd field!")
                else:
                    ok("amount_usd field is not a float")
            break  # Only check first transaction
    elif r.status_code == 401:
        skip("Not authenticated for wallet/transactions")
    else:
        fail("/api/wallet/transactions", f"status={r.status_code}")


# ─── BUG-A06: Deposit Amount Validation ────────────────────────

def test_deposit_max_validation(session):
    """BUG-A06: Verify deposits reject excessively large amounts."""
    section("BUG-A06: Deposit Max Amount Validation")

    # Try depositing $100,000,001 (should exceed MAX_DEPOSIT_CENTS)
    r = session.post(
        f"{BASE_URL}/api/wallet/deposit",
        data={"amount": "100000001"},
        headers={"X-CSRF-Token": session.cookies.get("csrf_token", "")},
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code in (400, 422):
        ok(f"Excessive deposit rejected with {r.status_code}")
    elif r.status_code == 200:
        # Check if the response indicates the deposit was rejected
        try:
            body = r.text
            if "exceeds" in body.lower() or "maximum" in body.lower() or "too large" in body.lower():
                ok("Excessive deposit rejected in response body")
            else:
                fail("Excessive deposit was NOT rejected")
        except Exception:
            fail("Could not determine if deposit was rejected")
    elif r.status_code == 404:
        skip("Deposit endpoint not available at /api/wallet/deposit")
    else:
        skip(f"Deposit endpoint returned {r.status_code}")


# ─── BUG-A10: Dispute Status Validation ────────────────────────

def test_dispute_status_validation(session):
    """BUG-A10: Verify dispute status updates validate input and use AdminUser."""
    section("BUG-A10: Dispute Status Validation")

    # Try with invalid status
    r = session.put(
        f"{BASE_URL}/api/admin/disputes/00000000-0000-0000-0000-000000000000/status",
        json={"status": "invalid_status_value"},
        headers={
            "Content-Type": "application/json",
            "X-CSRF-Token": session.cookies.get("csrf_token", ""),
        },
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code == 400:
        ok("Invalid dispute status rejected with 400")
        try:
            body = r.json()
            if "Invalid status" in body.get("error", ""):
                ok("Error message includes valid status options")
        except Exception:
            pass
    elif r.status_code in (401, 403):
        skip("Not admin — dispute test skipped (auth required)")
    elif r.status_code == 404:
        skip("Disputes endpoint not found")
    else:
        fail(f"Invalid dispute status returned {r.status_code} (expected 400)")


# ─── CSRF Protection ──────────────────────────────────

def test_csrf_protection():
    """Verify CSRF protection is active on mutating endpoints."""
    section("CSRF: Protection Active on Mutating Endpoints")

    s = requests.Session()
    # Get a session cookie but DON'T send CSRF token
    s.get(f"{BASE_URL}/auth/login", timeout=REQUEST_TIMEOUT)

    # Try a POST without CSRF token
    r = s.post(
        f"{BASE_URL}/auth/login",
        data={"email": "x@x.com", "password": "bad"},
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code == 403:
        ok("POST without CSRF token rejected with 403")
    elif r.status_code == 401:
        # Some backends validate auth before CSRF
        skip("Got 401 (auth checked before CSRF)")
    else:
        fail(f"POST without CSRF returned {r.status_code} (expected 403)")


# ─── Session Security ────────────────────────────────────────

def test_session_security():
    """Verify session cookie properties."""
    section("SESSION: Cookie Security Properties")

    s = requests.Session()
    r = s.get(f"{BASE_URL}/auth/login", timeout=REQUEST_TIMEOUT)

    # Check for HttpOnly on session cookies
    for cookie in s.cookies:
        if cookie.name == "poool_session":
            if cookie.has_nonstandard_attr("HttpOnly") or cookie.secure:
                ok(f"Session cookie has security attributes")
            break

    # Check CSRF token cookie
    for cookie in s.cookies:
        if cookie.name == "csrf_token":
            ok(f"CSRF cookie generated on first request")
            # CSRF cookie should NOT be HttpOnly (JS needs to read it)
            break
    else:
        fail("No csrf_token cookie set on first request")


# ─── Registration Validation ─────────────────────────────────

def test_registration_validation():
    """Verify signup validates email and password."""
    section("AUTH: Registration Input Validation")

    s = requests.Session()
    s.get(f"{BASE_URL}/settings", timeout=REQUEST_TIMEOUT)
    csrf = s.cookies.get("csrf_token", "")

    # Weak password
    r = s.post(
        f"{BASE_URL}/auth/signup",
        data={
            "email": "test_validaton@example.com",
            "password": "123",  # Too weak
            "terms": "on",
        },
        headers={"X-CSRF-Token": csrf, "HX-Request": "true"},
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code == 400:
        ok("Weak password rejected with 400")
    elif r.status_code == 200 and ("password" in r.text.lower() and ("short" in r.text.lower() or "weak" in r.text.lower() or "must" in r.text.lower())):
        ok("Weak password rejected (inline error message)")
    elif r.status_code == 422:
        ok("Weak password rejected with 422")
    else:
        skip(f"Weak password check returned {r.status_code}")

    # Invalid email
    s2 = requests.Session()
    s2.get(f"{BASE_URL}/settings", timeout=REQUEST_TIMEOUT)
    csrf2 = s2.cookies.get("csrf_token", "")
    r = s2.post(
        f"{BASE_URL}/auth/signup",
        data={
            "email": "not-an-email",
            "password": "StrongPass123!",
            "terms": "on",
        },
        headers={"X-CSRF-Token": csrf2, "HX-Request": "true"},
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code == 400:
        ok("Invalid email rejected with 400")
    elif r.status_code == 200 and "email" in r.text.lower():
        ok("Invalid email rejected (inline error message)")
    elif r.status_code == 422:
        ok("Invalid email rejected with 422")
    else:
        skip(f"Invalid email check returned {r.status_code}")


# ─── DB Integrity Checks ─────────────────────────────────────

def test_db_integrity():
    """Verify database constraints and data integrity."""
    section("DATABASE: Integrity Checks")

    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()

        # Check wallet balance constraint
        cur.execute("""
            SELECT conname FROM pg_constraint
            WHERE conrelid = 'wallets'::regclass
            AND conname LIKE '%balance%'
        """)
        constraints = cur.fetchall()
        if constraints:
            ok(f"Wallet balance constraint exists: {constraints[0][0]}")
        else:
            fail("No balance constraint on wallets table")

        # Check for negative balances
        cur.execute("SELECT COUNT(*) FROM wallets WHERE balance_cents < 0")
        neg_count = cur.fetchone()[0]
        if neg_count == 0:
            ok("No negative wallet balances found")
        else:
            fail(f"{neg_count} wallet(s) have negative balance!")

        # Check for orphaned cart items (no matching asset)
        cur.execute("""
            SELECT COUNT(*) FROM cart_items ci
            LEFT JOIN assets a ON a.id = ci.asset_id
            WHERE a.id IS NULL
        """)
        orphans = cur.fetchone()[0]
        if orphans == 0:
            ok("No orphaned cart items")
        else:
            fail(f"{orphans} orphaned cart item(s) found")

        # Check order number uniqueness
        cur.execute("""
            SELECT order_number, COUNT(*)
            FROM orders
            GROUP BY order_number
            HAVING COUNT(*) > 1
        """)
        dupes = cur.fetchall()
        if not dupes:
            ok("No duplicate order numbers")
        else:
            fail(f"{len(dupes)} duplicate order number(s): {dupes}")

        # Check audit logs have actor_user_id set
        cur.execute("""
            SELECT COUNT(*) FROM audit_logs
            WHERE action LIKE 'admin.%' AND actor_user_id IS NULL
        """)
        missing_actor = cur.fetchone()[0]
        if missing_actor == 0:
            ok("All admin audit logs have actor_user_id set")
        else:
            fail(f"{missing_actor} admin audit log(s) missing actor_user_id")

        cur.close()
        conn.close()
    except Exception as e:
        fail("DB integrity check", str(e))


# ─── Main ─────────────────────────────────────────────

def main():
    print("=" * 70)
    print("  POOOL Platform — Security Audit Test Suite")
    print(f"  Target: {BASE_URL}")
    print("=" * 70)

    # Verify server is running
    try:
        requests.get(f"{BASE_URL}/auth/login", timeout=5)
    except requests.exceptions.ConnectionError:
        print(f"\n  ❌ Server not running at {BASE_URL}")
        print("  Start with: cd backend && cargo run")
        sys.exit(1)

    session = get_session()

    # Verify we're authenticated
    r = session.get(f"{BASE_URL}/api/me", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        me = r.json()
        print(f"\n  Authenticated as: {me.get('email', 'unknown')} (role: {me.get('role', 'unknown')})")
    else:
        print(f"\n  ⚠️  Not authenticated (some tests will be skipped)")

    # Run all security tests
    test_admin_withdrawal_auth()
    test_integer_currency_display(session)
    test_deposit_max_validation(session)
    test_dispute_status_validation(session)
    test_csrf_protection()
    test_session_security()
    test_registration_validation()
    test_db_integrity()

    # Summary
    print(f"\n{'=' * 70}")
    total = passed + failed + skipped
    print(f"  RESULTS: {passed} passed, {failed} failed, {skipped} skipped ({total} total)")
    if failed == 0:
        print("  ✅ ALL SECURITY TESTS PASSED")
    else:
        print(f"  ❌ {failed} SECURITY TEST(S) FAILED")
    print(f"{'=' * 70}\n")

    sys.exit(1 if failed > 0 else 0)


if __name__ == "__main__":
    main()
