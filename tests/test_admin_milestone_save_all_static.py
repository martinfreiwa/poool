from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_admin_milestones_have_save_all_flow():
    template = (ROOT / "frontend/platform/admin/components/property-page-editor.html").read_text()
    script = (ROOT / "frontend/platform/static/js/admin-property-page-editor.js").read_text()

    assert 'id="btn-milestone-save-all"' in template
    assert 'id="milestone-save-status"' in template
    assert "Save milestones" in template

    assert "_wireMilestoneSaveAll()" in script
    assert "async _saveAllMilestones()" in script
    assert "async _saveMilestoneRow" in script
    assert 'row.dataset.dirty = "true"' in script
    assert 'row.dataset.newMilestone === "true" || row.dataset.dirty === "true"' in script
    assert "PATCH" in script
    assert "POST" in script
