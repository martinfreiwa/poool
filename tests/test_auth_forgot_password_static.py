from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_forgot_password_template_exposes_accessible_htmx_states():
    template = (ROOT / "frontend/platform/forgot-password.html").read_text()

    assert 'id="auth-error" role="alert" aria-live="assertive" tabindex="-1"' in template
    assert 'hx-post="/auth/forgot-password"' in template
    assert 'aria-busy="false"' in template
    assert "btn.setAttribute('aria-busy', 'true')" in template
    assert "forgot-password-success" in template
    assert "success.focus({ preventScroll: true })" in template


def test_forgot_password_backend_uses_html_errors_and_delivery_guard():
    routes = (ROOT / "backend/src/auth/routes.rs").read_text()
    service = (ROOT / "backend/src/auth/service.rs").read_text()
    email = (ROOT / "backend/src/common/email.rs").read_text()
    csrf = (ROOT / "backend/src/auth/csrf.rs").read_text()
    app = (ROOT / "backend/src/lib.rs").read_text()
    migration = (ROOT / "database/091_password_reset_email_outbox.sql").read_text()

    assert "wait_for_password_reset_response_floor" in routes
    assert 'state.config.app_env.eq_ignore_ascii_case("production")' in routes
    assert "email::resend_configured()" in routes
    assert "auth_form_error_response" in routes
    assert "render_auth_error_html" in routes
    assert "validation::validate_email(&email)" in service
    assert "password_reset_email_outbox" in service
    assert "send_password_reset_outbox_item(pool, outbox_id).await" in service
    assert "pub fn resend_configured()" in email
    assert "process_password_reset_outbox" in email
    assert "prt.expires_at > NOW()" in email
    assert 'env.eq_ignore_ascii_case("production")' in email
    assert "Security check failed. Please refresh the page and try again." in csrf
    assert "run_transactional_email_outbox_worker" in app
    assert "CREATE TABLE IF NOT EXISTS password_reset_email_outbox" in migration
