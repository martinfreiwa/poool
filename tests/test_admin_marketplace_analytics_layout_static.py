from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ANALYTICS_HTML = ROOT / "frontend/platform/admin/marketplace/analytics.html"


def test_analytics_page_uses_scoped_full_width_admin_shell():
    html = ANALYTICS_HTML.read_text(encoding="utf-8")

    assert 'class="admin-body dom-ready mp-analytics-page"' in html
    assert ".mp-analytics-page .admin-main" in html
    assert "margin-left: 0" in html
    assert "width: calc(100% - var(--admin-sidebar-width, 260px))" in html
    assert "body.admin-sidebar-collapsed.mp-analytics-page .admin-main" in html
    assert ".mp-analytics-page .admin-content" in html
    assert "max-width: none" in html


def test_analytics_top_controls_have_bounded_widths():
    html = ANALYTICS_HTML.read_text(encoding="utf-8")

    assert 'class="mp-range-label"' in html
    assert "#analytics-interval-picker" in html
    assert "width: 112px" in html
    assert ".mp-daterange input[type=\"date\"]" in html
    assert "width: 140px" in html
    assert ".mp-range-custom input[type=date]" in html
    assert "width: 132px" in html
    assert ".mp-asset-filter" in html
    assert "width: 160px" in html
    assert ".mp-analytics-updated > button" in html
    assert "margin-left: 0 !important" in html


def test_analytics_header_uses_responsive_two_column_layout():
    html = ANALYTICS_HTML.read_text(encoding="utf-8")

    assert ".mp-analytics-page .admin-page-header" in html
    assert "grid-template-columns: minmax(280px, 1fr) minmax(320px, auto)" in html
    assert ".mp-analytics-page .admin-page-subtitle" in html
    assert "text-align: right" in html
    assert "@media (max-width: 1100px)" in html
    assert "text-align: left" in html
