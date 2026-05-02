from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SECONDARY_HTML = ROOT / "frontend/platform/marketplace-secondary.html"
SECONDARY_JS = ROOT / "frontend/platform/static/js/marketplace-secondary.js"
SECONDARY_CSS = ROOT / "frontend/platform/static/css/marketplace-secondary.css"


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def test_secondary_market_removes_available_funded_status_toggle():
    template = read(SECONDARY_HTML)
    css = read(SECONDARY_CSS)

    assert "mp-sec__status-tabs" not in template
    assert "mp-sec__status-tab" not in template
    assert 'data-status="available"' not in template
    assert 'data-status="funded"' not in template
    assert ".mp-sec__status-tabs" not in css
    assert ".mp-sec__status-tab" not in css


def test_secondary_market_no_longer_filters_by_funding_status():
    source = read(SECONDARY_JS)

    assert "currentStatus" not in source
    assert "mp-sec__status-tab" not in source
    assert "rentStatus !== 'funded'" not in source
    assert "rentStatus === 'funded'" not in source

