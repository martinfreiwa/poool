from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_admin_support_filters_share_compact_search_row():
    html = (ROOT / "frontend/platform/admin/support.html").read_text()
    css = (ROOT / "frontend/platform/static/css/admin.css").read_text()

    row_start = html.index('class="admin-support-filter-row"')
    bulk_actions_start = html.index("<!-- Bulk Actions -->", row_start)
    filter_row = html[row_start:bulk_actions_start]

    assert 'class="admin-search admin-support-search"' in filter_row
    assert 'id="ticket-search"' in filter_row
    assert 'id="filter-status"' in filter_row
    assert 'id="filter-priority"' in filter_row
    assert 'id="filter-date"' in filter_row
    assert 'class="admin-support-count"' in filter_row
    assert 'style="width: 220px"' not in filter_row

    for selector in (
        ".admin-support-filter-row",
        ".admin-support-search",
        ".admin-support-filter-row .admin-select",
        ".admin-support-filter-row #filter-status",
        ".admin-support-filter-row #filter-priority",
        ".admin-support-filter-row #filter-date",
        ".admin-support-count",
    ):
        assert selector in css

    assert "width: 146px" in css
    assert "width: 142px" in css
    assert "width: 128px" in css
    assert ".admin-support-filter-row {\n  padding-block: 10px;" in css
    assert ".admin-support-filter-row .admin-select {\n  height: 32px;" in css
