from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[1]


def test_login_template_avoids_script_inner_html_and_names_icon_controls():
    html = (ROOT / "frontend/platform/login.html").read_text()
    scripts = "\n".join(re.findall(r"<script>(.*?)</script>", html, flags=re.S))

    assert ".innerHTML" not in scripts
    assert 'id="toggle-password" aria-label="Show password" aria-pressed="false"' in html
    carousel = (ROOT / "frontend/platform/components/auth-customer-carousel.html").read_text()
    assert 'data-auth-carousel-prev aria-label="Previous customer story"' in carousel
    assert 'data-auth-carousel-next aria-label="Next customer story"' in carousel
    assert 'id="login-form"' in html and 'aria-busy="false"' in html
    assert 'id="login-button" aria-busy="false"' in html
    assert "{% if google_enabled %}" in html
    assert "{% endif %}" in html


def test_failed_login_telemetry_does_not_include_raw_email_patterns():
    service = (ROOT / "backend/src/auth/service.rs").read_text()
    routes = (ROOT / "backend/src/auth/routes.rs").read_text()

    forbidden_service_patterns = [
        "Failed login: unknown email {}",
        "Failed login: OAuth-only account {}",
        "Login blocked: email not verified {}",
        "Failed login: wrong password for {}",
        "email: Some(email",
    ]
    for pattern in forbidden_service_patterns:
        assert pattern not in service

    assert "Rate limit exceeded for login on email:" not in routes
    assert "Rate limit exceeded on login (IP or email bucket)" in routes


def test_auth_htmx_csrf_failures_return_login_error_fragment():
    csrf = (ROOT / "backend/src/auth/csrf.rs").read_text()

    assert '"HX-Request"' in csrf
    assert "Security check failed. Please refresh the page and try again." in csrf
    assert 'class="auth-error-message"' in csrf


def test_successful_login_path_does_not_block_on_noncritical_side_effects():
    routes = (ROOT / "backend/src/auth/routes.rs").read_text()
    login_body = routes.split("pub async fn login_submit(", 1)[1].split(
        "fn spawn_login_side_effects(", 1
    )[0]

    assert "service::is_admin" not in login_body
    assert "service::get_user_settings(&state.db, user.id)" not in login_body
    assert "Login session creation timed out." in login_body
    assert "spawn_login_side_effects(" in login_body

    side_effects = routes.split("fn spawn_login_side_effects(", 1)[1].split(
        "// ─── 2FA Routes", 1
    )[0]
    assert "tokio::spawn" in side_effects
    assert 'Duration::from_secs(2)' in side_effects
    assert "crate::common::audit::log" in side_effects
    assert "crate::community::xp::track_login_streak" in side_effects


def test_login_time_2fa_challenge_is_enforced_for_enrolled_accounts():
    routes = (ROOT / "backend/src/auth/routes.rs").read_text()
    login_body = routes.split("pub async fn login_submit(", 1)[1].split(
        "fn spawn_login_side_effects(", 1
    )[0]
    oauth_callback = routes.split("pub async fn google_callback(", 1)[1].split(
        "fn google_oauth_redirect_uri(", 1
    )[0]

    assert 'route("/2fa", get(totp_verify_page).post(totp_verify_submit))' in routes
    assert 'route("/2fa/setup", get(totp_setup_page).post(totp_setup_submit))' in routes
    assert "service::user_totp_enabled(&state.db, user.id).await?" in login_body
    assert "service::user_totp_enabled(&state.db, user.id).await?" in oauth_callback
    assert '(false, "/auth/2fa")' in login_body
    assert '(false, "/auth/2fa")' in oauth_callback
    assert '(true, "/marketplace")' in login_body
    assert '(true, "/marketplace")' in oauth_callback


def test_totp_verify_errors_return_auth_html_fragments_not_json_bubbles():
    routes = (ROOT / "backend/src/auth/routes.rs").read_text()
    handler = routes.split("pub async fn totp_verify_submit(", 1)[1].split(
        "/// GET /auth/2fa/setup", 1
    )[0]

    assert "auth_form_error_response" in handler
    assert "login_error_response" not in handler
    assert '"/auth/2fa"' in handler
    assert "decrypt_stored_totp_secret(&secret)?" not in handler
    assert "rotate_session_token(&state.db, &session_token).await?" not in handler
