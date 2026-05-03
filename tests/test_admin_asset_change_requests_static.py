from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
HTML = ROOT / "frontend/platform/admin/asset-change-requests.html"
JS = ROOT / "frontend/platform/static/js/admin-change-requests.js"
CSS = ROOT / "frontend/platform/static/css/admin-change-requests.css"


def test_change_requests_anomaly_banner_removed():
    html = HTML.read_text(encoding="utf-8")
    js = JS.read_text(encoding="utf-8")
    css = CSS.read_text(encoding="utf-8")

    assert 'id="anomaly-banner"' not in html
    assert 'id="anomaly-dismiss"' not in html
    assert 'id="anomaly-text"' not in html
    assert "detectAnomalies" not in js
    assert "anomalyDismissed" not in js
    assert ".cr-anomaly-banner" not in css


def test_change_requests_reset_uses_secondary_button_style():
    html = HTML.read_text(encoding="utf-8")

    reset_start = html.index('id="filter-reset"')
    reset_end = html.index("</button>", reset_start)
    reset_button = html[reset_start:reset_end]

    assert 'class="admin-btn admin-btn--secondary admin-btn--sm"' in reset_button
    assert "admin-btn--ghost" not in reset_button


def test_change_requests_empty_state_uses_custom_svg_icon():
    js = JS.read_text(encoding="utf-8")
    css = CSS.read_text(encoding="utf-8")

    empty_start = js.index('class="cr-empty-icon"')
    empty_end = js.index('<div class="cr-empty-title">No change requests</div>', empty_start)
    empty_icon = js[empty_start:empty_end]

    assert "📭" not in js
    assert "<svg" in empty_icon
    assert 'viewBox="0 0 48 48"' in empty_icon
    assert ".cr-empty-icon svg" in css
