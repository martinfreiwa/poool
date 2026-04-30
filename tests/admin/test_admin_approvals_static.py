from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def body_between(source: str, start: str, end: str) -> str:
    start_index = source.index(start)
    end_index = source.index(end, start_index)
    return source[start_index:end_index]


def test_approval_approve_path_locks_row_checks_permissions_and_audits():
    approvals = read("backend/src/admin/approvals.rs")
    approve_body = body_between(
        approvals,
        "pub async fn api_admin_approvals_approve",
        "/// POST /api/admin/approvals/:id/reject",
    )

    assert 'require_permission(&state.db, "approvals.manage")' in approve_body
    assert "FOR UPDATE" in approve_body
    assert 'status != "pending"' in approve_body
    assert "Four-Eyes violation" in approve_body
    assert "admin.require_permission(&state.db, spec.permission)" in approve_body
    assert "execute_approved_action" in approve_body
    assert "rows_affected() != 1" in approve_body
    assert "approval_request.approved" in approve_body
    assert "tx.commit().await.map_err(ApiError::from)" in approve_body


def test_approval_create_and_reject_validate_action_contracts():
    approvals = read("backend/src/admin/approvals.rs")
    create_body = body_between(
        approvals,
        "pub async fn api_admin_approvals_create",
        "/// POST /api/admin/approvals/:id/approve",
    )
    reject_body = body_between(
        approvals,
        "pub async fn api_admin_approvals_reject",
        "/// Execute the actual business action",
    )

    assert "parse_action_contract(action_type, entity_type, entity_id)" in create_body
    assert "admin.require_permission(&state.db, spec.permission)" in create_body
    assert "approval_request.created" in create_body
    assert 'require_permission(&state.db, "approvals.manage")' in reject_body
    assert "FOR UPDATE" in reject_body
    assert "admin.require_permission(&state.db, spec.permission)" in reject_body
    assert "approval_request.rejected" in reject_body


def test_approval_executor_disables_unsupported_payout_and_uses_platform_settings_schema():
    approvals = read("backend/src/admin/approvals.rs")

    assert '"treasury.payout" => None' in approvals
    assert "treasury.payout is not enabled" in approvals
    assert "INSERT INTO platform_settings (key, value, value_type, updated_at, updated_by)" in approvals
    assert "ON CONFLICT (key)" in approvals


def test_approval_frontend_has_accessible_reject_modal_and_busy_buttons():
    js = read("frontend/platform/static/js/admin-approvals.js")
    html = read("frontend/platform/admin/approvals.html")

    assert "requestRejectionReason" in js
    assert 'role="dialog" aria-modal="true"' in js
    assert "textarea.focus()" in js
    assert "setCardBusy(id, true)" in js
    assert 'btn.setAttribute("aria-busy", busy ? "true" : "false")' in js
    assert "pooolConfirm" in js
    assert "approval-reject-modal" in html
