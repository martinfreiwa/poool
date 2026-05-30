import os
import uuid
from datetime import datetime, timedelta, timezone
from pathlib import Path

import psycopg2
import pytest
from argon2 import PasswordHasher
from playwright.sync_api import expect

from tests.e2e.conftest import BASE_URL, DB_URL, attach_session_cookie, cleanup_test_user


TEST_PASSWORD = "TestPass123!"
_PASSWORD_HASHER = PasswordHasher()
ROOT = Path(__file__).resolve().parents[2]


def _configured_env_value(name):
    value = os.environ.get(name)
    if value:
        return value

    env_path = ROOT / "backend" / ".env"
    if not env_path.exists():
        return ""

    prefix = f"{name}="
    for line in env_path.read_text().splitlines():
        stripped = line.strip()
        if stripped.startswith(prefix):
            return stripped[len(prefix):].strip().strip('"').strip("'")
    return ""


def _google_oauth_configured():
    return bool(
        _configured_env_value("GOOGLE_CLIENT_ID")
        and _configured_env_value("GOOGLE_CLIENT_SECRET")
    )


def db_connect():
    return psycopg2.connect(DB_URL)


def _create_login_user(*, email_prefix="e2e-auth-login", totp_enabled=False):
    email = f"{email_prefix}-{uuid.uuid4().hex[:10]}@poool.app"
    password_hash = _PASSWORD_HASHER.hash(TEST_PASSWORD)
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO users (email, password_hash, email_verified, status)
            VALUES (%s, %s, TRUE, 'active')
            RETURNING id
            """,
            (email, password_hash),
        )
        user_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO user_profiles (user_id, first_name, last_name, display_name)
            VALUES (%s, 'E2E', 'Login', 'E2E Login User')
            ON CONFLICT (user_id) DO UPDATE SET
                first_name = EXCLUDED.first_name,
                last_name = EXCLUDED.last_name,
                display_name = EXCLUDED.display_name
            """,
            (user_id,),
        )
        cur.execute(
            """
            INSERT INTO user_settings (user_id, totp_enabled, totp_secret)
            VALUES (%s, %s, %s)
            ON CONFLICT (user_id) DO UPDATE SET
                totp_enabled = EXCLUDED.totp_enabled,
                totp_secret = EXCLUDED.totp_secret,
                updated_at = NOW()
            """,
            (user_id, totp_enabled, "e2e-totp-placeholder" if totp_enabled else None),
        )
        conn.commit()
        return {"email": email, "password": TEST_PASSWORD, "user_id": user_id}
    finally:
        cur.close()
        conn.close()


def _latest_session(user_id):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT session_token, remember_me, is_2fa_verified, expires_at
            FROM user_sessions
            WHERE user_id = %s
            ORDER BY created_at DESC
            LIMIT 1
            """,
            (user_id,),
        )
        row = cur.fetchone()
        if not row:
            return None
        return {
            "session_token": row[0],
            "remember_me": row[1],
            "is_2fa_verified": row[2],
            "expires_at": row[3],
        }
    finally:
        cur.close()
        conn.close()


def _session_count(user_id):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute("SELECT COUNT(*) FROM user_sessions WHERE user_id = %s", (user_id,))
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def _create_verified_session(user_id):
    token = str(uuid.uuid4())
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO user_sessions (
                user_id, session_token, remember_me, is_2fa_verified, expires_at
            )
            VALUES (%s, %s, FALSE, TRUE, NOW() + INTERVAL '1 hour')
            """,
            (user_id, token),
        )
        conn.commit()
        return token
    finally:
        cur.close()
        conn.close()


def _login(page, user, *, remember=False):
    page.goto(f"{BASE_URL}/auth/login", wait_until="domcontentloaded")
    page.locator("#email-input").fill(user["email"])
    page.locator("#password-input").fill(user["password"])
    if remember:
        page.locator("#remember-checkbox").check()
    page.locator("#login-button").click()


def _assert_cookie_present(page):
    cookies = page.context.cookies(BASE_URL)
    assert any(cookie["name"] == "poool_session" and cookie["value"] for cookie in cookies)


def test_login_happy_path_sets_session_and_redirects(quality_page):
    page, tracker = quality_page
    user = _create_login_user()
    try:
        _login(page, user)
        page.wait_for_url("**/marketplace", timeout=15000)
        _assert_cookie_present(page)

        session = _latest_session(user["user_id"])
        assert session is not None
        assert session["remember_me"] is False
        assert session["is_2fa_verified"] is True
        assert datetime.now(timezone.utc) + timedelta(hours=20) < session["expires_at"]
        assert session["expires_at"] < datetime.now(timezone.utc) + timedelta(hours=30)
        tracker.assert_no_critical_errors()
    finally:
        cleanup_test_user(user["user_id"])


def test_login_invalid_credentials_render_error_without_session(quality_page):
    page, tracker = quality_page
    user = _create_login_user()
    try:
        page.goto(f"{BASE_URL}/auth/login", wait_until="domcontentloaded")
        page.locator("#email-input").fill(user["email"])
        page.locator("#password-input").fill("WrongPass123!")
        page.locator("#login-button").click()

        alert = page.locator("#auth-error [role='alert']").first
        expect(alert).to_be_visible(timeout=15000)
        expect(alert).to_contain_text("Invalid email or password")
        expect(page.locator("#login-button")).to_be_enabled()
        expect(page).to_have_url(f"{BASE_URL}/auth/login")
        assert _session_count(user["user_id"]) == 0
        assert not any(cookie["name"] == "poool_session" for cookie in page.context.cookies(BASE_URL))
        tracker.assert_no_critical_errors()
    finally:
        cleanup_test_user(user["user_id"])


def test_login_requires_csrf_and_returns_auth_error_fragment(quality_page):
    page, _tracker = quality_page
    page.goto(f"{BASE_URL}/auth/login", wait_until="domcontentloaded")

    result = page.evaluate(
        """
        async () => {
            const response = await fetch('/auth/login', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/x-www-form-urlencoded',
                    'HX-Request': 'true'
                },
                body: new URLSearchParams({
                    email: 'csrf-check@poool.app',
                    password: 'TestPass123!'
                }).toString()
            });
            return {status: response.status, text: await response.text()};
        }
        """
    )

    assert result["status"] == 403
    assert "auth-error-message" in result["text"]
    assert "Security check failed" in result["text"]


def test_login_remember_me_extends_session_duration(quality_page):
    page, tracker = quality_page
    user = _create_login_user(email_prefix="e2e-auth-login-remember")
    try:
        _login(page, user, remember=True)
        page.wait_for_url("**/marketplace", timeout=15000)
        _assert_cookie_present(page)

        session = _latest_session(user["user_id"])
        assert session is not None
        assert session["remember_me"] is True
        assert session["is_2fa_verified"] is True
        assert session["expires_at"] > datetime.now(timezone.utc) + timedelta(days=25)
        tracker.assert_no_critical_errors()
    finally:
        cleanup_test_user(user["user_id"])


def test_login_totp_enabled_redirects_to_2fa_and_blocks_protected_routes(quality_page):
    page, tracker = quality_page
    user = _create_login_user(email_prefix="e2e-auth-login-2fa", totp_enabled=True)
    try:
        _login(page, user)
        page.wait_for_url("**/auth/2fa", timeout=15000)
        _assert_cookie_present(page)

        session = _latest_session(user["user_id"])
        assert session is not None
        assert session["is_2fa_verified"] is False

        page.goto(f"{BASE_URL}/marketplace", wait_until="domcontentloaded")
        page.wait_for_url("**/auth/login**", timeout=15000)
        tracker.assert_no_critical_errors()
    finally:
        cleanup_test_user(user["user_id"])


def test_login_google_button_respects_disabled_oauth_and_controls_are_accessible(quality_page):
    page, tracker = quality_page
    page.goto(f"{BASE_URL}/auth/login", wait_until="domcontentloaded")

    if _google_oauth_configured():
        expect(page.locator("#google-button")).to_be_visible()
    else:
        expect(page.locator("#google-button")).to_have_count(0)

    toggle = page.get_by_label("Show password")
    expect(toggle).to_be_visible()
    toggle.click()
    expect(page.locator("#password-input")).to_have_attribute("type", "text")
    expect(page.get_by_label("Hide password")).to_have_attribute("aria-pressed", "true")
    expect(page.get_by_label("Previous customer story")).to_be_visible()
    expect(page.get_by_label("Next customer story")).to_be_visible()
    tracker.assert_page_loaded()
    tracker.assert_basic_a11y()
    tracker.assert_no_critical_errors()


def test_login_mobile_viewport_fits_without_horizontal_overflow(mobile_page):
    page, tracker = mobile_page
    page.goto(f"{BASE_URL}/auth/login", wait_until="domcontentloaded")
    expect(page.locator("#login-button")).to_be_visible()
    has_horizontal_overflow = page.evaluate(
        "() => document.documentElement.scrollWidth > window.innerWidth + 1"
    )
    assert has_horizontal_overflow is False
    tracker.assert_page_loaded()
    tracker.assert_no_critical_errors()


def test_authenticated_user_visiting_login_redirects_to_marketplace(quality_page):
    page, tracker = quality_page
    user = _create_login_user(email_prefix="e2e-auth-login-redirect")
    try:
        token = _create_verified_session(user["user_id"])
        attach_session_cookie(page.context, token)
        page.goto(f"{BASE_URL}/auth/login", wait_until="domcontentloaded")
        page.wait_for_url("**/marketplace", timeout=15000)
        tracker.assert_no_critical_errors()
    finally:
        cleanup_test_user(user["user_id"])
