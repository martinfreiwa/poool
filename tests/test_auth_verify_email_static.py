from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def function_body(source: str, marker: str, end_marker: str) -> str:
    return source.split(marker, 1)[1].split(end_marker, 1)[0]


def test_verify_email_page_consumes_token_and_renders_explicit_states():
    routes = read("backend/src/auth/routes.rs")
    body = function_body(
        routes,
        "pub async fn verify_email_page(",
        "// ─── Form Handlers",
    )

    assert "Query(params): Query<std::collections::HashMap<String, String>>" in body
    assert 'params.get("token")' in body
    assert "service::verify_email(&state.db, token).await" in body
    assert 'Redirect::to("/auth/verify-email?verified=1")' in body
    assert 'Redirect::to("/auth/verify-email?error=invalid_token")' in body
    assert '"Email verified"' in body
    assert '"Verification link expired"' in body
    assert "render_verify_email(&state, status, title, message)" in body


def test_resend_requires_session_throttles_and_surfaces_failures():
    routes = read("backend/src/auth/routes.rs")
    body = function_body(
        routes,
        "pub async fn resend_verification_submit(",
        "async fn render_verify_email",
    )

    assert "headers: HeaderMap" in body
    assert 'resend_verification:ip:{}' in body
    assert 'resend_verification:user:{}' in body
    assert "service::get_user_by_session_unverified" in body
    assert "Your session expired. Please sign in to resend the verification email." in body
    assert "let Err(error) = service::create_email_verification_token" in body
    assert "auth_form_error_response(" in body
    assert 'class="auth-success-message"' in body
    assert "let _ = service::create_email_verification_token" not in body
    assert 'Verification email resent successfully!"' not in body


def test_resend_token_creation_replaces_old_tokens_and_queues_durable_email():
    service = read("backend/src/auth/service.rs")
    body = function_body(
        service,
        "pub async fn create_email_verification_token(",
        "/// Send an email verification message",
    )

    assert "let mut tx = pool.begin().await?" in body
    assert "DELETE FROM email_verification_tokens WHERE user_id = $1" in body
    assert "INSERT INTO email_verification_tokens" in body
    assert "tx.commit().await?" in body
    assert "transactional_email_outbox" in body
    assert "'verify_email'" in body
    assert "send_transactional_outbox_item(pool, id).await" in body
    assert "DELETE FROM email_verification_tokens WHERE token_hash = $1" not in body
    assert "return Err(error);" not in body


def test_verify_email_template_has_accessible_feedback_and_loading_state():
    html = read("frontend/platform/verify-email.html")
    css = read("frontend/platform/static/css/login.css")

    assert 'class="mail-icon mail-icon--{{ status | default(' in html
    assert 'id="auth-error" role="status" aria-live="polite" tabindex="-1"' in html
    assert 'class="auth-error-message" role="alert" aria-live="assertive"' in html
    assert 'class="auth-success-message" role="status" aria-live="polite"' in html
    assert 'hx-post="/auth/resend-verification"' in html
    assert 'hx-target="#auth-error"' in html
    assert 'aria-busy="false"' in html
    assert 'class="auth-loading htmx-indicator"' in html
    assert "document.body.addEventListener('htmx:beforeRequest'" in html
    assert "button.disabled = true;" in html
    assert "button.disabled = false;" in html
    assert "feedback.focus({ preventScroll: true });" in html

    assert ".auth-success-message" in css
    assert ".auth-primary-link" in css
    assert "#get-started-button:disabled" in css
    assert '#get-started-button[aria-busy="true"]' in css
