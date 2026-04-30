#!/usr/bin/env python3
"""
POOOL Platform — E2E Affiliate Full Funnel
===========================================
End-to-End test covering the complete affiliate lifecycle:
  1. Affiliate Registration & Onboarding
  2. Admin Approval & Code Generation
  3. Client Click Tracking & Registration (Cookie Attribution)
  4. Client Checkout & Commission Generation (provisional)
  5. Holdback Worker (simulated) 30-day clearance
  6. Admin Batch Payout -> Treasury to Affiliate Wallet
"""

import os
import sys
import time
import uuid
import requests
import psycopg2

BASE_URL = os.environ.get("BASE_URL", "http://127.0.0.1:8888")
DB_DSN = os.environ.get("DATABASE_URL", "dbname=poool user=martin host=127.0.0.1")
REQUEST_TIMEOUT = 15

passed = 0
failed = 0


def ok(msg):
    global passed
    passed += 1
    print(f"  ✅  {msg}")


def fail(msg, detail=""):
    global failed
    failed += 1
    full = f"{msg}: {detail}" if detail else msg
    print(f"  ❌  {full}")
    sys.exit(1)


def section(name):
    print(f"\n{'─' * 70}")
    print(f"  {name}")
    print(f"{'─' * 70}")


def get_db():
    return psycopg2.connect(DB_DSN)


def main():
    print("=" * 70)
    print("  POOOL Platform — Affiliate Full Funnel Test")
    print(f"  Target: {BASE_URL}")
    print("=" * 70)

    # 1. Setup Test Identities
    affiliate_email = f"affiliate_{uuid.uuid4().hex[:8]}@example.com"
    client_email = f"client_{uuid.uuid4().hex[:8]}@example.com"
    password = "TestPassword123!"

    section("1. Affiliate Registration & Application")
    s_aff = requests.Session()
    
    # Needs CSRF Token
    s_aff.get(f"{BASE_URL}/auth/signup", timeout=REQUEST_TIMEOUT)
    csrf = s_aff.cookies.get("csrf_token", "")
    
    r = s_aff.post(
        f"{BASE_URL}/auth/signup",
        data={"email": affiliate_email, "password": password, "confirm_password": password, "terms_accepted": "on"},
        headers={"X-CSRF-Token": csrf, "HX-Request": "true"}
    )
    if r.status_code == 200 and "poool_session" in s_aff.cookies:
        ok(f"Created affiliate user: {affiliate_email}")
    elif r.status_code == 200:
        fail("Signup returned 200 but no session cookie!", r.text)
    else:
        fail("Failed to create affiliate user", r.status_code)

    # Login to get session (only needed if signup doesn't set poool_session, but since we check for it, we might be fine, keeping it just in case)
    r_login = s_aff.post(
        f"{BASE_URL}/auth/login",
        data={"email": affiliate_email, "password": password},
        headers={"X-CSRF-Token": csrf, "HX-Request": "true"}
    )
    if r_login.status_code != 200:
        fail(f"Login failed: {r_login.status_code}", r_login.text)

    s_aff.get(f"{BASE_URL}/settings", timeout=REQUEST_TIMEOUT)
    csrf = s_aff.cookies.get("csrf_token", "")

    conn = get_db()
    cur = conn.cursor()
    cur.execute("SELECT id FROM users WHERE email = %s", (affiliate_email,))
    aff_user_id = cur.fetchone()[0]
    cur.execute(
        "INSERT INTO kyc_records (user_id, status, provider) VALUES (%s, 'approved', 'manual') ON CONFLICT DO NOTHING",
        (aff_user_id,),
    )
    conn.commit()
    conn.close()

    # Submit onboarding
    r = s_aff.post(
        f"{BASE_URL}/api/affiliate/onboarding/submit",
        json={
            "exam_passed": True,
            "status": None,
            "traffic_source": "youtube",
            "audience_size": "5k_50k",
            "main_url": "https://youtube.com/test",
            "phone_number": "+1234567890",
            "tax_id": "12-34567",
            "company_name": "Test Media LLC",
            "accepted_policies": [
                "Affiliate Terms & Conditions",
                "Affiliate Code of Conduct",
                "Approved Marketing Materials Policy",
                "Qualified Referral & Payout Policy",
                "Affiliate Privacy Notice"
            ],
            "exam_answers": {"q1": "no", "q2": "no", "q3": "30days", "q4": "no", "q5": "no"}
        },
        headers={"X-CSRF-Token": csrf}
    )
    if r.status_code in (200, 201):
        ok("Submitted affiliate onboarding form successfully")
    else:
        fail("Onboarding submission failed", r.text)


    section("2. Admin UI: Approve Affiliate")
    # Find the affiliate in the DB
    conn = get_db()
    cur = conn.cursor()
    cur.execute("SELECT id FROM users WHERE email = %s", (affiliate_email,))
    aff_user_id = cur.fetchone()[0]

    # Elevate a mock admin account or use DB directly. The API requires an AdminUser.
    cur.execute("INSERT INTO user_roles (user_id, role_id) SELECT %s, id FROM roles WHERE name = 'admin' ON CONFLICT DO NOTHING", (aff_user_id,))
    conn.commit()

    # Admin approves
    r = s_aff.post(
        f"{BASE_URL}/api/admin/rewards/affiliates/{aff_user_id}/approve",
        json={"commission_rate_bps": 500, "assigned_tier": "gold"},
        headers={"X-CSRF-Token": csrf}
    )
    if r.status_code == 200:
        ok("Admin approval executed successfully via API")
    else:
        fail("Admin approval failed", r.text)

    cur.execute(
        "UPDATE affiliates SET tax_document_gcs_path = %s, is_tax_ready = TRUE WHERE user_id = %s",
        (f"e2e/affiliate-tax-docs/{aff_user_id}.pdf", aff_user_id),
    )
    conn.commit()
    ok("Seeded affiliate tax document gate for payout test")

    # Fetch referral code
    cur.execute("SELECT referral_code FROM affiliates WHERE user_id = %s", (aff_user_id,))
    code = cur.fetchone()[0]
    ok(f"Generated referral code: {code}")


    section("3. Client Journey: Click Link & Register")
    s_client = requests.Session()
    # Simulate a click on the tracking link
    r = s_client.get(f"{BASE_URL}/rewards/{code}", allow_redirects=False)
    
    cookie_ref = s_client.cookies.get("poool_referral")
    if cookie_ref == code:
        ok("poool_referral tracking cookie deposited successfully")
    else:
        fail("poool_referral cookie not set", r.headers)

    s_client.get(f"{BASE_URL}/auth/signup", timeout=REQUEST_TIMEOUT)
    client_csrf = s_client.cookies.get("csrf_token", "")

    # Client signs up (Cookie is sent automatically by Requests session)
    r = s_client.post(
        f"{BASE_URL}/auth/signup",
        data={"email": client_email, "password": password, "confirm_password": password, "terms_accepted": "on"},
        headers={"X-CSRF-Token": client_csrf, "HX-Request": "true"}
    )
    if r.status_code == 200:
        ok("Referred client registered successfully")
    else:
        fail("Referred client registration failed")

    s_client.post(
        f"{BASE_URL}/auth/login",
        data={"email": client_email, "password": password},
        headers={"X-CSRF-Token": client_csrf, "HX-Request": "true"}
    )

    # Verify attribution in Database
    cur.execute("SELECT id FROM users WHERE email = %s", (client_email,))
    client_user_id = cur.fetchone()[0]

    cur.execute("SELECT id FROM affiliate_referrals WHERE affiliate_id = %s AND referred_user_id = %s", (aff_user_id, client_user_id))
    referral_record = cur.fetchone()
    if referral_record:
        ok("Attribution successfully tracked in affiliate_referrals table")
        referral_id = referral_record[0]
    else:
        fail("No attribution record found!")


    section("4. Client Checkout: Commission Trigger")
    # Fast-lane testing: Create an asset and give the client ledger-backed money.
    cur.execute(
        """
        UPDATE wallets
           SET balance_cents = 1000000
         WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
         RETURNING id
        """,
        (client_user_id,),
    )
    client_wallet_id = cur.fetchone()[0]
    cur.execute("DELETE FROM wallet_transactions WHERE wallet_id = %s", (client_wallet_id,))
    cur.execute(
        """
        INSERT INTO wallet_transactions (
            wallet_id, type, status, amount_cents, currency, description, external_ref_id, completed_at
        )
        VALUES (%s, 'admin_credit', 'completed', 1000000, 'USD', 'E2E affiliate initial wallet funding', %s, NOW())
        """,
        (client_wallet_id, f"e2e-affiliate-initial-funding:{client_user_id}"),
    )
    
    # We need a dummy asset for checkout
    asset_id = str(uuid.uuid4())
    cur.execute(
        "INSERT INTO assets (id, developer_user_id, title, slug, asset_type, total_value_cents, token_price_cents, tokens_total, tokens_available, funding_status, published, featured, min_funding_tokens) "
        "VALUES (%s, %s, 'E2E Test Asset', %s, 'real_estate', 10000000, 100, 100000, 100000, 'funding_open', true, false, 0)",
        (asset_id, client_user_id, f"e2e-test-asset-{uuid.uuid4().hex[:6]}")
    )
    
    # KYC is required to perform checkout
    cur.execute(
        "INSERT INTO kyc_records (user_id, status, provider) VALUES (%s, 'approved', 'manual')",
        (client_user_id,)
    )
    
    conn.commit()

    # Add to cart
    r1 = s_client.post(f"{BASE_URL}/cart/add", data={"property_id": asset_id, "investment_amount": "1000"}, headers={"X-CSRF-Token": client_csrf})
    if r1.status_code != 200:
        fail(f"Failed to add to cart: {r1.status_code}", r1.text)

    # Checkout API: Requires cart_token.
    r_cart = s_client.get(f"{BASE_URL}/api/cart", timeout=REQUEST_TIMEOUT)
    if r_cart.status_code != 200:
        fail(f"Failed to get cart: {r_cart.status_code}", r_cart.text)
    
    cart_token = r_cart.json().get("cart_token")
    if not cart_token:
        fail("No cart_token returned in cart response", r_cart.text)

    r = s_client.post(
        f"{BASE_URL}/api/checkout",
        json={"payment_method": "wallet", "return_url": "http://localhost", "cart_token": cart_token, "agree_to_terms": True, "affiliate_disclosure_accepted": True},
        headers={"X-CSRF-Token": client_csrf}
    )
    if r.status_code == 200:
        ok("Referred client checkout successful (Commission generated!)")
    else:
        fail(f"Checkout failed ({r.status_code})", r.text)

    # Verify provisional commission generated
    cur.execute("SELECT id, provisional_amount_cents, status FROM affiliate_commissions WHERE referral_id = %s", (referral_id,))
    comms = cur.fetchall()
    if comms and comms[0][2] == "provisionally_tracked":
        ok(f"Commission provisionally tracked! Amount: {comms[0][1]} cents")
        commission_id = comms[0][0]
    else:
        fail("Commission not provisionally tracked")


    section("5. Holdback Expiration & Payout Engine")
    # Simulate DB expiration
    cur.execute("UPDATE affiliate_referrals SET status = 'qualified' WHERE id = %s", (referral_id,))
    cur.execute("UPDATE affiliate_commissions SET status = 'payable', provisional_amount_cents = 6000 WHERE id = %s", (commission_id,)) # Artificially bump > $50 for threshold
    
    # Ensure treasury has money
    cur.execute("SELECT id FROM wallets WHERE wallet_type = 'affiliate_treasury' AND currency = 'USD' LIMIT 1")
    treasury = cur.fetchone()
    if not treasury:
        cur.execute(
            "INSERT INTO wallets (user_id, wallet_type, balance_cents, currency) VALUES (NULL, 'affiliate_treasury', 9999999, 'USD') RETURNING id"
        )
        treasury_wallet_id = cur.fetchone()[0]
    else:
        cur.execute("UPDATE wallets SET balance_cents = 9999999 WHERE id = %s RETURNING id", (treasury[0],))
        treasury_wallet_id = cur.fetchone()[0]
    cur.execute(
        """
        INSERT INTO wallet_transactions (
            wallet_id, type, status, amount_cents, currency, description, external_ref_id, completed_at
        )
        SELECT %s, 'admin_credit', 'completed', 9999999 - COALESCE(SUM(amount_cents), 0),
               'USD', 'E2E affiliate treasury funding adjustment', %s, NOW()
          FROM wallet_transactions
         WHERE wallet_id = %s AND status = 'completed'
        HAVING 9999999 - COALESCE(SUM(amount_cents), 0) != 0
        """,
        (treasury_wallet_id, f"e2e-affiliate-treasury-funding:{treasury_wallet_id}", treasury_wallet_id),
    )
    conn.commit()

    # Admin issues batch payout (s_aff is still 'admin')
    r = s_aff.post(
        f"{BASE_URL}/api/admin/rewards/affiliates/{aff_user_id}/payout",
        headers={"X-CSRF-Token": csrf}
    )
    if r.status_code == 200:
        ok("Batch Payout EXECUTED via Finance Engine")
    else:
        fail("Batch payout failed", r.text)

    # Verify wallets
    cur.execute("SELECT balance_cents FROM wallets WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'", (aff_user_id,))
    aff_balance = cur.fetchone()[0]
    if aff_balance >= 6000:
        ok(f"Affiliate received funds! Wallet balance: {aff_balance} cents")
    else:
        fail(f"Affiliate wallet balance unchanged: {aff_balance}")

    cur.execute("SELECT COUNT(*) FROM payout_batches WHERE affiliate_id = %s AND total_amount_cents = 6000", (aff_user_id,))
    if cur.fetchone()[0] >= 1:
        ok("Payout batch persisted with exact cents")
    else:
        fail("Payout batch missing after affiliate payout")

    cur.execute("SELECT COUNT(*) FROM wallet_transactions WHERE external_ref_id IN (SELECT id::text FROM payout_batches WHERE affiliate_id = %s)", (aff_user_id,))
    if cur.fetchone()[0] >= 2:
        ok("Wallet ledger entries persisted for payout batch")
    else:
        fail("Payout wallet ledger entries missing")

    cur.close()
    conn.close()

    print(f"\n🎉 ALL {passed} TESTS PASSED! FULL FUNNEL VERIFIED.")


if __name__ == "__main__":
    main()
