from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_phase8_asset_circle_cta_shared_helper_is_authz_aware():
    helper = read("frontend/platform/static/js/asset-circle-cta.js")

    for token in [
        "window.PooolAssetCircleCta",
        '"/api/community/assets/" + encodeURIComponent(key) + "/circle"',
        'credentials: "same-origin"',
        "safeCircleUrl",
        'value.indexOf("/community/circle/") === 0',
        "access_state",
        "renderPropertyCta",
        "renderActionButton",
    ]:
        assert token in helper


def test_phase8_property_page_loads_asset_circle_cta_from_asset_id():
    page = read("frontend/platform/property.html")
    js = read("frontend/platform/static/js/property-detail.js")
    css = read("frontend/platform/static/css/property-detail.css")

    for token in [
        "'asset-circle-cta'",
        'class="asset-circle-cta-section"',
        'data-asset-circle-asset-id="{{ asset.id }}"',
        'aria-label="Asset investor discussion"',
    ]:
        assert token in page

    for token in [
        "initializePropertyAssetCircleCta",
        "PooolAssetCircleCta.fetchAssetCircle",
        "PooolAssetCircleCta.renderPropertyCta",
    ]:
        assert token in js

    for token in [
        ".asset-circle-cta-section",
        ".asset-circle-cta__button",
        ".asset-circle-cta__meta",
    ]:
        assert token in css


def test_phase8_portfolio_rows_expose_asset_circle_entry_points():
    page = read("frontend/platform/portfolio.html")
    service = read("frontend/platform/static/js/portfolio-service.js")
    data = read("frontend/platform/static/js/portfolio-data.js")
    css = read("frontend/platform/static/css/portfolio-assets-table.css")

    assert '<script src="/static/js/asset-circle-cta.js"></script>' in page
    assert "assetId: inv.asset_id" in service

    for token in [
        "hydratePortfolioAssetCircles",
        "PooolAssetCircleCta.fetchAssetCircle",
        "portfolio-asset-circle-slot",
        "mobile-asset-circle-slot",
        "data-asset-id",
        "renderActionButton(data, { variant: \"desktop\" })",
        "renderActionButton(data, { variant: \"mobile\" })",
    ]:
        assert token in data

    for token in [
        ".portfolio-assets-circle-btn",
        ".portfolio-asset-circle-slot",
        ".mobile-asset-circle-link",
    ]:
        assert token in css
