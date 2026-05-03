from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_admin_notifications_filters_share_compact_search_row():
    html = (ROOT / "frontend/platform/admin/notifications.html").read_text()
    css = (ROOT / "frontend/platform/static/css/admin.css").read_text()

    row_start = html.index('class="admin-notifications-filter-row"')
    table_start = html.index('<div class="admin-card">', row_start)
    filter_row = html[row_start:table_start]

    assert 'class="admin-search admin-notifications-search"' in filter_row
    assert 'id="notif-search"' in filter_row
    assert 'id="filter-type"' in filter_row
    assert 'id="filter-read"' in filter_row
    assert 'class="admin-notifications-count"' in filter_row
    assert 'style="width: 200px"' not in filter_row

    for selector in (
        ".admin-notifications-filter-row",
        ".admin-notifications-search",
        ".admin-notifications-filter-row .admin-select",
        ".admin-notifications-filter-row #filter-type",
        ".admin-notifications-filter-row #filter-read",
        ".admin-notifications-count",
    ):
        assert selector in css

    assert "width: 142px" in css
    assert "width: 112px" in css
