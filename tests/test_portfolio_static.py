from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_portfolio_desktop_section_headings_removed():
    html = read("frontend/platform/portfolio.html")

    assert 'id="key-financials-title"' not in html
    assert 'id="assets-title"' not in html
    desktop_html = html[: html.index("<!-- Mobile Portfolio Content")]
    assert "Key financials</h2>" not in desktop_html
    assert "My Assets</h2>" not in desktop_html
    assert 'id="key-financials-grid"' in html
    assert 'id="portfolio-assets-table"' in html


def test_portfolio_sections_use_design_spacing_token():
    base_css = read("frontend/platform/static/css/portfolio.css")
    value_css = read("frontend/platform/static/css/portfolio-value-card.css")
    enhanced_css = read("frontend/platform/static/css/portfolio-enhancements.css")

    assert "gap: var(--section-gap);" in base_css
    assert "gap: var(--section-gap, 24px);" in enhanced_css

    value_block = enhanced_css[
        enhanced_css.index(".portfolio-value-card {"):
        enhanced_css.index(".portfolio-value-card:hover", enhanced_css.index(".portfolio-value-card {"))
    ]
    key_section_block = enhanced_css[
        enhanced_css.index(".key-financials-section {"):
        enhanced_css.index(".key-financials-section h2", enhanced_css.index(".key-financials-section {"))
    ]
    key_grid_block = enhanced_css[
        enhanced_css.index(".key-financials-grid {"):
        enhanced_css.index(".key-financials-card {", enhanced_css.index(".key-financials-grid {"))
    ]
    assets_block = enhanced_css[
        enhanced_css.index(".assets-section {"):
        enhanced_css.index(".portfolio-assets-table", enhanced_css.index(".assets-section {"))
    ]

    assert "margin-bottom: 0 !important;" in value_block
    assert "margin-bottom: 0;" in value_css
    assert "margin-top: 0 !important;" in key_section_block
    assert "margin-top: 36px !important;" not in key_section_block
    assert "gap: var(--section-gap, 24px) !important;" in key_grid_block
    assert "margin-top: 0 !important;" in assets_block
    assert "margin-top: 40px !important;" not in assets_block


def test_portfolio_expanded_chart_controls_are_simple_and_compact():
    html = read("frontend/platform/portfolio.html")
    css = read("frontend/platform/static/css/portfolio-enhancements.css")

    actions_start = html.index('id="portfolio-value-actions"')
    actions_end = html.index('class="portfolio-show-more-btn', actions_start)
    actions_block = html[actions_start:actions_end]
    chart_header_start = html.index('id="portfolio-chart-header"')
    chart_header_end = html.index('id="portfolio-chart-container"', chart_header_start)
    chart_header_block = html[chart_header_start:chart_header_end]

    assert 'id="portfolio-chart-controls"' in actions_block
    assert 'aria-label="Chart time range"' in actions_block
    assert 'id="portfolio-chart-controls"' not in chart_header_block

    controls_css_start = css.index("#portfolio-body .portfolio-chart-controls {")
    controls_css_end = css.index("#portfolio-body .portfolio-expandable", controls_css_start)
    controls_block = css[controls_css_start:controls_css_end]
    tabs_css_start = css.index("#portfolio-body .portfolio-chart-tabs {", controls_css_end)
    tabs_css_end = css.index("#portfolio-body .portfolio-chart-tabs .chart-tab", tabs_css_start)
    tabs_block = css[tabs_css_start:tabs_css_end]
    active_css_start = css.index("#portfolio-body .chart-tab.active {", tabs_css_end)
    active_css_end = css.index("#portfolio-body .portfolio-chart-filter {", active_css_start)
    active_block = css[active_css_start:active_css_end]

    assert "display: none;" in controls_block
    assert "height: 32px !important;" in tabs_block
    assert "background: transparent !important;" in tabs_block
    assert "border: 0 !important;" in tabs_block
    assert "box-shadow: none !important;" in active_block
    assert "inset 0 -2px 0" not in active_block


def test_portfolio_chart_plot_area_is_constrained_inside_card():
    css = read("frontend/platform/static/css/portfolio-enhancements.css")

    container_start = css.index("#portfolio-body .portfolio-chart-container {")
    container_end = css.index("#portfolio-body .chart-grid", container_start)
    container_block = css[container_start:container_end]
    bars_start = css.index("#portfolio-body .chart-bars {")
    bars_end = css.index("#portfolio-body .chart-bar-week", bars_start)
    bars_block = css[bars_start:bars_end]
    trend_start = css.index("#portfolio-body .chart-trend-line {")
    trend_end = css.index("#portfolio-body .chart-x-axis", trend_start)
    trend_block = css[trend_start:trend_end]
    axis_start = css.index("#portfolio-body .chart-x-axis {")
    axis_end = css.index("#portfolio-body .quick-insights-card", axis_start)
    axis_block = css[axis_start:axis_end]

    assert "overflow: hidden;" in container_block
    assert "box-sizing: border-box;" in container_block
    assert "left: 52px;" in bars_block
    assert "right: 12px;" in bars_block
    assert "width: auto !important;" in bars_block
    assert "padding: 0 !important;" in bars_block
    assert "overflow: hidden;" in bars_block
    assert "left: 52px;" in trend_block
    assert "right: 12px;" in trend_block
    assert "left: 52px;" in axis_block
    assert "right: 12px;" in axis_block


def test_portfolio_asset_rows_use_icon_only_detail_action():
    js = read("frontend/platform/static/js/portfolio-data.js")
    css = read("frontend/platform/static/css/portfolio-assets-table.css")

    action_start = js.index('<div class="portfolio-assets-cell actions-col"')
    action_end = js.index("</div>", action_start)
    action_block = js[action_start:action_end]

    assert "portfolio-assets-detail-btn" in action_block
    assert 'aria-label="View details for ${title}"' in action_block
    assert 'title="View details"' in action_block
    assert "<svg" in action_block
    assert "See Details" not in action_block
    assert "ds-btn ds-btn--ghost" not in action_block

    assert ".portfolio-assets-detail-btn {" in css
    assert "width: 36px;" in css
    assert "height: 36px;" in css
    assert ".portfolio-assets-detail-btn:hover" in css
