from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text()


def test_affiliate_applications_page_requires_affiliates_manage():
    pages = read("backend/src/admin/pages.rs")

    assert 'relative == "admin/affiliate-applications"' in pages
    assert 'relative == "admin/affiliate-applications.html"' in pages
    assert '"affiliates.manage"' in pages


def test_affiliate_fraud_clean_route_is_canonical_and_permissioned():
    pages = read("backend/src/admin/pages.rs")
    sidebar = read("frontend/platform/static/js/admin-sidebar-loader.js")
    clean_template = REPO_ROOT / "frontend/platform/admin/affiliate-fraud.html"
    legacy_template = read("frontend/platform/admin/admin-affiliate-fraud.html")

    assert clean_template.exists()
    assert "{% include 'admin/admin-affiliate-fraud.html' %}" in clean_template.read_text()
    assert 'relative == "admin/affiliate-fraud"' in pages
    assert 'relative == "admin/affiliate-fraud.html"' in pages
    assert 'relative == "admin/admin-affiliate-fraud"' in pages
    assert 'relative == "admin/admin-affiliate-fraud.html"' in pages
    assert '"affiliates.manage"' in pages
    assert 'href="/admin/affiliate-fraud.html"' in sidebar
    assert '"/admin/admin-affiliate-fraud.html"' in sidebar
    assert 'onclick=' not in legacy_template
    assert "Freeze Node" not in legacy_template
    assert 'role="status"' in legacy_template
    assert "data-scan-type=\"ip_overlap\"" in legacy_template


def test_affiliate_nav_items_are_permission_mapped():
    guard = read("frontend/platform/static/js/admin-permission-guard.js")

    assert '"nav-affiliate-apps": "affiliates.manage"' in guard
    assert '"nav-affiliate-finance": "affiliates.manage"' in guard
    assert '"nav-affiliate-fraud": "affiliates.manage"' in guard


def test_affiliate_fraud_scan_api_is_permissioned_and_returns_graph_elements():
    backend = read("backend/src/admin/rewards.rs")
    service = read("backend/src/rewards/service.rs")

    assert "Query(query): Query<AffiliateFraudScanQuery>" in backend
    assert 'require_permission(&state.db, "affiliates.manage")' in backend
    assert '"Unsupported fraud scan type"' in backend
    assert "scan_affiliate_ip_overlaps" in backend
    assert "affiliate_fraud_flags_to_cytoscape_elements" in backend
    assert '"elements": elements' in backend
    assert "'affiliate_fraud.scan_viewed'" in backend
    assert "pub async fn scan_affiliate_ip_overlaps" in service
    assert "JOIN affiliates a ON a.referral_code = rc.code" in service
    assert "HAVING COUNT(DISTINCT a.user_id) > 1" in service
    assert "pub fn affiliate_fraud_flags_to_cytoscape_elements" in service
    assert '"label": "RING"' in service
    assert '"label": "IP Overlap"' in service


def test_affiliate_frontend_validation_matches_backend_contract():
    js = read("frontend/platform/admin/js/admin-affiliate-applications.js")
    html = read("frontend/platform/admin/affiliate-applications.html")
    backend = read("backend/src/admin/rewards.rs")

    assert "REFERRAL_CODE_PATTERN = /^[A-Z0-9_-]{3,20}$/" in js
    assert 'pattern="[A-Z0-9_-]{3,20}"' in html
    assert 'maxlength="20"' in html
    assert "AFFILIATE_REJECTION_REASON_MAX_CHARS: usize = 1000" in backend
    assert "REJECTION_REASON_MAX_LENGTH = 1000" in js
    assert 'maxlength="1000"' in html


def test_pending_applications_response_schema_is_validated():
    js = read("frontend/platform/admin/js/admin-affiliate-applications.js")

    assert "isValidPendingResponse(data)" in js
    assert "Array.isArray(data.pending)" in js
    assert "Number.isInteger(data.counts[key])" in js
    assert "Unexpected affiliate applications response." in js


def test_affiliate_finance_payout_rows_do_not_embed_profile_data_in_inline_js():
    js = read("frontend/platform/admin/js/admin-affiliate-finance.js")
    html = read("frontend/platform/admin/affiliate-finance.html")

    assert 'onclick="openPayoutModal' not in js
    assert "onclick=" not in html
    assert "release-payout-btn" in js
    assert "data-affiliate-name" in js
    assert 'tbody.addEventListener("click"' in js
    assert "button.dataset.affiliateName" in js
    assert 'role="dialog"' in html
    assert 'aria-modal="true"' in html
    assert 'event.key === "Escape"' in js
    assert 'event.key !== "Tab"' in js
    assert "previousFocus.focus()" in js
    assert 'event.target.id === "payout-modal"' in js


def test_affiliate_finance_page_and_pending_api_expose_tax_gate():
    pages = read("backend/src/admin/pages.rs")
    backend = read("backend/src/admin/rewards.rs")
    js = read("frontend/platform/admin/js/admin-affiliate-finance.js")

    assert 'relative == "admin/affiliate-finance"' in pages
    assert 'relative == "admin/affiliate-finance.html"' in pages
    assert '"affiliates.manage"' in pages
    assert "tax_document_gcs_path IS NOT NULL as tax_document_uploaded" in backend
    assert '"tax_document_uploaded": tax_document_uploaded' in backend
    assert '"payout_blocked_reason"' in backend
    assert "p.tax_document_uploaded !== true" in js
    assert "Tax Required" in js


def test_affiliate_batch_payout_updates_only_locked_commission_ids_and_closes_requests():
    backend = read("backend/src/admin/rewards.rs")

    assert "let commission_ids: Vec<uuid::Uuid> = commissions.iter().map(|c| c.id).collect();" in backend
    assert "WHERE id = ANY($2)" in backend
    assert "updated_commissions.rows_affected() != commissions.len() as u64" in backend
    assert "Payout commission set changed while processing batch" in backend
    assert "UPDATE affiliate_payout_requests" in backend
    assert "payout_batch_id = $2" in backend


def test_affiliate_finance_notification_action_is_wired():
    js = read("frontend/platform/admin/js/admin-affiliate-finance.js")

    assert 'document.querySelector(".admin-notification-btn")?.addEventListener("click"' in js
    assert 'window.location.href = "/admin/notifications"' in js


def test_affiliate_materials_downloads_and_guidelines_are_wired():
    html = read("frontend/platform/affiliate-materials.html")
    js_path = REPO_ROOT / "frontend/platform/static/js/affiliate-materials.js"
    guidelines_path = (
        REPO_ROOT / "frontend/platform/static/docs/POOOL-Affiliate-Brand-Guidelines.pdf"
    )

    assert js_path.exists()
    assert guidelines_path.exists()
    assert "/static/docs/POOOL-Affiliate-Brand-Guidelines.pdf" in html
    assert 'data-material-download="banner"' in html
    assert 'data-material-download="instagram"' in html
    assert 'data-material-download="story"' in html
    assert "data-download-all" in html
    assert "POOOL logo preview" in html


def test_affiliate_materials_page_locks_non_active_users_and_exposes_upload_status():
    html = read("frontend/platform/affiliate-materials.html")
    routes = read("backend/src/rewards/routes.rs")
    router = read("backend/src/rewards/mod.rs")

    assert "affiliate_status == 'active'" in html
    assert "affiliate-materials-locked-title" in html
    assert "affiliate-material-upload-form" in html
    assert "affiliate-materials-status-body" in html
    assert '"/api/affiliate/materials"' in router
    assert "api_affiliate_materials_list" in routes
    assert "require_active_affiliate_user_id" in routes


def test_affiliate_material_upload_validates_file_types_before_storage():
    routes = read("backend/src/rewards/routes.rs")

    assert "validate_affiliate_material_upload(" in routes
    assert "AFFILIATE_MATERIAL_MAX_BYTES" in routes
    assert "Unsupported file type. Upload PNG, JPG, SVG, PDF, MP4, or ZIP." in routes
    assert "Declared content type does not match the uploaded file." in routes
    assert "content_type" in routes
    assert '"application/octet-stream"' not in routes[
        routes.index("pub async fn api_affiliate_upload_material") :
    ]


def test_affiliate_referrals_assets_are_real_and_template_has_no_inline_handlers():
    html = read("frontend/platform/affiliate-referrals.html")

    assert (REPO_ROOT / "frontend/platform/static/js/affiliate-referrals.js").exists()
    assert (REPO_ROOT / "frontend/platform/static/css/affiliate-referrals.css").exists()
    assert "onclick=" not in html
    assert "onkeyup=" not in html
    assert "<script>" not in html
    assert "data-referral-filter" in html
    assert 'role="tablist"' in html
    assert 'aria-live="polite"' in html


def test_affiliate_referrals_frontend_renders_rows_safely_and_exports_server_csv():
    js = read("frontend/platform/static/js/affiliate-referrals.js")

    assert ".innerHTML" not in js
    assert "textContent" in js
    assert "document.createElement" in js
    assert 'const EXPORT_API = "/api/affiliate/commissions/export?format=csv&limit=200"' in js
    assert 'headers: { Accept: "text/csv" }' in js
    assert "Failed to load affiliate referrals" in js
    assert "Only active affiliates can view referral details." in js


def test_affiliate_referrals_backend_gates_page_and_export_safely():
    routes = read("backend/src/rewards/routes.rs")

    assert "pub async fn page_affiliate_referrals" in routes
    assert "is_active_affiliate(&state, user.id)" in routes
    assert 'Redirect::to("/affiliate/onboarding")' in routes
    assert "unwrap_or_default();" not in routes[routes.index("pub async fn api_affiliate_commissions_export"):routes.index("/// POST /api/affiliate/postback")]
    assert "amount_cents as f64" not in routes
    assert "format_cents_decimal(amount_cents)" in routes
    assert "csv_escape(&sub_id)" in routes
    assert "(total + limit - 1) / limit" in routes
