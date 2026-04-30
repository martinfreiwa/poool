from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
KYC_JS = ROOT / "frontend/platform/static/js/kyc-page.js"
KYC_TEMPLATE = ROOT / "frontend/platform/kyc.html"
KYC_SERVICE = ROOT / "backend/src/kyc/service.rs"
KYC_ROUTES = ROOT / "backend/src/kyc/routes.rs"
STORAGE_ROUTES = ROOT / "backend/src/storage/routes.rs"


def read(path: Path) -> str:
    return path.read_text()


def test_kyc_xhr_upload_sends_csrf_header_before_send():
    js = read(KYC_JS)
    assert 'getCookie("csrf_token")' in js
    assert 'xhr.setRequestHeader("X-CSRF-Token", decodeURIComponent(csrfToken))' in js
    assert js.index('xhr.setRequestHeader("X-CSRF-Token"') < js.index("xhr.send(formData)")


def test_provider_return_does_not_trust_local_storage_status():
    js = read(KYC_JS)
    pending_block_start = js.index('const returnedFromProvider = localStorage.getItem("poool_kyc_pending") === "true";')
    fetch_start = js.index('const statusResp = await fetch("/api/kyc/status")')
    pending_block = js[pending_block_start:fetch_start]
    assert 'this.status = "in_review"' not in pending_block
    assert "return;" not in pending_block


def test_manual_submit_uses_backend_in_review_status():
    js = read(KYC_JS)
    assert 'this.status = data.status || "in_review";' in js
    assert 'Json(serde_json::json!({"status": "in_review"}))' in read(KYC_ROUTES)


def test_manual_kyc_requires_and_links_uploaded_document():
    service = read(KYC_SERVICE)
    assert 'Identity document is required.' in service
    assert "INSERT INTO kyc_records (user_id, status, provider, document_type)" in service
    assert "VALUES ($1, 'in_review', 'manual', $2)" in service
    assert "UPDATE kyc_documents" in service
    assert "AND user_id = $3" in service
    assert "AND status = 'pending'" in service
    assert "AND kyc_record_id IS NULL" in service
    assert "linked.rows_affected() != 1" in service


def test_manual_kyc_persists_identity_fields_and_audit():
    service = read(KYC_SERVICE)
    for field in (
        "date_of_birth",
        "nationality",
        "address_line_1",
        "city",
        "country",
    ):
        assert field in service
    assert "'kyc.submitted'" in service
    assert "'kyc.initiated'" in service


def test_kyc_mutations_have_rate_limit_keys():
    routes = read(KYC_ROUTES)
    storage = read(STORAGE_ROUTES)
    assert '"kyc:{}:{}"' in routes
    assert 'require_kyc_rate_limit(&state, user_id, "submit")' in routes
    assert 'require_kyc_rate_limit(&state, user.id, "initiate")' in routes
    assert '"kyc:upload:{}"' in storage


def test_document_type_and_accept_contract_matches_backend():
    template = read(KYC_TEMPLATE)
    storage = read(STORAGE_ROUTES)
    service = read(KYC_SERVICE)
    assert 'value="driving_licence"' in template
    assert 'value="driving_license"' not in template
    assert 'accept="image/jpeg,image/png,image/webp,application/pdf"' in template
    assert "PNG, JPG, WebP or PDF" in template
    assert 'normalize_document_type(Some(&document_type))' in storage
    assert '"driving_licence" | "driving_license" => "driving_licence"' in service
