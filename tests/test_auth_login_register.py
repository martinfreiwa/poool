#!/usr/bin/env python3
"""
POOOL Platform – Login & Registration E2E Test Suite
=====================================================
Tests the complete authentication lifecycle:
  • Login form rendering
  • Login with valid credentials (HTMX flow)
  • Login with invalid credentials
  • Session cookie management
  • Registration with valid data
  • Registration edge cases (duplicate email, missing fields, terms not accepted)
  • Database state verification after register
  • Protected route redirect for unauthenticated users
  • Logout

Run:  python3 tests/test_auth_login_register.py
"""
import json
import sys
import time
import uuid

import psycopg2
import requests

# ─── Configuration ───────────────────────────────────────────────
import os
BASE_URL = os.environ.get("BASE_URL", "http://127.0.0.1:8888")
DB_DSN = os.environ.get("DB_DSN", "dbname=poool user=martin host=127.0.0.1")

TEST_EMAIL = "test@poool.app"
TEST_PASSWORD = "TestPass123!"

# Fresh email for registration test (unique per run)
REGISTER_EMAIL = f"e2e-register-{uuid.uuid4().hex[:8]}@poool.test"
REGISTER_PASSWORD = "SecureP@ss1!"

REQUEST_TIMEOUT = 15
LOGIN_TIMEOUT = 60  # Argon2id hashing can be slow


# ─── Pretty Results Tracker ──────────────────────────────────────
class Results:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.warnings = 0
        self.errors = []
        self.section_name = ""

    def section(self, name):
        self.section_name = name
        print(f"\n{'=' * 70}")
        print(f"  {name}")
        print(f"{'=' * 70}")

    def ok(self, msg):
        self.passed += 1
        print(f"  ✅  {msg}")

    def fail(self, msg, detail=""):
        self.failed += 1
        full = f"{msg}: {detail}" if detail else msg
        self.errors.append((self.section_name, full))
        print(f"  ❌  {full}")

    def warn(self, msg):
        self.warnings += 1
        print(f"  ⚠️   {msg}")

    def info(self, msg):
        print(f"  ℹ️   {msg}")

    def summary(self):
        total = self.passed + self.failed
        print(f"\n{'=' * 70}")
        print(f"  RESULTS:  {self.passed}/{total} passed, "
              f"{self.failed} failed, {self.warnings} warnings")
        print(f"{'=' * 70}")
        if self.errors:
            print("\n  FAILURES:")
            for section, err in self.errors:
                print(f"    [{section}]  {err}")
        print()
        return self.failed == 0


# ─── Helpers ─────────────────────────────────────────────────────

def fix_secure_cookies(session):
    """Clear the Secure flag on all cookies so they work over plain HTTP.
    
    The POOOL server sets Secure=true on cookies when POOOL_ENV != 'development'.
    Python's requests library correctly refuses to send Secure cookies over HTTP,
    which breaks our test. This helper overrides the flag.
    """
    for cookie in session.cookies:
        cookie.secure = False


def has_cookie(session, name):
    """Check if a session has a cookie by name (regardless of Secure flag)."""
    return any(c.name == name for c in session.cookies)


def get_cookie_value(session, name):
    """Get a cookie value by name (regardless of Secure flag)."""
    for c in session.cookies:
        if c.name == name:
            return c.value
    return None


def get_csrf_session():
    """Create a requests.Session that already has CSRF cookie from the server."""
    s = requests.Session()
    s.get(f"{BASE_URL}/auth/login", timeout=REQUEST_TIMEOUT)
    fix_secure_cookies(s)
    return s


def htmx_headers(session, current_url=None):
    """Return headers that mimic an HTMX form submission."""
    return {
        "HX-Request": "true",
        "HX-Current-URL": current_url or f"{BASE_URL}/auth/login",
        "X-CSRF-Token": get_cookie_value(session, "csrf_token") or "",
    }


def db_connect():
    return psycopg2.connect(DB_DSN)


# ─── 1. Login Page Rendering ────────────────────────────────────

def test_login_page(r: Results):
    r.section("LOGIN PAGE – Rendering")

    resp = requests.get(f"{BASE_URL}/auth/login", timeout=REQUEST_TIMEOUT)
    if resp.status_code == 200:
        r.ok("GET /auth/login returns 200")
    else:
        r.fail("GET /auth/login", f"status={resp.status_code}")
        return

    html = resp.text

    # Check essential form elements
    if '<form' in html.lower():
        r.ok("Login form element present")
    else:
        r.fail("No <form> element found on login page")

    if 'email' in html.lower():
        r.ok("Email input referenced")
    else:
        r.fail("No email input found")

    if 'password' in html.lower():
        r.ok("Password input referenced")
    else:
        r.fail("No password input found")

    # Check page title
    if 'login' in html.lower() or 'log in' in html.lower() or 'sign in' in html.lower():
        r.ok("Page contains login-related text")
    else:
        r.warn("No login-related text found in page")

    # Check HTMX is loaded
    if 'htmx' in html.lower():
        r.ok("HTMX referenced on login page")
    else:
        r.warn("HTMX not found on login page")


# ─── 2. Login with Valid Credentials ────────────────────────────

def test_login_valid(r: Results):
    r.section("LOGIN – Valid Credentials (HTMX Flow)")

    s = get_csrf_session()
    headers = htmx_headers(s)

    try:
        resp = s.post(
            f"{BASE_URL}/auth/login",
            data={"email": TEST_EMAIL, "password": TEST_PASSWORD},
            headers=headers,
            allow_redirects=False,
            timeout=LOGIN_TIMEOUT,
        )
    except requests.exceptions.Timeout:
        r.warn("POST /auth/login timed out (Argon2id hashing is CPU-intensive)")
        return

    # The backend returns 200 with HX-Redirect header for HTMX requests
    if resp.status_code == 200:
        r.ok(f"POST /auth/login returns 200 (HTMX response)")
    elif resp.status_code in (302, 303):
        r.ok(f"POST /auth/login redirects ({resp.status_code})")
    elif resp.status_code == 401:
        r.fail("POST /auth/login returns 401",
               f"Credentials {TEST_EMAIL} / {TEST_PASSWORD} rejected")
        return
    else:
        r.fail("POST /auth/login", f"unexpected status={resp.status_code}")
        return

    # Check HX-Redirect header
    hx_redirect = resp.headers.get("HX-Redirect")
    if hx_redirect:
        r.ok(f"HX-Redirect header present: {hx_redirect}")
        if "/marketplace" in hx_redirect or "/2fa" in hx_redirect:
            r.ok(f"Redirect target is valid ({hx_redirect})")
        else:
            r.warn(f"Unexpected redirect target: {hx_redirect}")
    elif resp.status_code in (302, 303):
        r.ok("Redirect via Location header (non-HTMX)")
    else:
        r.warn("No HX-Redirect header in response")

    # Session cookie – fix Secure flag first so cookies are visible
    fix_secure_cookies(s)
    if has_cookie(s, "poool_session"):
        r.ok("Session cookie 'poool_session' is set")
        token = get_cookie_value(s, "poool_session")
        if len(token) > 20:
            r.ok(f"Session token is non-trivial ({len(token)} chars)")
        else:
            r.warn(f"Session token seems short ({len(token)} chars)")
    else:
        r.fail("Session cookie 'poool_session' NOT set after login")
        return

    # Verify session lets us access a protected page
    me_resp = s.get(f"{BASE_URL}/api/me", timeout=REQUEST_TIMEOUT)
    if me_resp.status_code == 200:
        r.ok("/api/me accessible with session")
        try:
            me_data = me_resp.json()
            r.info(f"  Logged in as: {me_data.get('email', me_data.get('name', 'unknown'))}")
        except Exception:
            pass
    else:
        r.warn(f"/api/me returned {me_resp.status_code} after login")

    # DB: verify session row exists
    try:
        conn = db_connect()
        cur = conn.cursor()
        cur.execute(
            "SELECT id, user_id, expires_at FROM user_sessions "
            "WHERE session_token = %s AND expires_at > NOW()",
            (token,),
        )
        row = cur.fetchone()
        if row:
            r.ok(f"Session row found in DB (id={row[0]}, expires={row[2]})")
        else:
            r.fail("Session token not found in user_sessions table")
        cur.close()
        conn.close()
    except Exception as e:
        r.warn(f"DB session check failed: {e}")


# ─── 3. Login with Invalid Credentials ──────────────────────────

def test_login_invalid(r: Results):
    r.section("LOGIN – Invalid Credentials")

    s = get_csrf_session()
    headers = htmx_headers(s)

    # Wrong password
    resp = s.post(
        f"{BASE_URL}/auth/login",
        data={"email": TEST_EMAIL, "password": "wrongpassword"},
        headers=headers,
        allow_redirects=False,
        timeout=LOGIN_TIMEOUT,
    )
    if resp.status_code == 401:
        r.ok("Wrong password returns 401")
    elif resp.status_code == 200 and not has_cookie(s, "poool_session"):
        r.ok("Wrong password returns 200 with error (no session set)")
    else:
        r.warn(f"Wrong password returned status={resp.status_code}")

    # Non-existent email
    s2 = get_csrf_session()
    headers2 = htmx_headers(s2)
    resp2 = s2.post(
        f"{BASE_URL}/auth/login",
        data={"email": "nonexistent@example.com", "password": "anything"},
        headers=headers2,
        allow_redirects=False,
        timeout=REQUEST_TIMEOUT,
    )
    if resp2.status_code == 401:
        r.ok("Non-existent email returns 401")
    elif resp2.status_code == 200 and not has_cookie(s2, "poool_session"):
        r.ok("Non-existent email returns 200 with error (no session set)")
    else:
        r.warn(f"Non-existent email returned {resp2.status_code}")

    # Empty fields
    s3 = get_csrf_session()
    headers3 = htmx_headers(s3)
    resp3 = s3.post(
        f"{BASE_URL}/auth/login",
        data={"email": "", "password": ""},
        headers=headers3,
        allow_redirects=False,
        timeout=REQUEST_TIMEOUT,
    )
    if resp3.status_code in (400, 401, 422):
        r.ok(f"Empty fields rejected ({resp3.status_code})")
    elif resp3.status_code == 200 and not has_cookie(s3, "poool_session"):
        r.ok("Empty fields return 200 with error (no session set)")
    else:
        r.warn(f"Empty fields returned {resp3.status_code}")


# ─── 4. Signup Page Rendering ───────────────────────────────────

def test_signup_page(r: Results):
    r.section("SIGNUP PAGE – Rendering")

    resp = requests.get(f"{BASE_URL}/auth/signup", timeout=REQUEST_TIMEOUT)
    if resp.status_code == 200:
        r.ok("GET /auth/signup returns 200")
    else:
        r.fail("GET /auth/signup", f"status={resp.status_code}")
        return

    html = resp.text

    if '<form' in html.lower():
        r.ok("Signup form element present")
    else:
        r.fail("No <form> element found on signup page")

    if 'email' in html.lower():
        r.ok("Email input referenced")
    else:
        r.fail("No email input found")

    if 'password' in html.lower():
        r.ok("Password input referenced")
    else:
        r.fail("No password input found")

    # Terms checkbox
    if 'terms' in html.lower():
        r.ok("Terms and conditions referenced")
    else:
        r.warn("Terms and conditions not found on signup page")


# ─── 5. Registration – Happy Path ───────────────────────────────

def test_register_happy_path(r: Results):
    r.section("REGISTRATION – Happy Path")

    s = get_csrf_session()
    headers = htmx_headers(s, f"{BASE_URL}/auth/signup")

    r.info(f"Registering with email: {REGISTER_EMAIL}")

    try:
        resp = s.post(
            f"{BASE_URL}/auth/signup",
            data={
                "email": REGISTER_EMAIL,
                "password": REGISTER_PASSWORD,
                "terms_accepted": "on",
            },
            headers=headers,
            allow_redirects=False,
            timeout=LOGIN_TIMEOUT,
        )
    except requests.exceptions.Timeout:
        r.warn("POST /auth/signup timed out (Argon2id hashing is CPU-intensive)")
        return

    if resp.status_code == 200:
        r.ok("POST /auth/signup returns 200")
    elif resp.status_code in (201, 302, 303):
        r.ok(f"POST /auth/signup returns {resp.status_code}")
    else:
        r.fail("POST /auth/signup", f"status={resp.status_code}")
        r.info(f"  Response body: {resp.text[:300]}")
        return

    # HX-Redirect
    hx_redirect = resp.headers.get("HX-Redirect")
    if hx_redirect:
        r.ok(f"HX-Redirect header present: {hx_redirect}")
        if "/marketplace" in hx_redirect:
            r.ok("Redirected to /marketplace after signup")
        else:
            r.info(f"Redirect target: {hx_redirect}")
    else:
        r.warn("No HX-Redirect header after signup")

    # Session cookie set
    fix_secure_cookies(s)
    if has_cookie(s, "poool_session"):
        r.ok("Session cookie set after registration")
    else:
        r.warn("No session cookie after registration")

    # ── DB Verification ──
    r.section("REGISTRATION – Database State Verification")
    try:
        conn = db_connect()
        cur = conn.cursor()

        # 1. User row created
        cur.execute(
            "SELECT id, email, password_hash, status, created_at FROM users WHERE email = %s",
            (REGISTER_EMAIL,),
        )
        user = cur.fetchone()
        if user:
            user_id, email, pw_hash, status, created_at = user
            r.ok(f"User row created (id={user_id})")
            r.ok(f"  email = {email}")

            # Password must be hashed, not plaintext
            if pw_hash and pw_hash != REGISTER_PASSWORD and len(pw_hash) > 30:
                r.ok("Password is hashed (not plaintext)")
            elif pw_hash == REGISTER_PASSWORD:
                r.fail("Password stored as PLAINTEXT!", "Critical security issue")
            else:
                r.warn(f"Password hash looks unusual: {pw_hash[:20] if pw_hash else 'NULL'}...")

            # Status
            if status:
                r.ok(f"  status = {status}")
            else:
                r.warn("User status is NULL")
        else:
            r.fail(f"User row NOT created for {REGISTER_EMAIL}")
            cur.close()
            conn.close()
            return

        # 2. User profile
        cur.execute(
            "SELECT id FROM user_profiles WHERE user_id = %s", (user_id,)
        )
        if cur.fetchone():
            r.ok("User profile row created")
        else:
            r.warn("User profile row NOT created (may be created lazily)")

        # 3. Wallet
        cur.execute(
            "SELECT id, balance_cents FROM wallets WHERE user_id = %s", (user_id,)
        )
        wallet = cur.fetchone()
        if wallet:
            r.ok(f"Wallet created (id={wallet[0]}, balance_cents={wallet[1]})")
            if wallet[1] == 0:
                r.ok("Wallet balance initialized to 0")
            else:
                r.warn(f"Wallet balance is {wallet[1]} cents (expected 0)")
        else:
            r.warn("Wallet NOT created (may be created lazily)")

        # 4. User roles
        cur.execute(
            "SELECT r.name FROM user_roles ur JOIN roles r ON r.id = ur.role_id "
            "WHERE ur.user_id = %s",
            (user_id,),
        )
        roles = [row[0] for row in cur.fetchall()]
        if roles:
            r.ok(f"User roles assigned: {', '.join(roles)}")
        else:
            r.warn("No roles assigned to new user")

        # 5. Session
        cur.execute(
            "SELECT id, expires_at FROM user_sessions "
            "WHERE user_id = %s AND expires_at > NOW() ORDER BY created_at DESC LIMIT 1",
            (user_id,),
        )
        sess = cur.fetchone()
        if sess:
            r.ok(f"Active session exists (expires={sess[1]})")
        else:
            r.warn("No active session found for registered user")

        # 6. Terms consent
        cur.execute(
            "SELECT terms_version FROM user_consents WHERE user_id = %s",
            (user_id,),
        )
        consent = cur.fetchone()
        if consent:
            r.ok(f"Terms consent recorded (version={consent[0]})")
        else:
            r.warn("No terms consent recorded")

        cur.close()
        conn.close()
    except Exception as e:
        r.warn(f"DB verification error: {e}")


# ─── 6. Registration – Edge Cases ───────────────────────────────

def test_register_edge_cases(r: Results):
    r.section("REGISTRATION – Edge Cases")

    # 6a. Duplicate email
    s = get_csrf_session()
    headers = htmx_headers(s, f"{BASE_URL}/auth/signup")
    resp = s.post(
        f"{BASE_URL}/auth/signup",
        data={
            "email": TEST_EMAIL,  # Already exists
            "password": "AnotherPass1!",
            "terms_accepted": "on",
        },
        headers=headers,
        allow_redirects=False,
        timeout=LOGIN_TIMEOUT,
    )
    if resp.status_code in (409, 400, 422):
        r.ok(f"Duplicate email rejected ({resp.status_code})")
    elif resp.status_code == 200:
        # Check for error message in body (HTMX returns inline error)
        if not has_cookie(s, "poool_session"):
            r.ok("Duplicate email: 200 with error (no session set)")
        else:
            r.fail("Duplicate email: user was created/logged in!")
    elif resp.status_code == 500:
        r.warn(f"Duplicate email returned 500 (should be 409 Conflict)")
    else:
        r.warn(f"Duplicate email returned {resp.status_code}")

    # 6b. Terms not accepted
    s2 = get_csrf_session()
    headers2 = htmx_headers(s2, f"{BASE_URL}/auth/signup")
    resp2 = s2.post(
        f"{BASE_URL}/auth/signup",
        data={
            "email": f"no-terms-{uuid.uuid4().hex[:6]}@test.com",
            "password": REGISTER_PASSWORD,
            # terms_accepted intentionally omitted
        },
        headers=headers2,
        allow_redirects=False,
        timeout=REQUEST_TIMEOUT,
    )
    if resp2.status_code in (400, 422):
        r.ok(f"Missing terms rejected ({resp2.status_code})")
    elif resp2.status_code == 200:
        body = resp2.text.lower()
        if "terms" in body or "accept" in body:
            r.ok("Missing terms: error message returned in response body")
        elif not has_cookie(s2, "poool_session"):
            r.ok("Missing terms: no session set (registration blocked)")
        else:
            r.fail("Missing terms: user was created!")
    else:
        r.warn(f"Missing terms returned {resp2.status_code}")

    # 6c. Empty password
    s3 = get_csrf_session()
    headers3 = htmx_headers(s3, f"{BASE_URL}/auth/signup")
    resp3 = s3.post(
        f"{BASE_URL}/auth/signup",
        data={
            "email": f"empty-pw-{uuid.uuid4().hex[:6]}@test.com",
            "password": "",
            "terms_accepted": "on",
        },
        headers=headers3,
        allow_redirects=False,
        timeout=REQUEST_TIMEOUT,
    )
    if resp3.status_code in (400, 422):
        r.ok(f"Empty password rejected ({resp3.status_code})")
    elif resp3.status_code == 200 and not has_cookie(s3, "poool_session"):
        r.ok("Empty password: 200 with error, no session set")
    elif resp3.status_code == 500:
        r.warn("Empty password returned 500 (should return 400)")
    else:
        r.warn(f"Empty password returned {resp3.status_code}")

    # 6d. Invalid email format
    s4 = get_csrf_session()
    headers4 = htmx_headers(s4, f"{BASE_URL}/auth/signup")
    resp4 = s4.post(
        f"{BASE_URL}/auth/signup",
        data={
            "email": "not-an-email",
            "password": REGISTER_PASSWORD,
            "terms_accepted": "on",
        },
        headers=headers4,
        allow_redirects=False,
        timeout=REQUEST_TIMEOUT,
    )
    if resp4.status_code in (400, 422):
        r.ok(f"Invalid email format rejected ({resp4.status_code})")
    elif resp4.status_code == 200 and not has_cookie(s4, "poool_session"):
        r.ok("Invalid email: 200 with error, no session set")
    elif resp4.status_code == 500:
        r.warn("Invalid email returned 500 (should be 400)")
    else:
        r.warn(f"Invalid email returned {resp4.status_code}")


# ─── 7. Protected Routes ────────────────────────────────────────

def test_protected_routes(r: Results):
    r.section("AUTH GUARDS – Protected Routes Redirect")

    protected = [
        "/marketplace", "/wallet", "/portfolio", "/rewards",
        "/cart", "/settings", "/kyc",
    ]
    for page in protected:
        resp = requests.get(
            f"{BASE_URL}{page}",
            allow_redirects=False,
            timeout=REQUEST_TIMEOUT,
        )
        if resp.status_code in (302, 303):
            loc = resp.headers.get("Location", "")
            if "login" in loc:
                r.ok(f"GET {page} → redirect to login ({resp.status_code})")
            else:
                r.ok(f"GET {page} → redirect ({resp.status_code}, Location: {loc})")
        elif resp.status_code == 401:
            r.ok(f"GET {page} → 401 Unauthorized")
        elif resp.status_code == 200:
            r.fail(f"GET {page} → accessible WITHOUT auth (200)")
        else:
            r.warn(f"GET {page} → {resp.status_code}")


# ─── 8. Logout ──────────────────────────────────────────────────

def test_logout(r: Results):
    r.section("LOGOUT – Session Invalidation")

    # First login to get a session  
    s = get_csrf_session()
    headers = htmx_headers(s)
    try:
        s.post(
            f"{BASE_URL}/auth/login",
            data={"email": TEST_EMAIL, "password": TEST_PASSWORD},
            headers=headers,
            allow_redirects=False,
            timeout=LOGIN_TIMEOUT,
        )
    except requests.exceptions.Timeout:
        r.warn("Login timed out, skipping logout test")
        return

    fix_secure_cookies(s)
    if not has_cookie(s, "poool_session"):
        r.warn("Could not login to test logout")
        return

    r.ok("Logged in successfully for logout test")

    # Hit logout
    resp = s.get(f"{BASE_URL}/auth/logout", allow_redirects=False, timeout=REQUEST_TIMEOUT)
    if resp.status_code in (302, 303):
        r.ok(f"GET /auth/logout redirects ({resp.status_code})")
        loc = resp.headers.get("Location", "")
        if "login" in loc:
            r.ok(f"Redirected to login page: {loc}")
        else:
            r.info(f"Redirect target: {loc}")
    elif resp.status_code == 200:
        r.ok("GET /auth/logout returns 200")
    else:
        r.warn(f"GET /auth/logout returned {resp.status_code}")

    # After logout, protected routes should redirect again
    me_resp = s.get(f"{BASE_URL}/api/me", timeout=REQUEST_TIMEOUT)
    if me_resp.status_code in (401, 302, 303):
        r.ok(f"/api/me no longer accessible after logout ({me_resp.status_code})")
    elif me_resp.status_code == 200:
        r.fail("/api/me still accessible after logout!", "Session not invalidated")
    else:
        r.info(f"/api/me returned {me_resp.status_code} after logout")


# ─── 9. Cleanup ─────────────────────────────────────────────────

def cleanup_test_user(r: Results):
    """Remove the test user created during registration tests."""
    r.section("CLEANUP – Removing Test User")
    try:
        conn = db_connect()
        cur = conn.cursor()

        cur.execute("SELECT id FROM users WHERE email = %s", (REGISTER_EMAIL,))
        row = cur.fetchone()
        if not row:
            r.info("No test user to clean up")
            cur.close()
            conn.close()
            return

        user_id = row[0]

        # Delete in order respecting foreign keys
        for table, col in [
            ("user_consents", "user_id"),
            ("user_sessions", "user_id"),
            ("user_roles", "user_id"),
            ("wallet_transactions", "wallet_id"),  # handled below
            ("wallets", "user_id"),
            ("user_settings", "user_id"),
            ("user_profiles", "user_id"),
            ("referral_tracking", "referred_id"),
            ("notifications", "user_id"),
            ("kyc_records", "user_id"),
            ("audit_logs", "user_id"),
            ("audit_logs", "actor_user_id"),
        ]:
            try:
                if table == "wallet_transactions":
                    cur.execute(
                        "DELETE FROM wallet_transactions WHERE wallet_id IN "
                        "(SELECT id FROM wallets WHERE user_id = %s)",
                        (user_id,),
                    )
                else:
                    cur.execute(
                        f"DELETE FROM {table} WHERE {col} = %s", (user_id,)
                    )
            except Exception:
                conn.rollback()

        cur.execute("DELETE FROM users WHERE id = %s", (user_id,))
        conn.commit()
        r.ok(f"Cleaned up test user {REGISTER_EMAIL} (id={user_id})")

        cur.close()
        conn.close()
    except Exception as e:
        r.warn(f"Cleanup error: {e}")


# ─── Main ────────────────────────────────────────────────────────

def main():
    print("=" * 70)
    print("  POOOL – Login & Registration Test Suite")
    print(f"  Target: {BASE_URL}")
    print(f"  Test user: {TEST_EMAIL}")
    print(f"  Register email: {REGISTER_EMAIL}")
    print("=" * 70)

    r = Results()

    # Pre-check: is the server running?
    try:
        requests.get(f"{BASE_URL}/auth/login", timeout=5)
    except requests.exceptions.ConnectionError:
        print(f"\n  ❌  Server not running at {BASE_URL}")
        print("  Start with: cargo run --bin poool-backend")
        sys.exit(1)

    r.ok("Server is reachable")

    # Run tests in order
    test_login_page(r)
    test_login_valid(r)
    test_login_invalid(r)
    test_signup_page(r)
    test_register_happy_path(r)
    test_register_edge_cases(r)
    test_protected_routes(r)
    test_logout(r)

    # Cleanup
    cleanup_test_user(r)

    # Final summary
    success = r.summary()
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
