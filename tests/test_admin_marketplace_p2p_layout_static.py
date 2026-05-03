from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
P2P_HTML = ROOT / "frontend/platform/admin/marketplace/p2p.html"
ADMIN_MP_CSS = ROOT / "frontend/platform/static/css/admin-marketplace.css"


def test_p2p_kpi_cards_use_custom_svg_icons():
    html = P2P_HTML.read_text(encoding="utf-8")
    css = ADMIN_MP_CSS.read_text(encoding="utf-8")

    for emoji in ("📋", "⏳", "⚠️", "💰"):
        assert emoji not in html

    assert html.count('class="mp-p2p-kpi-icon') == 4
    assert html.count("<svg") >= 8

    for selector in (
        ".mp-p2p-kpi-icon",
        ".mp-p2p-kpi-icon--warning",
        ".mp-p2p-kpi-icon--danger",
        ".mp-p2p-kpi-icon--blue",
    ):
        assert selector in css


def test_p2p_filter_toolbar_uses_compact_controls():
    html = P2P_HTML.read_text(encoding="utf-8")
    css = ADMIN_MP_CSS.read_text(encoding="utf-8")

    toolbar_start = html.index('class="mp-filter-bar mp-p2p-toolbar"')
    toolbar_end = html.index('id="p2p-bulkbar"', toolbar_start)
    toolbar = html[toolbar_start:toolbar_end]

    assert 'class="admin-search mp-p2p-search"' in toolbar
    assert 'class="admin-select mp-p2p-filter-status"' in toolbar
    assert 'class="admin-select mp-p2p-filter-side"' in toolbar
    assert 'class="admin-select mp-p2p-filter-asset"' in toolbar
    assert 'class="admin-select mp-p2p-filter-range"' in toolbar
    assert 'class="mp-p2p-toolbar-actions"' in toolbar
    assert 'class="admin-input mp-p2p-threshold-input"' in toolbar
    assert 'class="mp-filter-label"' in toolbar
    assert 'style="max-width:280px;"' not in toolbar
    assert 'style="width:80px;"' not in toolbar
    assert 'style="gap:6px;"' not in toolbar

    for selector in (
        ".mp-p2p-toolbar",
        ".mp-p2p-search",
        ".mp-p2p-filter-status",
        ".mp-p2p-filter-side",
        ".mp-p2p-filter-asset",
        ".mp-p2p-filter-range",
        ".mp-p2p-toolbar-actions",
        ".mp-p2p-threshold-input",
    ):
        assert selector in css

    assert "max-width: 320px" in css
    assert "width: 172px" in css
    assert "width: 132px" in css
    assert "width: 140px" in css
