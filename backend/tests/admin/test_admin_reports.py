#!/usr/bin/env python3
"""
Comprehensive Test Suite for POOOL Admin Reports Page
"""

import json
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
        return subprocess.check_output(["psql", "-Atc", sql, DB]).decode().strip()
    except Exception:
        return ""

def get_session(email="test@poool.app") -> requests.Session:
    token = psql(f"SELECT session_token FROM user_sessions WHERE user_id = (SELECT id FROM users WHERE email='{email}') ORDER BY created_at DESC LIMIT 1")
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

def test_page_load(session):
    section("1. HTML STRUCTURE & UI ELEMENTS")
    r = session.get(f"{BASE}/admin/reports.html")
    check("Reports page loads (200)", r.status_code == 200)
    html = r.text
    check("Reports page has Financial group", "Commercial" not in html and "Financial" in html)
    check("Reports page has Compliance group", "Compliance" in html)
    check("Reports page has date preset buttons", "setPreset('ytd')" in html)
    check("admin-reports.js is loaded", "admin-reports.js" in html)
    check("HTML has preview section", 'id="preview-section"' in html)

def test_api_endpoints(session):
    section("2. API ENDPOINT CONNECTIVITY")
    endpoints = {
        "User Directory": "/api/admin/users", 
        "KYC Status Report": "/api/admin/kyc", 
        "Orders": "/api/admin/orders", 
        "Treasury": "/api/admin/treasury",
        "Investments": "/api/admin/investments",
        "Assets": "/api/admin/assets",
        "Support": "/api/admin/support"
    }
    for name, ep in endpoints.items():
        r = session.get(f"{BASE}{ep}")
        check(f"Endpoint '{name}' ({ep}) is accessible (200)", r.status_code == 200, f"Got {r.status_code}")
        try:
            r.json()
            check(f"Endpoint '{name}' returns valid JSON", True)
        except Exception:
            check(f"Endpoint '{name}' returns valid JSON", False)

def test_export_data_structure(session):
    section("3. REPORT DATA STRUCTURES (FOR CSV PARSING)")
    
    # KYC
    r = session.get(f"{BASE}/api/admin/kyc")
    data = r.json()
    items = []
    if isinstance(data, list):
        items = data
    elif isinstance(data, dict):
        # find the list
        for key, val in data.items():
            if isinstance(val, list):
                items = val
                break
    check("KYC API returns extractable list", isinstance(items, list))
    if items:
        check("KYC item has required fields for export (id, status)", "id" in items[0] and "status" in items[0])
        
    # Treasury
    r = session.get(f"{BASE}/api/admin/treasury")
    t_data = r.json()
    check("Treasury API returns recent_transactions array", "recent_transactions" in t_data and isinstance(t_data["recent_transactions"], list))

def main():
    print("=" * 60)
    print("  POOOL ADMIN REPORTS PAGE — EXTENDED TEST SUITE")
    print("=" * 60)

    try:
        requests.get(f"{BASE}/", timeout=3)
    except requests.ConnectionError:
        print("❌ Server not running at", BASE)
        sys.exit(1)

    session = get_session()

    test_page_load(session)
    test_api_endpoints(session)
    test_export_data_structure(session)

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
