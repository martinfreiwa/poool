from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_developer_asset_documents_use_title_and_icon_download_links():
    js = (ROOT / "frontend/platform/static/js/developer-asset-detail.js").read_text(encoding="utf-8")
    css = (ROOT / "frontend/platform/static/css/developer-asset-detail.css").read_text(encoding="utf-8")

    assert "function createDocumentItem" in js
    assert "function documentDownloadUrl" in js
    assert "function documentTitle" in js
    assert "title.textContent = documentTitle(doc)" in js
    assert "details.textContent = `${formatDocumentType(doc.document_type)} · ${formatFileSize(doc.file_size)}`" in js
    assert "`/api/documents/${encodeURIComponent(doc.id)}/download`" in js
    assert "action.className = \"document-icon-action\"" in js
    assert "action.setAttribute(\"aria-label\", `View ${documentTitle(doc)}`)" in js
    assert "document.getElementById(\"documents-list\").innerHTML = docs" not in js
    assert "document-type-badge" not in js
    assert "View</a>" not in js

    for selector in (
        ".document-item__meta",
        ".document-title",
        ".document-details",
        ".document-icon-action",
        ".document-icon-action svg",
    ):
        assert selector in css


def test_developer_asset_detail_api_exposes_document_ids_and_titles():
    routes = (ROOT / "backend/src/developer/routes.rs").read_text(encoding="utf-8")

    assert "SELECT document_type, COALESCE(title, document_type), file_size_bytes, id FROM asset_documents" in routes
    assert '"documents": docs.iter().map(|d| serde_json::json!({"document_type": d.0, "title": d.1, "file_size": d.2, "id": d.3}))' in routes
