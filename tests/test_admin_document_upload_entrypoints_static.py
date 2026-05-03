from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_document_upload_is_available_from_missing_document_sections():
    template = (ROOT / "frontend/platform/admin/developer-submission-review.html").read_text()
    script = (ROOT / "frontend/platform/static/js/admin-submission-review.js").read_text()

    assert "document-missing-row" in template
    assert "toggle-document-upload-btn" in template
    assert "admin-document-upload-panel" in template
    assert "admin-document-file" in template

    assert "function adminOpenDocumentUpload" in script
    assert "Upload document" in script
    assert "Upload ${catName.toLowerCase()} document" in script
    assert "adminOpenDocumentUpload('${esc(defaultType)}')" in script
    assert 'document.getElementById("admin-document-visible")' in script
    assert 'formData.append("file", file)' in script
