#!/usr/bin/env python3
"""
Referral System Test Suite
==========================
Tests the full referral loop:
1. Referrer exists with a code and a tier.
2. New user signs up using that referral code.
3. Verify referral_tracking entry is created with reward amounts from tier.
4. Admin qualifies the referral.
5. Admin marks the referral as paid.
6. Verify rewards_balances are updated for both users.
"""

import json
import subprocess
import sys
import requests
import uuid
import time

BASE = "http://localhost:8888"
DB = "poool"

# ─── Helpers ─────────────────────────────────────────────────────

passed = 0
failed = 0
errors = []

def psql(sql: str) -> str:
    return subprocess.check_output(["psql", "-Atc", sql, DB]).decode().strip()

def get_session(email="admin@poool.finance") -> requests.Session:
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

# ─── Setup ──────────────────────────────────────────────────────

def setup_test_data():
    # Ensure we have at least one tier with a referral_bonus
    psql("INSERT INTO tiers (name, min_invest, cashback_pct, referral_bonus, badge_color, sort_order) \
          VALUES ('TestTier', 0, 1.0, 2500, '#000000', 0) \
          ON CONFLICT (name) DO UPDATE SET referral_bonus = 2500")
    
    # Ensure test@poool.app has this tier
    psql("INSERT INTO user_tiers (user_id, tier_id, invested_12m) \
          VALUES ((SELECT id FROM users WHERE email='test@poool.app'), (SELECT id FROM tiers WHERE name='TestTier'), 0) \
          ON CONFLICT (user_id) DO NOTHING")

    # Give test@poool.app a referral code
    psql("INSERT INTO referral_codes (user_id, code) \
          VALUES ((SELECT id FROM users WHERE email='test@poool.app'), 'TESTCODE') \
          ON CONFLICT (user_id) DO UPDATE SET code = 'TESTCODE'")
    
    # Ensure test@poool.app has a session
    psql("INSERT INTO user_sessions (id, user_id, session_token, expires_at) \
          VALUES (gen_random_uuid(), (SELECT id FROM users WHERE email='test@poool.app'), 'test-session-token', NOW() + INTERVAL '1 day') \
          ON CONFLICT DO NOTHING")

# ─── Tests ───────────────────────────────────────────────────────

def test_referral_loop():
    section("REFERRAL LOOP TEST")
    
    # 1. Signup a new user with 'TESTCODE'
    new_email = f"referred_{int(time.time())}@example.com"
    signup_data = {
        "email": new_email,
        "password": "Password123!",
        "terms_accepted": "on",
        "referral_code": "TESTCODE"
    }
    
    print(f"  → Signing up new user: {new_email}")
    r = requests.post(f"{BASE}/auth/signup", data=signup_data)
    check("Signup with referral code → 200/Redirect", r.status_code in (200, 302, 303))
    
    # 2. Verify referral_tracking record
    ref_track = psql(f"SELECT id FROM referral_tracking WHERE referred_id = (SELECT id FROM users WHERE email='{new_email}')")
    check("Referral tracking record created", len(ref_track) > 0)
    
    if not ref_track: return

    # Check rewards amounts
    referrer_reward = int(psql(f"SELECT referrer_reward FROM referral_tracking WHERE id='{ref_track}'"))
    referred_reward = int(psql(f"SELECT referred_reward FROM referral_tracking WHERE id='{ref_track}'"))
    check(f"Referrer reward is correct ($25.00): {referrer_reward}", referrer_reward == 2500)
    check(f"Referred reward is correct ($5.00): {referred_reward}", referred_reward == 500)

    # 3. Admin: Qualify the referral
    admin_session = get_session()
    r = admin_session.patch(f"{BASE}/api/admin/rewards/referrals/{ref_track}", json={"status": "qualified"})
    check("Admin: Qualify referral → 200", r.status_code == 200)
    
    status = psql(f"SELECT status FROM referral_tracking WHERE id='{ref_track}'")
    check("Status updated to 'qualified'", status == "qualified")

    # 4. Admin: Mark as Paid
    r = admin_session.patch(f"{BASE}/api/admin/rewards/referrals/{ref_track}", json={"status": "paid"})
    check("Admin: Mark as paid → 200", r.status_code == 200)
    
    # 5. Verify Rewards Balances
    time.sleep(1)
    
    referrer_bal = int(psql("SELECT referrals FROM rewards_balances WHERE user_id = (SELECT id FROM users WHERE email='test@poool.app')"))
    referred_bal = int(psql(f"SELECT referrals FROM rewards_balances WHERE user_id = (SELECT id FROM users WHERE email='{new_email}')"))
    
    check(f"Referrer balance updated (+2500): {referrer_bal}", referrer_bal >= 2500)
    check(f"Referred balance updated (+500): {referred_bal}", referred_bal == 500)

    # 6. Verify Rewards Overview for Referrer
    referrer_session = get_session("test@poool.app")
    r = referrer_session.get(f"{BASE}/api/rewards")
    check("Referrer GET /api/rewards → 200", r.status_code == 200)
    data = r.json()
    check(f"Referrer dashboard shows balance {data.get('referrals')}", data.get("referrals") == referrer_bal)
    check(f"Referrer dashboard shows code TESTCODE", data.get("referral_code") == "TESTCODE")

    # 7. Verify Rewards Overview for Referred
    referred_session = get_session(new_email)
    r = referred_session.get(f"{BASE}/api/rewards")
    check("Referred GET /api/rewards → 200", r.status_code == 200)
    data = r.json()
    check(f"Referred dashboard shows balance {data.get('referrals')}", data.get("referrals") == referred_bal)

def main():
    print("=" * 60)
    print("  POOOL REFERRAL SYSTEM — TEST SUITE")
    print("=" * 60)

    try:
        requests.get(f"{BASE}/", timeout=3)
    except:
        print("❌ Server not running at", BASE)
        sys.exit(1)

    setup_test_data()
    test_referral_loop()

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
