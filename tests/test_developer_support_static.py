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


def test_support_submit_button_uses_primary_design_contract():
    html = read("frontend/platform/support.html")
    css = read("frontend/platform/static/css/support.css")

    assert 'id="submit-ticket-btn"' in html
    assert 'class="ds-btn ds-btn--primary support-submit-btn"' in html
    assert "body#support-body #submit-ticket-btn.support-submit-btn" in css
    assert "background: var(--btn-primary-bg, #0000FF) !important;" in css
    assert "color: var(--btn-primary-color, #98FB96) !important;" in css
    assert "min-height: 44px !important;" in css
    assert "justify-content: center !important;" in css
    assert "letter-spacing: 0 !important;" in css
    assert "body#support-body #submit-ticket-btn.support-submit-btn svg" in css
    assert "stroke: currentColor;" in css
    submit_block = css.split("/* ── Primary submit", 1)[1].split("/* ── Ghost", 1)[0]
    assert "color: #FFFFFF" not in submit_block
    assert "letter-spacing: -0.01em" not in submit_block


def test_support_card_headers_and_dropdowns_use_shared_ui_patterns():
    html = read("frontend/platform/support.html")
    css = read("frontend/platform/static/css/support.css")

    assert '<div class="support-card-title-group">' in html
    assert "Send us the details and we will route your request to the right team." in html
    assert "Track open, pending, and resolved support conversations." in html
    assert 'id="ticket-category" class="ds-select form-select support-select"' in html
    assert 'id="ticket-priority" class="ds-select form-select support-select"' in html
    assert ".support-card-title-group" in css
    assert ".support-card-header p" in css
    assert "padding-left: 48px;" in css
    assert "#submit-ticket.support-form-card" in css
    assert "overflow: visible;" in css
    assert ".support-form-card .support-form-group > .poool-dropdown .poool-dropdown__trigger" in css
    assert ".support-form-card .support-form-group > .poool-dropdown .poool-dropdown__panel" in css
    assert ".support-form-card .support-form-group > .poool-dropdown .poool-dropdown__option--selected" in css
    assert ".support-form-card .support-form-group > .poool-dropdown .poool-dropdown__check" in css


def test_support_knowledge_base_has_subtitle_and_opens_by_default():
    html = read("frontend/platform/support.html")
    css = read("frontend/platform/static/css/support.css")

    assert '<details class="ds-card support-form-card support-faq-card support-faq-disclosure" id="faq" open>' in html
    assert '<span class="support-faq-summary__title">Knowledge base</span>' in html
    assert "Find quick answers before opening a new support conversation." in html
    assert ".support-faq-summary__title" in css
    assert ".support-faq-summary p" in css
    assert "font-size: 13px;" in css
    assert "font-weight: 400;" in css


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
