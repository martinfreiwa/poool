from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text()


def test_primary_escrow_api_has_marketplace_permission_gate():
    source = read("backend/src/admin/primary_escrow.rs")
    handler = source[
        source.index("pub async fn api_admin_primary_escrow_list") :
        source.index("/// Core Abort & Auto-Refund Worker")
    ]

    assert "admin: AdminUser" in handler
    assert '"marketplace.view"' in handler
    assert '"marketplace.manage"' in handler
    assert '"marketplace.compliance"' in handler
    assert "Missing marketplace permission" in handler


def test_primary_escrow_release_uses_four_eyes_approval_contract():
    primary = read("backend/src/admin/primary_escrow.rs")
    approvals = read("backend/src/admin/approvals.rs")
    mod_rs = read("backend/src/admin/mod.rs")

    assert "api_admin_primary_escrow_release_request" in primary
    assert "admin.require_permission(pool, \"marketplace.manage\")" in primary
    assert "'primary_escrow.release'" in primary
    assert "admin_approval_requests" in primary
    assert "Release request created. A different administrator must approve it." in primary
    assert '"primary_escrow.release" => Some(ApprovalActionSpec' in approvals
    assert 'permission: "marketplace.manage"' in approvals
    assert "execute_primary_escrow_release(" in approvals
    assert '"/api/admin/primary-escrow/:asset_id/release-request"' in mod_rs


def test_primary_escrow_refund_worker_claims_expired_assets_once():
    source = read("backend/src/admin/primary_escrow.rs")
    worker = source[source.index("async fn process_expired_escrow_refunds") :]

    assert "FOR UPDATE SKIP LOCKED" in worker
    assert "UPDATE assets" in worker
    assert "AND funding_status IN ('funding_open', 'funding_in_progress')" in worker
    assert "FOR UPDATE" in worker
    assert "'primary_escrow.auto_refund'" in worker


def test_primary_escrow_page_renders_backend_data_safely():
    html = read("frontend/platform/admin/marketplace/primary-escrow.html")

    assert "insertAdjacentHTML" not in html
    assert ".innerHTML" not in html
    assert "textContent" in html
    assert "document.createElement" in html
    assert "encodeURIComponent(camp.asset_id" in html
    assert "role=\"status\"" in html
    assert "role=\"alert\"" in html
    assert "role', 'progressbar'" in html


def test_primary_escrow_release_control_is_not_fake_action():
    html = read("frontend/platform/admin/marketplace/primary-escrow.html")

    assert "Close & Release Escrow" not in html
    assert "alert(" not in html
    assert "Request Release" in html
    assert "notarization_reference" in html
    assert "requestRelease(camp" in html
    assert "/release-request" in html
    assert "csrfHeaders()" in html


def test_primary_escrow_nav_item_is_permission_mapped():
    guard = read("frontend/platform/static/js/admin-permission-guard.js")

    assert '"nav-mp-primary-escrow": "marketplace.view"' in guard
