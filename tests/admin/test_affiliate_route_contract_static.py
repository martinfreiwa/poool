from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text()


def test_affiliate_applications_page_requires_affiliates_manage():
    pages = read("backend/src/admin/pages.rs")

    assert 'relative == "admin/affiliate-applications"' in pages
    assert 'relative == "admin/affiliate-applications.html"' in pages
    assert '"affiliates.manage"' in pages


def test_affiliate_nav_items_are_permission_mapped():
    guard = read("frontend/platform/static/js/admin-permission-guard.js")

    assert '"nav-affiliate-apps": "affiliates.manage"' in guard
    assert '"nav-affiliate-finance": "affiliates.manage"' in guard
    assert '"nav-affiliate-fraud": "affiliates.manage"' in guard


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
