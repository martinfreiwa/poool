import os
import uuid

import psycopg2
import pytest
import requests


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
ORDER_PRICE_CENTS = 8_000
ORDER_QUANTITY = 120


def db_connect():
    return psycopg2.connect(DB_URL)


def create_user(cur, *, email_prefix, roles=()):
    email = f"{email_prefix}-{uuid.uuid4().hex[:8]}@poool.app"
    session_token = str(uuid.uuid4())
    cur.execute(
        """
        INSERT INTO users (email, email_verified, status)
        VALUES (%s, TRUE, 'active')
        RETURNING id
        """,
        (email,),
    )
    user_id = cur.fetchone()[0]
    cur.execute(
        """
        INSERT INTO user_sessions (user_id, session_token, remember_me, expires_at)
        VALUES (%s, %s, FALSE, NOW() + INTERVAL '1 hour')
        """,
        (user_id, session_token),
    )
    for role in roles:
        cur.execute(
            """
            INSERT INTO user_roles (user_id, role_id, is_active)
            SELECT %s, id, TRUE FROM roles WHERE name = %s
            """,
            (user_id, role),
        )
    return {"id": user_id, "email": email, "session_token": session_token}


def create_asset(cur):
    slug = f"e2e-marketplace-orderbook-{uuid.uuid4().hex[:8]}"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published
        )
        VALUES (
            'E2E Marketplace Orderbook Asset', %s, 'E2E orderbook fixture',
            'Seeded by the admin marketplace orderbook E2E test.', 'real_estate',
            100000000, 10000, 10000, 9000, 'funded', TRUE
        )
        RETURNING id
        """,
        (slug,),
    )
    return cur.fetchone()[0]


def create_open_order(cur, *, user_id, asset_id, side, price_cents, quantity):
    cur.execute(
        """
        INSERT INTO market_orders (
            user_id, asset_id, side, order_type, price_cents, quantity,
            status, idempotency_key, expires_at
        )
        VALUES (%s, %s, %s, 'limit', %s, %s, 'open', %s, NOW() + INTERVAL '90 days')
        RETURNING id
        """,
        (user_id, asset_id, side, price_cents, quantity, str(uuid.uuid4())),
    )
    return cur.fetchone()[0]


def admin_session(session_token):
    session = requests.Session()
    session.cookies.set("poool_session", session_token)
    response = session.get(f"{BASE_URL}/admin/marketplace/orderbook", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin orderbook page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_marketplace_orderbook_live_assets_rebuild_and_audit():
    created_user_ids = []
    created_order_ids = []
    asset_id = None

    conn = db_connect()
    conn.autocommit = False
    cur = conn.cursor()
    try:
        admin = create_user(cur, email_prefix="e2e-ob-admin", roles=("admin",))
        buy_user = create_user(cur, email_prefix="e2e-ob-buy")
        sell_user = create_user(cur, email_prefix="e2e-ob-sell")
        created_user_ids.extend([admin["id"], buy_user["id"], sell_user["id"]])

        asset_id = create_asset(cur)
        bid_order_id = create_open_order(
            cur,
            user_id=buy_user["id"],
            asset_id=asset_id,
            side="buy",
            price_cents=ORDER_PRICE_CENTS,
            quantity=ORDER_QUANTITY,
        )
        ask_order_id = create_open_order(
            cur,
            user_id=sell_user["id"],
            asset_id=asset_id,
            side="sell",
            price_cents=ORDER_PRICE_CENTS + 500,
            quantity=ORDER_QUANTITY + 30,
        )
        created_order_ids.extend([bid_order_id, ask_order_id])
        conn.commit()

        session = admin_session(admin["session_token"])

        assets_response = session.get(f"{BASE_URL}/api/admin/marketplace/orderbook/assets", timeout=10)
        assert assets_response.status_code == 200, assets_response.text
        assets = assets_response.json()
        seeded_assets = [asset for asset in assets if asset["id"] == str(asset_id)]
        assert seeded_assets
        assert seeded_assets[0]["title"] == "E2E Marketplace Orderbook Asset"
        assert seeded_assets[0]["active_orders"] == 2

        orderbook_response = session.get(
            f"{BASE_URL}/api/admin/marketplace/orderbook/{asset_id}",
            timeout=10,
        )
        assert orderbook_response.status_code == 200, orderbook_response.text
        orderbook = orderbook_response.json()
        assert orderbook["asset_id"] == str(asset_id)
        assert orderbook["asset_title"] == "E2E Marketplace Orderbook Asset"
        assert any(
            level["price_cents"] == ORDER_PRICE_CENTS
            and level["total_quantity"] == ORDER_QUANTITY
            and level["order_count"] == 1
            for level in orderbook["bids"]
        )
        assert any(
            level["price_cents"] == ORDER_PRICE_CENTS + 500
            and level["total_quantity"] == ORDER_QUANTITY + 30
            and level["order_count"] == 1
            for level in orderbook["asks"]
        )

        rebuild_response = session.post(
            f"{BASE_URL}/api/admin/marketplace/orderbook/rebuild",
            timeout=10,
        )
        assert rebuild_response.status_code == 200, rebuild_response.text
        rebuild = rebuild_response.json()
        assert rebuild["success"] is True
        assert rebuild["orders_restored"] >= 2

        cur.execute(
            """
            SELECT COUNT(*)
            FROM audit_logs
            WHERE actor_user_id = %s
              AND action = 'marketplace.orderbook.rebuilt'
              AND entity_type = 'marketplace_orderbook'
              AND (new_state->>'orders_restored')::int >= 2
            """,
            (admin["id"],),
        )
        assert cur.fetchone()[0] >= 1
    finally:
        conn.rollback()
        cur.execute("DELETE FROM audit_logs WHERE actor_user_id = ANY(%s::uuid[])", ([str(uid) for uid in created_user_ids],))
        cur.execute("DELETE FROM market_orders WHERE id = ANY(%s::uuid[])", ([str(oid) for oid in created_order_ids],))
        if asset_id is not None:
            cur.execute("DELETE FROM investments WHERE asset_id = %s", (asset_id,))
            cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
        if created_user_ids:
            cur.execute("DELETE FROM user_sessions WHERE user_id = ANY(%s::uuid[])", ([str(uid) for uid in created_user_ids],))
            cur.execute("DELETE FROM wallets WHERE user_id = ANY(%s::uuid[])", ([str(uid) for uid in created_user_ids],))
            cur.execute("DELETE FROM user_roles WHERE user_id = ANY(%s::uuid[])", ([str(uid) for uid in created_user_ids],))
            cur.execute("DELETE FROM users WHERE id = ANY(%s::uuid[])", ([str(uid) for uid in created_user_ids],))
        conn.commit()
        cur.close()
        conn.close()
