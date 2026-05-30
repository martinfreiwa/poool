import os
import uuid

import psycopg2
from playwright.sync_api import expect

from tests.e2e.conftest import cleanup_test_user, create_e2e_user


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def get_db_connection():
    return psycopg2.connect(DB_URL)


def _seed_trade_fixture():
    buyer = create_e2e_user(email_prefix="e2e-mp-buyer", display_name="E2E MP Buyer")
    seller = create_e2e_user(email_prefix="e2e-mp-seller", display_name="E2E MP Seller")
    slug = f"e2e-marketplace-analytics-{uuid.uuid4().hex[:10]}"

    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO assets (
                title, slug, asset_type, total_value_cents, token_price_cents,
                tokens_total, tokens_available, funding_status, published
            )
            VALUES (%s, %s, 'real_estate', 10000000, 10000, 1000, 900, 'funded', TRUE)
            RETURNING id
            """,
            ("E2E Marketplace Analytics Asset", slug),
        )
        asset_id = cur.fetchone()[0]

        cur.execute(
            """
            INSERT INTO market_orders (
                user_id, asset_id, side, order_type, price_cents, quantity,
                quantity_filled, status
            )
            VALUES (%s, %s, 'buy', 'limit', 10000, 2, 2, 'filled')
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
            VALUES (%s, %s, 'sell', 'limit', 10000, 2, 2, 'filled')
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
            VALUES (%s, %s, %s, %s, %s, 10000, 2, 150, 75, NOW())
            RETURNING id
            """,
            (
                asset_id,
                buy_order_id,
                sell_order_id,
                buyer["user_id"],
                seller["user_id"],
            ),
        )
        trade_id = cur.fetchone()[0]
        conn.commit()
        return {
            "asset_id": asset_id,
            "buy_order_id": buy_order_id,
            "sell_order_id": sell_order_id,
            "trade_id": trade_id,
            "buyer": buyer,
            "seller": seller,
        }
    finally:
        cur.close()
        conn.close()


def _cleanup_trade_fixture(fixture):
    conn = get_db_connection()
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


def test_admin_marketplace_analytics_authenticated_e2e(admin_page):
    page, tracker = admin_page
    fixture = _seed_trade_fixture()
    try:
        with page.expect_response(lambda r: "/api/admin/marketplace/stats" in r.url and r.status == 200):
            with page.expect_response(lambda r: "/api/admin/marketplace/trades" in r.url and r.status == 200):
                tracker.navigate_and_check(f"{BASE_URL}/admin/marketplace/analytics", timeout=30000)

        expect(page).to_have_title("Trading Analytics | Admin | POOOL")
        metabase = page.evaluate(
            """() => {
                const card = document.getElementById('metabase-card');
                return {
                    baseUrl: card?.dataset.metabaseBaseUrl || '',
                    publicDashboardPath: card?.dataset.metabasePublicDashboardPath || '',
                    cardDisplay: card ? getComputedStyle(card).display : '',
                    frameSrc: document.getElementById('metabase-frame')?.getAttribute('src') || '',
                    openDisabled: document.getElementById('btn-open-metabase')?.disabled ?? true,
                };
            }"""
        )
        expect(page.locator("#metabase-frame")).to_have_attribute("title", "Marketplace analytics dashboard")
        if metabase["baseUrl"] and metabase["publicDashboardPath"]:
            expect(page.locator("#metabase-empty")).to_be_hidden()
            assert metabase["frameSrc"], "Configured Metabase should set iframe src"
        else:
            assert metabase["cardDisplay"] == "none"
            assert metabase["openDisabled"] is True

        expect(page.locator(".mp-analytics-stat-label", has_text="Trades 24h")).to_be_visible()
        expect(page.locator(".mp-analytics-stat-label", has_text="Volume 24h")).to_be_visible()
        expect(page.locator(".mp-analytics-stat-label", has_text="Fees 24h")).to_be_visible()
        expect(page.locator("#analytics-stats-grid")).to_contain_text("Trades 24h")
        expect(page.locator("#analytics-stats-grid")).not_to_contain_text("Stats unavailable")
        expect(page.locator("#analytics-volume-chart")).not_to_contain_text("Trade data unavailable")
        expect(page.locator("#analytics-assets-chart")).not_to_contain_text("Asset data unavailable")

        stats = page.request.get(f"{BASE_URL}/api/admin/marketplace/stats")
        assert stats.ok
        stats_json = stats.json()
        assert stats_json["trades_24h"] >= 1
        assert stats_json["volume_24h_cents"] >= 20000
        assert stats_json["fees_collected_24h_cents"] >= 150

        trades = page.request.get(f"{BASE_URL}/api/admin/marketplace/trades?limit=200")
        assert trades.ok
        trades_json = trades.json()
        assert isinstance(trades_json["data"], list)
        assert any(row["id"] == str(fixture["trade_id"]) for row in trades_json["data"])

        buyer_id = fixture["buyer"]["user_id"]
        buyer_trades = page.request.get(
            f"{BASE_URL}/api/admin/marketplace/trades?user_id={buyer_id}&side=buy"
        )
        assert buyer_trades.ok
        assert any(row["id"] == str(fixture["trade_id"]) for row in buyer_trades.json()["data"])

        invalid_side = page.request.get(f"{BASE_URL}/api/admin/marketplace/trades?side=buy")
        assert invalid_side.status == 400

        tracker.assert_no_critical_errors()
    finally:
        _cleanup_trade_fixture(fixture)
