from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def test_admin_assets_filters_use_compact_wrapping_layout():
    html = (ROOT / "frontend/platform/admin/assets.html").read_text()
    css = (ROOT / "frontend/platform/static/css/admin.css").read_text()

    row_start = html.index('class="admin-assets-filter-row"')
    panel_start = html.index('id="advanced-filter-panel"', row_start)
    filter_row = html[row_start:panel_start]

    assert 'class="admin-search admin-assets-filter-search"' in filter_row
    assert 'class="admin-assets-filter-primary"' in filter_row
    assert 'id="asset-search"' in filter_row
    assert 'id="filter-type" class="admin-select admin-assets-filter-select"' in filter_row
    assert 'id="filter-status" class="admin-select admin-assets-filter-select"' in filter_row
    assert 'class="admin-assets-filter-check"' in filter_row
    assert 'class="admin-assets-count"' in filter_row
    assert 'class="admin-assets-filter-actions"' in filter_row
    assert 'class="admin-select admin-assets-saved-views"' in filter_row
    assert 'style="width: 220px"' not in filter_row
    assert 'style="max-width:160px"' not in filter_row

    for selector in (
        ".admin-assets-filter-row",
        ".admin-assets-filter-primary",
        ".admin-assets-filter-search",
        ".admin-assets-filter-select",
        ".admin-assets-filter-row .admin-search",
        ".admin-assets-filter-check",
        ".admin-assets-count",
        ".admin-assets-filter-actions",
        ".admin-assets-saved-views",
    ):
        assert selector in css

    asset_css_start = css.index(".admin-assets-filter-row")
    asset_css_end = css.index(".admin-users-filter-row #filter-role", asset_css_start)
    asset_css = css[asset_css_start:asset_css_end]

    assert "width: 178px" not in asset_css
    assert "grid-template-columns: minmax(220px, 1fr) 138px 154px auto auto" in css
    assert "width: 136px" in css
    assert "height: 32px" in css
