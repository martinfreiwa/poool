#!/usr/bin/env python3
"""
Comprehensive Admin Dashboard Test Suite for POOOL Platform
===========================================================
Covers:
  1. Access Control (Admin vs Regular User)
  2. Stats Overview API
  3. System Health API
  4. Asset & Submission Management
  5. Dividend Distribution Tool (Calculate & Process)
  6. Rewards & Balances Management
"""

import json
import subprocess
import sys
import requests
import uuid

BASE = "http://localhost:8888"
DB = "poool"

# ─── Helpers ─────────────────────────────────────────────────────

passed = 0
failed = 0
errors = []

def psql(sql: str) -> str:
    return subprocess.check_output(["psql", "-Atc", sql, DB]).decode().strip()

def get_session(email="test@poool.app") -> requests.Session:
    token = psql(
        f"SELECT session_token FROM user_sessions "
        f"WHERE user_id = (SELECT id FROM users WHERE email='{email}') "
        f"ORDER BY created_at DESC LIMIT 1"
    )
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
        if detail: msg += f"  — {detail}"
        print(msg)
        errors.append(name)

def section(title: str):
    print(f"\n{'─'*60}\n  {title}\n{'─'*60}")

# ─── Tests ───────────────────────────────────────────────────────

def test_admin_access_control():
    section("1. ADMIN ACCESS CONTROL")
    
    # Try accessing admin API with no session
    r = requests.get(f"{BASE}/api/admin/system")
    check("Access /api/admin/system (no session) → 403 or redirect", r.status_code in (403, 302, 303))

    # Check that a regular user (if exists) cannot access
    reg_user = psql("SELECT email FROM users u LEFT JOIN user_roles ur ON u.id = ur.user_id WHERE ur.user_id IS NULL LIMIT 1")
    if reg_user:
        s = get_session(reg_user)
        r = s.get(f"{BASE}/api/admin/system")
        check(f"Regular User ({reg_user}) access → 403", r.status_code == 403)

def test_stats_overview(session):
    section("2. STATS OVERVIEW API")
    r = session.get(f"{BASE}/api/admin/stats/overview")
    check("GET /api/admin/stats/overview → 200", r.status_code == 200)
    data = r.json()
    for key in ["total_users", "new_users_24h", "aum_cents", "deposits_24h_cents"]:
        check(f"Response has {key}", key in data)

def test_system_health(session):
    section("3. SYSTEM HEALTH API")
    r = session.get(f"{BASE}/api/admin/system")
    check("GET /api/admin/system → 200", r.status_code == 200)
    data = r.json()
    check("API health reported", data.get("api_healthy") is True)
    check("DB size present", "db_size" in data)
    check("Table stats list present", "tables" in data)

def test_dividend_tool(session):
    section("4. DIVIDEND TOOL (CALCULATE & PROCESS)")
    
    # Find active asset with investors
    asset_id = psql("SELECT asset_id FROM investments WHERE status = 'active' LIMIT 1")
    if not asset_id:
        print("  ⚠️ Skip: No active assets with investors found.")
        return

    # Calculate preview
    p_resp = session.post(f"{BASE}/api/admin/dividends/calculate", json={
        "asset_id": asset_id,
        "total_amount_cents": 100000 # $1000.00
    })
    check("POST /api/admin/dividends/calculate → 200", p_resp.status_code == 200)
    calc_data = p_resp.json()
    check("Splits calculated", len(calc_data.get("splits", [])) > 0)
    check("Total tokens reported", calc_data.get("total_tokens", 0) > 0)

    # Process batch
    pr_resp = session.post(f"{BASE}/api/admin/dividends/process", json={
        "asset_id": asset_id,
        "total_amount_cents": 5000 # $50.00 test
    })
    check("POST /api/admin/dividends/process → 200", pr_resp.status_code == 200)
    res = pr_resp.json()
    check("Success status in response", res.get("status") == "success")
    check("Payout ID returned", "payout_id" in res)

def test_rewards_management(session):
    section("5. REWARDS & BALANCES")
    
    r = session.get(f"{BASE}/api/admin/rewards")
    check("GET /api/admin/rewards → 200", r.status_code == 200)
    data = r.json()
    check("Tiers data present", "tiers" in data)
    
    if data.get("balances"):
        user = data["balances"][0]
        uid = user["user_id"]
        
        # Test adjustment
        adj_resp = session.post(f"{BASE}/api/admin/users/{uid}/balance", json={
            "amount_cents": 1000, # +$10.00
            "wallet_type": "rewards",
            "reason": "Test Suite Adjustment"
        })
        check("POST reward adjustment → 200", adj_resp.status_code == 200, 
              f"got {adj_resp.status_code}: {adj_resp.text}")
        
        # Test tier override
        tier_resp = session.post(f"{BASE}/api/admin/users/{uid}/profile", json={
            "tier": "Premium"
        })
        check("POST tier override → 200", tier_resp.status_code == 200)

def main():
    print("=" * 60)
    print("  POOOL ADMIN DASHBOARD — COMPREHENSIVE TEST SUITE")
    print("=" * 60)

    try:
        requests.get(f"{BASE}/", timeout=3)
    except:
        print("❌ Server not running at", BASE)
        sys.exit(1)

    session = get_session()
    
    test_admin_access_control()
    test_stats_overview(session)
    test_system_health(session)
    test_dividend_tool(session)
    test_rewards_management(session)

    print(f"\n{'='*60}")
    total = passed + failed
    print(f"  RESULTS: {passed}/{total} passed, {failed} failed")
    if errors:
        print("\n  FAILURES:")
        for e in errors: print(f"    • {e}")
    print("=" * 60)
    sys.exit(0 if failed == 0 else 1)

if __name__ == "__main__":
    main()
