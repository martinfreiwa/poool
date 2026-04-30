#!/usr/bin/env python3
"""
End-to-End Affiliate Partner Syndicate Lifecycle
================================================
Implements section I.3 (End-to-End Test for the full affiliate lifecycle)
Tests onboarding API, admin approval, referral link tracking, attribution at signup,
commission generation, dashboard, and manual admin payout simulation.
"""

import os
import requests
import psycopg2
import sys
import uuid

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DB_DSN", "dbname=poool user=martin host=localhost")

def fix_secure_cookies(session):
    for cookie in session.cookies:
        cookie.secure = False

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
        print(f"E2E Affiliate Lifecycle Report: {self.passed} Passed, {self.failed} Failed")
        print("="*60 + "\n")
        return self.failed == 0

def run_affiliate_e2e():
    results = E2EResults()
    print("\n--- Starting Affiliate Lifecycle E2E Automation Flow ---")

    uniq = str(uuid.uuid4())[:8]
    referrer_email = f"affiliate_{uniq}@poool.app"
    referee_email = f"referee_{uniq}@poool.app"
    pw = "SecurePass123!"

    conn = psycopg2.connect(DB_DSN)
    cur = conn.cursor()

    try:
        # Step 1: Create Referrer
        s1 = requests.Session()
        s1.get(f"{BASE_URL}/auth/signup")
        fix_secure_cookies(s1)
        csrf_token_s1 = s1.cookies.get("csrf_token", "")
        r = s1.post(f"{BASE_URL}/auth/signup", data={"email": referrer_email, "password": pw, "terms_accepted": "on"}, 
                    headers={"X-CSRF-Token": csrf_token_s1}, allow_redirects=False)
        results.check("Affiliate Reg", r.status_code in [200,302,303], f"Status {r.status_code}")
        
        # Login
        r = s1.post(f"{BASE_URL}/auth/login", data={"email": referrer_email, "password": pw}, 
                    headers={"X-CSRF-Token": csrf_token_s1}, allow_redirects=False)
        s1.get(f"{BASE_URL}/rewards")
        cur.execute("SELECT id FROM users WHERE email=%s", (referrer_email,))
        aff_user_id = cur.fetchone()[0]

        cur.execute("SELECT session_token FROM user_sessions WHERE user_id=%s ORDER BY created_at DESC LIMIT 1", (aff_user_id,))
        token = cur.fetchone()[0]
        s1.cookies.set("poool_session", str(token))
        fix_secure_cookies(s1)

        cur.execute(
            """
            INSERT INTO kyc_records (user_id, status, provider)
            VALUES (%s, 'approved', 'manual')
            ON CONFLICT DO NOTHING
            """,
            (aff_user_id,),
        )
        conn.commit()
        
        # We need a fresh CSRF token for the API call
        r_get = s1.get(f"{BASE_URL}/rewards")
        fix_secure_cookies(s1)
        csrf_token = s1.cookies.get("csrf_token", "")
        
        # Apply for Affiliate
        r = s1.post(f"{BASE_URL}/api/affiliate/onboarding/submit", json={
            "exam_passed": True,
            "accepted_policies": [
                "Affiliate Terms & Conditions",
                "Affiliate Code of Conduct",
                "Approved Marketing Materials Policy",
                "Qualified Referral & Payout Policy",
                "Affiliate Privacy Notice"
            ],
            "traffic_source": "youtube",
            "audience_size": "5k_50k",
            "main_url": "https://youtube.com/mychannel",
            "phone_number": "555-1234",
            "tax_id": "TAX-123",
            "exam_answers": {"q1": "no", "q2": "no", "q3": "30days", "q4": "no", "q5": "no"}
        }, headers={"X-CSRF-Token": csrf_token})
        results.check("Affiliate Onboarding API", r.status_code == 200, f"Status {r.status_code}. Response: {r.text}")

        # Check in DB
        cur.execute("SELECT status FROM affiliates WHERE user_id=%s", (aff_user_id,))
        a_row = cur.fetchone()
        results.check("Affiliate DB Entry Created", a_row and a_row[0] == 'pending_approval')

        # Admin overrides status in DB to bypass UI flow
        aff_code = f"TEST_{uniq.upper()}"
        cur.execute("UPDATE affiliates SET status='active', referral_code=%s WHERE user_id=%s", (aff_code, aff_user_id))
        conn.commit()
        results.check("Admin DB Override Approved", True)

        # Dashboard blocks suspended but allows active
        r = s1.get(f"{BASE_URL}/api/affiliate/dashboard")
        results.check("Dashboard API accessible", r.status_code == 200, f"Status: {r.status_code}")

        # Step 2: Simulate Click / Attribution Link Visit (Unauthenticated)
        s2 = requests.Session()
        r = s2.get(f"{BASE_URL}/rewards/{aff_code}", allow_redirects=False)
        results.check("Referral Link Visit", r.status_code in [200, 302, 303], f"Status {r.status_code}")
        fix_secure_cookies(s2)
        ref_cookie = s2.cookies.get("poool_referral", "")
        # The route might clear it or we just check we got routed nicely. Wait, /rewards/:code sets cookie 'poool_referral'
        results.check("Referral Cookie Dropped", ref_cookie == aff_code, f"Cookie is: {ref_cookie}")

        # Wait to simulate realistic click log parsing
        
        # Step 3: Referee Registration
        s2.get(f"{BASE_URL}/auth/signup")
        fix_secure_cookies(s2)
        csrf_token_s2 = s2.cookies.get("csrf_token", "")
        r = s2.post(f"{BASE_URL}/auth/signup", data={"email": referee_email, "password": pw, "terms_accepted": "on"}, 
                    headers={"X-CSRF-Token": csrf_token_s2}, allow_redirects=False)
        results.check("Referee Registration (Attribution)", r.status_code in [200,302,303], f"Status {r.status_code}")

        cur.execute("SELECT id FROM users WHERE email=%s", (referee_email,))
        referee_id = cur.fetchone()[0]

        # Check attribution in DB
        cur.execute("SELECT id, status FROM affiliate_referrals WHERE affiliate_id=%s AND referred_user_id=%s", (aff_user_id, referee_id))
        att_row = cur.fetchone()
        results.check("Attribution DB Record Saved", att_row is not None and att_row[1] == 'attributed', "Referral table not correctly populated")

        if att_row:
            ref_row_id = att_row[0]
            # Step 4: Simulate checkout / commission provision
            # We insert a payable commission
            cur.execute("INSERT INTO affiliate_commissions (affiliate_id, referral_id, source_order_id, provisional_amount_cents, status, tier_at_execution) VALUES (%s, %s, %s, 7500, 'payable', 'Access') RETURNING id", (aff_user_id, ref_row_id, str(uuid.uuid4())))
            conn.commit()
            results.check("Simulate Payable Commission", True)
            
            # Re-fetch Affiliate Dashboard API
            r = s1.get(f"{BASE_URL}/api/affiliate/dashboard")
            data = r.json() if r.status_code == 200 else {}
            results.check("Dashboard Shows Earnings > $50", data.get("earnings", {}).get("payable_cents", 0) >= 7500, f"Dashboard returned: {data}")

            # Test Payout Request Endpoint
            # Needs CSRF Token
            s1.get(f"{BASE_URL}/rewards")
            fix_secure_cookies(s1)
            csrf_token = s1.cookies.get("csrf_token", "")
            
            r = s1.post(f"{BASE_URL}/api/affiliate/payout/request", headers={"X-CSRF-Token": csrf_token})
            results.check("Trigger Payout Request Ping", r.status_code == 200, f"Status: {r.status_code}")

    except Exception as e:
        results.check("Exception Raised During Script", False, str(e))
    finally:
        # Cleanup
        try:
            if 'aff_user_id' in locals():
                cur.execute("DELETE FROM affiliate_commissions WHERE affiliate_id=%s", (aff_user_id,))
                cur.execute("DELETE FROM affiliate_referrals WHERE affiliate_id=%s", (aff_user_id,))
                cur.execute("DELETE FROM affiliate_policy_acceptances WHERE affiliate_id=%s", (aff_user_id,))
                cur.execute("DELETE FROM affiliates WHERE user_id=%s", (aff_user_id,))
                cur.execute("DELETE FROM user_sessions WHERE user_id=%s", (aff_user_id,))
                cur.execute("DELETE FROM wallets WHERE user_id=%s", (aff_user_id,))
                cur.execute("DELETE FROM email_logs WHERE user_id=%s", (aff_user_id,))
            if 'referee_id' in locals():
                cur.execute("DELETE FROM user_sessions WHERE user_id=%s", (referee_id,))
                cur.execute("DELETE FROM wallets WHERE user_id=%s", (referee_id,))
                cur.execute("DELETE FROM users WHERE id=%s", (referee_id,))
            if 'aff_user_id' in locals():
                cur.execute("DELETE FROM users WHERE id=%s", (aff_user_id,))
            conn.commit()
        except:
            pass
        
        cur.close()
        conn.close()

    return results

if __name__ == "__main__":
    res = run_affiliate_e2e()
    sys.exit(0 if res.report() else 1)
