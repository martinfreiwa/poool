from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_admin_developer_submissions_filter_toolbar_is_compact():
    html = (ROOT / "frontend/platform/admin/developer-submissions.html").read_text()
    css = (ROOT / "frontend/platform/static/css/admin.css").read_text()
    js = (ROOT / "frontend/platform/static/js/admin-submissions.js").read_text()

    row_start = html.index('class="admin-submissions-filter-row"')
    advanced_start = html.index("<!-- Advanced filters panel -->", row_start)
    filter_row = html[row_start:advanced_start]

    assert 'id="page-subtitle"' not in html
    assert 'id="anomaly-banner"' not in html
    assert 'id="sub-count-label"' not in html
    assert 'id="filter-mine"' not in html
    assert 'id="filter-include-test"' not in html

    assert 'id="last-updated-indicator"' in filter_row
    assert 'id="sub-search"' in filter_row
    assert 'class="admin-search admin-submissions-filter-control"' in filter_row
    assert 'id="filter-status"' in filter_row
    assert 'id="filter-type"' in filter_row
    assert 'id="filter-age"' in filter_row
    assert 'id="filter-risk"' in filter_row
    assert 'style="width: 220px' not in filter_row
    assert 'style="width: 160px' not in filter_row

    for selector in (
        ".admin-submissions-filter-row",
        ".admin-submissions-filter-control",
        ".admin-submissions-updated",
        ".admin-submissions-progress-track",
        ".admin-submissions-progress-fill",
        ".admin-submissions-progress-label",
    ):
        assert selector in css

    assert "background: var(--admin-success" in css
    assert "admin-submissions-progress-fill" in js
    assert "Show only mine" not in js
