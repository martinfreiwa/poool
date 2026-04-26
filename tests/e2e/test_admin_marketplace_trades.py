import os
import uuid

import psycopg2
import pytest
from playwright.sync_api import expect

from tests.e2e.conftest import cleanup_test_user, create_e2e_user


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def db_connect():
    return psycopg2.connect(DB_URL)


def _create_asset(cur, *, title, slug_prefix):
    slug = f"{slug_prefix}-{uuid.uuid4().hex[:8]}"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published
        )
        VALUES (
            %s, %s, 'E2E trade fixture',
            'Seeded by the admin marketplace trades E2E test.', 'real_estate',
            100000000, 10000, 10000, 9000, 'funded', TRUE
        )
        RETURNING id
        """,
        (title, slug),
    )
    return cur.fetchone()[0]


def _create_order(cur, *, user_id, asset_id, side, price_cents, quantity):
    cur.execute(
        """
        INSERT INTO market_orders (
            user_id, asset_id, side, order_type, price_cents, quantity,
            quantity_filled, status, idempotency_key, expires_at
        )
        VALUES (%s, %s, %s, 'limit', %s, %s, %s, 'filled', %s, NOW() + INTERVAL '90 days')
        RETURNING id
        """,
        (user_id, asset_id, side, price_cents, quantity, quantity, str(uuid.uuid4())),
    )
    return cur.fetchone()[0]


def _create_trade(cur, *, asset_id, buy_order_id, sell_order_id, buyer_id, seller_id, status, executed_at_sql):
    cur.execute(
        f"""
        INSERT INTO trade_history (
            asset_id, buy_order_id, sell_order_id, buyer_user_id, seller_user_id,
            price_cents, quantity, fee_cents, fee_bps, on_chain_status, executed_at
        )
        VALUES (%s, %s, %s, %s, %s, 12500, 4, 250, 50, %s, {executed_at_sql})
        RETURNING id
        """,
        (asset_id, buy_order_id, sell_order_id, buyer_id, seller_id, status),
    )
    return cur.fetchone()[0]


def _seed_trade_fixture():
    buyer = create_e2e_user(email_prefix="e2e-trades-buyer", display_name="E2E Trades Buyer")
    seller = create_e2e_user(email_prefix="e2e-trades-seller", display_name="E2E Trades Seller")
    conn = db_connect()
    cur = conn.cursor()
    try:
        safe_asset_id = _create_asset(
            cur,
            title="E2E <img src=x onerror=alert(1)> Trade Asset",
            slug_prefix="e2e-marketplace-trades",
        )
        other_asset_id = _create_asset(
            cur,
            title="E2E Marketplace Trades Other Asset",
            slug_prefix="e2e-marketplace-trades-other",
        )

        buy_order_id = _create_order(
            cur,
            user_id=buyer["user_id"],
            asset_id=safe_asset_id,
            side="buy",
            price_cents=12500,
            quantity=4,
        )
        sell_order_id = _create_order(
            cur,
            user_id=seller["user_id"],
            asset_id=safe_asset_id,
            side="sell",
            price_cents=12500,
            quantity=4,
        )
        other_buy_order_id = _create_order(
            cur,
            user_id=buyer["user_id"],
            asset_id=other_asset_id,
            side="buy",
            price_cents=12500,
            quantity=4,
        )
        other_sell_order_id = _create_order(
            cur,
            user_id=seller["user_id"],
            asset_id=other_asset_id,
            side="sell",
            price_cents=12500,
            quantity=4,
        )

        confirmed_trade_id = _create_trade(
            cur,
            asset_id=safe_asset_id,
            buy_order_id=buy_order_id,
            sell_order_id=sell_order_id,
            buyer_id=buyer["user_id"],
            seller_id=seller["user_id"],
            status="confirmed",
            executed_at_sql="NOW()",
        )
        failed_trade_id = _create_trade(
            cur,
            asset_id=other_asset_id,
            buy_order_id=other_buy_order_id,
            sell_order_id=other_sell_order_id,
            buyer_id=buyer["user_id"],
            seller_id=seller["user_id"],
            status="failed",
            executed_at_sql="NOW() - INTERVAL '2 days'",
        )
        conn.commit()
        return {
            "buyer": buyer,
            "seller": seller,
            "asset_id": safe_asset_id,
            "other_asset_id": other_asset_id,
            "order_ids": [buy_order_id, sell_order_id, other_buy_order_id, other_sell_order_id],
            "trade_ids": [confirmed_trade_id, failed_trade_id],
            "confirmed_trade_id": confirmed_trade_id,
            "failed_trade_id": failed_trade_id,
        }
    finally:
        cur.close()
        conn.close()


def _cleanup_trade_fixture(fixture):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute("DELETE FROM trade_history WHERE id = ANY(%s::uuid[])", ([str(v) for v in fixture["trade_ids"]],))
        cur.execute("DELETE FROM market_orders WHERE id = ANY(%s::uuid[])", ([str(v) for v in fixture["order_ids"]],))
        cur.execute("DELETE FROM assets WHERE id = ANY(%s::uuid[])", ([str(fixture["asset_id"]), str(fixture["other_asset_id"])],))
        conn.commit()
    finally:
        cur.close()
        conn.close()
    cleanup_test_user(fixture["buyer"]["user_id"])
    cleanup_test_user(fixture["seller"]["user_id"])


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_marketplace_trades_api_filters_and_csv_export(admin_page):
    page, _tracker = admin_page
    fixture = _seed_trade_fixture()
    try:
        filtered = page.request.get(
            f"{BASE_URL}/api/admin/marketplace/trades"
            f"?asset_id={fixture['asset_id']}&on_chain_status=confirmed&limit=50"
        )
        assert filtered.ok, filtered.text()
        data = filtered.json()
        assert any(row["id"] == str(fixture["confirmed_trade_id"]) for row in data["data"])
        assert all(row["asset_id"] == str(fixture["asset_id"]) for row in data["data"])
        assert all(row["on_chain_status"] == "confirmed" for row in data["data"])

        invalid_status = page.request.get(
            f"{BASE_URL}/api/admin/marketplace/trades?on_chain_status=settled"
        )
        assert invalid_status.status == 400

        csv_response = page.request.get(
            f"{BASE_URL}/api/admin/marketplace/trades/export.csv"
            f"?asset_id={fixture['asset_id']}&on_chain_status=confirmed"
        )
        assert csv_response.ok, csv_response.text()
        assert "text/csv" in csv_response.headers["content-type"]
        csv = csv_response.text()
        assert str(fixture["confirmed_trade_id"]) in csv
        assert "confirmed" in csv
        assert "E2E <img src=x onerror=alert(1)> Trade Asset" in csv
    finally:
        _cleanup_trade_fixture(fixture)


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_marketplace_trades_page_filters_safe_render_and_error_state(admin_page):
    page, tracker = admin_page
    fixture = _seed_trade_fixture()
    try:
        tracker.navigate_and_check(f"{BASE_URL}/admin/marketplace/trades")
        row = page.locator(f"tr[data-trade-id='{fixture['confirmed_trade_id']}']")
        expect(row).to_be_visible()
        expect(row).to_contain_text("E2E <img src=x onerror=alert(1)> Trade Asset")
        expect(row).to_contain_text("Confirmed")
        expect(page.locator("img[src='x']")).to_have_count(0)

        page.locator("#filter-asset").evaluate(
            "(select, value) => { select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); }",
            str(fixture["asset_id"]),
        )
        page.locator("#filter-status").evaluate(
            "(select, value) => { select.value = value; select.dispatchEvent(new Event('change', { bubbles: true })); }",
            "confirmed",
        )
        with page.expect_response(
            lambda response: "/api/admin/marketplace/trades?" in response.url
            and f"asset_id={fixture['asset_id']}" in response.url
            and "on_chain_status=confirmed" in response.url
            and response.status == 200
        ):
            page.locator("#btn-apply-filter").click()
        expect(row).to_be_visible()
        expect(page.locator(f"tr[data-trade-id='{fixture['failed_trade_id']}']")).to_have_count(0)

        with page.expect_response(
            lambda response: "/api/admin/marketplace/trades/export.csv" in response.url
            and response.status == 200
        ):
            page.locator("#btn-export-csv").click()
        expect(page.locator("#btn-export-pdf")).to_be_disabled()

        tracker.assert_no_critical_errors()
    finally:
        _cleanup_trade_fixture(fixture)


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_marketplace_trades_api_error_shows_retry_without_mock_rows(admin_page):
    page, _tracker = admin_page
    page.route(
        "**/api/admin/marketplace/trades?*",
        lambda route: route.fulfill(
            status=500,
            content_type="application/json",
            body='{"error":"forced trade history failure"}',
        ),
    )

    page.goto(f"{BASE_URL}/admin/marketplace/trades", wait_until="networkidle")
    expect(page.get_by_text("Unable to load trade history")).to_be_visible()
    expect(page.get_by_role("button", name="Retry")).to_be_visible()
    expect(page.locator("tbody#trades-body tr")).to_have_count(1)
    expect(page.locator("tbody#trades-body")).not_to_contain_text("TRD-")
