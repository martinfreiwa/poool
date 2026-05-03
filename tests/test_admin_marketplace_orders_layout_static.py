from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ORDERS_HTML = ROOT / "frontend/platform/admin/marketplace/orders.html"
ADMIN_MP_CSS = ROOT / "frontend/platform/static/css/admin-marketplace.css"


def test_open_orders_filter_bar_uses_compact_controls():
    html = ORDERS_HTML.read_text(encoding="utf-8")
    css = ADMIN_MP_CSS.read_text(encoding="utf-8")

    filter_start = html.index('class="mp-filter-bar mp-orders-filter-bar"')
    filter_end = html.index("<!-- View Options Toolbar -->", filter_start)
    filter_bar = html[filter_start:filter_end]

    assert 'class="admin-input mp-orders-filter-search"' in filter_bar
    assert 'class="admin-select mp-orders-filter-side"' in filter_bar
    assert 'class="admin-select mp-orders-filter-page-size"' in filter_bar
    assert 'class="mp-orders-filter-actions"' in filter_bar
    assert 'class="mp-orders-autorefresh"' in filter_bar
    assert 'style="min-width:260px;"' not in filter_bar
    assert 'style="margin-left:auto;' not in filter_bar
    assert 'style="font-size:12px;' not in filter_bar

    for selector in (
        ".mp-orders-filter-bar",
        ".mp-orders-filter-search",
        ".mp-orders-filter-side",
        ".mp-orders-filter-page-size",
        ".mp-orders-filter-actions",
        ".mp-orders-autorefresh",
    ):
        assert selector in css

    assert "max-width: 420px" in css
    assert "width: 132px" in css
    assert "width: 124px" in css


def test_open_orders_columns_menu_respects_hidden_attribute():
    html = ORDERS_HTML.read_text(encoding="utf-8")
    css = ADMIN_MP_CSS.read_text(encoding="utf-8")
    js = (ROOT / "frontend/platform/static/js/mp-orders.js").read_text(encoding="utf-8")

    columns_start = html.index('id="orders-columns-menu"')
    columns_open_tag = html[columns_start:html.index(">", columns_start)]

    assert "hidden" in columns_open_tag
    assert ".mp-columns-menu[hidden]" in css
    assert "display: none !important" in css
    assert "menu.hidden = true;" in js
    assert "menu.setAttribute('aria-hidden', 'true')" in js
    assert "menu.setAttribute('aria-hidden', String(!nextOpen))" in js
    assert "btn.setAttribute('aria-expanded', 'false')" in js
