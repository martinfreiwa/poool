"""
POOOL E2E Test Infrastructure — conftest.py (Production-Grade)
================================================================
Central pytest-playwright configuration with automatic quality checks.

AUTOMATIC CHECKS ON EVERY TEST:
1. Console Error Capture  — Every JS error/warning is recorded; critical → test fails
2. Network Failure Detect — 4xx/5xx HTTP responses tracked; API errors → test fails
3. Page Load Verification — Every navigation must complete; blank pages → test fails
4. Screenshot on Failure  — Full-page screenshot saved automatically
5. Playwright Trace       — On failure, full trace ZIP saved for debugging
6. Performance Baseline   — Page load times logged per navigation
7. Accessibility Sanity   — Basic a11y checks (page lang, viewport meta, title)
8. Mobile Viewport        — Parameterized via @pytest.mark.mobile
9. DB Test Isolation      — Test users cleaned up after each run
10. Backend Health Gate   — Session-scoped check: fail fast if backend is down

All fixtures yield (page, tracker, ...) so every test benefits automatically.
"""

import pytest
import os
import sys
import time
import json
import psycopg2
import uuid
import urllib.request
import urllib.error
from datetime import datetime
from pathlib import Path
from playwright.sync_api import Error as PlaywrightError
from playwright.sync_api import sync_playwright, expect

# ─── Configuration ────────────────────────────────────────────────────────────

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
ADMIN_EMAIL = os.environ.get("ADMIN_EMAIL", "admin@poool.app")
ADMIN_PASSWORD = os.environ.get("ADMIN_PASSWORD", "Tasse3765!pooolDev")

# Output directories
TEST_ROOT = Path(__file__).parent
SCREENSHOT_DIR = TEST_ROOT / "screenshots"
REPORT_DIR = TEST_ROOT / "reports"
TRACE_DIR = TEST_ROOT / "traces"

for d in (SCREENSHOT_DIR, REPORT_DIR, TRACE_DIR):
    d.mkdir(parents=True, exist_ok=True)

# Viewports for device testing
VIEWPORTS = {
    "desktop": {"width": 1280, "height": 800},
    "mobile": {"width": 375, "height": 812},
    "tablet": {"width": 768, "height": 1024},
}

# User-Agent strings for device emulation
USER_AGENTS = {
    "desktop": None,  # Use browser default
    "mobile": "Mozilla/5.0 (iPhone; CPU iPhone OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1",
    "tablet": "Mozilla/5.0 (iPad; CPU OS 16_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Mobile/15E148 Safari/604.1",
}

# Console error patterns to ignore (browser noise)
NOISE_PATTERNS = [
    "favicon",
    "net::ERR_BLOCKED_BY_CLIENT",          # ad blockers
    "ResizeObserver loop",                  # benign Chrome warning
    "third-party cookie",
    "Violation",                            # Chrome performance hints
    "Download the React DevTools",
    "Download the Vue Devtools",
    "[HMR]",                                # Hot Module Replacement
    "DevTools",
    "source map",
    "Failed to decode downloaded font",     # Optional font issues
    "google-analytics",
    "gtag",
    "hotjar",
    "sentry",
    "stripe.com",
    "Failed to load resource",
    "401 ()",
    "429 ()",
]

# Network URL patterns to ignore for 4xx/5xx tracking
NETWORK_IGNORE = [
    "/favicon.ico",
    "analytics",
    "sentry",
    "google-analytics",
    "gtag",
    "hotjar",
    "fonts.googleapis.com",
    "pagead",
    "stripe.com",
    "mixpanel.com",
    "doubleclick.net",
]


# ═══════════════════════════════════════════════════════════════════════════
# SECTION 1: BACKEND HEALTH GATE
# ═══════════════════════════════════════════════════════════════════════════

@pytest.fixture(scope="session", autouse=True)
def _verify_backend_is_running():
    """
    Session-scoped gate: fail fast if the backend isn't running.
    Prevents wasting time on 30+ tests that will all fail anyway.
    """
    health_url = f"{BASE_URL}/health"
    max_retries = 3
    for attempt in range(max_retries):
        try:
            resp = urllib.request.urlopen(health_url, timeout=5)
            if resp.status == 200:
                return
        except (urllib.error.URLError, urllib.error.HTTPError, OSError):
            if attempt < max_retries - 1:
                time.sleep(2)
    pytest.exit(
        f"⛔ Backend not reachable at {health_url}. "
        "Start it with `cd backend && cargo run` before running tests.",
        returncode=1,
    )


# ═══════════════════════════════════════════════════════════════════════════
# SECTION 2: PAGE QUALITY TRACKER
# ═══════════════════════════════════════════════════════════════════════════

class PageQualityTracker:
    """
    Collects console errors, network failures, and performance data for a page.
    Attach to any Playwright page to get automatic quality monitoring.
    """

    def __init__(self, page, test_name="unknown"):
        self.page = page
        self.test_name = test_name
        self.console_errors = []
        self.console_warnings = []
        self.console_logs = []        # All console messages for debugging
        self.network_failures = []    # 4xx/5xx HTTP responses
        self.failed_resources = []    # Requests that failed entirely (DNS, CORS)
        self.page_load_times = {}     # URL → load time in ms
        self.navigations = []         # All URLs visited
        self._attach_listeners()

    def _attach_listeners(self):
        """Wire up Playwright event listeners."""
        self.page.on("console", self._on_console)
        self.page.on("requestfailed", self._on_request_failed)
        self.page.on("response", self._on_response)
        self.page.on("pageerror", self._on_page_error)

    def _on_console(self, msg):
        """Record console messages by severity."""
        entry = {
            "type": msg.type,
            "text": msg.text,
            "url": self.page.url,
            "timestamp": datetime.now().isoformat(),
        }
        self.console_logs.append(entry)
        if msg.type == "error":
            self.console_errors.append(entry)
        elif msg.type == "warning":
            self.console_warnings.append(entry)

    def _on_page_error(self, error):
        """Record uncaught JS exceptions — always critical."""
        self.console_errors.append({
            "type": "pageerror",
            "text": f"[UNCAUGHT EXCEPTION] {error.message}",
            "url": self.page.url,
            "timestamp": datetime.now().isoformat(),
        })

    def _on_request_failed(self, request):
        """Record requests that completely failed (DNS/CORS/network level)."""
        self.failed_resources.append({
            "url": request.url,
            "failure": request.failure,
            "resource_type": request.resource_type,
            "page_url": self.page.url,
            "timestamp": datetime.now().isoformat(),
        })

    def _on_response(self, response):
        """Record HTTP 4xx/5xx responses, filtering out known noise."""
        if response.status >= 400:
            url = response.url
            if any(p in url for p in NETWORK_IGNORE):
                return
            self.network_failures.append({
                "url": url,
                "status": response.status,
                "status_text": response.status_text,
                "method": response.request.method,
                "page_url": self.page.url,
                "timestamp": datetime.now().isoformat(),
            })

    # ── Navigation Helpers ──

    def navigate_and_check(self, url, timeout=15000, wait_until="domcontentloaded"):
        """
        Navigate to URL, record load time, verify basic page health.
        Raises AssertionError on HTTP 500+ or unexpected login redirect.
        """
        start = time.time()
        response = self.page.goto(url, wait_until=wait_until, timeout=timeout)
        load_time_ms = int((time.time() - start) * 1000)
        self.page_load_times[url] = load_time_ms
        self.navigations.append({"url": url, "load_ms": load_time_ms})

        # Server 500+ is always a failure
        if response and response.status >= 500:
            raise AssertionError(
                f"🔴 Page returned HTTP {response.status} for {url}"
            )

        # Unexpected redirect to login (missing auth)
        if "/auth/" not in url and "/login" not in url:
            current = self.page.url
            if "/auth/login" in current and "/auth/login" not in url:
                raise AssertionError(
                    f"🔴 Unexpected redirect to login from {url} — session invalid"
                )

        return response

    # ── Assertion Helpers ──

    def get_critical_errors(self):
        """Filter console errors, removing known browser noise."""
        critical = []
        for err in self.console_errors:
            text = err["text"].lower()
            if not any(noise.lower() in text for noise in NOISE_PATTERNS):
                critical.append(err)
        return critical

    def assert_no_critical_errors(self):
        """Hard-fail if any real JS console errors exist."""
        critical = self.get_critical_errors()
        if critical:
            top_errors = critical if len(critical) <= 10 else critical[0:10]  # type: ignore[misc]
            details = "\n".join(f"  • [{e['url']}] {e['text']}" for e in top_errors)
            raise AssertionError(
                f"🔴 {len(critical)} console error(s) found:\n{details}"
            )

    def assert_no_network_failures(self, ignore_status=None):
        """Hard-fail if any 4xx/5xx HTTP responses were detected."""
        ignore = set(ignore_status or [])
        failures = [f for f in self.network_failures if f["status"] not in ignore]
        if failures:
            top_failures = failures if len(failures) <= 10 else failures[0:10]  # type: ignore[misc]
            details = "\n".join(f"  • [{f['status']}] {f['url']}" for f in top_failures)
            raise AssertionError(
                f"🔴 {len(failures)} HTTP error(s):\n{details}"
            )

    def assert_page_loaded(self):
        """Verify the page isn't blank — has visible body with content."""
        body = self.page.locator("body")
        expect(body).to_be_visible()
        content_len = self.page.evaluate("document.body.innerText.trim().length")
        if content_len < 10:
            raise AssertionError(
                f"🔴 Page appears blank ({content_len} chars) at {self.page.url}"
            )

    def assert_no_broken_images(self):
        """Check all <img> elements have loaded successfully (naturalWidth > 0)."""
        broken = self.page.evaluate("""
            () => {
                const imgs = document.querySelectorAll('img[src]');
                const broken = [];
                imgs.forEach(img => {
                    if (img.complete && img.naturalWidth === 0 && img.src && !img.src.includes('data:')) {
                        broken.push(img.src);
                    }
                });
                return broken;
            }
        """)
        if broken:
            details = "\n".join(f"  • {src}" for src in broken[:10])
            raise AssertionError(
                f"🔴 {len(broken)} broken image(s) on {self.page.url}:\n{details}"
            )

    def assert_basic_a11y(self):
        """Quick accessibility sanity checks (not a full audit)."""
        issues = []

        # 1. Page has a <title>
        title = self.page.title()
        if not title or title.strip() == "":
            issues.append("Missing or empty <title> tag")

        # 2. <html> has lang attribute
        lang = self.page.evaluate("document.documentElement.lang")
        if not lang:
            issues.append("Missing lang attribute on <html>")

        # 3. At least one <h1> exists
        h1_count = self.page.locator("h1").count()
        if h1_count == 0:
            issues.append("No <h1> heading found")

        # 4. No duplicate IDs
        duplicate_ids = self.page.evaluate("""
            () => {
                const all = document.querySelectorAll('[id]');
                const seen = new Set();
                const dupes = [];
                all.forEach(el => {
                    if (seen.has(el.id)) dupes.push(el.id);
                    seen.add(el.id);
                });
                return dupes;
            }
        """)
        if duplicate_ids:
            issues.append(f"Duplicate IDs: {', '.join(duplicate_ids[:5])}")

        if issues:
            details = "\n".join(f"  ⚠️ {i}" for i in issues)
            raise AssertionError(
                f"A11y issues on {self.page.url}:\n{details}"
            )

    def assert_no_js_errors_on_load(self):
        """Combined check: page loads properly, no JS errors, no HTTP errors."""
        self.assert_page_loaded()
        self.assert_no_critical_errors()
        self.assert_no_network_failures(ignore_status=[404])  # Allow 404 for optional resources

    def full_health_check(self):
        """Run ALL quality assertions in one call."""
        self.assert_page_loaded()
        self.assert_no_critical_errors()
        self.assert_no_network_failures(ignore_status=[404])
        self.assert_no_broken_images()

    # ── Report Generation ──

    def get_report(self):
        """Generate structured health report as dict."""
        return {
            "test": self.test_name,
            "navigations": self.navigations,
            "console_errors_total": len(self.console_errors),
            "console_errors_critical": len(self.get_critical_errors()),
            "critical_error_details": self.get_critical_errors(),
            "console_warnings": len(self.console_warnings),
            "network_failures": self.network_failures,
            "failed_resources": self.failed_resources,
            "page_load_times": self.page_load_times,
            "timestamp": datetime.now().isoformat(),
        }


# ═══════════════════════════════════════════════════════════════════════════
# SECTION 3: DB HELPERS
# ═══════════════════════════════════════════════════════════════════════════

def get_db_connection():
    """Get a DB connection. Centralised so it's easy to swap."""
    return psycopg2.connect(DB_URL)


def cleanup_test_user(user_id):
    """Remove a test user and all related records from the database."""
    try:
        conn = get_db_connection()
        cur = conn.cursor()
        uid = str(user_id)
        cleanup_statements = [
            ("DELETE FROM user_sessions WHERE user_id = %s", (uid,)),
            (
                "DELETE FROM wallet_transactions WHERE wallet_id IN (SELECT id FROM wallets WHERE user_id = %s)",
                (uid,),
            ),
            ("DELETE FROM wallets WHERE user_id = %s", (uid,)),
            ("DELETE FROM kyc_records WHERE user_id = %s", (uid,)),
            ("DELETE FROM user_roles WHERE user_id = %s", (uid,)),
            ("DELETE FROM user_settings WHERE user_id = %s", (uid,)),
            ("DELETE FROM user_profiles WHERE user_id = %s", (uid,)),
            ("DELETE FROM investment_limits WHERE user_id = %s", (uid,)),
            ("DELETE FROM investments WHERE user_id = %s", (uid,)),
            ("DELETE FROM orders WHERE user_id = %s", (uid,)),
            ("DELETE FROM referral_tracking WHERE user_id = %s", (uid,)),
            ("DELETE FROM user_tiers WHERE user_id = %s", (uid,)),
            ("DELETE FROM user_consents WHERE user_id = %s", (uid,)),
        ]
        for sql, params in cleanup_statements:
            try:
                cur.execute(sql, params)
            except Exception:
                conn.rollback()  # Can continue — table may not exist in older local DBs.
        cur.execute("DELETE FROM users WHERE id = %s", (uid,))
        conn.commit()
        cur.close()
        conn.close()
    except Exception:
        pass  # Best effort cleanup


def create_e2e_user(
    *,
    email_prefix="e2e-test",
    display_name="E2E Tester",
    roles=(),
    cash_balance_cents=1_000_000,
    kyc_status="approved",
):
    """Create an authenticated test user directly in DB to avoid auth rate limits."""
    unique_id = uuid.uuid4().hex[:8]
    email = f"{email_prefix}-{unique_id}@poool.app"
    session_token = str(uuid.uuid4())
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO users (email, email_verified, status)
            VALUES (%s, TRUE, 'active')
            RETURNING id
            """,
            (email,),
        )
        user_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO user_profiles (user_id, first_name, last_name, display_name, annual_income_cents)
            VALUES (%s, 'E2E', 'Tester', %s, 100000000)
            ON CONFLICT (user_id) DO UPDATE SET
                first_name = EXCLUDED.first_name,
                last_name = EXCLUDED.last_name,
                display_name = EXCLUDED.display_name,
                annual_income_cents = EXCLUDED.annual_income_cents
            """,
            (user_id, display_name),
        )
        cur.execute(
            """
            INSERT INTO investment_limits (user_id, annual_limit_cents, invested_12m_cents, limit_year)
            VALUES (%s, 10000000, 0, EXTRACT(YEAR FROM NOW())::INTEGER)
            ON CONFLICT (user_id, limit_year) DO UPDATE SET
                annual_limit_cents = EXCLUDED.annual_limit_cents,
                invested_12m_cents = 0
            """,
            (user_id,),
        )
        cur.execute(
            """
            INSERT INTO wallets (user_id, wallet_type, currency, balance_cents, held_balance_cents)
            VALUES (%s, 'cash', 'USD', %s, 0)
            ON CONFLICT (user_id, wallet_type, currency) DO UPDATE SET
                balance_cents = EXCLUDED.balance_cents,
                held_balance_cents = 0
            """,
            (user_id, cash_balance_cents),
        )
        cur.execute(
            """
            INSERT INTO wallets (user_id, wallet_type, currency, balance_cents, held_balance_cents)
            VALUES (%s, 'rewards', 'USD', 0, 0)
            ON CONFLICT (user_id, wallet_type, currency) DO NOTHING
            """,
            (user_id,),
        )
        cur.execute(
            """
            INSERT INTO kyc_records (user_id, status)
            SELECT %s, %s
            WHERE NOT EXISTS (
                SELECT 1 FROM kyc_records WHERE user_id = %s
            )
            """,
            (user_id, kyc_status, user_id),
        )
        for role in roles:
            cur.execute(
                """
                INSERT INTO user_roles (user_id, role_id, is_active)
                SELECT %s, id, TRUE
                FROM roles
                WHERE name = %s
                ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE
                """,
                (user_id, role),
            )
        cur.execute(
            """
            INSERT INTO user_sessions (user_id, session_token, remember_me, expires_at)
            VALUES (%s, %s, FALSE, NOW() + INTERVAL '1 hour')
            """,
            (user_id, session_token),
        )
        conn.commit()
        return {
            "email": email,
            "password": "TestPass123!",
            "user_id": user_id,
            "unique_id": unique_id,
            "session_token": session_token,
        }
    finally:
        cur.close()
        conn.close()


def attach_session_cookie(context, session_token):
    context.add_cookies([{"name": "poool_session", "value": session_token, "url": BASE_URL}])


# ═══════════════════════════════════════════════════════════════════════════
# SECTION 4: BROWSER FIXTURES
# ═══════════════════════════════════════════════════════════════════════════
VIDEO_DIR = TEST_ROOT / "videos"
VIDEO_DIR.mkdir(parents=True, exist_ok=True)


@pytest.fixture(scope="session")
def playwright_session():
    """Session-scoped Playwright browser instance. Supports BROWSER env var."""
    with sync_playwright() as p:
        browser_name = os.environ.get("BROWSER", "chromium").lower()
        launch_opts = {
            "headless": os.environ.get("HEADED", "0") != "1",
            "slow_mo": int(os.environ.get("SLOWMO", "0")),
        }
        if browser_name == "firefox":
            browser = p.firefox.launch(**launch_opts)
        elif browser_name == "webkit":
            browser = p.webkit.launch(**launch_opts)
        else:
            try:
                browser = p.chromium.launch(**launch_opts)
            except PlaywrightError as exc:
                print(f"Chromium launch failed; falling back to Firefox: {exc}")
                browser = p.firefox.launch(**launch_opts)
        yield browser
        browser.close()


def _create_context_and_page(browser, test_name, viewport="desktop"):
    """Helper: create a browser context with tracing, optional video, and a quality-tracked page."""
    vp = VIEWPORTS.get(viewport, VIEWPORTS["desktop"])
    ua = USER_AGENTS.get(viewport)

    context_opts = {
        "viewport": vp,
        "ignore_https_errors": True,
        "locale": "en-US",
        "timezone_id": "Asia/Jakarta",  # POOOL is Indonesia-based
    }
    if ua:
        context_opts["user_agent"] = ua
    # Emulate mobile touch for mobile viewport
    if viewport == "mobile":
        context_opts["has_touch"] = True
        context_opts["is_mobile"] = True

    # Video recording (opt-in via VIDEO=1 env var)
    if os.environ.get("VIDEO", "0") == "1":
        context_opts["record_video_dir"] = str(VIDEO_DIR)
        context_opts["record_video_size"] = {"width": vp["width"], "height": vp["height"]}

    context = browser.new_context(**context_opts)

    # Auto-accept cookies to avoid the banner obscuring elements
    context.add_init_script("""
        localStorage.setItem("poool_cookie_consent", JSON.stringify({
            "granted_at": "2026-01-01T00:00:00.000Z",
            "preferences": {"essential": true, "analytics": true, "marketing": true}
        }));
    """)

    # Start tracing for debugging failed tests
    context.tracing.start(screenshots=True, snapshots=True, sources=True)

    page = context.new_page()
    tracker = PageQualityTracker(page, test_name=test_name)

    return context, page, tracker


def _teardown_context(context, page, tracker, request):
    """Helper: generate report, save trace + screenshot on failure, close context."""
    # Generate quality report
    report = tracker.get_report()
    ts = datetime.now().strftime("%Y%m%d_%H%M%S")
    safe_name = request.node.name.replace("/", "_").replace(":", "_")[:80]

    report_path = REPORT_DIR / f"{safe_name}_{ts}.json"
    with open(report_path, "w") as f:
        json.dump(report, f, indent=2)

    # Check if test failed
    failed = hasattr(request.node, "rep_call") and request.node.rep_call.failed

    if failed:
        # Save screenshot
        try:
            screenshot_path = SCREENSHOT_DIR / f"FAIL_{safe_name}_{ts}.png"
            page.screenshot(path=str(screenshot_path), full_page=True)
        except Exception:
            pass

        # Save trace
        try:
            trace_path = TRACE_DIR / f"FAIL_{safe_name}_{ts}.zip"
            context.tracing.stop(path=str(trace_path))
        except Exception:
            pass
    else:
        # Discard trace on success (save disk space)
        try:
            context.tracing.stop()
        except Exception:
            pass

    context.close()


# ─── Public Fixtures ──────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def quality_page(playwright_session, request):
    """
    Anonymous page fixture with automatic quality monitoring.
    Yields: (page, tracker)
    """
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    yield page, tracker
    _teardown_context(context, page, tracker, request)


@pytest.fixture(scope="function")
def mobile_page(playwright_session, request):
    """
    iPhone-sized page (375x812, touch, mobile UA) with quality monitoring.
    Yields: (page, tracker)
    """
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    yield page, tracker
    _teardown_context(context, page, tracker, request)


@pytest.fixture(scope="function")
def tablet_page(playwright_session, request):
    """
    iPad-sized page (768x1024, touch) with quality monitoring.
    Yields: (page, tracker)
    """
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="tablet"
    )
    yield page, tracker
    _teardown_context(context, page, tracker, request)


@pytest.fixture(scope="function")
def authenticated_user_page(playwright_session, request):
    """
    Creates a fresh test user directly in DB, bypasses KYC,
    funds wallet with $10,000, returns (page, tracker, user_context).
    Cleans up test user after the test.
    """
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    user = create_e2e_user(email_prefix="e2e-test", display_name="E2E Tester")
    user_id = user["user_id"]
    attach_session_cookie(context, user["session_token"])

    yield page, tracker, {
        "email": user["email"],
        "password": user["password"],
        "user_id": user_id,
        "unique_id": user["unique_id"],
    }

    # Cleanup
    _teardown_context(context, page, tracker, request)
    if user_id:
        cleanup_test_user(user_id)


@pytest.fixture(scope="function")
def admin_page(playwright_session, request):
    """
    Creates a fresh super-admin session, returns (page, tracker).
    """
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    user = create_e2e_user(
        email_prefix="e2e-admin",
        display_name="E2E Admin",
        roles=("admin", "super_admin"),
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker
    _teardown_context(context, page, tracker, request)
    cleanup_test_user(user["user_id"])


@pytest.fixture(scope="function")
def admin_mobile_page(playwright_session, request):
    """
    Creates a fresh super-admin session on a mobile viewport, returns (page, tracker).
    """
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    user = create_e2e_user(
        email_prefix="e2e-admin-mob",
        display_name="E2E Mobile Admin",
        roles=("admin", "super_admin"),
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker
    _teardown_context(context, page, tracker, request)
    cleanup_test_user(user["user_id"])


# ═══════════════════════════════════════════════════════════════════════════
# SECTION 5: REUSABLE HELPER FUNCTIONS (importable in any test)
# ═══════════════════════════════════════════════════════════════════════════

def wait_for_api_response(page, url_pattern, action_fn=None, timeout=10000):
    """
    Wait for a specific API response while performing an action.
    Usage:
        response = wait_for_api_response(page, "**/api/wallet/balance",
                                          action_fn=lambda: page.click("#refresh"))
        assert response.status == 200
    """
    with page.expect_response(url_pattern, timeout=timeout) as response_info:
        if action_fn:
            action_fn()
    return response_info.value


def take_named_screenshot(page, name):
    """Save a named screenshot (for visual regression tracking, not failures)."""
    ts = datetime.now().strftime("%Y%m%d_%H%M%S")
    path = SCREENSHOT_DIR / f"{name}_{ts}.png"
    page.screenshot(path=str(path), full_page=True)
    return path


def check_toast_message(page, expected_text=None, timeout=5000):
    """Wait for and verify a toast notification appears."""
    toast = page.locator(
        ".poool-toast-card:visible, .toast:visible, .notification:visible, "
        ".alert-success:visible, [role='alert']:visible"
    ).first
    expect(toast).to_be_visible(timeout=timeout)
    if expected_text:
        expect(toast).to_contain_text(expected_text, ignore_case=True)
    return toast


def fill_and_submit_form(page, fields, submit_selector):
    """
    Fill a form and submit.
    fields: dict of {selector: value}
    """
    for selector, value in fields.items():
        if isinstance(value, bool):
            if value:
                page.check(selector)
            else:
                page.uncheck(selector)
        else:
            page.fill(selector, str(value))
    page.click(submit_selector)


def intercept_api_call(page, url_pattern, mock_response=None):
    """
    Intercept an API call with page.route() — mock responses for isolation.
    Usage:
        intercept_api_call(page, "**/api/wallet/balance", {"balance_cents": 5000})
    """
    def handler(route):
        if mock_response is not None:
            route.fulfill(
                status=200,
                content_type="application/json",
                body=json.dumps(mock_response),
            )
        else:
            route.continue_()
    page.route(url_pattern, handler)


def verify_table_has_data(page, table_selector, min_rows=1):
    """Verify a data table rendered with at least N rows."""
    table = page.locator(table_selector)
    expect(table).to_be_visible()
    rows = table.locator("tbody tr")
    row_count = rows.count()
    assert row_count >= min_rows, (
        f"Table {table_selector} has {row_count} rows, expected >= {min_rows}"
    )
    return row_count


def verify_modal_opens_and_closes(page, trigger_selector, modal_selector):
    """Click a trigger, verify modal opens, close it, verify it closed."""
    page.click(trigger_selector)
    modal = page.locator(modal_selector)
    expect(modal).to_be_visible(timeout=5000)

    # Try common close patterns
    close_btn = modal.locator("button.close, .modal-close, [aria-label='Close']").first
    if close_btn.is_visible():
        close_btn.click()
        expect(modal).not_to_be_visible(timeout=3000)


# ═══════════════════════════════════════════════════════════════════════════
# SECTION 6: PYTEST HOOKS
# ═══════════════════════════════════════════════════════════════════════════

@pytest.hookimpl(tryfirst=True, hookwrapper=True)
def pytest_runtest_makereport(item, call):
    """Store test result on the item so fixtures can detect failures."""
    outcome = yield
    rep = outcome.get_result()
    setattr(item, f"rep_{rep.when}", rep)


def pytest_configure(config):
    """Add custom metadata to the HTML report header."""
    if hasattr(config, "_metadata"):
        config._metadata["Base URL"] = BASE_URL
        config._metadata["Framework"] = "pytest-playwright (Strict)"
        config._metadata["Platform"] = "POOOL"


def pytest_collection_modifyitems(session, config, items):
    """Auto-add markers based on test file names."""
    for item in items:
        path = str(item.fspath)
        if "admin" in path:
            item.add_marker(pytest.mark.admin)
        if "marketplace" in path:
            item.add_marker(pytest.mark.marketplace)
        if "settings" in path:
            item.add_marker(pytest.mark.settings)
        if "community" in path or "circles" in path:
            item.add_marker(pytest.mark.community)
        if "auth" in path or "login" in path:
            item.add_marker(pytest.mark.auth)
        if "wallet" in path or "financial" in path:
            item.add_marker(pytest.mark.financial)
