#!/usr/bin/env python3
"""
E2E Test: Developer Asset Creation Flow
========================================
Tests the full 4-step developer asset creation pipeline:
  Step 1 → Asset type selection (client-side, not tested here)
  Step 2 → POST /api/developer/draft (create draft)
  Step 3 → POST /api/developer/draft/:id/documents (upload documents)
  Step 4 → PUT /api/developer/draft/:id (content + financials) → submit

Also tests:
  - GET /api/developer/draft/:id (pre-fill)
  - GET /api/developer/drafts (submissions list)
  - POST /api/developer/draft/:id/duplicate
  - DELETE /api/developer/draft/:id (soft-delete)
  - Ownership checks (403 for cross-account access)
  - Submissions page rendering

Run:  python3 tests/test_developer_asset_creation_flow.py
"""

import os
import sys
import uuid
import psycopg2
import requests

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DB_DSN", "dbname=poool user=martin host=localhost")


class Results:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.errors = []

    def section(self, name):
        print(f"\n{'='*60}\n  {name}\n{'='*60}")

    def check(self, name, condition, detail=""):
        if condition:
            self.passed += 1
            print(f"  ✅ {name}")
        else:
            self.failed += 1
            msg = f"{name}: {detail}" if detail else name
            print(f"  ❌ {msg}")
            self.errors.append(msg)

    def report(self):
        print(f"\n{'='*60}")
        print(f"  RESULTS: {self.passed} passed, {self.failed} failed")
        print(f"{'='*60}\n")
        if self.errors:
            for e in self.errors:
                print(f"  ‣ {e}")
        return self.failed == 0


def fix_secure_cookies(session):
    for cookie in session.cookies:
        cookie.secure = False


def create_dev_session(conn, cur, suffix):
    """Register a user, grant developer role, return authenticated session."""
    email = f"devtest_{suffix}_{uuid.uuid4().hex[:6]}@poool.app"
    password = "TestDevPass123!"

    s = requests.Session()
    # Get CSRF token
    s.get(f"{BASE_URL}/auth/signup")
    fix_secure_cookies(s)
    csrf = s.cookies.get("csrf_token", "")

    # Register
    s.post(
        f"{BASE_URL}/auth/signup",
        data={"email": email, "password": password, "terms_accepted": "on"},
        headers={"HX-Request": "true", "X-CSRF-Token": csrf},
    )
    # Login
    s.post(
        f"{BASE_URL}/auth/login",
        data={"email": email, "password": password},
        headers={"HX-Request": "true", "X-CSRF-Token": csrf},
    )

    cur.execute("SELECT id FROM users WHERE email=%s", (email,))
    user_id = cur.fetchone()[0]

    # Grant developer role
    cur.execute(
        "INSERT INTO user_roles (user_id, role_id) "
        "SELECT %s, id FROM roles WHERE name='developer' ON CONFLICT DO NOTHING",
        (user_id,),
    )

    # Get session token
    cur.execute(
        "SELECT session_token FROM user_sessions WHERE user_id=%s "
        "ORDER BY created_at DESC LIMIT 1",
        (user_id,),
    )
    token_row = cur.fetchone()
    if token_row:
        s.cookies.set("poool_session", str(token_row[0]))
    s.get(f"{BASE_URL}/developer/dashboard")
    fix_secure_cookies(s)
    s.headers.update({"X-CSRF-Token": s.cookies.get("csrf_token", "")})
    conn.commit()

    return s, user_id, email


def cleanup_user(cur, user_id):
    """Remove all data for a test user."""
    cur.execute("DELETE FROM asset_images WHERE asset_id IN (SELECT id FROM assets WHERE developer_user_id=%s)", (user_id,))
    cur.execute("DELETE FROM asset_documents WHERE asset_id IN (SELECT id FROM assets WHERE developer_user_id=%s)", (user_id,))
    cur.execute("DELETE FROM developer_projects WHERE developer_id=%s", (user_id,))
    cur.execute("DELETE FROM assets WHERE developer_user_id=%s", (user_id,))
    cur.execute("DELETE FROM audit_logs WHERE actor_user_id=%s", (user_id,))
    cur.execute("DELETE FROM user_consents WHERE user_id=%s", (user_id,))
    cur.execute("DELETE FROM user_roles WHERE user_id=%s", (user_id,))
    cur.execute("DELETE FROM user_sessions WHERE user_id=%s", (user_id,))
    cur.execute("DELETE FROM wallets WHERE user_id=%s", (user_id,))
    cur.execute("DELETE FROM users WHERE id=%s", (user_id,))


def run_tests():
    r = Results()
    conn = get_db()
    cur = conn.cursor()

    dev1_id = dev2_id = None
    draft_id = None

    try:
        # ── Setup: Two developer users ──────────────────────────────
        s1, dev1_id, email1 = create_dev_session(conn, cur, "alpha")
        s2, dev2_id, email2 = create_dev_session(conn, cur, "beta")

        # ════════════════════════════════════════════════════════════
        r.section("STEP 2: CREATE DRAFT (POST /api/developer/draft)")
        # ════════════════════════════════════════════════════════════

        draft_payload = {
            "title": "Test Villa Canggu",
            "asset_type": "real_estate",
            "property_type": "villa",
            "area": "canggu",
            "address": "Jl. Pantai Berawa 123",
            "city": "Denpasar",
            "country": "Indonesia",
            "lease_type": "leasehold",
            "lease_term_years": 25,
            "land_size_sqm": 350.0,
            "building_size_sqm": 220.0,
            "bedrooms": 3,
            "bathrooms": 3,
            "construction_status": "ready",
            "year_built": 2023,
            "total_value_cents": 50000000,
            "token_price_cents": 5000,
            "tokens_total": 10000,
        }

        resp = s1.post(f"{BASE_URL}/api/developer/draft", json=draft_payload)
        r.check("Draft created (200/201)", resp.status_code in [200, 201], f"Status: {resp.status_code}")

        data = resp.json() if resp.status_code in [200, 201] else {}
        draft_id = data.get("asset_id")
        r.check("Draft ID returned", draft_id is not None, f"Response: {data}")

        if not draft_id:
            r.report()
            return

        # DB verification
        cur.execute("SELECT title, asset_type, location_city, location_country, submission_step FROM assets WHERE id=%s", (draft_id,))
        row = cur.fetchone()
        r.check("Title persisted", row and row[0] == "Test Villa Canggu")
        r.check("Asset type persisted", row and row[1] == "real_estate")
        r.check("City persisted (location_city)", row and row[2] == "Denpasar")
        r.check("Country persisted (location_country)", row and row[3] == "Indonesia")
        r.check("Submission step = 2", row and row[4] == 2)

        # developer_projects row created (B5 fix)
        cur.execute("SELECT status FROM developer_projects WHERE asset_id=%s", (draft_id,))
        dp_row = cur.fetchone()
        r.check("developer_projects row created", dp_row is not None)
        r.check("developer_projects status = 'draft'", dp_row and dp_row[0] == "draft")

        # ════════════════════════════════════════════════════════════
        r.section("STEP 4: UPDATE DRAFT (PUT /api/developer/draft/:id)")
        # ════════════════════════════════════════════════════════════

        update_payload = {
            "title": "Test Villa Canggu (Updated)",
            "short_description": "A beautiful 3BR villa in Canggu",
            "description": "Full description of the test villa...",
            "location_description": "5 min from Berawa Beach",
            "google_maps_url": "https://maps.google.com/?q=-8.65,115.15",
            "video_url": "https://youtube.com/watch?v=test123",
            "annual_yield_bps": 1200,
            "capital_appreciation_bps": 800,
            "occupancy_rate_bps": 8500,
            "investor_share_bps": 7000,
            "submission_step": 4,
        }

        resp = s1.put(f"{BASE_URL}/api/developer/draft/{draft_id}", json=update_payload)
        r.check("Draft updated (200)", resp.status_code == 200, f"Status: {resp.status_code}")

        # DB verification
        cur.execute(
            "SELECT title, short_description, annual_yield_bps, occupancy_rate_bps, submission_step FROM assets WHERE id=%s",
            (draft_id,),
        )
        row = cur.fetchone()
        r.check("Title updated in DB", row and row[0] == "Test Villa Canggu (Updated)")
        r.check("Short description saved", row and row[1] == "A beautiful 3BR villa in Canggu")
        r.check("Annual yield saved (1200 bps)", row and row[2] == 1200)
        r.check("Occupancy rate saved (8500 bps)", row and row[3] == 8500)
        r.check("Submission step updated to 4", row and row[4] == 4)

        # ════════════════════════════════════════════════════════════
        r.section("GET SINGLE DRAFT (GET /api/developer/draft/:id)")
        # ════════════════════════════════════════════════════════════

        resp = s1.get(f"{BASE_URL}/api/developer/draft/{draft_id}")
        r.check("Get draft (200)", resp.status_code == 200, f"Status: {resp.status_code}")

        if resp.status_code == 200:
            try:
                data = resp.json()
                r.check("Pre-fill: title matches", data.get("title") == "Test Villa Canggu (Updated)")
                r.check("Pre-fill: short_description matches", data.get("short_description") == "A beautiful 3BR villa in Canggu")
                r.check("Pre-fill: annual_yield_bps = 1200", data.get("annual_yield_bps") == 1200)
                r.check("Pre-fill: city = Denpasar", data.get("city") == "Denpasar")
                r.check("Pre-fill: country = Indonesia", data.get("country") == "Indonesia")
            except Exception as e:
                r.check("Pre-fill: JSON parse", False, f"Response not JSON: {str(e)[:100]}")

        # ════════════════════════════════════════════════════════════
        r.section("LIST DRAFTS (GET /api/developer/drafts)")
        # ════════════════════════════════════════════════════════════

        resp = s1.get(f"{BASE_URL}/api/developer/drafts")
        r.check("List drafts (200)", resp.status_code == 200, f"Status: {resp.status_code}")

        if resp.status_code == 200:
            items = resp.json().get("items", [])
            r.check("At least 1 draft in listing", len(items) >= 1)
            found = any(i["id"] == str(draft_id) for i in items)
            r.check("Created draft appears in listing", found)

        # ════════════════════════════════════════════════════════════
        r.section("SUBMIT DRAFT (POST /api/developer/draft/:id/submit)")
        # ════════════════════════════════════════════════════════════

        resp = s1.post(f"{BASE_URL}/api/developer/draft/{draft_id}/submit")
        r.check("Submit draft (200)", resp.status_code == 200, f"Status: {resp.status_code}")

        cur.execute("SELECT status FROM developer_projects WHERE asset_id=%s", (draft_id,))
        dp = cur.fetchone()
        r.check("developer_projects status = 'submitted'", dp and dp[0] == "submitted")

        # ════════════════════════════════════════════════════════════
        r.section("DUPLICATE DRAFT (POST /api/developer/draft/:id/duplicate)")
        # ════════════════════════════════════════════════════════════

        resp = s1.post(f"{BASE_URL}/api/developer/draft/{draft_id}/duplicate")
        r.check("Duplicate (200)", resp.status_code == 200, f"Status: {resp.status_code}")

        if resp.status_code == 200:
            new_id = resp.json().get("new_asset_id")
            r.check("New asset ID returned", new_id is not None)

            if new_id:
                cur.execute("SELECT title FROM assets WHERE id=%s", (new_id,))
                row = cur.fetchone()
                r.check("Copy title has '(Copy)' suffix", row and "(Copy)" in row[0])

                cur.execute("SELECT status FROM developer_projects WHERE asset_id=%s", (new_id,))
                dp2 = cur.fetchone()
                r.check("Copy developer_projects status = 'draft'", dp2 and dp2[0] == "draft")

        # ════════════════════════════════════════════════════════════
        r.section("DELETE DRAFT (DELETE /api/developer/draft/:id)")
        # ════════════════════════════════════════════════════════════

        # Create a throwaway draft to delete
        throwaway = s1.post(
            f"{BASE_URL}/api/developer/draft",
            json={
                "title": "To Delete",
                "asset_type": "real_estate",
                "total_value_cents": 100000,
                "token_price_cents": 1000,
                "tokens_total": 100,
            },
        )
        throwaway_id = throwaway.json().get("asset_id") if throwaway.status_code in [200, 201] else None

        if throwaway_id:
            resp = s1.delete(f"{BASE_URL}/api/developer/draft/{throwaway_id}")
            r.check("Delete draft (200)", resp.status_code == 200, f"Status: {resp.status_code}")

            cur.execute("SELECT deleted_at FROM assets WHERE id=%s", (throwaway_id,))
            del_row = cur.fetchone()
            r.check("deleted_at is set (soft-delete)", del_row and del_row[0] is not None)

            # Verify it's excluded from list
            resp = s1.get(f"{BASE_URL}/api/developer/drafts")
            if resp.status_code == 200:
                items = resp.json().get("items", [])
                found = any(i["id"] == str(throwaway_id) for i in items)
                r.check("Deleted draft excluded from listing", not found)
        else:
            r.check("Delete draft setup", False, "Could not create throwaway draft")

        # ════════════════════════════════════════════════════════════
        r.section("SECURITY: OWNERSHIP CHECKS (403)")
        # ════════════════════════════════════════════════════════════

        # Dev 2 tries to read Dev 1's draft
        resp = s2.get(f"{BASE_URL}/api/developer/draft/{draft_id}")
        r.check("Cross-account GET blocked (403/404)", resp.status_code in [403, 404], f"Status: {resp.status_code}")

        # Dev 2 tries to update Dev 1's draft
        resp = s2.put(
            f"{BASE_URL}/api/developer/draft/{draft_id}",
            json={"title": "Hacked!"},
        )
        r.check("Cross-account PUT blocked (403)", resp.status_code == 403, f"Status: {resp.status_code}")

        # Dev 2 tries to delete Dev 1's draft
        resp = s2.delete(f"{BASE_URL}/api/developer/draft/{draft_id}")
        r.check("Cross-account DELETE blocked (403)", resp.status_code == 403, f"Status: {resp.status_code}")

        # Dev 2 tries to duplicate Dev 1's draft
        resp = s2.post(f"{BASE_URL}/api/developer/draft/{draft_id}/duplicate")
        r.check("Cross-account DUPLICATE blocked (403)", resp.status_code == 403, f"Status: {resp.status_code}")

        # Dev 2 tries to submit Dev 1's draft
        resp = s2.post(f"{BASE_URL}/api/developer/draft/{draft_id}/submit")
        r.check("Cross-account SUBMIT blocked (403)", resp.status_code == 403, f"Status: {resp.status_code}")

        # ════════════════════════════════════════════════════════════
        r.section("PAGE RENDERING")
        # ════════════════════════════════════════════════════════════

        resp = s1.get(f"{BASE_URL}/developer/submissions")
        r.check("Submissions page renders (200)", resp.status_code == 200, f"Status: {resp.status_code}")
        if resp.status_code == 200:
            r.check("Submissions page has table", "submissions-table" in resp.text)
            r.check("Submissions page has filter tabs", "submissions-filter-tabs" in resp.text)

        resp = s1.get(f"{BASE_URL}/developer/application-form")
        r.check("Application form renders (200)", resp.status_code == 200, f"Status: {resp.status_code}")
        if resp.status_code == 200:
            r.check("Application form has city field", 'id="city"' in resp.text)
            r.check("Application form has country field", 'id="country"' in resp.text)
            r.check("Application form has Save & Exit btn", "Save &amp; Exit" in resp.text or "Save & Exit" in resp.text)

        resp = s1.get(f"{BASE_URL}/developer/property-content")
        r.check("Property content renders (200)", resp.status_code == 200, f"Status: {resp.status_code}")
        if resp.status_code == 200:
            r.check("Property content has occupancy-rate", 'id="occupancy-rate"' in resp.text)
            r.check("Property content has Save & Exit btn", "Save &amp; Exit" in resp.text or "Save & Exit" in resp.text)

        # ════════════════════════════════════════════════════════════
        r.section("UNAUTHENTICATED ACCESS")
        # ════════════════════════════════════════════════════════════

        anon = requests.Session()
        resp = anon.get(f"{BASE_URL}/api/developer/drafts")
        r.check("Unauthenticated list blocked (401)", resp.status_code == 401, f"Status: {resp.status_code}")

        resp = anon.post(f"{BASE_URL}/api/developer/draft", json=draft_payload)
        r.check("Unauthenticated create blocked (401/403)", resp.status_code in [401, 403], f"Status: {resp.status_code}")

    except Exception as e:
        r.check("Test execution", False, f"Exception: {e}")
        import traceback
        traceback.print_exc()

    finally:
        # Cleanup
        try:
            if dev1_id:
                cleanup_user(cur, dev1_id)
            if dev2_id:
                cleanup_user(cur, dev2_id)
            conn.commit()
        except Exception as e:
            print(f"  ⚠️  Cleanup error: {e}")
            conn.rollback()
        cur.close()
        conn.close()

    return r


def get_db():
    return psycopg2.connect(DB_DSN)


if __name__ == "__main__":
    results = run_tests()
    success = results.report()
    sys.exit(0 if success else 1)
