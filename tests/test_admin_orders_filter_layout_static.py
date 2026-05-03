from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_admin_orders_filters_share_compact_search_row():
    html = (ROOT / "frontend/platform/admin/orders.html").read_text()
    css = (ROOT / "frontend/platform/static/css/admin.css").read_text()

    row_start = html.index('class="admin-orders-filter-row"')
    chips_start = html.index('id="order-quick-chips"', row_start)
    filter_row = html[row_start:chips_start]

    assert 'class="admin-search admin-orders-search"' in filter_row
    assert 'id="order-search"' in filter_row
    assert 'id="order-filter-status"' in filter_row
    assert 'id="order-filter-range"' in filter_row
    assert 'class="admin-orders-count"' in filter_row
    assert 'style="width: 220px"' not in filter_row

    for selector in (
        ".admin-orders-filter-row",
        ".admin-orders-search",
        ".admin-orders-filter-row .admin-select",
        "#order-filter-status",
        "#order-filter-range",
        ".admin-orders-count",
    ):
        assert selector in css

    assert "height: 34px" in css
    assert css.count("width: 178px") >= 3
    assert css.count("min-width: 178px") >= 3
