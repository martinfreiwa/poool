from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_admin_deposits_filters_share_compact_search_row():
    html = (ROOT / "frontend/platform/admin/deposits.html").read_text()
    css = (ROOT / "frontend/platform/static/css/admin.css").read_text()

    row_start = html.index('class="admin-deposits-filter-row"')
    panel_start = html.index('id="advanced-filter-panel"', row_start)
    filter_row = html[row_start:panel_start]

    assert 'class="admin-search admin-deposits-search"' in filter_row
    assert 'id="deposit-search"' in filter_row
    assert 'id="filter-status"' in filter_row
    assert 'id="filter-currency"' in filter_row
    assert 'id="filter-provider"' in filter_row
    assert 'style="width: 220px"' not in filter_row

    for selector in (
        ".admin-deposits-filter-row",
        ".admin-deposits-search",
        ".admin-deposits-filter-row .admin-select",
        ".admin-deposits-filter-row #filter-status",
        ".admin-deposits-filter-row #filter-currency",
        ".admin-deposits-filter-row #filter-provider",
    ):
        assert selector in css

    assert "height: 34px" in css
    assert css.count("width: 178px") >= 7
    assert css.count("min-width: 178px") >= 7
