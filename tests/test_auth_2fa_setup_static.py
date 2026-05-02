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


def test_2fa_setup_template_uses_designed_branded_layout_without_inline_styles():
    html = read("frontend/platform/auth-2fa-setup.html")

    assert 'class="auth-2fa-setup-page"' in html
    assert 'class="auth-setup-panel"' in html
    assert 'class="auth-setup-qr"' in html
    assert 'class="auth-setup-secret"' in html
    assert 'class="auth-setup-side-panel"' in html
    assert "/static/images/logos/poool-icon.svg" in html
    assert "Set up two-factor authentication" in html
    assert "Pool Logo" not in html
    assert "style=" not in html


def test_2fa_setup_css_is_scoped_to_auth_setup_page():
    css = read("frontend/platform/static/css/login.css")

    assert ".auth-2fa-setup-page" in css
    assert ".auth-setup-panel" in css
    assert ".auth-setup-step__number" in css
    assert ".auth-setup-qr" in css
    assert ".auth-setup-secret" in css
    assert ".auth-setup-side-panel" in css
    assert ".auth-setup-checklist__item" in css
