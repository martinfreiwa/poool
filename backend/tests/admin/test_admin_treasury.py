#!/usr/bin/env python3
"""
Comprehensive Treasury Page Test Suite for POOOL Admin
"""

import json
import re
import subprocess
import sys
import requests

BASE = "http://localhost:8888"
DB = "poool"

passed = 0
failed = 0
errors = []

def psql(sql: str) -> str:
    try:
        return subprocess.check_output(
            ["psql", "-Atc", sql, DB]
        ).decode().strip()
    except Exception as e:
        print(f"Error running psql: {e}")
        return ""

def get_session(email="test@poool.app") -> requests.Session:
    token = psql(
        f"SELECT session_token FROM user_sessions "
        f"WHERE user_id = (SELECT id FROM users WHERE email='{email}') "
        f"ORDER BY created_at DESC LIMIT 1"
    )
    if not token:
        print(f"Warning: No session token found for {email}")
    s = requests.Session()
    s.cookies.set("poool_session", token)
    return s

def check(name: str, condition: bool, detail: str = ""):
    global passed, failed
    if condition:
        passed += 1
        print(f"  ✅ {name}")
    else:
        failed += 1
        msg = f"  ❌ {name}"
        if detail:
            msg += f"  — {detail}"
        print(msg)
        errors.append(name)

def section(title: str):
    print(f"\n{'─'*60}")
    print(f"  {title}")
    print(f"{'─'*60}")

def setup_data():
    """Ensure some transactions exist to test the API properly."""
    user_id = psql("SELECT id FROM users WHERE email='test@poool.app'")
    wallet_id = psql(f"SELECT id FROM wallets WHERE user_id='{user_id}' LIMIT 1")
    if not wallet_id:
        # Create a wallet
        psql(f"INSERT INTO wallets (user_id, balance_cents) VALUES ('{user_id}', 100000)")
        wallet_id = psql(f"SELECT id FROM wallets WHERE user_id='{user_id}' LIMIT 1")
        
    # Insert a dummy transaction if there are none
    tx_count = int(psql("SELECT count(*) FROM wallet_transactions") or 0)
    if tx_count == 0:
        psql(f"INSERT INTO wallet_transactions (wallet_id, type, amount_cents, status, description) VALUES ('{wallet_id}', 'deposit', 50000, 'completed', 'Test Deposit')")

def test_admin_access(session):
    section("1. ADMIN ACCESS CONTROL")
    r = requests.get(f"{BASE}/admin/treasury.html", allow_redirects=False)
    check("Unauthenticated access redirects to login", r.status_code in (302, 303))
    r = requests.get(f"{BASE}/api/admin/treasury", allow_redirects=False)
    check("Unauthenticated API access denied", r.status_code in (302, 303, 401, 403))
    r = session.get(f"{BASE}/api/admin/treasury")
    check("Admin access to API allowed (200)", r.status_code == 200, f"got {r.status_code}")

def test_treasury_api_structure(session):
    section("2. TREASURY API STRUCTURE & DATA INTEGRITY")
    r = session.get(f"{BASE}/api/admin/treasury")
    data = r.json()

    check("Response has 'stats' field", "stats" in data)
    s = data["stats"]
    for field in ["total_balance_cents", "wallet_count", "total_deposits_cents", "net_revenue_cents"]:
        check(f"Stats has '{field}' field", field in s)
        check(f"Stats '{field}' is a number", isinstance(s[field], (int, float)))

    check("Response has 'type_breakdown' field", "type_breakdown" in data)
    check("type_breakdown is a list", isinstance(data["type_breakdown"], list))
    if data["type_breakdown"]:
        tb = data["type_breakdown"][0]
        for tf in ["type", "total_cents", "count"]:
            check(f"type_breakdown item has '{tf}'", tf in tb)

    check("Response has 'dividend_stats' field", "dividend_stats" in data)
    ds = data["dividend_stats"]
    for f in ["total_paid_cents", "paid_count", "scheduled_cents", "scheduled_count", 
              "processing_cents", "processing_count", "failed_cents", "failed_count"]:
        check(f"dividend_stats has '{f}'", f in ds)

    check("Response has 'recent_transactions' field", "recent_transactions" in data)
    check("recent_transactions is a list", isinstance(data["recent_transactions"], list))
    if data["recent_transactions"]:
        tx = data["recent_transactions"][0]
        # Must contain all necessary fields for the frontend
        required_tx_fields = ["id", "type", "status", "amount_cents", "description", "created_at", "user_email", "user_name"]
        for required in required_tx_fields:
            check(f"Transaction has field '{required}'", required in tx)

def test_treasury_page_elements(session):
    section("3. TREASURY PAGE UI ELEMENTS & FEATURES")
    r = session.get(f"{BASE}/admin/treasury.html")
    html = r.text

    check("Page title is correct", "Treasury" in html)
    check("Export CSV button exists", "exportTreasuryCSV()" in html)
    check("Transaction table exists", "tx-table-body" in html)
    
    # Sortable headers
    check("Sortable User header", 'data-sort="user_name"' in html)
    check("Sortable Type header", 'data-sort="type"' in html)
    check("Sortable Amount header", 'data-sort="amount_cents"' in html)
    check("Sortable Status header", 'data-sort="status"' in html)
    check("Sortable Date header", 'data-sort="created_at"' in html)
    
    # Pagination elements
    check("Pagination info element exists", 'id="pagination-info"' in html)
    check("Prev page button exists", 'id="prev-page"' in html)
    check("Next page button exists", 'id="next-page"' in html)

    # Filter element
    check("Transaction type filter exists", 'id="tx-type-filter"' in html)

def test_csv_export_availability(session):
    section("4. JS INTEGRATION")
    r = session.get(f"{BASE}/admin/treasury.html")
    html = r.text
    check("admin-treasury.js script is linked", "admin-treasury.js" in html)

def main():
    print("=" * 60)
    print("  POOOL ADMIN TREASURY PAGE — EXTENDED TEST SUITE")
    print("=" * 60)

    try:
        requests.get(f"{BASE}/", timeout=3)
    except requests.ConnectionError:
        print("❌ Server not running at", BASE)
        sys.exit(1)

    setup_data()
    session = get_session()

    test_admin_access(session)
    test_treasury_api_structure(session)
    test_treasury_page_elements(session)
    test_csv_export_availability(session)

    print(f"\n{'='*60}")
    total = passed + failed
    print(f"  RESULTS: {passed}/{total} passed, {failed} failed")
    if errors:
        print(f"\n  FAILURES:")
        for e in errors:
            print(f"    • {e}")
    print(f"{'='*60}")
    sys.exit(0 if failed == 0 else 1)

if __name__ == "__main__":
    main()
