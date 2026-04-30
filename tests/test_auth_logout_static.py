from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_logout_get_is_non_mutating_and_post_deletes_session():
    routes = read("backend/src/auth/routes.rs")

    assert '.route("/logout", get(logout_page).post(logout))' in routes
    assert "pub async fn logout_page(" in routes
    assert 'method="post" action="/auth/logout"' in routes

    logout_page_body = routes.split("pub async fn logout_page(", 1)[1].split(
        "fn expired_session_cookie", 1
    )[0]
    assert "delete_session" not in logout_page_body

    logout_post_body = routes.split("pub async fn logout(", 1)[1].split(
        "// ─── OAuth Routes", 1
    )[0]
    assert "service::delete_session" in logout_post_body


def test_logout_cookie_expiry_matches_root_session_cookie_attributes():
    routes = read("backend/src/auth/routes.rs")
    helper = routes.split("fn expired_session_cookie()", 1)[1].split(
        "/// POST /logout", 1
    )[0]

    assert '.path("/")' in helper
    assert ".http_only(true)" in helper
    assert ".secure(cookie_is_secure())" in helper
    assert ".same_site(axum_extra::extract::cookie::SameSite::Lax)" in helper
    assert ".max_age(time::Duration::seconds(0))" in helper
    assert ".add(expired_session_cookie())" in routes
    assert ".remove(Cookie::from(SESSION_COOKIE))" not in routes


def test_platform_logout_alias_supports_post_and_get_interstitial():
    main = read("backend/src/main.rs")

    assert '"/logout",' in main
    assert "get(auth::routes::logout_page).post(auth::routes::logout)" in main


def test_shared_logout_controls_submit_post_with_csrf_fallback():
    profile = read("frontend/platform/static/js/profile-dropdown.js")
    mobile = read("frontend/platform/static/js/mobile-navigation.js")

    for source in (profile, mobile):
        assert 'form.method = "POST";' in source
        assert 'form.action = "/auth/logout";' in source
        assert 'input.name = "csrf_token";' in source
        assert 'window.location.href = "/logout";' in source

    assert 'window.submitPooolLogout = submitPooolLogout;' in profile
    assert "window.submitPooolLogout();" in mobile


def test_auth_e2e_logout_smoke_uses_post_and_checks_cookie_expiry():
    test_file = read("tests/test_auth_login_register.py")

    assert "GET /auth/logout renders POST interstitial" in test_file
    assert "s.post(" in test_file
    assert 'f"{BASE_URL}/auth/logout"' in test_file
    assert "Logout expires poool_session at Path=/" in test_file
    assert "Logout session expiry preserves HttpOnly and SameSite=Lax" in test_file
