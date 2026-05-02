from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_2fa_setup_template_preserves_totp_setup_contract():
    html = read("frontend/platform/auth-2fa-setup.html")

    assert 'src="data:image/png;base64,{{ qr_code }}"' in html
    assert "{{ secret }}" in html
    assert 'name="setup_token" value="{{ setup_token }}"' in html
    assert 'hx-post="/auth/2fa/setup"' in html
    assert 'hx-target="#auth-error"' in html
    assert 'id="totp-code-input"' in html
    assert 'name="code"' in html
    assert 'autocomplete="one-time-code"' in html
    assert 'id="login-button"' in html
    assert 'id="loading-indicator"' in html


def test_2fa_setup_template_uses_designed_branded_layout():
    html = read("frontend/platform/auth-2fa-setup.html")

    assert "/static/images/icons/logo-pool.svg" in html
    assert "Set up two-factor authentication" in html
    assert "Pool Logo" not in html
    assert "Authenticator setup QR code" in html
    assert "POOOL Security" in html


def test_2fa_setup_template_has_back_link_and_no_header_lock_badge():
    html = read("frontend/platform/auth-2fa-setup.html")

    assert 'href="/settings" aria-label="Back to previous page"' in html
    assert ">Back</a>" in html or "Back\n    </a>" in html
    assert "Account security" not in html
    assert "<!-- Eyebrow badge -->" not in html


def test_2fa_setup_button_uses_design_primary_colors():
    html = read("frontend/platform/auth-2fa-setup.html")

    assert "background:#0000FF; color:#98FB96;" in html
    assert "border:none; border-radius:8px;" in html
    assert "font-size:16px; font-weight:700;" in html


def test_2fa_setup_success_response_renders_confirmation_instead_of_redirecting():
    routes = read("backend/src/auth/routes.rs")

    setup_submit = routes.split("pub async fn totp_setup_submit", 1)[1].split(
        "/// POST /auth/2fa/step-up",
        1,
    )[0]

    assert "auth-success-message" in setup_submit
    assert "Two-factor authentication connected successfully." in setup_submit
    assert 'href="/marketplace"' in setup_submit
    assert 'HeaderValue::from_static("/marketplace")' not in setup_submit
