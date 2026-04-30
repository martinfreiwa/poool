from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[1]


def test_login_template_avoids_script_inner_html_and_names_icon_controls():
    html = (ROOT / "frontend/platform/login.html").read_text()
    scripts = "\n".join(re.findall(r"<script>(.*?)</script>", html, flags=re.S))

    assert ".innerHTML" not in scripts
    assert 'id="toggle-password" aria-label="Show password" aria-pressed="false"' in html
    assert 'id="prev-arrow" aria-label="Previous testimonial"' in html
    assert 'id="next-arrow" aria-label="Next testimonial"' in html
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
    assert "Rate limit exceeded for login on submitted email bucket" in routes


def test_auth_htmx_csrf_failures_return_login_error_fragment():
    csrf = (ROOT / "backend/src/auth/csrf.rs").read_text()

    assert '"HX-Request"' in csrf
    assert "Security check failed. Please refresh the page and try again." in csrf
    assert 'class="auth-error-message"' in csrf
