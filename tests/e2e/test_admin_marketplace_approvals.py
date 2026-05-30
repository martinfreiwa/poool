import os
import uuid

import psycopg2
import pytest
import requests


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
ORDER_PRICE_CENTS = 8000
ORDER_QUANTITY = 600
ORDER_HOLD_CENTS = ORDER_PRICE_CENTS * ORDER_QUANTITY


def db_connect():
    return psycopg2.connect(DB_URL)


def create_user(cur, *, email_prefix, roles=(), cash_balance_cents=0, held_balance_cents=0):
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
        INSERT INTO wallets (user_id, wallet_type, currency, balance_cents, held_balance_cents)
        VALUES (%s, 'cash', 'USD', %s, %s)
        """,
        (user_id, cash_balance_cents, held_balance_cents),
    )
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
    slug = f"e2e-marketplace-approvals-{uuid.uuid4().hex[:8]}"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published
        )
        VALUES (
            'E2E Marketplace Approvals Asset', %s, 'E2E approval fixture',
            'Seeded by the admin marketplace approvals E2E test.', 'real_estate',
            100000000, 10000, 10000, 9000, 'funded', TRUE
        )
        RETURNING id
        """,
        (slug,),
    )
    return cur.fetchone()[0]


def create_pending_order(cur, *, user_id, asset_id, side, quantity, price_cents=10000):
    cur.execute(
        """
        INSERT INTO market_orders (
            user_id, asset_id, side, order_type, price_cents, quantity,
            status, idempotency_key, expires_at
        )
        VALUES (%s, %s, %s, 'limit', %s, %s, 'pending_review', %s, NOW() + INTERVAL '90 days')
        RETURNING id
        """,
        (user_id, asset_id, side, price_cents, quantity, str(uuid.uuid4())),
    )
    return cur.fetchone()[0]


def admin_session(session_token):
    session = requests.Session()
    session.cookies.set("poool_session", session_token)
    response = session.get(f"{BASE_URL}/admin/marketplace/approvals", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin approvals page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def assert_audit_log(cur, *, order_id, action):
    cur.execute(
        """
        SELECT COUNT(*)
        FROM audit_logs
        WHERE entity_type = 'market_order'
          AND entity_id = %s
          AND action = %s
        """,
        (order_id, action),
    )
    assert cur.fetchone()[0] == 1


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_marketplace_approvals_hold_release_audit_and_orderbook_visibility():
    created_user_ids = []
    created_order_ids = []
    asset_id = None

    conn = db_connect()
    conn.autocommit = False
    cur = conn.cursor()
    try:
        admin = create_user(cur, email_prefix="e2e-mp-admin", roles=("admin", "super_admin"))
        buy_user = create_user(
            cur,
            email_prefix="e2e-mp-buy",
            cash_balance_cents=10_000_000,
            held_balance_cents=ORDER_HOLD_CENTS,
        )
        rejected_buy_user = create_user(
            cur,
            email_prefix="e2e-mp-reject-buy",
            cash_balance_cents=10_000_000,
            held_balance_cents=ORDER_HOLD_CENTS,
        )
        sell_user = create_user(cur, email_prefix="e2e-mp-sell")
        created_user_ids.extend([admin["id"], buy_user["id"], rejected_buy_user["id"], sell_user["id"]])

        asset_id = create_asset(cur)
        approve_buy_order_id = create_pending_order(
            cur,
            user_id=buy_user["id"],
            asset_id=asset_id,
            side="buy",
            quantity=ORDER_QUANTITY,
            price_cents=ORDER_PRICE_CENTS,
        )
        reject_buy_order_id = create_pending_order(
            cur,
            user_id=rejected_buy_user["id"],
            asset_id=asset_id,
            side="buy",
            quantity=ORDER_QUANTITY,
            price_cents=ORDER_PRICE_CENTS,
        )
        reject_sell_order_id = create_pending_order(
            cur, user_id=sell_user["id"], asset_id=asset_id, side="sell", quantity=700
        )
        created_order_ids.extend([approve_buy_order_id, reject_buy_order_id, reject_sell_order_id])
        cur.execute(
            """
            INSERT INTO investments (
                user_id, asset_id, tokens_owned, held_tokens,
                purchase_value_cents, current_value_cents, status
            )
            VALUES (%s, %s, 1000, 700, 10000000, 10000000, 'active')
            """,
            (sell_user["id"], asset_id),
        )
        conn.commit()

        session = admin_session(admin["session_token"])

        approvals_response = session.get(f"{BASE_URL}/api/admin/marketplace/approvals", timeout=10)
        assert approvals_response.status_code == 200, approvals_response.text
        approvals = approvals_response.json()
        seeded = {row["id"]: row for row in approvals if row["id"] in {str(oid) for oid in created_order_ids}}
        assert set(seeded) == {str(oid) for oid in created_order_ids}
        assert seeded[str(approve_buy_order_id)]["asset_name"] == "E2E Marketplace Approvals Asset"
        assert seeded[str(approve_buy_order_id)]["review_reason"] == "Order quantity exceeds 5% supply threshold"
        assert seeded[str(approve_buy_order_id)]["supply_impact_bps"] == ORDER_QUANTITY

        approve_response = session.post(
            f"{BASE_URL}/api/admin/marketplace/approvals/{approve_buy_order_id}/approve",
            json={"reason": "E2E approves buy order for orderbook visibility"},
            timeout=10,
        )
        assert approve_response.status_code == 200, approve_response.text
        approve_body = approve_response.json()
        assert approve_body["status"] == "approved"
        assert "orderbook_synced" in approve_body

        orderbook_response = session.get(
            f"{BASE_URL}/api/admin/marketplace/orderbook/{asset_id}",
            timeout=10,
        )
        assert orderbook_response.status_code == 200, orderbook_response.text
        bids = orderbook_response.json()["bids"]
        assert any(
            level["price_cents"] == ORDER_PRICE_CENTS
            and level["total_quantity"] >= ORDER_QUANTITY
            and level["order_count"] >= 1
            for level in bids
        )

        reject_sell_response = session.post(
            f"{BASE_URL}/api/admin/marketplace/approvals/{reject_sell_order_id}/reject",
            json={"reason": "E2E rejects sell order to release token hold"},
            timeout=10,
        )
        assert reject_sell_response.status_code == 200, reject_sell_response.text
        assert reject_sell_response.json()["status"] == "rejected"

        reject_buy_response = session.post(
            f"{BASE_URL}/api/admin/marketplace/approvals/{reject_buy_order_id}/reject",
            json={"reason": "E2E rejects buy order to release wallet hold"},
            timeout=10,
        )
        assert reject_buy_response.status_code == 200, reject_buy_response.text
        assert reject_buy_response.json()["status"] == "rejected"

        cur.execute(
            """
            SELECT status, cancel_reason
            FROM market_orders
            WHERE id = %s
            """,
            (approve_buy_order_id,),
        )
        assert cur.fetchone() == ("open", "E2E approves buy order for orderbook visibility")
        cur.execute(
            """
            SELECT balance_cents, held_balance_cents
            FROM wallets
            WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
            """,
            (buy_user["id"],),
        )
        assert cur.fetchone() == (10_000_000, ORDER_HOLD_CENTS)
        assert_audit_log(cur, order_id=approve_buy_order_id, action="marketplace.order.approved")

        cur.execute(
            """
            SELECT status, cancel_reason
            FROM market_orders
            WHERE id = %s
            """,
            (reject_sell_order_id,),
        )
        assert cur.fetchone() == ("rejected", "E2E rejects sell order to release token hold")
        cur.execute(
            """
            SELECT tokens_owned, held_tokens
            FROM investments
            WHERE user_id = %s AND asset_id = %s
            """,
            (sell_user["id"], asset_id),
        )
        assert cur.fetchone() == (1000, 0)
        assert_audit_log(cur, order_id=reject_sell_order_id, action="marketplace.order.rejected")

        cur.execute(
            """
            SELECT status, cancel_reason
            FROM market_orders
            WHERE id = %s
            """,
            (reject_buy_order_id,),
        )
        assert cur.fetchone() == ("rejected", "E2E rejects buy order to release wallet hold")
        cur.execute(
            """
            SELECT balance_cents, held_balance_cents
            FROM wallets
            WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
            """,
            (rejected_buy_user["id"],),
        )
        assert cur.fetchone() == (10_000_000, 0)
        assert_audit_log(cur, order_id=reject_buy_order_id, action="marketplace.order.rejected")
    finally:
        conn.rollback()
        cur.execute("DELETE FROM audit_logs WHERE entity_id = ANY(%s::uuid[])", ([str(oid) for oid in created_order_ids],))
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
