from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_admin_audit_filters_share_compact_search_row():
    html = (ROOT / "frontend/platform/admin/audit-logs.html").read_text()
    css = (ROOT / "frontend/platform/static/css/admin.css").read_text()

    row_start = html.index('class="admin-audit-filter-row"')
    table_start = html.index("<!-- Logs Table -->", row_start)
    filter_row = html[row_start:table_start]

    assert 'class="admin-search admin-audit-search"' in filter_row
    assert 'id="audit-search"' in filter_row
    assert 'id="filter-entity"' in filter_row
    assert 'id="filter-action"' in filter_row
    assert 'class="admin-audit-count"' in filter_row
    assert 'id="audit-export-csv"' in filter_row
    assert 'style="width: 200px"' not in filter_row

    for selector in (
        ".admin-audit-filter-row",
        ".admin-audit-search",
        ".admin-audit-filter-row .admin-select",
        ".admin-audit-filter-row #filter-entity",
        ".admin-audit-filter-row #filter-action",
        ".admin-audit-count",
    ):
        assert selector in css

    assert "width: 154px" in css
    assert "width: 146px" in css
