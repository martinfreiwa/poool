"""
POOOL E2E Tests - Commodities Marketplace
=========================================
Authenticated browser coverage for commodity search, filters, HTMX tab swaps,
and semantic card navigation.
"""

import re
import uuid

import pytest
from playwright.sync_api import expect

from tests.e2e.conftest import (
    DB_URL,
    BASE_URL,
    _create_context_and_page,
    _teardown_context,
    attach_session_cookie,
    cleanup_test_user,
    create_e2e_user,
)
import psycopg2


@pytest.fixture(scope="function")
def mobile_authenticated_user_page(playwright_session, request):
    """Create an authenticated investor session in a mobile viewport."""
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    user = create_e2e_user(
        email_prefix="e2e-commodities-mobile",
        display_name="E2E Commodities Mobile",
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker, user

    _teardown_context(context, page, tracker, request)
    cleanup_test_user(user["user_id"])


@pytest.fixture
def commodity_assets():
    """Create available and funded commodity assets with deterministic filter data."""
    unique = uuid.uuid4().hex[:8]
    available_slug = f"e2e-commodity-available-{unique}"
    funded_slug = f"e2e-commodity-funded-{unique}"
    available_title = f"E2E Commodity Available {unique}"
    funded_title = f"E2E Commodity Funded {unique}"

    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO assets (
                title, slug, short_description, description, asset_type,
                location_city, location_country, total_value_cents, token_price_cents,
                tokens_total, tokens_available, annual_yield_bps,
                capital_appreciation_bps, funding_status, published, featured,
                term_months, land_size_sqm, operator_name, fixed_roi_bps,
                revenue_min_cents, revenue_max_cents, expenses_cents,
                net_profit_min_cents, net_profit_max_cents, investor_payout_cents,
                operator_split_pct, poool_split_pct, min_funding_tokens
            )
            VALUES
                (%s, %s, 'E2E available commodity', 'E2E available commodity description',
                 'commodity', 'Bali', 'ID', 24000000, 10000, 2400, 1200, 1250,
                 300, 'funding_open', TRUE, TRUE, 6, 600000, 'E2E Operator',
                 1250, 2800000, 3200000, 600000, 2200000, 2600000, 2400000,
                 20, 5, 1200),
                (%s, %s, 'E2E funded commodity', 'E2E funded commodity description',
                 'commodity', 'Bali', 'ID', 36000000, 10000, 3600, 0, 1400,
                 400, 'funded', TRUE, TRUE, 14, 800000, 'E2E Operator',
                 1400, 4000000, 4400000, 800000, 3200000, 3600000, 3400000,
                 20, 5, 3600)
            RETURNING id, slug, title
            """,
            (available_title, available_slug, funded_title, funded_slug),
        )
        rows = cur.fetchall()
        for asset_id, _, title in rows:
            cur.execute(
                """
                INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
                VALUES (%s, '/static/images/seed/villa1.webp', %s, 0, TRUE)
                """,
                (asset_id, title),
            )
        conn.commit()
        yield {
            "available": {"id": rows[0][0], "slug": rows[0][1], "title": rows[0][2]},
            "funded": {"id": rows[1][0], "slug": rows[1][1], "title": rows[1][2]},
        }
    finally:
        conn.rollback()
        cur.execute("DELETE FROM assets WHERE slug IN (%s, %s)", (available_slug, funded_slug))
        conn.commit()
        cur.close()
        conn.close()


@pytest.mark.marketplace
def test_commodities_marketplace_filters_tabs_and_keyboard_link(
    authenticated_user_page, commodity_assets
):
    page, tracker, _user = authenticated_user_page
    available = commodity_assets["available"]
    funded = commodity_assets["funded"]
    available_card = page.locator(f".property-card[data-property-id='{available['slug']}']")
    funded_card = page.locator(f".property-card[data-property-id='{funded['slug']}']")

    tracker.navigate_and_check(f"{BASE_URL}/commodities-marketplace")
    expect(page.locator("#commodities-page-title")).to_contain_text("Commodities")
    expect(available_card).to_be_visible()
    expect(funded_card).not_to_be_visible()

    expect(available_card).to_have_attribute("data-commodity-type", "agriculture")
    expect(available_card).to_have_attribute("data-duration", re.compile(r"6 months"))
    expect(available_card.locator(".property-card-link")).to_have_attribute(
        "href", f"/commodity/{available['slug']}"
    )

    page.locator("#filter-bar-search-input").fill(available["title"])
    page.locator("#filter-bar-search-btn").click()
    expect(available_card).to_be_visible()

    page.locator("#filter-bar-investment-select").select_option("0-6", force=True)
    expect(available_card).to_be_visible()

    page.locator("#filter-bar-property-select").select_option("agriculture", force=True)
    expect(available_card).to_be_visible()

    page.locator("#filter-bar-more-filters").click()
    expect(page.locator("#commodities-extra-filters")).to_be_visible()
    page.locator("#filter-commodity-type").select_option("agriculture")
    page.locator("#filter-min-price").fill("200000")
    page.locator("#filter-max-price").fill("300000")
    page.locator("#filter-min-yield").fill("10")
    page.locator("#extra-filter-apply").click()
    expect(available_card).to_be_visible()

    page.locator("#filter-bar-clear-btn").click()
    expect(available_card).to_be_visible()

    page.locator("#filter-bar-tab-funded").click()
    expect(page.locator("#filter-bar-tab-funded")).to_have_class(re.compile(r"\bactive\b"))
    expect(funded_card).to_be_visible()
    expect(funded_card).to_have_attribute("data-duration", re.compile(r"14 months"))
    expect(funded_card.locator(".property-card-link")).to_have_attribute(
        "href", f"/commodity/{funded['slug']}"
    )

    funded_card.locator(".property-card-link").focus()
    with page.expect_navigation(url=re.compile(rf".*/commodity/{funded['slug']}")):
        page.keyboard.press("Enter")

    tracker.assert_no_critical_errors()


@pytest.mark.marketplace
@pytest.mark.mobile
def test_commodities_marketplace_mobile_smoke(mobile_authenticated_user_page, commodity_assets):
    page, tracker, _user = mobile_authenticated_user_page
    available = commodity_assets["available"]

    tracker.navigate_and_check(f"{BASE_URL}/commodities-marketplace")
    card = page.locator(f".property-card[data-property-id='{available['slug']}']")
    expect(card).to_be_visible()
    expect(page.locator("#filter-bar-more-filters")).to_be_visible()
    expect(card.locator(".property-card-link")).to_be_visible()

    tracker.assert_no_critical_errors()
