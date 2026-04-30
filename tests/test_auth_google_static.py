from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_google_buttons_are_gated_by_oauth_configuration():
    login = read("frontend/platform/login.html")
    signup = read("frontend/platform/signup.html")

    for html in (login, signup):
        assert "{% if google_enabled %}" in html
        assert "{% endif %}" in html
        assert "onclick=\"window.location.href='/auth/google'\"" in html


def test_google_callback_does_not_log_token_response_payloads():
    routes = read("backend/src/auth/routes.rs")

    assert 'No access_token in Google response: {:?}' not in routes
    assert "Google OAuth token response missing access_token" in routes
    assert "response_keys = ?response_keys" in routes


def test_google_callback_uses_configurable_provider_endpoints_for_mocking():
    config = read("backend/src/config.rs")
    routes = read("backend/src/auth/routes.rs")

    assert "google_oauth_token_url" in config
    assert "google_oauth_userinfo_url" in config
    assert "GOOGLE_OAUTH_TOKEN_URL" in config
    assert "GOOGLE_OAUTH_USERINFO_URL" in config
    assert ".post(&state.config.google_oauth_token_url)" in routes
    assert ".get(&state.config.google_oauth_userinfo_url)" in routes


def test_google_callback_uses_configurable_provider_endpoints():
    config = read("backend/src/config.rs")
    routes = read("backend/src/auth/routes.rs")

    assert "google_oauth_token_url" in config
    assert "GOOGLE_OAUTH_TOKEN_URL" in config
    assert "google_oauth_userinfo_url" in config
    assert "GOOGLE_OAUTH_USERINFO_URL" in config
    assert 'post(&state.config.google_oauth_token_url)' in routes
    assert 'get(&state.config.google_oauth_userinfo_url)' in routes
    assert '.post("https://oauth2.googleapis.com/token")' not in routes
    assert '.get("https://www.googleapis.com/oauth2/v2/userinfo")' not in routes


def test_google_callback_clears_transient_oauth_cookies_on_error():
    routes = read("backend/src/auth/routes.rs")

    assert "fn clear_oauth_cookies" in routes
    assert 'remove(Cookie::from("oauth_state"))' in routes
    assert 'remove(Cookie::from("oauth_pkce"))' in routes
    assert 'remove(Cookie::from("oauth_link"))' in routes
    assert "let jar = clear_oauth_cookies(jar);" in routes


def test_oauth_registration_log_omits_raw_email():
    service = read("backend/src/auth/service.rs")

    assert '"New OAuth user registered: {} ({}) via {}"' not in service
    assert '"New OAuth user registered"' in service
    assert "user_id = %user.id" in service
    assert "provider = provider" in service
