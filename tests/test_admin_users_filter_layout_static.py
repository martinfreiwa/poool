from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_admin_users_filters_share_compact_search_row():
    html = (ROOT / "frontend/platform/admin/users.html").read_text()
    css = (ROOT / "frontend/platform/static/css/admin.css").read_text()

    row_start = html.index('class="admin-users-filter-row"')
    table_start = html.index("<!-- Users Table -->", row_start)
    filter_row = html[row_start:table_start]

    assert 'class="admin-search admin-users-search"' in filter_row
    assert 'id="user-search-input"' in filter_row
    assert 'id="filter-role"' in filter_row
    assert 'id="filter-kyc"' in filter_row
    assert 'id="filter-status"' in filter_row
    assert 'class="admin-users-count"' in filter_row
    assert 'style="width: 240px"' not in filter_row

    for selector in (
        ".admin-users-filter-row",
        ".admin-users-search",
        ".admin-users-filter-row .admin-select",
        ".admin-users-filter-row #filter-role",
        ".admin-users-filter-row #filter-kyc",
        ".admin-users-filter-row #filter-status",
        ".admin-users-count",
    ):
        assert selector in css

    assert "width: 132px" in css
    assert "width: 128px" in css
    assert "width: 134px" in css
