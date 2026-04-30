from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ROUTES = ROOT / "backend/src/developer/routes.rs"
TEMPLATE = ROOT / "frontend/platform/developer/submissions.html"
JS = ROOT / "frontend/platform/static/js/developer-submissions.js"


def _handler_body(source: str, name: str) -> str:
    marker = f"pub async fn {name}"
    start = source.index(marker)
    next_marker = source.find("\n/// ", start + len(marker))
    if next_marker == -1:
        return source[start:]
    return source[start:next_marker]


def _js_function_body(source: str, signature: str) -> str:
    start = source.index(signature)
    next_marker = source.find("\nasync function ", start + len(signature))
    if next_marker == -1:
        next_marker = source.find("\nfunction ", start + len(signature))
    if next_marker == -1:
        return source[start:]
    return source[start:next_marker]


def test_developer_submission_mutations_require_developer_api_gate():
    source = ROUTES.read_text()
    for handler in (
        "api_developer_submit_draft",
        "api_developer_duplicate_draft",
        "api_developer_delete_draft",
    ):
        body = _handler_body(source, handler)
        assert "require_developer_api(&jar, &state).await?" in body
        assert "middleware::get_current_user(&jar, &state.db).await" not in body


def test_developer_submission_mutations_fail_on_missing_project_rows():
    source = ROUTES.read_text()
    submit = _handler_body(source, "api_developer_submit_draft")
    duplicate = _handler_body(source, "api_developer_duplicate_draft")
    delete = _handler_body(source, "api_developer_delete_draft")

    assert "Developer project record missing" in submit
    assert "project_update.rows_affected() != 1" in submit
    assert "SELECT project_name FROM developer_projects WHERE asset_id = $1 LIMIT 1" in duplicate
    assert "Developer project record missing" in duplicate
    assert "project_insert.rows_affected() != 1" in duplicate
    assert "Developer project record missing" in delete
    assert "unwrap_or_else(|| \"draft\".to_string())" not in delete


def test_developer_submissions_uses_shared_accessible_confirmation():
    template = TEMPLATE.read_text()
    script = JS.read_text()

    assert "'poool-confirm'" in template
    assert "window.pooolConfirm" in script
    assert "async function confirmBulkDelete()" in script
    assert "async function confirmDelete(assetId, title)" in script
    assert "document.createElement(\"div\")" not in _js_function_body(script, "async function confirmDelete")
    assert "sub-modal-overlay" not in _js_function_body(script, "async function confirmBulkDelete")
