from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_developer_asset_milestones_and_orders_use_branded_empty_states():
    js = (ROOT / "frontend/platform/static/js/developer-asset-detail.js").read_text(encoding="utf-8")
    css = (ROOT / "frontend/platform/static/css/developer-asset-detail.css").read_text(encoding="utf-8")

    assert "function createBrandedEmptyState" in js
    assert "function createEmptyStateIcon" in js
    assert "No milestones yet" in js
    assert "Project roadmap updates will appear here once milestones are added." in js
    assert "No orders yet" in js
    assert "Investor orders for this asset will appear here once activity starts." in js
    assert "No investors yet" in js
    assert "Ownership details will appear here after investors join this asset." in js
    assert "icon: \"investors\"" in js
    assert "cell.className = \"ad-empty-cell\"" in js
    assert "No milestones defined" not in js
    assert "No orders found" not in js

    for selector in (
        ".ad-branded-empty",
        ".ad-branded-empty::before",
        ".ad-branded-empty__mark",
        ".ad-branded-empty__icon",
        ".ad-branded-empty__title",
        ".ad-branded-empty__text",
        ".ad-empty-cell",
    ):
        assert selector in css

    assert "POOOL" in js
    assert "linear-gradient(90deg, var(--btn-primary-bg, #0000FF), #98FB96)" in css
