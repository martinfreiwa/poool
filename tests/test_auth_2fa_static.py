from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_2fa_template_has_accessible_code_input_and_error_region():
    html = read("frontend/platform/auth-2fa.html")

    assert 'id="auth-error" aria-live="polite"' in html
    assert 'label for="code-input"' in html
    assert 'id="code-input"' in html
    assert 'autocomplete="one-time-code"' in html
    assert 'id="loading-indicator"' in html
    assert 'role="status"' in html


def test_2fa_template_handles_htmx_error_and_loading_states_locally():
    html = read("frontend/platform/auth-2fa.html")

    assert 'hx-ext="response-targets"' not in html
    assert 'hx-target-400' not in html
    assert "htmx:beforeSwap" in html
    assert "evt.detail.shouldSwap = true" in html
    assert "htmx:beforeRequest" in html
    assert "button.disabled = true" in html
    assert "htmx:afterRequest" in html
    assert "button.disabled = false" in html


def test_2fa_route_uses_dedicated_rate_limit_buckets_and_html_errors():
    routes = read("backend/src/auth/routes.rs")

    assert ".check_dual(" in routes
    assert '&format!("2fa:ip:{}"' in routes
    assert '&format!("2fa:user:{}"' in routes
    assert 'Invalid authentication code.' in routes
    assert "return Ok(login_error_response(" in routes
    assert 'HeaderValue::from_static("/marketplace")' in routes
