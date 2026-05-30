import os
import re
import uuid

import psycopg2
import pytest
from playwright.sync_api import expect

from tests.e2e.conftest import cleanup_test_user, create_e2e_user


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def db_connect():
    return psycopg2.connect(DB_URL)


def seed_recent_trade_fixture():
    buyer = create_e2e_user(email_prefix="e2e-mp-index-buyer", display_name="E2E MP Index Buyer")
    seller = create_e2e_user(email_prefix="e2e-mp-index-seller", display_name="E2E MP Index Seller")
    slug = f"e2e-marketplace-index-{uuid.uuid4().hex[:10]}"
    asset_title = "E2E <img src=x onerror=alert(1)> Marketplace Index Asset"

    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO assets (
                title, slug, short_description, description, asset_type,
                total_value_cents, token_price_cents, tokens_total, tokens_available,
                funding_status, published
            )
            VALUES (
                %s, %s, 'E2E marketplace index fixture',
                'Seeded by the marketplace index E2E test.', 'real_estate',
                10000000, 10000, 1000, 900, 'funded', TRUE
            )
            RETURNING id
            """,
            (asset_title, slug),
        )
        asset_id = cur.fetchone()[0]

        cur.execute(
            """
            INSERT INTO market_orders (
                user_id, asset_id, side, order_type, price_cents, quantity,
                quantity_filled, status
            )
            VALUES (%s, %s, 'buy', 'limit', 12500, 4, 4, 'filled')
            RETURNING id
            """,
            (buyer["user_id"], asset_id),
        )
        buy_order_id = cur.fetchone()[0]

        cur.execute(
            """
            INSERT INTO market_orders (
                user_id, asset_id, side, order_type, price_cents, quantity,
                quantity_filled, status
            )
            VALUES (%s, %s, 'sell', 'limit', 12500, 4, 4, 'filled')
            RETURNING id
            """,
            (seller["user_id"], asset_id),
        )
        sell_order_id = cur.fetchone()[0]

        cur.execute(
            """
            INSERT INTO trade_history (
                asset_id, buy_order_id, sell_order_id, buyer_user_id, seller_user_id,
                price_cents, quantity, fee_cents, fee_bps, executed_at
            )
            VALUES (%s, %s, %s, %s, %s, 12500, 4, 250, 50, NOW())
            RETURNING id
            """,
            (asset_id, buy_order_id, sell_order_id, buyer["user_id"], seller["user_id"]),
        )
        trade_id = cur.fetchone()[0]
        conn.commit()
        return {
            "asset_id": asset_id,
            "asset_title": asset_title,
            "buy_order_id": buy_order_id,
            "sell_order_id": sell_order_id,
            "trade_id": trade_id,
            "buyer": buyer,
            "seller": seller,
        }
    finally:
        cur.close()
        conn.close()


def cleanup_recent_trade_fixture(fixture):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute("DELETE FROM trade_history WHERE id = %s", (fixture["trade_id"],))
        cur.execute(
            "DELETE FROM market_orders WHERE id IN (%s, %s)",
            (fixture["buy_order_id"], fixture["sell_order_id"]),
        )
        cur.execute("DELETE FROM assets WHERE id = %s", (fixture["asset_id"],))
        conn.commit()
    finally:
        cur.close()
        conn.close()

    cleanup_test_user(fixture["buyer"]["user_id"])
    cleanup_test_user(fixture["seller"]["user_id"])


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_marketplace_index_authenticated_e2e(admin_page):
    page, tracker = admin_page
    fixture = seed_recent_trade_fixture()
    try:
        tracker.navigate_and_check(f"{BASE_URL}/admin/marketplace/")
        page.wait_for_function(
            """
            () => window.PooolMarketplaceOverview
                && Object.values(window.PooolMarketplaceOverview.getLastErrors()).every((value) => value === null)
            """,
            timeout=15000,
        )

        expect(page).to_have_title("Marketplace Overview | Admin | POOOL")
        expect(page.locator("#kpi-trading-status")).not_to_have_text("Loading")
        expect(page.locator("#live-trades-body")).to_contain_text("E2E <img src=x onerror=alert(1)> Marketplace Index Asset")
        expect(page.locator("#live-trades-body img[src='x']")).to_have_count(0)
        expect(page.locator("#health-dot-db")).to_have_attribute("aria-label", re.compile("Database:"))
        expect(page.locator("#health-dot-matching")).to_have_attribute(
            "aria-label",
            re.compile("Matching engine:", re.IGNORECASE),
        )
        expect(page.locator("#health-dot-ws")).to_have_attribute(
            "aria-label",
            re.compile("WebSocket gateway:", re.IGNORECASE),
        )

        stats = page.request.get(f"{BASE_URL}/api/admin/marketplace/stats")
        assert stats.ok
        stats_json = stats.json()
        assert stats_json["trades_24h"] >= 1
        assert stats_json["volume_24h_cents"] >= 50000
        assert stats_json["fees_collected_24h_cents"] >= 250
        expect(page.locator("#kpi-open-orders")).to_have_text(f"{stats_json['open_orders']:,}")
        expect(page.locator("#kpi-volume")).to_have_text(re.compile(r"^\$[0-9,]+$"))

        trades = page.request.get(f"{BASE_URL}/api/admin/marketplace/recent-trades")
        assert trades.ok
        assert any(row["id"] == str(fixture["trade_id"]) for row in trades.json())

        health = page.request.get(f"{BASE_URL}/api/admin/marketplace/health")
        assert health.ok
        health_json = health.json()
        assert health_json["database_connected"] is True
        assert health_json["database_latency_ms"] >= 0
        assert health_json["websocket_status"] in {"healthy", "degraded", "not_tracked", "not_configured"}
        assert health_json["matching_engine_status"] in {"healthy", "degraded", "not_configured"}

        errors = page.evaluate("window.PooolMarketplaceOverview.getLastErrors()")
        assert errors == {"stats": None, "trades": None, "health": None}
        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures()
    finally:
        cleanup_recent_trade_fixture(fixture)
