from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]


def read(path: str) -> str:
    return (REPO_ROOT / path).read_text()


def test_blockchain_sync_page_and_nav_require_treasury_read():
    pages = read("backend/src/admin/pages.rs")
    guard = read("frontend/platform/static/js/admin-permission-guard.js")

    assert 'relative == "admin/blockchain-sync"' in pages
    assert 'relative == "admin/blockchain-sync.html"' in pages
    assert '"treasury.read"' in pages
    assert '"nav-blockchain-sync": "treasury.read"' in guard


def test_blockchain_sync_api_requires_read_and_manage_permissions():
    backend = read("backend/src/admin/blockchain.rs")

    assert 'const BLOCKCHAIN_READ_PERMISSION: &str = "treasury.read";' in backend
    assert ".require_permission(pool, BLOCKCHAIN_READ_PERMISSION)" in backend
    assert "admin\n        .require_permission(pool, BLOCKCHAIN_CONTROL_PERMISSION)" in backend


def test_force_kyc_sync_rechecks_eligibility_and_audits_atomically():
    backend = read("backend/src/admin/blockchain.rs")

    assert "FOR UPDATE" in backend
    assert "k.status = 'approved'" in backend
    assert 'user.2 != "active"' in backend
    assert "pool\n        .begin()" in backend
    assert ".execute(&mut *tx)" in backend
    assert "Audit log failed" in backend
    assert "tx.commit()" in backend


def test_blockchain_sync_status_does_not_mask_core_db_failures():
    backend = read("backend/src/admin/blockchain.rs")
    sync_handler = backend.split("pub async fn api_admin_blockchain_sync_status", 1)[1].split(
        "pub async fn api_admin_blockchain_force_kyc_sync", 1
    )[0]

    assert ".fetch_one(pool)\n        .await\n        .unwrap_or" not in sync_handler
    assert ".fetch_all(pool)\n    .await\n    .unwrap_or_default()" not in sync_handler
    assert ".unwrap_or_default()" not in sync_handler
    assert ".ok()\n    .flatten()" not in sync_handler
    assert "Failed to read whitelist queue" in sync_handler


def test_blockchain_sync_frontend_empty_badge_and_no_cdn_htmx():
    js = read("frontend/platform/static/js/admin-blockchain-sync.js")
    html = read("frontend/platform/admin/blockchain-sync.html")

    assert 'badge.textContent = "0 pending"' in js
    assert "buildWhitelistRow" in js
    assert "addEventListener(\"click\"" in js
    assert "unpkg.com/htmx" not in html
