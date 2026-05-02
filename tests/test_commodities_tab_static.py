from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def test_commodities_tab_does_not_mask_database_errors():
    routes = read("backend/src/assets/routes.rs")
    handler = routes.split("pub async fn api_commodities_tab", 1)[1].split(
        "#[derive(Serialize)]", 1
    )[0]

    assert "match sqlx::query" in handler
    assert "Commodities tab query failed" in handler
    assert "StatusCode::INTERNAL_SERVER_ERROR" in handler
    assert ".unwrap_or_default()" not in handler


def test_commodities_tab_fragment_matches_filter_and_link_contract():
    routes = read("backend/src/assets/routes.rs")
    renderer = routes.split("fn render_commodity_card", 1)[1].split(
        "pub async fn page_marketplace", 1
    )[0]

    required_fragments = [
        'data-price="{price_dollars}"',
        'data-duration="{duration_data}"',
        'data-commodity-type="agriculture"',
        'class="property-card-link"',
        'href="/commodity/{slug}"',
        'data-yield="{annual_yield_data}"',
        'ds-progress__fill',
    ]

    for fragment in required_fragments:
        assert fragment in renderer

    assert "onclick=\"window.location.href='/commodity/{slug}'\"" not in renderer
    assert "commodity_status_label" not in renderer
    assert "<span>{status_label}</span>" not in renderer


def test_commodities_cards_use_property_size_icon_without_status_meta():
    routes = read("backend/src/assets/routes.rs")
    renderer = routes.split("fn render_commodity_card", 1)[1].split(
        "pub async fn page_marketplace", 1
    )[0]
    template = read("frontend/platform/commodities-marketplace.html")
    square_meter_icon = (
        '<path d="M4 4h16v16H4z"/><path d="M9 4v16"/>'
        '<path d="M4 9h16"/>'
    )
    diamond_icon = '<path d="M12 22L2 12 12 2l10 10z"/>'

    assert square_meter_icon in renderer
    assert square_meter_icon in template
    assert diamond_icon not in renderer
    assert diamond_icon not in template
    assert "{% set funding_status_label" not in template


def test_commodities_parent_template_uses_display_safe_values():
    template = read("frontend/platform/commodities-marketplace.html")

    assert "asset.land_size_sqm /" not in template
    assert "asset.capital_appreciation_bps /" not in template
    assert "asset.annual_yield_bps /" not in template
    assert "asset.land_size_hectares" in template
    assert "asset.total_value_usd" in template
    assert "asset.funded_percentage" in template
    assert 'data-yield="{{ asset.annual_yield_percent }}"' in template


def test_commodities_filter_dropdowns_use_matching_icons():
    template = read("frontend/platform/commodities-marketplace.html")

    assert (
        'id="filter-bar-investment-icon" src="/static/images/icons/calendar-check-02.svg"'
        in template
    )
    assert (
        'id="filter-bar-property-icon" src="/static/images/icons/coins-stacked-03.svg"'
        in template
    )
    assert 'id="filter-bar-investment-icon" src="/static/images/icons/dollar.svg"' not in template
    assert 'id="filter-bar-property-icon" src="/static/images/icons/home-03.svg"' not in template


def test_marketplace_search_reinitialization_is_idempotent():
    search_js = read("frontend/platform/static/js/marketplace-search.js")

    assert "function getPropertyGrid()" in search_js
    assert 'searchInput.dataset.marketplaceSearchReady === "true"' in search_js
    assert 'document.body.dataset.marketplaceDropdownFilterReady !== "true"' in search_js
    assert 'investmentVal === "7-12"' in search_js
    assert 'investmentVal === "13plus"' in search_js
