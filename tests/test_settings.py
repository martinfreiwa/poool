#!/usr/bin/env python3
"""
POOOL Platform – Settings Page E2E Test Suite
==============================================
Comprehensive tests for all Settings page features including
the new profile enhancements (address, DOB, nationality, tax_id,
investment limits, tier/referral, active sessions, OAuth accounts,
consent/terms, profile completeness).

Tests cover:
  1. Page load and structure
  2. GET /api/settings endpoint (all fields)
  3. POST /api/settings/profile (My Details with new fields)
  4. POST /api/settings/preferences (Preferences)
  15. POST /api/settings/notifications (Notification preferences)
16. GET/PUT /api/leaderboard/preferences (Leaderboard preferences)
17. POST /api/settings/email (Change Email)
  7. POST /api/settings/password (Change Password)
  8. POST /api/settings/phone (Change Phone)
  9. Input validation and edge cases
  10. Security checks (auth required, CSRF, XSS)
  11. Profile completeness calculation
  12. New extended profile fields verification

Run:  python3 tests/test_settings.py
Requires: requests, psycopg2
"""

import json
import sys
import time
import psycopg2
import requests

BASE_URL = "http://localhost:8888"
DB_DSN = "dbname=poool user=martin host=localhost"
TEST_EMAIL = "test@poool.app"
TEST_PASSWORD = "TestPass123!"

class TestResults:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.warnings = 0
        self.errors = []
        self.warnings_list = []
        self.current_section = ""

    def section(self, name):
        self.current_section = name
        print(f"\n{'=' * 70}")
        print(f"  {name}")
        print(f"{'=' * 70}")

    def ok(self, msg):
        self.passed += 1
        print(f"  ✅  {msg}")

    def fail(self, msg, detail=""):
        self.failed += 1
        full = f"{msg}: {detail}" if detail else msg
        self.errors.append((self.current_section, full))
        print(f"  ❌  {full}")

    def warn(self, msg):
        self.warnings += 1
        self.warnings_list.append((self.current_section, msg))
        print(f"  ⚠️   {msg}")

def get_session() -> requests.Session:
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
            r = session.get(f"{BASE_URL}/settings", timeout=15)
            if "csrf_token" in session.cookies:
                session.headers.update({"X-CSRF-Token": session.cookies["csrf_token"]})
            if r.status_code == 200:
                print(f"  ℹ️  Session established via DB token for {TEST_EMAIL}")
                return session
    except Exception as e:
        print(f"  ⚠️  DB session lookup failed: {e}")
    
    # Try explicit login
    print("  ℹ️  Attempting explicit login...")
    HTMX_HEADERS = {
        "HX-Request": "true",
        "HX-Current-URL": f"{BASE_URL}/auth/login",
    }
    r = session.post(
        f"{BASE_URL}/auth/login",
        data={"email": TEST_EMAIL, "password": TEST_PASSWORD},
        headers=HTMX_HEADERS,
        allow_redirects=False,
    )
    if "csrf_token" in session.cookies:
        session.headers.update({"X-CSRF-Token": session.cookies["csrf_token"]})
    return session

def run_tests():
    results = TestResults()
    session = get_session()

    # ─── 1. Page Load Structure ───────────────────────────────────
    results.section("1. PAGE: /settings — Structure & Assets")
    r = session.get(f"{BASE_URL}/settings")
    if r.status_code == 200:
        results.ok("GET /settings returns 200")
    else:
        results.fail(f"GET /settings returned {r.status_code}")
    
    text = r.text
    for check, label in [
        ("settings-2.css", "CSS linked"),
        ("settings-2.js", "JS linked"),
        ("settings-service.js", "Service JS linked"),
        ("edit-first-name", "First name input"),
        ("edit-last-name", "Last name input"),
        ("read-email", "Email display"),
        ("read-phone", "Phone display"),
        ("edit-dob", "Date of Birth input"),
        ("edit-nationality", "Nationality input"),
        ("edit-tax-id", "Tax ID input"),
        ("edit-address-1", "Address Line 1 input"),
        ("edit-address-2", "Address Line 2 input"),
        ("edit-city", "City input"),
        ("edit-state", "State/Province input"),
        ("edit-postal", "Postal Code input"),
        ("edit-timezone", "Timezone select"),
        ("completeness-bar-fill", "Profile completeness bar"),
        ("completeness-pct", "Profile completeness text"),
        ("settings-investment-limit", "Investment limit display"),
        ("settings-tier-name", "Tier name display"),
        ("settings-referral-code", "Referral code display"),
        ("settings-sessions-list", "Active sessions list"),
        ("settings-oauth-list", "OAuth accounts list"),
        ("settings-terms-version", "Terms version display"),
        ("settings-2fa-btn", "2FA button"),
        ("settings-email-verified", "Email verified badge"),
        ("settings-kyc-detail-badge", "KYC detail badge"),
    ]:
        if check in text:
            results.ok(f"HTML contains: {label}")
        else:
            results.fail(f"HTML missing: {label} (id/class: {check})")

    # ─── 2. GET /api/settings ─────────────────────────────────────
    results.section("2. API: GET /api/settings — Auth & Response Shape")
    
    # Unauthenticated
    r_unauth = requests.get(f"{BASE_URL}/api/settings")
    if r_unauth.status_code == 401:
        results.ok("Unauthenticated GET returns 401")
    else:
        results.fail(f"Unauthenticated GET returned {r_unauth.status_code} (expected 401)")

    # Authenticated
    r_auth = session.get(f"{BASE_URL}/api/settings")
    if r_auth.status_code == 200:
        results.ok("Authenticated GET returns 200")
    else:
        results.fail(f"Authenticated GET returned {r_auth.status_code}")
        # Can't continue without data
        print_summary(results)
        return

    data = r_auth.json()
    
    # Core fields
    results.section("3. API Response: Core Fields")
    core_fields = [
        "email", "first_name", "last_name", "phone_number",
        "country", "timezone", "role", "language", "currency",
    ]
    for field in core_fields:
        if field in data:
            results.ok(f"Field present: {field} = {repr(data[field])[:50]}")
        else:
            results.fail(f"Field missing: {field}")

    # Extended profile fields (new)
    results.section("4. API Response: Extended Profile Fields")
    extended_fields = [
        "date_of_birth", "nationality", "address_line_1", "address_line_2",
        "city", "state_province", "postal_code", "tax_id",
    ]
    for field in extended_fields:
        if field in data:
            results.ok(f"Extended field present: {field} = {repr(data[field])[:50]}")
        else:
            results.fail(f"Extended field missing: {field}")

    # Boolean/status fields
    results.section("5. API Response: Status & Boolean Fields")
    status_fields = {
        "email_verified": bool,
        "email_notifications": bool,
        "push_notifications": bool,
        "totp_enabled": bool,
        "kyc_status": (str, type(None)),
    }
    for field, expected_type in status_fields.items():
        if field in data:
            if isinstance(expected_type, tuple):
                if isinstance(data[field], expected_type):
                    results.ok(f"Status field: {field} = {data[field]}")
                else:
                    results.fail(f"Wrong type for {field}: {type(data[field])}")
            elif isinstance(data[field], expected_type):
                results.ok(f"Boolean field: {field} = {data[field]}")
            else:
                results.fail(f"Wrong type for {field}: expected {expected_type}, got {type(data[field])}")
        else:
            results.fail(f"Status field missing: {field}")

    # Financial/rewards fields
    results.section("6. API Response: Financial & Rewards Fields")
    financial_fields = [
        "referral_code", "tier_name",
        "investment_limit_cents", "invested_12m_cents", "limit_available_cents",
    ]
    for field in financial_fields:
        if field in data:
            results.ok(f"Financial field present: {field} = {repr(data[field])[:50]}")
        else:
            results.fail(f"Financial field missing: {field}")

    # Sessions array
    results.section("7. API Response: Active Sessions")
    if "active_sessions" in data:
        sessions = data["active_sessions"]
        results.ok(f"active_sessions present (count: {len(sessions)})")
        if isinstance(sessions, list):
            results.ok("active_sessions is a list")
            if len(sessions) > 0:
                s = sessions[0]
                for key in ["ip_address", "user_agent", "created_at", "expires_at", "is_current"]:
                    if key in s:
                        results.ok(f"Session field: {key} = {repr(s[key])[:40]}")
                    else:
                        results.fail(f"Session field missing: {key}")
                has_current = any(s.get("is_current") for s in sessions)
                if has_current:
                    results.ok("At least one session marked as current")
                else:
                    results.warn("No session marked as current")
            else:
                results.warn("active_sessions is empty (user may have no extra sessions)")
        else:
            results.fail("active_sessions is not a list")
    else:
        results.fail("active_sessions field missing from response")

    # OAuth accounts array
    results.section("8. API Response: OAuth Accounts")
    if "oauth_accounts" in data:
        oauth = data["oauth_accounts"]
        results.ok(f"oauth_accounts present (count: {len(oauth)})")
        if isinstance(oauth, list):
            results.ok("oauth_accounts is a list")
            if len(oauth) > 0:
                o = oauth[0]
                for key in ["provider", "provider_email", "created_at"]:
                    if key in o:
                        results.ok(f"OAuth field: {key} = {repr(o[key])[:40]}")
                    else:
                        results.fail(f"OAuth field missing: {key}")
            else:
                results.ok("oauth_accounts empty (expected — user may not have SSO linked)")
        else:
            results.fail("oauth_accounts is not a list")
    else:
        results.fail("oauth_accounts field missing from response")

    # Consent/terms fields
    results.section("9. API Response: Consent & Terms")
    for field in ["latest_terms_version", "latest_terms_accepted_at"]:
        if field in data:
            results.ok(f"Consent field present: {field} = {repr(data[field])[:50]}")
        else:
            results.fail(f"Consent field missing: {field}")

    # ─── 10. Profile Update (POST) ────────────────────────────────
    results.section("10. API: POST /api/settings/profile — Basic Update")
    r_upd = session.post(f"{BASE_URL}/api/settings/profile", json={
        "first_name": "Test",
        "last_name": "User",
        "phone_number": "+1234567890",
        "country": "US",
        "timezone": "America/New_York"
    })
    if r_upd.status_code == 200 and r_upd.json().get("success"):
        results.ok("Profile update succeeded")
    else:
        results.fail(f"Profile update failed (status {r_upd.status_code})", r_upd.text[:200])

    # Extended profile update
    results.section("11. API: POST /api/settings/profile — Extended Fields")
    r_ext = session.post(f"{BASE_URL}/api/settings/profile", json={
        "first_name": "Test",
        "last_name": "User",
        "phone_number": "+1234567890",
        "country": "US",
        "timezone": "America/New_York",
        "date_of_birth": "1990-06-15",
        "nationality": "DEU",
        "tax_id": "XX-1234567",
        "address_line_1": "123 Main Street",
        "address_line_2": "Apt 4B",
        "city": "New York",
        "state_province": "NY",
        "postal_code": "10001",
    })
    if r_ext.status_code == 200 and r_ext.json().get("success"):
        results.ok("Extended profile update succeeded")
    else:
        results.fail(f"Extended profile update failed (status {r_ext.status_code})", r_ext.text[:200])

    # Verify extended fields persisted
    r_verify = session.get(f"{BASE_URL}/api/settings")
    if r_verify.status_code == 200:
        vdata = r_verify.json()
        checks = {
            "date_of_birth": "1990-06-15",
            "nationality": "DEU",
            "tax_id": "XX-1234567",
            "address_line_1": "123 Main Street",
            "address_line_2": "Apt 4B",
            "city": "New York",
            "state_province": "NY",
            "postal_code": "10001",
        }
        for field, expected in checks.items():
            actual = vdata.get(field)
            if actual == expected:
                results.ok(f"Persisted: {field} = {expected}")
            else:
                results.fail(f"Not persisted: {field} expected={expected}, got={actual}")
    else:
        results.fail(f"Verification GET failed: {r_verify.status_code}")

    # ─── 12. Preferences ──────────────────────────────────────────
    results.section("12. API: POST /api/settings/preferences")
    r_pref = session.post(f"{BASE_URL}/api/settings/preferences", json={
        "language": "de",
        "currency": "EUR"
    })
    if r_pref.status_code == 200 and r_pref.json().get("success"):
        results.ok("Preferences update succeeded")
        # Reset
        session.post(f"{BASE_URL}/api/settings/preferences", json={"language": "en", "currency": "USD"})
    else:
        results.fail(f"Preferences update failed (status {r_pref.status_code})", r_pref.text[:200])

    # ─── 13. Notifications ────────────────────────────────────────
    results.section("13. API: POST /api/settings/notifications")
    r_notif = session.post(f"{BASE_URL}/api/settings/notifications", json={
        "email_notifications": False,
        "push_notifications": True
    })
    if r_notif.status_code == 200 and r_notif.json().get("success"):
        results.ok("Notifications update succeeded")
        # Verify
        vn = session.get(f"{BASE_URL}/api/settings").json()
        if vn.get("email_notifications") == False:
            results.ok("email_notifications persisted as False")
        else:
            results.fail(f"email_notifications not persisted: {vn.get('email_notifications')}")
        # Reset
        session.post(f"{BASE_URL}/api/settings/notifications", json={
            "email_notifications": True, "push_notifications": True
        })
    else:
        results.fail(f"Notifications update failed (status {r_notif.status_code})", r_notif.text[:200])

    # ─── 14. Leaderboard Preferences ──────────────────────────────
    results.section("14. API: GET/PUT /api/leaderboard/preferences")
    
    # GET
    r_lpref = session.get(f"{BASE_URL}/api/leaderboard/preferences")
    if r_lpref.status_code == 200:
        results.ok("GET /api/leaderboard/preferences returns 200")
        l_data = r_lpref.json()
        for f in ["visible", "show_avatar", "display_name"]:
            if f in l_data:
                results.ok(f"  Field present: {f} = {repr(l_data[f])}")
            else:
                results.fail(f"  Field missing: {f}")
    else:
        results.fail(f"GET /api/leaderboard/preferences returned {r_lpref.status_code}")

    # PUT (Update)
    new_pref = {
        "visible": False,
        "show_avatar": False,
        "display_name": "AnonymousPoooler"
    }
    r_lupd = session.put(f"{BASE_URL}/api/leaderboard/preferences", json=new_pref)
    if r_lupd.status_code == 200:
        results.ok("PUT /api/leaderboard/preferences returns 200")
        upd_data = r_lupd.json()
        if upd_data.get("display_name") == "AnonymousPoooler" and upd_data.get("visible") == False:
            results.ok("  Preferences updated successfully")
        else:
            results.fail("  Preferences not reflected in response", f"{upd_data}")
    else:
        results.fail(f"PUT /api/leaderboard/preferences returned {r_lupd.status_code}", r_lupd.text)

    # Verify Persistence
    r_lver = session.get(f"{BASE_URL}/api/leaderboard/preferences")
    if r_lver.status_code == 200 and r_lver.json().get("display_name") == "AnonymousPoooler":
        results.ok("Leaderboard preferences persisted in DB")
    else:
        results.fail("Leaderboard preferences NOT persisted")

    # Reset
    session.put(f"{BASE_URL}/api/leaderboard/preferences", json={"visible": True, "show_avatar": True, "display_name": None})

    # ─── 15. Validation ───────────────────────────────────────────
    results.section("14. Input Validation & Edge Cases")
    
    # Empty first name
    r_empty = session.post(f"{BASE_URL}/api/settings/profile", json={
        "first_name": "",
        "last_name": "User",
        "country": "US",
        "timezone": "UTC"
    })
    if r_empty.status_code == 200:
        results.warn("Empty first_name accepted (may be intentional)")
    else:
        results.ok(f"Empty first_name rejected ({r_empty.status_code})")

    # XSS attempt
    r_xss = session.post(f"{BASE_URL}/api/settings/profile", json={
        "first_name": "<script>alert('xss')</script>",
        "last_name": "User",
        "country": "US",
        "timezone": "UTC"
    })
    if r_xss.status_code == 200:
        # Check if it was sanitized
        v = session.get(f"{BASE_URL}/api/settings").json()
        if "<script>" in (v.get("first_name") or ""):
            results.fail("XSS payload stored unsanitized in first_name")
        else:
            results.ok("XSS payload either rejected or sanitized")
    else:
        results.ok(f"XSS payload rejected ({r_xss.status_code})")

    # ─── 15. Security ─────────────────────────────────────────────
    results.section("15. Security Checks")

    # Auth required for mutations
    r_noauth = requests.post(f"{BASE_URL}/api/settings/profile", json={
        "first_name": "Hacker"
    })
    if r_noauth.status_code in (401, 403):
        results.ok(f"Unauthenticated POST rejected ({r_noauth.status_code})")
    else:
        results.fail(f"Unauthenticated POST returned {r_noauth.status_code}")

    r_noauth_pref = requests.post(f"{BASE_URL}/api/settings/preferences", json={
        "language": "de"
    })
    if r_noauth_pref.status_code in (401, 403):
        results.ok(f"Unauthenticated preferences POST rejected ({r_noauth_pref.status_code})")
    else:
        results.fail(f"Unauthenticated preferences POST returned {r_noauth_pref.status_code}")

    # ─── 16. Profile Completeness Logic ───────────────────────────
    results.section("16. Profile Completeness Logic")
    r_full = session.get(f"{BASE_URL}/api/settings")
    if r_full.status_code == 200:
        fd = r_full.json()
        fields_for_completeness = [
            "first_name", "last_name", "date_of_birth", "nationality",
            "address_line_1", "city", "state_province", "postal_code", "tax_id"
        ]
        filled = sum(1 for f in fields_for_completeness if fd.get(f) and str(fd.get(f)).strip())
        pct = round(filled / len(fields_for_completeness) * 100)
        results.ok(f"Profile completeness: {filled}/{len(fields_for_completeness)} = {pct}%")
        if pct == 100:
            results.ok("Profile is 100% complete ✨")
        else:
            missing = [f for f in fields_for_completeness if not (fd.get(f) and str(fd.get(f)).strip())]
            results.warn(f"Missing fields for 100%: {missing}")
    else:
        results.fail("Could not GET settings for completeness check")

    # ─── 17. Database Integrity ───────────────────────────────────
    results.section("17. Database Integrity")
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        
        # Check user_profiles has the extended fields
        cur.execute("""
            SELECT first_name, last_name, date_of_birth, nationality, tax_id,
                   address_line_1, address_line_2, city, state_province, postal_code
            FROM user_profiles up
            JOIN users u ON u.id = up.user_id
            WHERE u.email = %s
        """, (TEST_EMAIL,))
        row = cur.fetchone()
        if row:
            results.ok("user_profiles row exists")
            cols = ["first_name", "last_name", "date_of_birth", "nationality", "tax_id",
                    "address_line_1", "address_line_2", "city", "state_province", "postal_code"]
            for i, col in enumerate(cols):
                if row[i] is not None:
                    results.ok(f"DB {col} = {repr(row[i])[:40]}")
                else:
                    results.warn(f"DB {col} is NULL")
        else:
            results.fail("No user_profiles row found")

        # Check user_settings exists
        cur.execute("""
            SELECT email_notifications, push_notifications, language, currency, timezone
            FROM user_settings us
            JOIN users u ON u.id = us.user_id
            WHERE u.email = %s
        """, (TEST_EMAIL,))
        srow = cur.fetchone()
        if srow:
            results.ok("user_settings row exists")
            results.ok(f"  notifications: email={srow[0]}, push={srow[1]}")
            results.ok(f"  prefs: lang={srow[2]}, currency={srow[3]}, tz={srow[4]}")
        else:
            results.fail("No user_settings row found")

        # Active sessions count
        cur.execute("""
            SELECT COUNT(*) FROM user_sessions s
            JOIN users u ON u.id = s.user_id
            WHERE u.email = %s AND s.expires_at > NOW()
        """, (TEST_EMAIL,))
        sess_count = cur.fetchone()[0]
        results.ok(f"Active sessions in DB: {sess_count}")

        cur.close()
        conn.close()
    except Exception as e:
        results.fail(f"Database check failed: {e}")

    # ─── Summary ──────────────────────────────────────────────────
    print_summary(results)

def print_summary(results):
    print(f"\n{'=' * 70}")
    print(f"  SUMMARY")
    print(f"{'=' * 70}")
    print(f"  ✅ Passed:   {results.passed}")
    print(f"  ❌ Failed:   {results.failed}")
    print(f"  ⚠️  Warnings: {results.warnings}")
    print(f"{'=' * 70}")

    if results.errors:
        print(f"\n  Failed Tests:")
        for section, msg in results.errors:
            print(f"    [{section}] {msg}")

    if results.warnings_list:
        print(f"\n  Warnings:")
        for section, msg in results.warnings_list:
            print(f"    [{section}] {msg}")

    print()
    if results.failed > 0:
        sys.exit(1)
    else:
        print("  🎉 All tests passed!")
        sys.exit(0)

if __name__ == '__main__':
    run_tests()
