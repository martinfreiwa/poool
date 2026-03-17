#!/usr/bin/env python3
"""
POOOL Admin Dashboard – Comprehensive Test Suite
================================================
Industry-standard testing for all admin dashboard pages, endpoints, and components.

Pages tested:
  /admin/                 → Dashboard Overview
  /admin/users.html      → User Management
  /admin/kyc.html        → KYC & AML Review
  /admin/support.html    → Support Tickets
  /admin/submissions.html→ Developer Submissions
  /admin/assets.html     → Live Asset Management
  /admin/orders.html     → Order History
  /admin/deposits.html   → Deposit Management
  /admin/treasury.html   → Treasury Overview
  /admin/rewards.html    → Rewards & Referrals
  /admin/notifications.html→ Platform Notifications
  /admin/audit-logs.html → System Audit Trail
  /admin/reports.html    → Reports & Analytics
  /admin/email-marketing.html → Email Engine
  /admin/system.html     → System Health
  /admin/settings.html   → Platform Settings

Run:  python3 tests/admin/test_admin_dashboard.py
"""

import json
import sys
import os
import time
from collections import defaultdict
from html.parser import HTMLParser
import psycopg2
import requests

# ═══════════════════════════════════════════════════════════════════
# Configuration
# ═══════════════════════════════════════════════════════════════════
BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DATABASE_URL", "dbname=poool user=martin host=localhost")
ADMIN_EMAIL = os.environ.get("ADMIN_EMAIL", "test@poool.app")
ADMIN_PASSWORD = os.environ.get("ADMIN_PASSWORD", "TestPass123!")
REQUEST_TIMEOUT = 10
LOGIN_TIMEOUT = 30

# ═══════════════════════════════════════════════════════════════════
# HTML Analyzer
# ═══════════════════════════════════════════════════════════════════
class AdminHTMLAnalyzer(HTMLParser):
    def __init__(self):
        super().__init__()
        self.title = ""
        self._in_title = False
        self.scripts = []
        self.stylesheets = []
        self.ids = []
        self.links = []
        self.has_sidebar = False
        self.has_admin_css = False
        self.has_htmx = False
        self.has_alpine = False
        self.active_nav_id = None
        self.meta_viewport = False
        self.has_breadcrumbs = False
        self.htmx_elements = []
        self.has_theme_engine = False
        self.has_permission_guard = False

    def handle_starttag(self, tag, attrs):
        attr_dict = dict(attrs)
        
        if tag == "title":
            self._in_title = True
        
        if "id" in attr_dict:
            self.ids.append(attr_dict["id"])
            
        if tag == "link" and attr_dict.get("rel") == "stylesheet":
            href = attr_dict.get("href", "")
            self.stylesheets.append(href)
            if "admin.css" in href:
                self.has_admin_css = True
                
        if tag == "script":
            src = attr_dict.get("src", "")
            if src:
                self.scripts.append(src)
                if "htmx" in src: self.has_htmx = True
                if "alpine" in src: self.has_alpine = True
                if "admin-theme" in src: self.has_theme_engine = True
                if "admin-permission-guard" in src: self.has_permission_guard = True
                
        # Sidebar detection (now more robust as it can be aside or nav)
        cls = attr_dict.get("class") or ""
        cls = cls.lower()
        if (tag == "aside" or tag == "nav") and ("admin-sidebar" in cls or "sidebar" in cls):
            self.has_sidebar = True
        
        id_attr = attr_dict.get("id") or ""
        if id_attr == "admin-sidebar-placeholder":
            self.has_sidebar = True

        if tag == "a" and "href" in attr_dict:
            href = attr_dict.get("href") or ""
            self.links.append(href)
            # Match strictly "active" or "admin-nav-item active"
            classes = (attr_dict.get("class") or "").split()
            if "active" in classes:
                self.active_nav_id = attr_dict.get("id")

        if tag == "meta" and attr_dict.get("name") == "viewport":
            self.meta_viewport = True

        if tag == "nav" and "breadcrumb" in (attr_dict.get("class") or "").lower():
            self.has_breadcrumbs = True


        # Track HTMX attributes for integrity checks
        if any(attr.startswith("hx-") for attr in attr_dict):
            self.htmx_elements.append({
                "tag": tag,
                "attrs": attr_dict
            })

    def handle_data(self, data):
        if self._in_title:
            self.title += data

    def handle_endtag(self, tag):
        if tag == "title":
            self._in_title = False

# ═══════════════════════════════════════════════════════════════════
# Test Framework
# ═══════════════════════════════════════════════════════════════════
class TestResults:
    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.warnings = 0
        self.errors = []
        self.current_section = ""

    def section(self, name):
        self.current_section = name
        print(f"\n{'='*80}")
        print(f"  SECTION: {name}")
        print(f"{'='*80}")

    def ok(self, msg):
        self.passed += 1
        print(f"  ✅ [PASS] {msg}")

    def fail(self, msg, detail=""):
        self.failed += 1
        self.errors.append((self.current_section, msg))
        print(f"  ❌ [FAIL] {msg}")
        if detail: print(f"     → {detail}")

    def warn(self, msg):
        self.warnings += 1
        print(f"  ⚠️  [WARN] {msg}")

    def info(self, msg):
        print(f"  ℹ️  [INFO] {msg}")

def get_session_by_role(role_name="admin"):
    """Get a session for a specific role ('admin', 'super_admin', 'investor')."""
    session = requests.Session()
    try:
        conn = psycopg2.connect(DB_DSN)
        cur = conn.cursor()
        query = """
            SELECT u.email, s.session_token FROM user_sessions s
            JOIN users u ON u.id = s.user_id
            JOIN user_roles ur ON ur.user_id = u.id
            JOIN roles r ON r.id = ur.role_id
            WHERE r.name = %s AND s.expires_at > NOW()
            {filter}
            ORDER BY s.created_at DESC LIMIT 1
        """
        filter_clause = ""
        if role_name == "investor":
            filter_clause = "AND NOT EXISTS (SELECT 1 FROM user_roles ur2 JOIN roles r2 ON ur2.role_id = r2.id WHERE ur2.user_id = u.id AND r2.name IN ('admin', 'super_admin'))"
        
        cur.execute(query.replace("{filter}", filter_clause), (role_name,))
        row = cur.fetchone()
        cur.close()
        conn.close()
        
        if row:
            session.cookies.set("poool_session", row[1])
            # Rapid verify
            r = session.get(f"{BASE_URL}/api/me", timeout=5)
            if r.status_code == 200:
                data = r.json()
                if data.get("role") == role_name:
                    return session
    except: pass
    
    # Fallback to login for admin only
    if role_name in ("admin", "super_admin"):
        try:
            r = session.post(f"{BASE_URL}/auth/login", 
                             data={"email": ADMIN_EMAIL, "password": ADMIN_PASSWORD},
                             timeout=LOGIN_TIMEOUT)
            if "poool_session" in session.cookies:
                return session
        except: pass
    return None

def get_admin_session():
    """Get an authenticated admin session, trying super_admin first then admin."""
    return get_session_by_role("super_admin") or get_session_by_role("admin")

def analyze_html(html_str):
    analyzer = AdminHTMLAnalyzer()
    analyzer.feed(html_str)
    return analyzer

# ═══════════════════════════════════════════════════════════════════
# Test Suites
# ═══════════════════════════════════════════════════════════════════

def test_admin_page(session, results, path, name, expected_id=None, expected_nav=None):
    results.section(f"PAGE: {name} ({path})")
    start_time = time.time()
    try:
        r = session.get(f"{BASE_URL}{path}", timeout=REQUEST_TIMEOUT)
        load_time = time.time() - start_time
        
        if r.status_code != 200:
            results.fail(f"HTTP Status: {r.status_code}", f"Expected 200 for {path}")
            return None
        
        results.ok(f"Page accessible (200 OK) in {load_time:.2f}s")
        if load_time > 1.0:
            results.warn(f"Page load too slow: {load_time:.2f}s")
            
        html = analyze_html(r.text)

        # --- Security Headers ---
        sec_headers = {
            "X-Content-Type-Options": "nosniff",
            "X-Frame-Options": "DENY",
            "X-XSS-Protection": "1; mode=block"
        }
        for header, expected in sec_headers.items():
            if header in r.headers:
                results.ok(f"Security Header '{header}' present")
            else:
                results.warn(f"Security Header '{header}' MISSING")
        
        if html.has_sidebar: results.ok("Admin Sidebar present")
        else: results.fail("Admin Sidebar MISSING")
        
        if html.has_admin_css: results.ok("admin.css loaded")
        else: results.fail("admin.css MISSING")
        
        if html.has_htmx: results.ok("HTMX loaded")
        else: results.warn("HTMX missing (standard on admin pages)")

        if html.meta_viewport: results.ok("Viewport meta tag present (responsive design)")
        else: results.warn("Viewport meta tag MISSING")

        if html.has_theme_engine: results.ok("Theme engine loaded")
        else: results.warn("Theme engine MISSING")

        if html.has_permission_guard: results.ok("RBAC permission guard loaded")
        else: results.fail("RBAC permission guard MISSING (Critical Security)")

        if html.has_breadcrumbs: results.ok("Breadcrumbs found")
        else: results.warn("Breadcrumbs MISSING")
        
        if expected_id:
            if expected_id in html.ids: results.ok(f"Element #{expected_id} found")
            else: results.warn(f"Element #{expected_id} MISSING (Likely JS loaded)")
            
        if expected_nav:
            if expected_nav == html.active_nav_id: results.ok(f"Nav item '{expected_nav}' is ACTIVE")
            elif expected_nav in html.ids: results.warn(f"Nav item '{expected_nav}' exists but NOT ACTIVE")
            else: results.warn(f"Nav item '{expected_nav}' MISSING from sidebar (Likely JS loaded)")

        # --- Detailed Element Checks for Dashboard ---
        if path == "/admin/":
            kpis = [
                "kpi-total-users", "kpi-new-users", "kpi-aum", "kpi-deposits-24h",
                "kpi-deposits-count", "kpi-pending-kyc", "kpi-live-assets", 
                "kpi-funded-assets", "kpi-pending-deposits", "kpi-open-tickets",
                "kpi-rewards-liability", "badge-kyc", "badge-deposits", "badge-support",
                "activity-feed", "recent-orders-table", "pending-deposits-table",
                "health-indicators", "health-db", "health-psp", "health-kyc", "health-email",
                "dashboard-range", "dashboard-date", "admin-global-search"
            ]
            for kpi in kpis:
                if kpi in html.ids: results.ok(f"Dashboard element #{kpi} found")
                else: results.warn(f"Dashboard element #{kpi} MISSING (expected for populated UI)")

        # Verify HTMX elements integrity
        for hx in html.htmx_elements:
            if "hx-swap" in hx["attrs"] and "hx-target" not in hx["attrs"]:
                results.warn(f"HTMX element <{hx['tag']}> has hx-swap but NO hx-target")

        # Verify static file logic for each page iteratively (CSRF checks + CSS resets)
        for js_src in html.scripts:
            # Only test our own scripts
            if "static" in js_src and not js_src.startswith("http"):
                js_r = session.get(f"{BASE_URL}{js_src}", timeout=5)
                if js_r.status_code == 200:
                    js_text = js_r.text
                    if 'fetch(' in js_text and ('method: "POST"' in js_text or "method: 'POST'" in js_text or 'method: "DELETE"' in js_text or "method: 'DELETE'" in js_text):
                        if "x-csrf-token" not in js_text.lower():
                            results.warn(f"JS {js_src} on {path} makes POST/DELETE fetch without CSRF token header")
                else:
                    results.warn(f"JS load failed {js_src} on {path}")

        for css_href in html.stylesheets:
            if "admin.css" in css_href:
                css_r = session.get(f"{BASE_URL}{css_href}", timeout=5)
                if css_r.status_code == 200:
                    if "box-sizing: border-box" not in css_r.text.lower():
                        results.warn(f"admin.css on {path} is MISSING global box-sizing reset")

        return html
    except Exception as e:
        import traceback
        traceback.print_exc()
        results.fail(f"Error testing {path}", str(e))
        return None

def test_rbac_access_control(results):
    results.section("RBAC SECURITY")
    
    # 1. Unauthenticated (Anonymous)
    session_anon = requests.Session()
    paths = ["/admin/", "/admin/users.html", "/admin/settings.html"]
    for path in paths:
        try:
            r = session_anon.get(f"{BASE_URL}{path}", allow_redirects=False, timeout=5)
            if r.status_code in (301, 302, 303, 401, 403):
                results.ok(f"Unauthenticated access to {path} blocked (Status {r.status_code})")
            else:
                results.fail(f"Unauthenticated access to {path} allowed!", f"Status: {r.status_code}")
        except Exception as e:
            results.fail(f"Error testing unauthorized access to {path}", str(e))

    # 2. Investor Role (Should NOT have admin access)
    session_investor = get_session_by_role("investor")
    if session_investor:
        for path in paths:
            try:
                r = session_investor.get(f"{BASE_URL}{path}", allow_redirects=False, timeout=5)
                # Investor should be 403 Forbidden or redirected back to homepage/not-authorized
                if r.status_code in (401, 403, 301, 302, 303):
                    results.ok(f"Investor access to {path} blocked (Status {r.status_code})")
                else:
                    results.fail("Investor ACCESSED /admin page!", f"Path: {path}, Status: {r.status_code}")
            except Exception as e:
                results.fail(f"Error testing investor access to {path}", str(e))
    else:
        results.warn("No active 'investor' session found in DB to test RBAC.")

def test_sidebar_links_integrity(session, results, html):
    results.section("SIDEBAR LINKS INTEGRITY")
    if not html or not html.has_sidebar:
        results.fail("Cannot check sidebar links: Sidebar missing")
        return

    # Filter links that look like admin pages
    admin_links = sorted(list(set([l for l in html.links if l.startswith("/admin/") and not l.endswith(".ico") and "?" not in l])))
    
    results.info(f"Checking {len(admin_links)} sidebar links...")
    for link in admin_links:
        try:
            r = session.head(f"{BASE_URL}{link}", timeout=5)
            if r.status_code == 200:
                results.ok(f"Link {link} is valid (200 OK)")
            elif r.status_code == 404:
                results.fail(f"Broken sidebar link: {link} (404)")
            else:
                results.warn(f"Sidebar link {link} returned {r.status_code}")
        except:
            results.fail(f"Timeout checking link {link}")

def test_admin_api_health_enhanced(session, results):
    results.section("ADMIN API HEALTH & SCHEMA")
    endpoints = [
        ("/api/admin/stats/overview", "Overall Stats", ["total_users", "pending_kyc", "aum_cents", "user_trend", "deposit_trend", "range_label"]),
        ("/api/admin/users", "User List", ["id", "email", "role"]),
        ("/api/admin/kyc", "KYC Queue", ["id", "user_id"]),
        ("/api/admin/orders", "Order History", ["id", "user_id", "total_cents"]),
        ("/api/admin/assets", "Assets", ["id", "title", "asset_type"]),
        ("/api/admin/deposits", "Deposits", ["id", "status", "amount_cents"]),
        ("/api/admin/support", "Support Tickets", ["id", "subject", "status"]),
        ("/api/admin/audit-logs", "Audit Logs", ["id", "action", "timestamp"]),
        ("/api/admin/notifications", "Notifications", ["id", "message"]),
        ("/api/admin/system", "System Health", ["db_connected", "email_connected"]),
    ]
    
    for url, desc, required_keys in endpoints:
        try:
            r = session.get(f"{BASE_URL}{url}", timeout=5)
            if r.status_code == 200:
                results.ok(f"{desc} API ({url}) returns 200")
                data = r.json()
                results.ok(f"  Valid JSON")
                
                # If it's a list, check the first item
                if isinstance(data, list) and len(data) > 0:
                    item = data[0]
                    missing = [k for k in required_keys if k not in item]
                    if not missing: results.ok(f"  API response item schema valid for '{desc}'")
                    else: results.warn(f"  API response item MISSING keys: {', '.join(missing)}")
                elif isinstance(data, dict):
                    # Special check for user list if wrapped in object
                    items = data.get("users") or data.get("records") or data.get("assets") or data.get("orders")
                    if items and isinstance(items, list) and len(items) > 0:
                        item = items[0]
                        missing = [k for k in required_keys if k not in item]
                        if not missing: results.ok(f"  API response record schema valid for '{desc}'")
                        else: results.warn(f"  API response record MISSING keys: {', '.join(missing)}")
                    else:
                        # Direct dict check (for stats/health)
                        missing = [k for k in required_keys if k not in data]
                        if not missing: results.ok(f"  API response dictionary schema valid for '{desc}'")
                        else: results.warn(f"  API response dictionary MISSING keys: {', '.join(missing)}")
            elif r.status_code == 404:
                results.warn(f"{desc} API ({url}) NOT IMPLEMENTED (404)")
            else:
                results.fail(f"{desc} API ({url}) failed", f"Status: {r.status_code}")
        except Exception as e:
            results.fail(f"Error calling {url}", str(e))

def test_admin_data_consistency(session, results):
    results.section("DATA CONSISTENCY (List -> Detail)")
    try:
        # 1. Get user list
        r = session.get(f"{BASE_URL}/api/admin/users", timeout=5)
        users_data = r.json()
        users = users_data if isinstance(users_data, list) else (users_data.get("users") or [])
        
        if not users:
            results.warn("No users found to test consistency.")
            return

        test_user = users[0]
        user_id = test_user["id"]
        expected_email = test_user["email"]

        # 2. Get user detail
        r_detail = session.get(f"{BASE_URL}/api/admin/users/{user_id}", timeout=5)
        if r_detail.status_code == 200:
            detail = r_detail.json()
            if detail.get("email") == expected_email:
                results.ok(f"Consistency check PASSED for User ID {user_id} ({expected_email})")
            else:
                results.fail(f"Consistency check FAILED for User ID {user_id}!", 
                             f"List email: {expected_email}, Detail email: {detail.get('email')}")
        else:
            results.warn(f"User detail API returned {r_detail.status_code}, consistency check skipped.")
    except Exception as e:
        results.fail("Error in data consistency test", str(e))

# ═══════════════════════════════════════════════════════════════════
# Main Runner
# ═══════════════════════════════════════════════════════════════════
def main():
    print("\n" + "╔" + "═"*78 + "╗")
    print("║" + " "*24 + "POOOL ADMIN DASHBOARD TEST SUITE" + " "*22 + "║")
    print("╚" + "═"*78 + "╝")
    
    results = TestResults()
    session = get_admin_session()
    
    if not session:
        print("\n  ❌ CRITICAL: Failed to authenticate as Admin.")
        print("     Ensure backend is running and an admin user exists.")
        sys.exit(1)
    
    # Security & RBAC
    test_rbac_access_control(results)
    
    # Core Admin Overview Pages
    dashboard_html = test_admin_page(session, results, "/admin/", "Dashboard", "nav-dashboard", "nav-dashboard")
    if dashboard_html:
        # Check specific dashboard components
        dashboard_elements = [
            "kpi-total-users", "kpi-aum", "kpi-deposits-24h", "kpi-pending-kyc",
            "kpi-live-assets", "kpi-pending-deposits", "kpi-open-tickets", "kpi-rewards-liability",
        ]
        for elem in dashboard_elements:
            if elem in dashboard_html.ids:
                results.ok(f"  Dashboard element #{elem} found")
            else:
                results.warn(f"  Dashboard element #{elem} MISSING")
                
        # Check sidebar links
        test_sidebar_links_integrity(session, results, dashboard_html)

    test_admin_page(session, results, "/admin/users.html", "Users List", "nav-users", "nav-users")
    test_admin_page(session, results, "/admin/kyc.html", "KYC & AML Queue", "nav-kyc", "nav-kyc")
    test_admin_page(session, results, "/admin/support.html", "Support Tickets List", "nav-support", "nav-support")
    test_admin_page(session, results, "/admin/developer-submissions.html", "Developer Submissions", "nav-submissions", "nav-submissions")
    test_admin_page(session, results, "/admin/assets.html", "Live Assets List", "nav-assets", "nav-assets")
    test_admin_page(session, results, "/admin/orders.html", "Orders History", "nav-orders", "nav-orders")
    test_admin_page(session, results, "/admin/deposits.html", "Deposits Management", "nav-deposits", "nav-deposits")
    test_admin_page(session, results, "/admin/treasury.html", "Treasury Overview", "nav-treasury", "nav-treasury")
    test_admin_page(session, results, "/admin/rewards.html", "Rewards Configuration", "nav-rewards", "nav-rewards")
    test_admin_page(session, results, "/admin/notifications.html", "System Notifications", "nav-notifications", "nav-notifications")
    test_admin_page(session, results, "/admin/audit-logs.html", "Audit Trail", "nav-audit", "nav-audit")
    test_admin_page(session, results, "/admin/reports.html", "Analytics Reports", "nav-reports", "nav-reports")
    test_admin_page(session, results, "/admin/email-marketing.html", "Email Engine", "nav-email", "nav-email")
    test_admin_page(session, results, "/admin/system.html", "System Health Monitor", "nav-system", "nav-system")
    test_admin_page(session, results, "/admin/settings.html", "Global Platform Settings", "nav-settings", "nav-settings")
    test_admin_page(session, results, "/admin/admins.html", "Admin Management", "nav-admins", "nav-admins")
    test_admin_page(session, results, "/admin/roles.html", "Roles & Permissions", "nav-roles", "nav-roles")
    test_admin_page(session, results, "/admin/dividends.html", "Dividend Distribution Tool", "nav-dividends")
    test_admin_page(session, results, "/admin/approvals.html", "Admin Approvals Queue", "nav-approvals")
    
    # Detailed Item Pages (Requiring Params / Mock Data in URL)
    test_admin_page(session, results, "/admin/user-details.html?id=1", "Specific User Details", "user-content")
    test_admin_page(session, results, "/admin/asset-details.html?id=1", "Specific Asset Details", "asset-sc-details")
    test_admin_page(session, results, "/admin/support-ticket.html?id=1", "Specific Support Ticket", "ticket-thread")
    test_admin_page(session, results, "/admin/developer-submission-review.html?id=1", "Developer Submission Review", "review-content")
    
    # API & Data Verification
    test_admin_api_health_enhanced(session, results)
    test_admin_data_consistency(session, results)
    
    # Final Summary
    print("\n" + "═"*80)
    print(f"  FINAL SUMMARY")
    print("═"*80)
    print(f"  ✅ Passed:   {results.passed}")
    print(f"  ❌ Failed:   {results.failed}")
    print(f"  ⚠️  Warnings: {results.warnings}")
    
    if results.failed > 0:
        print("\n  Failures to fix:")
        for section, err in results.errors:
            print(f"  - [{section}] {err}")
        sys.exit(1)
    else:
        print("\n  🎉 ALL ENHANCED ADMIN DASHBOARD TESTS PASSED!")
        sys.exit(0)

if __name__ == "__main__":
    main()
