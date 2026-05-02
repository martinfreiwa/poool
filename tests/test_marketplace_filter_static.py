from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MARKETPLACE_HTML = ROOT / "frontend/platform/marketplace.html"
MARKETPLACE_CSS = ROOT / "frontend/platform/static/css/marketplace.css"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def test_marketplace_filter_labels_do_not_render_info_icons():
    template = read(MARKETPLACE_HTML)

    assert 'id="filter-bar-investment-label"' in template
    assert 'id="filter-bar-property-label"' in template
    assert "filter-bar-investment-help" not in template
    assert "filter-bar-property-help" not in template


def test_marketplace_search_input_is_contained_by_wrapper():
    css = read(MARKETPLACE_CSS)

    assert ".search-input-wrapper {" in css
    assert "min-width: 0;" in css
    assert "overflow: hidden;" in css
    assert ".search-input {" in css
    assert "width: auto;" in css
    assert "max-width: 100%;" in css
    assert "flex: 1 1 auto;" in css
    assert ".search-input:focus {" in css
