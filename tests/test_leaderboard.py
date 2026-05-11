#!/usr/bin/env python3
"""
POOOL Platform – Leaderboard Page E2E Test Suite
================================================
Comprehensive tests for Leaderboard features including
rankings, user rank, visibility preferences, and scores.

Run:  python3 tests/test_leaderboard.py
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
            r = session.get(f"{BASE_URL}/api/me", timeout=15)
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

    # 1. Page Load
    results.section("1. PAGE: /leaderboard")
    r = session.get(f"{BASE_URL}/leaderboard")
    if r.status_code == 200:
        results.ok("GET /leaderboard returns 200")
    else:
        results.fail(f"GET /leaderboard returned {r.status_code}")

    # 2. Rankings API
    results.section("2. API: GET /api/leaderboard")
    r = session.get(f"{BASE_URL}/api/leaderboard?timeframe=alltime&page=1")
    if r.status_code == 200:
        results.ok("GET /api/leaderboard returns 200")
        data = r.json()
        if "rankings" in data and isinstance(data["rankings"], list):
            results.ok(f"Rankings returned: {len(data['rankings'])} entries")
        else:
            results.fail("Rankings field missing or not a list")
    else:
        results.fail(f"GET /api/leaderboard returned {r.status_code}")

    # 3. User Rank API
    results.section("3. API: GET /api/leaderboard/me")
    r = session.get(f"{BASE_URL}/api/leaderboard/me?timeframe=alltime")
    if r.status_code == 200:
        results.ok("GET /api/leaderboard/me returns 200")
        data = r.json()
        # `rank` is Option<i32> server-side; the field is always present, may be null.
        assert "rank" in data, "missing 'rank' field on /api/leaderboard/me"
        results.ok(f"User rank: {data['rank']}")
        # `total_score` was dropped in migration 046 — the rank response now
        # exposes `metric_value` plus a `metrics` sub-object.
        assert "metric_value" in data, "missing 'metric_value' field on /api/leaderboard/me"
        results.ok(f"User metric_value: {data['metric_value']}")
        assert "metrics" in data and isinstance(data["metrics"], dict), \
            "missing 'metrics' object on /api/leaderboard/me"
        for key in (
            "total_invested_cents",
            "asset_count",
            "portfolio_roi_bps",
            "affiliate_count",
            "referral_revenue_cents",
            "highest_investment_cents",
        ):
            assert key in data["metrics"], f"missing metrics.{key} on /api/leaderboard/me"
        results.ok("metrics breakdown present (6 fields)")
    else:
        results.fail(f"GET /api/leaderboard/me returned {r.status_code}")

    # 4. Visibility Toggle Logic
    results.section("4. LOGIC: Visibility Settings")
    # Toggle visibility off
    r_off = session.put(f"{BASE_URL}/api/leaderboard/preferences", json={"visible": False})
    if r_off.status_code == 200:
        results.ok("Set visible=False succeeded")
    
    # Check preferences API
    r_pver = session.get(f"{BASE_URL}/api/leaderboard/preferences")
    if r_pver.status_code == 200 and r_pver.json().get("visible") == False:
        results.ok("Visibility preference correctly False")
    else:
        results.fail("Visibility preference NOT False")

    # Toggle visibility on
    r_on = session.put(f"{BASE_URL}/api/leaderboard/preferences", json={"visible": True})
    if r_on.status_code == 200:
        results.ok("Set visible=True succeeded")
    
    r_pver2 = session.get(f"{BASE_URL}/api/leaderboard/preferences")
    if r_pver2.status_code == 200 and r_pver2.json().get("visible") == True:
        results.ok("Visibility preference correctly True")
    else:
        results.fail("Visibility preference NOT True")

    # 5. Anonymous Name Logic
    results.section("5. LOGIC: Anonymous Display Name")
    anon_name = "ShadowInvestor"
    session.put(f"{BASE_URL}/api/leaderboard/preferences", json={"display_name": anon_name})
    
    r = session.get(f"{BASE_URL}/api/leaderboard?timeframe=alltime")
    data = r.json()
    me = next((e for e in data.get("rankings", []) if e.get("is_current_user")), None)
    if me and me.get("display_name") == anon_name:
        results.ok(f"Display name correctly set to anonymous: {anon_name}")
    else:
        results.fail(f"Display name NOT updated in rankings (got {me.get('display_name') if me else 'N/A'})")

    # Reset
    session.put(f"{BASE_URL}/api/leaderboard/preferences", json={"display_name": None})

    # Summary
    print(f"\nSUMMARY: {results.passed} PASSED, {results.failed} FAILED")
    if results.failed > 0:
        sys.exit(1)

if __name__ == '__main__':
    run_tests()
