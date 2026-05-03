from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_developer_review_card_loads_avatar_and_stays_compact():
    template = (ROOT / "frontend/platform/admin/developer-submission-review.html").read_text()
    script = (ROOT / "frontend/platform/static/js/admin-submission-review.js").read_text()
    backend = (ROOT / "backend/src/admin/developer_projects.rs").read_text()

    assert "developer_avatar_url" in backend
    assert "developer_profile_logo_url" in backend
    assert "avatar_url" in backend
    assert "logo_url" in backend

    assert ".dev-profile-row" in template
    assert "grid-template-columns: 40px minmax(0, 1fr) auto" in template
    assert "padding: 12px 16px" in template
    assert "margin-bottom: 16px" in template

    assert "dev.avatar_url || dev.logo_url || dev.asset_developer_logo_url" in script
    assert "dev-profile-avatar" in script
    assert "onerror=\"this.style.display='none'" in script
