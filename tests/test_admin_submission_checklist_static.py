from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_submission_approval_checklist_uses_publication_readiness_checks():
    template = (ROOT / "frontend/platform/admin/developer-submission-review.html").read_text()
    script = (ROOT / "frontend/platform/static/js/admin-submission-review.js").read_text()
    backend = (ROOT / "backend/src/admin/developer_projects.rs").read_text()

    expected = [
        "chk-kyc",
        "chk-legal",
        "chk-investor-docs",
        "chk-financials",
        "chk-math",
        "chk-media",
        "chk-property-content",
        "chk-risk",
    ]
    removed = ["chk-tax", "chk-spv", "chk-loc", "chk-video", "chk-gmap", "chk-fields"]

    for check_id in expected:
        assert f'id="{check_id}"' in template
        assert f'"{check_id}"' in backend

    for check_id in removed:
        assert f'id="{check_id}"' not in template
        assert f'"{check_id}"' not in backend

    assert "Investor-visible documents selected" in template
    assert "Property page content complete" in template
    assert "Risk disclosures and admin notes reviewed" in template
    assert "chk-investor-docs" in script
    assert "chk-financials" in script
    assert "chk-media" in script
