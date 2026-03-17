#!/usr/bin/env python3
"""
POOOL Platform - Comprehensive Test Suite
Tests pages, API endpoints, database state and frontend elements.
Run: python3 tests/test_platform.py
"""
import json
import sys
from collections import defaultdict
from html.parser import HTMLParser

import psycopg2
import requests

BASE_URL = "http://127.0.0.1:8888"
DB_DSN = "dbname=poool user=martin host=127.0.0.1"
TEST_EMAIL = "test@poool.app"
TEST_PASSWORD = "TestPass123!"
LOGIN_TIMEOUT = 60
REQUEST_TIMEOUT = 15


# ─── HTML Parser ──────────────────────────────────────────────────

class HTMLAnalyzer(HTMLParser):
    def __init__(self):
        super().__init__()
        self.title = ""
        self.in_title = False
        self.scripts = []
        self.stylesheets = []
        self.images = []
        self.links = []
        self.ids = set()
        self.h1_tags = []
        self.in_h1 = False
        self._cur_h1 = ""
        self.has_sidebar = False
        self.has_mobile_nav = False
        self.has_user_data = False

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "title":
            self.in_title = True
        if tag == "h1":
            self.in_h1 = True
            self._cur_h1 = ""
        if "id" in a:
            self.ids.add(a["id"])
            if "sidebar" in a["id"]:
                self.has_sidebar = True
        cls = a.get("class", "")
        if "sidebar" in cls.lower():
            self.has_sidebar = True
        if "mobile-burger" in cls or "mobile-nav" in cls:
            self.has_mobile_nav = True
        if tag == "script":
            src = a.get("src", "")
            if src:
                self.scripts.append(src)
                if "user-data" in src:
                    self.has_user_data = True
        if tag == "link" and a.get("rel") == "stylesheet":
            href = a.get("href", "")
            if href:
                self.stylesheets.append(href)
        if tag == "img":
            src = a.get("src", "")
            if src:
                self.images.append(src)
        if tag == "a":
            href = a.get("href", "")
            if href:
                self.links.append(href)

    def handle_data(self, data):
        if self.in_title:
            self.title += data
        if self.in_h1:
            self._cur_h1 += data

    def handle_endtag(self, tag):
        if tag == "title":
            self.in_title = False
        if tag == "h1":
            self.in_h1 = False
            if self._cur_h1.strip():
                self.h1_tags.append(self._cur_h1.strip())


# ─── Results ──────────────────────────────────────────────────────

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
        print(f"  \u2705  {msg}")

    def fail(self, msg, detail=""):
        self.failed += 1
        full = f"{msg}: {detail}" if detail else msg
        self.errors.append((self.current_section, full))
        print(f"  \u274c  {full}")

    def warn(self, msg):
        self.warnings += 1
        self.warnings_list.append((self.current_section, msg))
        print(f"  \u26a0\ufe0f   {msg}")

    def info(self, msg):
        print(f"  \u2139\ufe0f   {msg}")


# ─── Helpers ──────────────────────────────────────────────────────

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
            r = session.get(f"{BASE_URL}/api/me", timeout=REQUEST_TIMEOUT)
            if r.status_code == 200:
                return session
    except Exception:
        pass
    try:
        resp = session.get(f"{BASE_URL}/settings", timeout=REQUEST_TIMEOUT)
        if "csrf_token" in session.cookies:
            session.headers.update({"X-CSRF-Token": session.cookies["csrf_token"]})

        resp = session.post(
            f"{BASE_URL}/auth/login",
            data={"email": TEST_EMAIL, "password": TEST_PASSWORD},
            headers={"HX-Request": "true", "HX-Current-URL": f"{BASE_URL}/auth/login", "X-CSRF-Token": session.cookies.get("csrf_token", "")},
            allow_redirects=False,
            timeout=LOGIN_TIMEOUT,
        )
        if resp.status_code in (200, 302, 303):
            if "poool_session" in session.cookies.get_dict():
                return session
    except requests.exceptions.Timeout:
        pass
    return session


def analyze(html_text: str) -> HTMLAnalyzer:
    a = HTMLAnalyzer()
    try:
        a.feed(html_text)
    except Exception:
        pass
    return a


def check_static(session: requests.Session, path: str) -> bool:
    if not path.startswith("/") or path.startswith("//"):
        return True
    try:
        r = session.head(f"{BASE_URL}{path}", timeout=5)
        return r.status_code == 200
    except Exception:
        return False


# ─── Database Tests ───────────────────────────────────────────────

def test_database(results: TestResults):
    results.section("DATABASE \u2013 Schema & Data Integrity")
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
    except Exception as e:
        results.fail("DB connection failed", str(e))
        return

    cur.execute(
        "SELECT table_name FROM information_schema.tables "
        "WHERE table_schema = 'public' ORDER BY table_name"
    )
    tables = {r[0] for r in cur.fetchall()}

    for t in [
        "users", "user_profiles", "user_sessions", "wallets",
        "wallet_transactions", "assets", "asset_images", "investments",
        "cart_items", "orders", "roles", "user_roles", "user_settings",
        "notifications", "kyc_records",
    ]:
        if t in tables:
            results.ok(f"Table '{t}' exists")
        else:
            results.fail(f"Table '{t}' MISSING")

    results.section("DATABASE \u2013 User Data Integrity")
    cur.execute("SELECT id, email FROM users")
    for uid, email in cur.fetchall():
        cur.execute(
            "SELECT first_name, last_name, phone_number, date_of_birth, country "
            "FROM user_profiles WHERE user_id = %s",
            (uid,),
        )
        row = cur.fetchone()
        if row:
            empty = [
                f for f, v in zip(
                    ["first_name", "last_name", "phone_number", "date_of_birth", "country"],
                    row,
                )
                if not v
            ]
            if empty:
                results.warn(f"Profile incomplete \u2013 missing: {', '.join(empty)}")
            else:
                results.ok(f"Profile complete for {email}")
        else:
            results.fail(f"Profile MISSING for {email}")

        cur.execute(
            "SELECT COUNT(*) FROM user_settings WHERE user_id = %s", (uid,)
        )
        if cur.fetchone()[0] == 0:
            results.warn(f"User settings MISSING for {email}")

    results.section("DATABASE \u2013 Assets & Marketplace Data")
    cur.execute("SELECT id, title FROM assets")
    assets = cur.fetchall()
    if not assets:
        results.warn("No assets \u2013 marketplace will be empty")
    else:
        results.ok(f"{len(assets)} assets found")
        for aid, title in assets:
            cur.execute(
                "SELECT COUNT(*) FROM asset_images WHERE asset_id = %s", (aid,)
            )
            if cur.fetchone()[0] == 0:
                results.warn(f"No images for asset '{title}'")

    cur.close()
    conn.close()


# ─── Auth Tests ───────────────────────────────────────────────────

def test_authentication(results: TestResults):
    results.section("AUTHENTICATION \u2013 Login & Session Flow")

    r = requests.get(f"{BASE_URL}/auth/login", allow_redirects=False)
    if r.status_code == 200:
        results.ok("GET /auth/login returns 200")
    else:
        results.fail("GET /auth/login", f"status={r.status_code}")

    for page in ["/marketplace", "/wallet", "/portfolio", "/rewards", "/cart",
                 "/settings", "/kyc"]:
        r = requests.get(
            f"{BASE_URL}{page}", allow_redirects=False, timeout=REQUEST_TIMEOUT
        )
        if r.status_code in (302, 303):
            results.ok(f"GET {page} redirects unauthenticated ({r.status_code})")
        elif r.status_code == 200:
            results.fail(f"GET {page} accessible WITHOUT auth!")
        else:
            results.warn(f"GET {page} returned {r.status_code}")

    # The backend login route is HTMX-first: it returns 200 + HX-Redirect header.
    # We must send HX-Request: true to get that response; without it the
    # browser would receive 200 with no redirect instruction.
    s = requests.Session()
    s.get(f"{BASE_URL}/settings", timeout=REQUEST_TIMEOUT)
    HTMX_HEADERS = {
        "HX-Request": "true",
        "HX-Current-URL": f"{BASE_URL}/auth/login",
        "X-CSRF-Token": s.cookies.get("csrf_token", "")
    }
    try:
        r = s.post(
            f"{BASE_URL}/auth/login",
            data={"email": TEST_EMAIL, "password": TEST_PASSWORD},
            headers=HTMX_HEADERS,
            allow_redirects=False,
            timeout=LOGIN_TIMEOUT,
        )
        if r.status_code == 200:
            results.ok(f"POST /auth/login returns 200 (HTMX response)")
        elif r.status_code in (302, 303):
            results.ok(f"POST /auth/login redirects ({r.status_code})")
        elif r.status_code == 401:
            results.fail(
                "POST /auth/login returns 401",
                f"Password '{TEST_PASSWORD}' does not match DB hash for {TEST_EMAIL}"
            )
        else:
            results.fail("POST /auth/login", f"status={r.status_code}")

        if r.headers.get("HX-Redirect"):
            results.ok(f"Login returns HX-Redirect: {r.headers['HX-Redirect']}")
        elif r.status_code in (302, 303):
            results.ok("Login redirects via Location header")
        else:
            results.fail(
                "Login missing HX-Redirect header",
                "Backend did not return HX-Redirect on successful login"
            )

        if "poool_session" in s.cookies.get_dict():
            results.ok("Session cookie 'poool_session' set after login")
        else:
            results.fail(
                "Session cookie 'poool_session' NOT SET",
                "Login succeeded but no session cookie returned"
            )
    except requests.exceptions.Timeout:
        results.warn("POST /auth/login timed out (Argon2id is CPU-intensive)")

    # Invalid login
    s_inv = requests.Session()
    s_inv.get(f"{BASE_URL}/settings", timeout=REQUEST_TIMEOUT)
    r = s_inv.post(
        f"{BASE_URL}/auth/login",
        data={"email": "x@x.com", "password": "bad"},
        headers={"X-CSRF-Token": s_inv.cookies.get("csrf_token", "")},
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code == 401:
        results.ok("Invalid login handled (status=401)")
    else:
        results.warn(f"Invalid login returned {r.status_code} (expected 401)")

    # Root redirect
    r = requests.get(f"{BASE_URL}/", allow_redirects=False)
    if r.status_code in (301, 302, 303):
        results.ok("Root / redirects to login")
        loc = r.headers.get("Location", "")
        if "login" in loc or "marketplace" in loc:
            results.ok("Redirect target is correct (login or marketplace)")
        else:
            results.warn(f"Unexpected redirect target: {loc}")
    else:
        results.warn(f"Root / returned {r.status_code}")


# ─── Page Helper ──────────────────────────────────────────────────

def test_page(session, results, path, check_sidebar=True,
              check_css=None, check_ids=None, check_text=None):
    r = session.get(f"{BASE_URL}{path}", allow_redirects=True,
                    timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok(f"GET {path} returns 200")
    elif r.status_code in (302, 303):
        results.fail(f"GET {path} redirects (auth issue?)", r.headers.get("Location", ""))
        return None, None
    elif r.status_code == 404:
        results.fail(f"GET {path} returns 404")
        return None, None
    else:
        results.fail(f"GET {path}", f"status={r.status_code}")
        return None, None

    html = analyze(r.text)

    if len(r.text) > 1000:
        results.ok(f"Content: {len(r.text):,} bytes")
    else:
        results.warn(f"Content seems small: {len(r.text)} bytes")

    if check_sidebar:
        if html.has_sidebar:
            results.ok("Sidebar present")
        else:
            results.warn("No sidebar detected")

    if check_css:
        for css in check_css:
            if any(css in s for s in html.stylesheets) or css in r.text:
                results.ok(f"CSS: /static/css/{css}")
            else:
                results.warn(f"CSS '{css}' not found")

    if check_ids:
        for eid in check_ids:
            if eid in html.ids or eid in r.text:
                results.ok(f"Element '{eid}' found")
            else:
                results.fail(f"Element '{eid}' NOT FOUND")

    if check_text:
        for text in check_text:
            if text in r.text:
                results.ok(f"Text '{text}' found")
            else:
                results.warn(f"Text '{text}' not found")

    return html, r


# ─── Wallet Tests ────────────────────────────────────────────────

def test_wallet(session, results):
    results.section("PAGE: /wallet")
    html, r = test_page(
        session, results, "/wallet",
        check_css=["wallet.css"],
        check_ids=["wallet-balance-card-cash-amount",
                   "wallet-balance-card-rewards-amount"],
    )
    if r:
        for kw, desc in [
            ("wallet-loading-layer", "Loading state layer present"),
            ("wallet-content-layer", "Content state layer present"),
            ("wallet-service.js", "wallet-service.js referenced"),
            ("wallet.js", "wallet.js referenced"),
            ("USD", "USD balance shown"),
        ]:
            if kw in r.text:
                results.ok(desc)
            else:
                results.warn(f"{desc} \u2013 NOT FOUND")

        r2 = session.get(f"{BASE_URL}/api/wallet/balance")
        if r2.status_code == 200:
            results.ok("/api/wallet/balance returns 200")
            try:
                d = r2.json()
                results.ok(f"  Cash: {d.get('cash_display')}, "
                           f"Rewards: {d.get('rewards_display')}")
            except Exception:
                results.warn("  /api/wallet/balance not valid JSON")
        else:
            results.fail("/api/wallet/balance", f"status={r2.status_code}")

        r3 = session.get(f"{BASE_URL}/api/wallet/transactions")
        if r3.status_code == 200:
            results.ok("/api/wallet/transactions returns 200")
            try:
                d = r3.json()
                results.ok(f"  {d.get('count', 0)} transactions returned")
            except Exception:
                results.warn("  /api/wallet/transactions not valid JSON")
        else:
            results.fail("/api/wallet/transactions", f"status={r3.status_code}")


# ─── Portfolio Tests ─────────────────────────────────────────────

def test_portfolio_api(session, results):
    """Test the /api/portfolio endpoint."""
    results.section("API – /api/portfolio (Portfolio System)")

    # Unauthenticated request
    r = requests.get(f"{BASE_URL}/api/portfolio", timeout=REQUEST_TIMEOUT)
    if r.status_code == 401:
        results.ok("GET /api/portfolio returns 401 when unauthenticated")
    else:
        results.fail("GET /api/portfolio unauth", f"expected 401, got {r.status_code}")

    # Authenticated request
    r = session.get(f"{BASE_URL}/api/portfolio", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok("GET /api/portfolio returns 200")
        try:
            data = r.json()
            # Top-level fields
            for field in ["investments", "total_value_cents", "total_purchase_cents",
                          "total_rental_cents", "investment_count"]:
                if field in data:
                    results.ok(f"  /api/portfolio contains '{field}': {data.get(field)}")
                else:
                    results.fail(f"  /api/portfolio MISSING field '{field}'")

            # Annual limit
            if "annual_limit" in data and data["annual_limit"]:
                al = data["annual_limit"]
                results.ok(f"  Annual limit: ${al.get('annual_limit_cents', 0)/100:,.2f}")
                if al.get("available_cents", -1) >= 0:
                    results.ok(f"  Available: ${al['available_cents']/100:,.2f}")
                else:
                    results.warn("  annual_limit.available_cents missing or negative")
            else:
                results.warn("  No annual_limit data in portfolio response")

            # Investments array
            investments = data.get("investments", [])
            if isinstance(investments, list):
                results.ok(f"  {len(investments)} investment(s) returned")
                for inv in investments[:2]:  # Check first 2
                    for field in ["id", "asset_title", "tokens_owned",
                                  "current_value_cents", "status"]:
                        if field in inv:
                            results.ok(f"    Investment.{field}: {inv[field]}")
                        else:
                            results.warn(f"    Investment missing '{field}'")
            else:
                results.fail("  'investments' is not a list")

        except Exception as e:
            results.fail("/api/portfolio JSON parse error", str(e))
    elif r.status_code == 404:
        results.fail("GET /api/portfolio", "404 – endpoint NOT IMPLEMENTED")
    else:
        results.fail("GET /api/portfolio", f"status={r.status_code}")

    # DB verification
    results.section("DATABASE – Portfolio Integrity")
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()

        # investments table
        cur.execute("""
            SELECT COUNT(*) FROM investments i
            JOIN users u ON u.id = i.user_id
            WHERE u.email = %s
        """, (TEST_EMAIL,))
        count = cur.fetchone()[0]
        if count > 0:
            results.ok(f"investments: {count} row(s) for test user")
        else:
            results.warn("investments: 0 rows for test user (portfolio will be empty)")

        # investment_limits table
        cur.execute("""
            SELECT annual_limit_cents, invested_12m_cents
            FROM investment_limits
            WHERE user_id = (SELECT id FROM users WHERE email = %s)
        """, (TEST_EMAIL,))
        lim = cur.fetchone()
        if lim:
            results.ok(f"investment_limits: limit={lim[0]/100:.2f}, invested={lim[1]/100:.2f}")
        else:
            results.warn("investment_limits: no row for test user")

        cur.close()
        conn.close()
    except Exception as e:
        results.fail("Portfolio DB check", str(e))

def test_portfolio(session, results):
    results.section("PAGE: /portfolio")
    test_page(session, results, "/portfolio", check_css=["portfolio.css"])
    test_portfolio_api(session, results)


# ─── Rewards Tests ───────────────────────────────────────────────

def test_rewards(session, results):
    results.section("PAGE: /rewards")
    test_page(session, results, "/rewards", check_css=["rewards.css"])

    r = session.get(f"{BASE_URL}/api/rewards")
    if r.status_code == 200:
        results.ok("/api/rewards returns 200")
        try:
            d = r.json()
            for field in ["total_balance", "cashback", "referrals", "promotions"]:
                if field in d:
                    results.ok(f"  Field '{field}': {d[field]}")
                else:
                    results.fail(f"  Field '{field}' MISSING from /api/rewards")
        except Exception:
            results.warn("  /api/rewards not valid JSON")
    elif r.status_code == 404:
        results.warn("/api/rewards returns 404")
    else:
        results.fail("/api/rewards", f"status={r.status_code}")


# ─── Cart Tests ──────────────────────────────────────────────────

def test_cart(session, results):
    results.section("PAGE: /cart")
    test_page(session, results, "/cart", check_css=["cart.css"])

    r = session.get(f"{BASE_URL}/api/cart")
    if r.status_code == 200:
        results.ok(f"/api/cart \u2013 {r.json().get('count', 0)} items")
    else:
        results.fail("/api/cart", f"status={r.status_code}")


# ─── Static Resources ────────────────────────────────────────────

def test_static_resources(session, results):
    results.section("STATIC RESOURCES \u2013 CSS & JS")

    css_files = [
        "/static/css/main.css",
        "/static/css/sidebar-navigation.css",
        "/static/css/marketplace.css",
        "/static/css/wallet.css",
        "/static/css/portfolio.css",
        "/static/css/cart.css",
        "/static/css/rewards.css",
        "/static/css/settings.css",
    ]
    js_files = [
        "/static/js/user-data.js",
        "/static/js/cart.js",
        "/static/js/portfolio-data.js",
        "/static/js/wallet-service.js",
        "/static/js/wallet.js",
    ]

    for path in css_files:
        try:
            r = session.get(f"{BASE_URL}{path}", timeout=5)
            if r.status_code == 200:
                results.ok(f"CSS: {path}")
                if "box-sizing: border-box" not in r.text and "box-sizing: border-box" not in r.text.lower() and path == "/static/css/admin.css":
                     results.warn(f"CSS {path} missing global box-sizing reset")
            else:
                results.warn(f"CSS NOT FOUND: {path}")
        except Exception:
            results.warn(f"CSS ERROR: {path}")

    for path in js_files:
        try:
            r = session.get(f"{BASE_URL}{path}", timeout=5)
            if r.status_code == 200:
                results.ok(f"JS: {path}")
                # Analyze for fetch calls without CSRF protection
                js_content = r.text
                if 'fetch(' in js_content and ('method: "POST"' in js_content or "method: 'POST'" in js_content):
                    if "x-csrf-token" not in js_content.lower() and "x-csrf-token" not in js_content:
                        results.warn(f"JS {path} appears to make POST requests via fetch without x-csrf-token header")
            else:
                results.warn(f"JS NOT FOUND: {path}")
        except Exception:
            results.warn(f"JS ERROR: {path}")


# ─── Navigation Consistency ──────────────────────────────────────

def test_navigation_consistency(session, results):
    results.section("NAVIGATION \u2013 Sidebar & Links Consistency")
    pages = ["/marketplace", "/wallet", "/portfolio", "/rewards",
             "/cart", "/settings", "/kyc"]
    settings_count = 0
    for pg in pages:
        try:
            r = session.get(f"{BASE_URL}{pg}", timeout=REQUEST_TIMEOUT)
        except Exception:
            continue
        if r.status_code != 200:
            continue
        html = analyze(r.text)
        if html.has_sidebar:
            results.ok(f"Sidebar present on {pg}")
        else:
            results.warn(f"[PAGE: {pg}]   No sidebar detected")
        if '"/settings"' in r.text or "'/settings'" in r.text:
            settings_count += 1

    msg = f"'/settings' link found on {settings_count}/{len(pages)} pages"
    if settings_count >= len(pages) - 1:
        results.ok(msg)
    else:
        results.warn(f"[NAVIGATION \u2013 Sidebar & Links Consistency]   {msg}")


# ─── API Endpoints ───────────────────────────────────────────────

def test_api_endpoints(session, results):
    results.section("API ENDPOINTS")

    r = session.get(f"{BASE_URL}/api/me")
    if r.status_code == 200:
        d = r.json()
        results.ok(f"/api/me \u2013 name: {d.get('name')}, role: {d.get('role')}")
    else:
        results.fail("/api/me", f"status={r.status_code}")

    r = session.get(f"{BASE_URL}/api/cart")
    if r.status_code == 200:
        results.ok(f"/api/cart \u2013 {r.json().get('count', 0)} items")
    else:
        results.fail("/api/cart", f"status={r.status_code}")

    for ep, desc in [
        ("/api/assets", "Assets listing API"),
        ("/api/notifications", "Notifications API"),
        ("/api/settings", "Settings API"),
        ("/api/kyc/status", "KYC status API"),
        ("/api/rewards", "Rewards data API"),
    ]:
        r = session.get(f"{BASE_URL}{ep}")
        if r.status_code == 200:
            results.ok(f"{ep} \u2013 {desc} exists")
        elif r.status_code == 404:
            results.warn(f"{ep} \u2013 {desc} NOT FOUND (404)")
        else:
            results.info(f"{ep} \u2013 status {r.status_code}")


# ─── Missing Features ────────────────────────────────────────────

def test_missing_features(results: TestResults):
    results.section("MISSING FEATURES \u2013 Gap Analysis")
    try:
        r = requests.get(f"{BASE_URL}/checkout", allow_redirects=False, timeout=5)
        if r.status_code == 404:
            results.warn("/checkout route NOT defined")
        else:
            results.ok(f"/checkout route IS defined (status {r.status_code})")
    except Exception:
        results.warn("/checkout route NOT accessible")


# ─── Dashboard UI Checks ─────────────────────────────────────────

def test_dashboard_ui(session, results):
    results.section("DASHBOARD UI \u2013 Common Elements")

    pages_with_sidebar = [
        "/marketplace", "/wallet", "/portfolio", "/rewards",
        "/cart", "/settings", "/kyc",
    ]
    for pg in pages_with_sidebar:
        try:
            r = session.get(f"{BASE_URL}{pg}", timeout=REQUEST_TIMEOUT)
        except Exception:
            continue
        if r.status_code != 200:
            continue
        html = analyze(r.text)
        if html.has_sidebar:
            results.ok(f"Sidebar present")
        else:
            results.warn(f"No sidebar detected")
        if html.has_user_data:
            results.ok(f"user-data.js present")
        else:
            results.warn(f"user-data.js not present")
        css = "/static/css/sidebar-navigation.css"
        if any("sidebar-navigation" in s for s in html.stylesheets) or "sidebar-navigation" in r.text:
            results.ok(f"CSS: {css}")
        else:
            results.warn(f"CSS missing: {css}")
        break  # Only check once as template is shared


# ─── Main ─────────────────────────────────────────────────────────

def main():
    print("=" * 70)
    print("  POOOL Platform \u2013 Comprehensive Test Suite")
    print(f"  Target: {BASE_URL}")
    print("=" * 70)

    results = TestResults()

    # Verify server is running
    try:
        requests.get(f"{BASE_URL}/auth/login", timeout=5)
    except requests.exceptions.ConnectionError:
        print(f"\n  Server not running at {BASE_URL}")
        print("  Start with: cargo run --bin poool-backend")
        sys.exit(1)

    session = get_session()

    # Database
    test_database(results)

    # Auth
    test_authentication(results)

    # Marketplace
    results.section("PAGE: /marketplace")
    html, r = test_page(session, results, "/marketplace",
                        check_css=["marketplace.css"])
    if r:
        if "property-card" in r.text:
            results.ok("Property cards rendered")
        else:
            results.warn("[PAGE: /marketplace]   No property cards rendered \u2013 assets table may be empty")

    # Commodities
    results.section("PAGE: /commodities-marketplace")
    html, r = test_page(session, results, "/commodities-marketplace",
                        check_css=["marketplace.css"])
    if r:
        if "commodit" in r.text.lower():
            results.ok("Commodity-specific content found")
        else:
            results.warn("[PAGE: /commodities-marketplace]   No commodity-specific content found")

    # Transactions
    results.section("PAGE: /transactions")
    html_tx, r_tx = test_page(session, results, "/transactions",
                              check_text=["Transactions", "All transactions", "wallet-body"])
    if r_tx:
        if "marketplace-body" in r_tx.text:
            results.fail("Found incorrect marketplace-body in transactions")

    # Wallet
    test_wallet(session, results)

    # Portfolio
    test_portfolio(session, results)

    # Rewards
    test_rewards(session, results)

    # Cart
    test_cart(session, results)
    
    # E2E test
    test_cart_and_payment_e2e(session, results)

    # Support
    results.section("PAGE: /support")
    test_page(session, results, "/support")
    test_support_e2e(session, results)

    # Settings
    results.section("PAGE: /settings")
    test_page(session, results, "/settings", check_css=["settings.css"])

    # Account Deletion
    results.section("PAGE: /account-deletion")
    test_page(session, results, "/account-deletion", check_sidebar=False, check_css=["error-pages.css"])

    # KYC
    results.section("PAGE: /kyc")
    test_page(session, results, "/kyc")

    # Privacy Policy
    results.section("PAGE: /privacy-policy")
    test_page(session, results, "/privacy-policy")

    # Checkout
    results.section("PAGE: /checkout")
    r = session.get(f"{BASE_URL}/checkout", allow_redirects=False,
                    timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok("GET /checkout returns 200")
    elif r.status_code in (302, 303):
        results.ok(f"GET /checkout redirects ({r.status_code})")
    elif r.status_code == 404:
        results.fail("GET /checkout returns 404")
    else:
        results.warn(f"GET /checkout: {r.status_code}")

    # Static resources
    test_static_resources(session, results)

    # Dashboard UI
    test_dashboard_ui(session, results)

    # Navigation
    test_navigation_consistency(session, results)

    # API endpoints
    test_api_endpoints(session, results)

    # Missing features
    test_missing_features(results)

    # Developer wizard
    test_developer_wizard_e2e(session, results)

    # ── Summary ───────────────────────────────────────────────────
    total = results.passed + results.failed + results.warnings
    print(f"\n{'=' * 70}")
    print("  \U0001f4ca TEST SUMMARY")
    print(f"{'=' * 70}")
    print(f"\n  Total tests:  {total}")
    print(f"  \u2705 Passed:     {results.passed}")
    print(f"  \u274c Failed:     {results.failed}")
    print(f"  \u26a0\ufe0f  Warnings:   {results.warnings}")

    if results.warnings_list:
        print(f"\n  {chr(9472) * 60}")
        print(f"  \u26a0\ufe0f  WARNINGS ({results.warnings}):")
        print(f"  {chr(9472) * 60}")
        for section, msg in results.warnings_list:
            print(f"    [{section}]   {msg}")

    print()
    if results.failed == 0:
        print(f"  {chr(9472) * 60}")
        print(f"  \U0001f389 ALL TESTS PASSED!")
    else:
        print(f"  {chr(9472) * 60}")
        print(f"  \u26d4 {results.failed} test(s) FAILED:")
        for section, msg in results.errors:
            print(f"    [{section}] {msg}")

    print("=" * 70)
    sys.exit(0 if results.failed == 0 else 1)



def test_cart_and_payment_e2e(session, results):
    """End-to-end test: add to cart, view cart, proceed to checkout, wallet checkout."""
    results.section("E2E: Cart → Checkout → Payment Flow")

    import psycopg2
    conn = psycopg2.connect("dbname=poool user=martin host=127.0.0.1")
    cur = conn.cursor()

    # 1. Get test user and a valid asset
    cur.execute("SELECT id FROM users WHERE email = %s", (TEST_EMAIL,))
    user_row = cur.fetchone()
    if not user_row:
        results.fail("Test user not found in database")
        return
    user_id = str(user_row[0])

    cur.execute("SELECT id, token_price_cents FROM assets WHERE published = true AND tokens_available > 0 LIMIT 1")
    asset_row = cur.fetchone()
    if not asset_row:
        results.fail("No published assets with available tokens — cannot test cart flow")
        return
    asset_id, token_price_cents = str(asset_row[0]), asset_row[1]

    # 2. Ensure test user has sufficient wallet balance (>= 1 token price)
    cur.execute("""
        INSERT INTO wallets (user_id, wallet_type, currency, balance_cents)
        VALUES (%s, 'cash', 'USD', %s)
        ON CONFLICT (user_id, wallet_type, currency)
        DO UPDATE SET balance_cents = GREATEST(wallets.balance_cents, %s)
    """, (user_id, token_price_cents * 5, token_price_cents * 5))
    conn.commit()
    results.ok(f"Test wallet funded with ${token_price_cents * 5 / 100:.2f} USD")

    # 3. Clear existing cart items for test user to ensure clean state
    cur.execute("DELETE FROM cart_items WHERE user_id = %s", (user_id,))
    conn.commit()

    # 4. Add item to cart via POST /cart/add
    csrf_token = session.cookies.get("csrf_token", "")
    headers = {"X-CSRF-Token": csrf_token}
    
    add_resp = session.post(
        f"{BASE_URL}/cart/add",
        data={
            "property_id": asset_id,
            "investment_amount": str(token_price_cents // 100)  # 1 token worth
        },
        headers=headers,
        allow_redirects=False,
    )
    if add_resp.status_code in (302, 303):
        results.ok(f"POST /cart/add → redirect {add_resp.headers.get('Location', '?')}")
    elif add_resp.status_code == 200:
        results.ok("POST /cart/add → 200 OK")
    else:
        results.fail("POST /cart/add", f"status={add_resp.status_code}")
        cur.close(); conn.close(); return

    # 5. Verify cart has the item via GET /api/cart
    cart_resp = session.get(f"{BASE_URL}/api/cart")
    if cart_resp.status_code == 200:
        cart_data = cart_resp.json()
        item_count = cart_data.get("count", 0)
        total_cents = cart_data.get("total_cents", 0)
        if item_count > 0:
            results.ok(f"GET /api/cart: {item_count} item(s), total ${total_cents/100:.2f}")
        else:
            results.fail("GET /api/cart: cart is empty after add_to_cart")
            cur.close(); conn.close(); return
    else:
        results.fail("GET /api/cart", f"status={cart_resp.status_code}")
        cur.close(); conn.close(); return

    # 6. Test cart page renders correctly
    cart_page_resp = session.get(f"{BASE_URL}/cart")
    if cart_page_resp.status_code == 200:
        results.ok("GET /cart returns 200 with items")
        if "cart-item-card" in cart_page_resp.text or "cart-page-content" in cart_page_resp.text:
            results.ok("  Cart page shows item cards")
        else:
            results.warn("  Cart page HTML may not contain expected cart item markup")
    else:
        results.fail("GET /cart", f"status={cart_page_resp.status_code}")

    # 7. Test quantity update via POST /cart/update
    cart_item_id = cart_data["items"][0]["id"]
    update_resp = session.post(
        f"{BASE_URL}/cart/update",
        data={"cart_item_id": cart_item_id, "tokens_quantity": "2"},
        headers={"Content-Type": "application/x-www-form-urlencoded", "X-CSRF-Token": csrf_token},
    )
    if update_resp.status_code == 200:
        update_data = update_resp.json()
        if update_data.get("success"):
            results.ok("POST /cart/update → quantity updated to 2")
        else:
            results.warn(f"POST /cart/update → unexpected response: {update_data}")
    else:
        results.fail("POST /cart/update", f"status={update_resp.status_code}")

    # 8. Verify the checkout page is accessible (cart is not empty)
    checkout_resp = session.get(f"{BASE_URL}/checkout", allow_redirects=False)
    if checkout_resp.status_code == 200:
        results.ok("GET /checkout returns 200 (cart is populated)")
        if "checkout" in checkout_resp.text.lower() or "payment" in checkout_resp.text.lower():
            results.ok("  Checkout page content detected")
    elif checkout_resp.status_code in (302, 303):
        loc = checkout_resp.headers.get("Location", "?")
        results.fail(f"GET /checkout redirects to {loc} — cart may be empty or route missing")
        cur.close(); conn.close(); return
    else:
        results.fail("GET /checkout", f"status={checkout_resp.status_code}")
        cur.close(); conn.close(); return

    # 9. Wallet balance API returns correct data
    wallets_resp = session.get(f"{BASE_URL}/api/wallets")
    if wallets_resp.status_code == 200:
        wallets_data = wallets_resp.json()
        wallets = wallets_data.get("wallets", [])
        cash_wallet = next((w for w in wallets if w.get("wallet_type") == "cash" and w.get("currency") == "USD"), None)
        if cash_wallet:
            results.ok(f"GET /api/wallets: USD cash balance = ${cash_wallet['balance_cents']/100:.2f}")
        else:
            results.warn("GET /api/wallets: no USD cash wallet found (checkout wallet payment may fail)")
    else:
        results.fail("GET /api/wallets", f"status={wallets_resp.status_code}")

    # 10. Execute the wallet checkout via POST /checkout
    import io
    checkout_post_resp = session.post(
        f"{BASE_URL}/checkout",
        data={
            "payment_method": "wallet",
            "payment_currency": "USD",
        },
        headers={"X-CSRF-Token": csrf_token},
        allow_redirects=False,
    )
    if checkout_post_resp.status_code in (200, 302, 303):
        hx_redirect = checkout_post_resp.headers.get("HX-Redirect", "")
        location = checkout_post_resp.headers.get("Location", "")
        if "payment-success" in hx_redirect or "payment-success" in location:
            results.ok("POST /checkout → redirected to /payment-success ✅")
        else:
            results.ok(f"POST /checkout → status={checkout_post_resp.status_code}, redirect={hx_redirect or location}")
    elif checkout_post_resp.status_code == 400:
        error_html = checkout_post_resp.text
        results.fail("POST /checkout returned 400", error_html[:200])
        cur.close(); conn.close(); return
    else:
        results.fail("POST /checkout", f"status={checkout_post_resp.status_code}")
        cur.close(); conn.close(); return

    # 11. Verify order was created in database
    cur.execute("""
        SELECT order_number, status, total_cents, payment_method
        FROM orders WHERE user_id = %s ORDER BY created_at DESC LIMIT 1
    """, (user_id,))
    order = cur.fetchone()
    if order:
        order_num, status, total, method = order
        results.ok(f"Order created: {order_num}, status={status}, total=${total/100:.2f}, method={method}")
        if status == "completed":
            results.ok("  Order status is 'completed' ✅")
        else:
            results.warn(f"  Order status is '{status}' (expected 'completed' for wallet payment)")
    else:
        results.fail("No order found in database after checkout")

    # 12. Verify investment was created
    cur.execute("""
        SELECT tokens_owned, status FROM investments WHERE user_id = %s
        AND asset_id = %s ORDER BY updated_at DESC LIMIT 1
    """, (user_id, asset_id))
    investment = cur.fetchone()
    if investment:
        tokens, inv_status = investment
        results.ok(f"Investment created: {tokens} tokens, status={inv_status}")
    else:
        results.warn("No investment record found after checkout")

    # 13. Verify invoice was generated
    cur.execute("""
        SELECT invoice_number, status FROM invoices WHERE user_id = %s
        ORDER BY issued_at DESC LIMIT 1
    """, (user_id,))
    invoice = cur.fetchone()
    if invoice:
        inv_num, inv_status = invoice
        results.ok(f"Invoice generated: {inv_num}, status={inv_status}")
    else:
        results.warn("No invoice found after checkout")

    # 14. Verify wallet transaction was logged
    cur.execute("""
        SELECT type, status, amount_cents FROM wallet_transactions wt
        JOIN wallets w ON wt.wallet_id = w.id
        WHERE w.user_id = %s AND w.currency = 'USD'
        ORDER BY wt.created_at DESC LIMIT 1
    """, (user_id,))
    tx = cur.fetchone()
    if tx:
        tx_type, tx_status, tx_amount = tx
        results.ok(f"Wallet transaction logged: type={tx_type}, status={tx_status}, amount={tx_amount/100:.2f}")
    else:
        results.warn("No wallet transaction found — wallet deduction may not have been logged")

    # 15. Verify cart was cleared after successful checkout
    cur.execute("SELECT COUNT(*) FROM cart_items WHERE user_id = %s", (user_id,))
    remaining = cur.fetchone()[0]
    if remaining == 0:
        results.ok("Cart cleared after successful checkout ✅")
    else:
        results.fail(f"Cart NOT cleared after checkout — {remaining} items remain")

    # 16. Test GET /api/orders/latest
    latest_order_resp = session.get(f"{BASE_URL}/api/orders/latest")
    if latest_order_resp.status_code == 200:
        order_data = latest_order_resp.json()
        results.ok(f"GET /api/orders/latest: order #{order_data.get('order_number')}")
        for field in ["order_number", "total_cents", "payment_currency", "status", "items"]:
            if field in order_data:
                results.ok(f"  Field '{field}' present")
            else:
                results.warn(f"  Field '{field}' MISSING from /api/orders/latest")
    else:
        results.fail("GET /api/orders/latest", f"status={latest_order_resp.status_code}")

    # 17. Test payment-success page 
    success_resp = session.get(f"{BASE_URL}/payment-success")
    if success_resp.status_code == 200:
        results.ok("GET /payment-success returns 200")
    else:
        results.warn(f"GET /payment-success returned {success_resp.status_code}")

    cur.close()
    conn.close()
def test_support_e2e(session, results):
    results.section("E2E: Support Tickets")
    import psycopg2
    conn = psycopg2.connect("dbname=poool user=martin host=127.0.0.1")
    cur = conn.cursor()

    cur.execute("SELECT id FROM users WHERE email = %s", (TEST_EMAIL,))
    user_row = cur.fetchone()
    if not user_row:
        results.fail("Test user not found")
        return
    user_id = str(user_row[0])

    # Create ticket
    files = {
        'subject': (None, 'E2E Test Ticket'),
        'message': (None, 'This is an end-to-end test ticket.'),
        'priority': (None, 'normal'),
        'category': (None, 'general'),
        'context': (None, '{}')
    }
    csrf_token = session.cookies.get("csrf_token", "")
    resp = session.post(f"{BASE_URL}/api/support/tickets", files=files, headers={"X-CSRF-Token": csrf_token})
    if resp.status_code == 200:
        results.ok(f"POST /api/support/tickets returns 200: {resp.json().get('message')}")
    else:
        results.fail(f"POST /api/support/tickets returned {resp.status_code}")
        return

    # Fetch it
    resp = session.get(f"{BASE_URL}/api/support/tickets")
    if resp.status_code == 200:
        tickets = resp.json().get("tickets", [])
        ticket = next((t for t in tickets if t["subject"] == "E2E Test Ticket"), None)
        if ticket:
            results.ok("GET /api/support/tickets found the created ticket")
            ticket_id = ticket["id"]
        else:
            results.fail("Ticket not found in list")
            return
    else:
        results.fail(f"GET /api/support/tickets returned {resp.status_code}")
        return

    # Reply to ticket
    resp = session.post(f"{BASE_URL}/api/support/tickets/{ticket_id}/reply", json={"message": "This is a reply"}, headers={"X-CSRF-Token": csrf_token})
    if resp.status_code == 200:
        results.ok("POST /api/support/tickets/reply returns 200")
    else:
        results.fail(f"POST /api/support/tickets/reply returned {resp.status_code}")

    # Mark resolved via DB (simulating admin)
    cur.execute("UPDATE support_tickets SET status = 'resolved' WHERE id = %s", (ticket_id,))
    conn.commit()

    # Reopen ticket
    resp = session.put(f"{BASE_URL}/api/support/tickets/{ticket_id}/reopen", headers={"X-CSRF-Token": csrf_token})
    if resp.status_code == 200:
        results.ok("PUT /api/support/tickets/reopen returns 200")
    else:
        results.fail(f"PUT /api/support/tickets/reopen returned {resp.status_code}")

    # Clean up
    cur.execute("DELETE FROM support_ticket_replies WHERE ticket_id = %s", (ticket_id,))
    cur.execute("DELETE FROM support_tickets WHERE id = %s", (ticket_id,))
    conn.commit()
    cur.close()
    conn.close()


# ─── Developer Wizard E2E ─────────────────────────────────────────

def test_developer_wizard_e2e(session, results):
    """
    End-to-end test of the developer asset submission wizard.

    Steps covered:
      1. Pages load (add-asset, application-form, document-upload-step3, property-content)
      2. POST /api/developer/draft — create a draft
      3. GET /api/developer/draft/:id — prefill data (used by PooolDropdown sync)
      4. PUT /api/developer/draft/:id — update draft (step 2 Save / re-edit)
      5. GET /api/developer/submissions — submissions list page
      6. GET /api/developer/drafts — listing API
      7. PUT /api/developer/draft/:id — step 4 content update
      8. POST /api/developer/draft/:id/submit — final submission
      9. DB verification — asset row + developer_project row
      10. Cleanup
    """
    results.section("E2E: Developer Asset Submission Wizard")

    import uuid as _uuid
    import psycopg2

    conn = psycopg2.connect("dbname=poool user=martin host=127.0.0.1")
    cur = conn.cursor()

    # ── Ensure test user has the 'developer' role ─────────────────────────
    cur.execute("SELECT id FROM users WHERE email = %s", (TEST_EMAIL,))
    user_row = cur.fetchone()
    if not user_row:
        results.fail("Developer wizard E2E: test user not found")
        conn.close()
        return
    user_id = str(user_row[0])

    cur.execute("SELECT id FROM roles WHERE name = 'developer'")
    role_row = cur.fetchone()
    if not role_row:
        results.warn("Developer wizard E2E: no 'developer' role in DB — skipping wizard tests")
        conn.close()
        return
    role_id = str(role_row[0])

    # Grant developer role if missing (idempotent)
    cur.execute(
        "INSERT INTO user_roles (user_id, role_id) VALUES (%s, %s) ON CONFLICT DO NOTHING",
        (user_id, role_id),
    )
    conn.commit()
    results.ok("Developer role granted (or already present) for test user")

    csrf = session.cookies.get("csrf_token", "")
    headers_json = {"Content-Type": "application/json", "X-CSRF-Token": csrf}

    # ── 1. Wizard pages load ─────────────────────────────────────────────
    for path, label in [
        ("/developer/dashboard", "Developer Dashboard"),
        ("/developer/add-asset", "Add Asset (type selector)"),
        ("/developer/application-form", "Application Form (step 2)"),
        ("/developer/document-upload-step3", "Document Upload (step 3)"),
        ("/developer/property-content", "Property Content (step 4)"),
        ("/developer/submissions", "Submissions list"),
    ]:
        r = session.get(f"{BASE_URL}{path}", allow_redirects=True, timeout=REQUEST_TIMEOUT)
        if r.status_code == 200:
            results.ok(f"GET {path} — {label} returns 200")
        elif r.status_code in (302, 303):
            results.fail(f"GET {path} — {label} redirects (auth/role issue?)",
                         r.headers.get("Location", ""))
        else:
            results.fail(f"GET {path} — {label}", f"status={r.status_code}")

    # ── 2. Step 2: Create a new draft ────────────────────────────────────
    draft_payload = {
        "title": "E2E Test Villa",
        "asset_type": "real_estate",
        "property_type": "villa",
        "area": "canggu",
        "address": "Jl. Test No. 99",
        "city": "Badung",
        "country": "Indonesia",
        "lease_type": "leasehold",
        "lease_term_years": 25,
        "land_size_sqm": 500.0,
        "building_size_sqm": 250.0,
        "bedrooms": 3,
        "bathrooms": 3,
        "construction_status": "ready",
        "year_built": 2022,
        "total_value_cents": 50000000,   # $500,000
        "token_price_cents": 50000,       # $500
        "tokens_total": 1000,
    }
    r = session.post(
        f"{BASE_URL}/api/developer/draft",
        json=draft_payload,
        headers=headers_json,
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code == 200:
        data = r.json()
        draft_id = data.get("asset_id")
        if draft_id:
            results.ok(f"POST /api/developer/draft — draft created: {draft_id}")
        else:
            results.fail("POST /api/developer/draft — response missing 'asset_id'", str(data))
            conn.close(); return
    else:
        results.fail("POST /api/developer/draft", f"status={r.status_code} body={r.text[:200]}")
        conn.close(); return

    # ── 3. GET /api/developer/draft/:id — prefill check ──────────────────
    # This is the request that PooolDropdown prefill logic calls after navigation
    r = session.get(f"{BASE_URL}/api/developer/draft/{draft_id}", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        prefill = r.json()
        results.ok(f"GET /api/developer/draft/{draft_id} — prefill data returned")
        # Check every field the PooolDropdown sync relies on
        for field, expected in [
            ("title",               "E2E Test Villa"),
            ("property_type",       "villa"),
            ("area",                "canggu"),
            ("location_address",    "Jl. Test No. 99"),
            ("city",                "Badung"),
            ("country",             "Indonesia"),
            ("lease_type",          "leasehold"),
            ("lease_term_years",    25),
            ("bedrooms",            3),
            ("bathrooms",           3),
            ("construction_status", "ready"),
            ("year_built",          2022),
        ]:
            actual = prefill.get(field)
            if actual == expected:
                results.ok(f"  Prefill '{field}' = {expected!r} ✓")
            else:
                results.fail(f"  Prefill '{field}'", f"expected {expected!r}, got {actual!r}")

        # Financials are stored as cents — verify they round-tripped correctly
        if prefill.get("total_value_cents") == 50000000:
            results.ok("  Prefill 'total_value_cents' = 50000000 ✓")
        else:
            results.fail("  Prefill 'total_value_cents'",
                         f"expected 50000000, got {prefill.get('total_value_cents')}")
    else:
        results.fail(f"GET /api/developer/draft/{draft_id}", f"status={r.status_code}")
        conn.close(); return

    # ── 4. PUT /api/developer/draft/:id — step 2 Save & re-edit ─────────
    # Simulates the user hitting 'Previous' and changing some fields,
    # then continuing — which now sends a PUT instead of a POST.
    update_step2 = {
        "title": "E2E Test Villa (Updated)",
        "property_type": "villa",
        "area": "uluwatu",
        "lease_type": "freehold",
        "construction_status": "construction",
        "bedrooms": 4,
        "year_built": 2024,
        "submission_step": 2,
    }
    r = session.put(
        f"{BASE_URL}/api/developer/draft/{draft_id}",
        json=update_step2,
        headers=headers_json,
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code == 200:
        results.ok(f"PUT /api/developer/draft/{draft_id} (step 2 update) — OK")
    else:
        results.fail(f"PUT /api/developer/draft/{draft_id} step 2",
                     f"status={r.status_code} body={r.text[:200]}")

    # Verify the update was persisted
    r = session.get(f"{BASE_URL}/api/developer/draft/{draft_id}")
    if r.status_code == 200:
        updated = r.json()
        for field, expected in [
            ("title",               "E2E Test Villa (Updated)"),
            ("area",                "uluwatu"),
            ("construction_status", "construction"),
            ("bedrooms",            4),
            ("year_built",          2024),
        ]:
            actual = updated.get(field)
            if actual == expected:
                results.ok(f"  PUT persisted '{field}' = {expected!r} ✓")
            else:
                results.fail(f"  PUT did not persist '{field}'",
                             f"expected {expected!r}, got {actual!r}")
    else:
        results.warn("Could not re-fetch draft after step 2 PUT to verify")

    # ── 5. GET /developer/submissions (page, not API) ─────────────────────
    r = session.get(f"{BASE_URL}/developer/submissions", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        results.ok("GET /developer/submissions page renders after draft creation")
    else:
        results.warn(f"GET /developer/submissions returned {r.status_code}")

    # ── 6. GET /api/developer/drafts — listing ────────────────────────────
    r = session.get(f"{BASE_URL}/api/developer/drafts", timeout=REQUEST_TIMEOUT)
    if r.status_code == 200:
        listing = r.json()
        drafts = listing if isinstance(listing, list) else listing.get("drafts", listing.get("submissions", []))
        match = any(str(d.get("id", "")) == str(draft_id) for d in drafts)
        if match:
            results.ok(f"GET /api/developer/drafts — draft {draft_id} visible in listing")
        else:
            results.warn(f"GET /api/developer/drafts — draft {draft_id} not found in listing (may be paginated)")
    elif r.status_code == 404:
        results.warn("GET /api/developer/drafts — 404, endpoint may not exist yet")
    else:
        results.warn(f"GET /api/developer/drafts — status={r.status_code}")

    # ── 7. PUT /api/developer/draft/:id — step 4 content (Save & Exit) ───
    step4_payload = {
        "short_description": "Beautiful E2E test villa in Uluwatu with ocean views.",
        "description": "This is the full property description for the E2E test asset.",
        "location_description": "Located in the famous Uluwatu area of Bali.",
        "google_maps_url": "https://maps.google.com/?q=uluwatu+bali",
        "video_url": "https://youtube.com/watch?v=test",
        "annual_yield_bps": 800,          # 8%
        "capital_appreciation_bps": 500,  # 5%
        "investor_share_bps": 7000,       # 70%
        "occupancy_rate_bps": 9000,       # 90%
        "submission_step": 4,
    }
    r = session.put(
        f"{BASE_URL}/api/developer/draft/{draft_id}",
        json=step4_payload,
        headers=headers_json,
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code == 200:
        results.ok(f"PUT /api/developer/draft/{draft_id} (step 4 content) — OK")
    else:
        results.fail(f"PUT /api/developer/draft/{draft_id} step 4",
                     f"status={r.status_code} body={r.text[:200]}")

    # Verify step 4 content persisted
    r = session.get(f"{BASE_URL}/api/developer/draft/{draft_id}")
    if r.status_code == 200:
        draft4 = r.json()
        for field, expected in [
            ("short_description", "Beautiful E2E test villa in Uluwatu with ocean views."),
            ("annual_yield_bps",  800),
            ("investor_share_bps", 7000),
        ]:
            actual = draft4.get(field)
            if actual == expected:
                results.ok(f"  Step 4 persisted '{field}' = {expected!r} ✓")
            else:
                results.fail(f"  Step 4 did NOT persist '{field}'",
                             f"expected {expected!r}, got {actual!r}")
    else:
        results.warn("Could not re-fetch draft after step 4 PUT to verify")

    # ── 8. POST /api/developer/draft/:id/submit — final submission ────────
    r = session.post(
        f"{BASE_URL}/api/developer/draft/{draft_id}/submit",
        headers={"X-CSRF-Token": csrf},
        timeout=REQUEST_TIMEOUT,
    )
    if r.status_code == 200:
        results.ok(f"POST /api/developer/draft/{draft_id}/submit — submitted for review ✅")
    else:
        results.fail(
            f"POST /api/developer/draft/{draft_id}/submit",
            f"status={r.status_code} body={r.text[:200]}",
        )

    # ── 9. DB verification ───────────────────────────────────────────────
    cur.execute(
        "SELECT title, submission_step FROM assets WHERE id = %s",
        (draft_id,),
    )
    asset_row = cur.fetchone()
    if asset_row:
        title_db, step_db = asset_row
        results.ok(f"DB: asset row found — title='{title_db}', submission_step={step_db}")
        if step_db == 5:
            results.ok("DB: submission_step = 5 (submitted) ✓")
        else:
            results.warn(f"DB: submission_step = {step_db} (expected 5 after submit)")
    else:
        results.fail(f"DB: asset row for {draft_id} NOT FOUND")

    cur.execute(
        "SELECT status FROM developer_projects WHERE asset_id = %s",
        (draft_id,),
    )
    proj_row = cur.fetchone()
    if proj_row:
        proj_status = proj_row[0]
        results.ok(f"DB: developer_projects row found — status='{proj_status}'")
        if proj_status in ("submitted", "in_review", "approved"):
            results.ok(f"DB: project status '{proj_status}' is post-submission ✓")
        else:
            results.warn(f"DB: project status='{proj_status}' (expected 'submitted')")
    else:
        results.warn("DB: no developer_projects row found for this draft")

    # ── 10. Cleanup — remove test draft ─────────────────────────────────
    try:
        cur.execute("DELETE FROM developer_projects WHERE asset_id = %s", (draft_id,))
        cur.execute("DELETE FROM asset_documents WHERE asset_id = %s", (draft_id,))
        cur.execute("DELETE FROM asset_images WHERE asset_id = %s", (draft_id,))
        cur.execute("DELETE FROM assets WHERE id = %s", (draft_id,))
        conn.commit()
        results.ok("Cleanup: test draft and related rows removed from DB")
    except Exception as e:
        results.warn(f"Cleanup failed (manual cleanup may be needed): {e}")

    cur.close()
    conn.close()


if __name__ == "__main__":
    main()
