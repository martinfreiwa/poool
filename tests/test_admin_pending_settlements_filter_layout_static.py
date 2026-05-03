from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_pending_settlements_filters_share_one_compact_row():
    html = (ROOT / "frontend/platform/admin/pending-settlements.html").read_text()

    row_start = html.index('class="settlements-filter-bar"')
    table_start = html.index("<!-- Settlements Table -->", row_start)
    filter_row = html[row_start:table_start]

    assert 'class="admin-search settlements-filter-control"' in filter_row
    assert 'id="settlement-search"' in filter_row
    assert 'id="filter-match-status" class="admin-select settlements-filter-control"' in filter_row
    assert 'id="filter-asset" class="admin-select settlements-filter-control"' in filter_row
    assert 'class="settlements-filter-count"' in filter_row
    assert 'style="width: 220px;"' not in filter_row

    for expected in (
        ".settlements-filter-bar",
        ".settlements-filter-control",
        "flex: 0 0 190px",
        "width: 190px",
        "min-width: 190px",
        ".settlements-filter-bar .admin-search",
        ".settlements-filter-bar .admin-select",
    ):
        assert expected in html
