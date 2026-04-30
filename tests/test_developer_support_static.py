from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_support_js_avoids_persistent_sensitive_drafts():
    js = read("frontend/platform/static/js/support.js")

    assert "localStorage?.setItem" not in js
    assert 'removeItem("poool:support-ticket-draft")' in js
    assert "function saveDraft()" in js


def test_support_js_has_retryable_error_and_delegated_ticket_actions():
    js = read("frontend/platform/static/js/support.js")

    assert "function renderTicketsError" in js
    assert 'id="support-retry-btn"' in js
    assert "function bindTicketActions" in js
    assert "data-ticket-toggle" in js
    assert "window._submitReply" not in js
    assert "window._reopenTicket" not in js


def test_support_template_and_css_expose_accessible_upload_trigger():
    html = read("frontend/platform/support.html")
    css = read("frontend/platform/static/css/support.css")

    assert 'class="drop-zone-content"' in html
    assert 'for="ticket-attachment"' in html
    assert 'role="button"' in html
    assert 'tabindex="0"' in html
    assert ".drop-zone-content:focus-visible" in css
    assert ".ticket-card-header:focus-visible" in css


def test_support_backend_rate_limits_and_transactional_replies():
    handlers = read("backend/src/support/handlers.rs")
    db = read("backend/src/support/db.rs")

    assert "require_support_rate_limit" in handlers
    assert 'support:{}:{}' in handlers
    assert 'support:create' in handlers or '"create"' in handlers
    assert 'support:reply' in handlers or '"reply"' in handlers
    assert 'support:reopen' in handlers or '"reopen"' in handlers
    assert "StatusCode::TOO_MANY_REQUESTS" in handlers

    add_reply = db.split("pub async fn add_reply", 1)[1].split("/// Updates a ticket", 1)[0]
    assert "let mut tx = pool.begin().await?" in add_reply
    assert ".execute(&mut *tx)" in add_reply
    assert "tx.commit().await" in add_reply


def test_support_attachment_signature_and_transactional_metadata():
    service = read("backend/src/support/service.rs")
    db = read("backend/src/support/db.rs")

    assert "attachment_signature_matches" in service
    assert "bytes.starts_with" in service
    assert "Attachment upload is not configured" in service
    assert "upload_private" in service
    assert "uploaded_attachment" in service

    create_ticket = db.split("pub async fn create_ticket_v2", 1)[1].split("pub async fn add_initial_reply", 1)[0]
    assert "attachment: Option<(&str, &str)>" in create_ticket
    assert "support_ticket_attachments" in create_ticket
    assert ".execute(&mut *tx)" in create_ticket
