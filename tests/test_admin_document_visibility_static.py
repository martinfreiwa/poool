from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_asset_document_visibility_is_admin_controlled_end_to_end():
    migration = (ROOT / "database/118_asset_document_visibility.sql").read_text()
    initial_schema = (ROOT / "database/001_initial_schema.sql").read_text()
    admin_assets = (ROOT / "backend/src/admin/assets.rs").read_text()
    admin_review = (ROOT / "backend/src/admin/developer_projects.rs").read_text()
    admin_routes = (ROOT / "backend/src/admin/mod.rs").read_text()
    property_routes = (ROOT / "backend/src/assets/routes.rs").read_text()
    storage_routes = (ROOT / "backend/src/storage/routes.rs").read_text()
    frontend = (ROOT / "frontend/platform/static/js/admin-submission-review.js").read_text()

    assert "ADD COLUMN IF NOT EXISTS is_investor_visible" in migration
    assert "is_investor_visible BOOLEAN NOT NULL DEFAULT FALSE" in initial_schema
    assert "is_investor_visible: Option<bool>" in admin_assets
    assert "is_investor_visible = " in admin_assets
    assert "patch(api_admin_asset_document_update)" in admin_routes
    assert ".post(api_admin_asset_document_update)" in admin_routes
    assert '"is_investor_visible": d.6' in admin_review
    assert "AND is_investor_visible = TRUE" in property_routes
    assert "asset_published && is_investor_visible" in storage_routes
    assert "adminToggleDocumentVisibility" in frontend
    assert "adminUpdateDocumentRequest" in frontend
    assert 'res.status === 405 ? request("POST")' in frontend
    assert "Visible on property page" in frontend
