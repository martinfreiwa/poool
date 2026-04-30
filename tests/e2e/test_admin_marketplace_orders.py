import os
import uuid

import psycopg2
import pytest
import requests
from playwright.sync_api import expect


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
ORDER_PRICE_CENTS = 8_000
ORDER_QUANTITY = 120
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
        ON CONFLICT (user_id, wallet_type, currency) DO UPDATE SET
            balance_cents = EXCLUDED.balance_cents,
            held_balance_cents = EXCLUDED.held_balance_cents
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


def create_asset(cur, *, title="E2E Marketplace Orders Asset"):
    slug = f"e2e-marketplace-orders-{uuid.uuid4().hex[:8]}"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published
        )
        VALUES (
            %s, %s, 'E2E orders fixture',
            'Seeded by the admin marketplace orders E2E test.', 'real_estate',
            100000000, 10000, 10000, 9000, 'funded', TRUE
        )
        RETURNING id
        """,
        (title, slug),
    )
    return cur.fetchone()[0]


def create_open_order(cur, *, user_id, asset_id, side="buy", quantity=ORDER_QUANTITY, seconds_ago=0):
    cur.execute(
        """
        INSERT INTO market_orders (
            user_id, asset_id, side, order_type, price_cents, quantity,
            status, idempotency_key, expires_at, created_at, updated_at
        )
        VALUES (
            %s, %s, %s, 'limit', %s, %s,
            'open', %s, NOW() + INTERVAL '90 days',
            NOW() - (%s * INTERVAL '1 second'),
            NOW() - (%s * INTERVAL '1 second')
        )
        RETURNING id
        """,
        (
            user_id,
            asset_id,
            side,
            ORDER_PRICE_CENTS,
            quantity,
            str(uuid.uuid4()),
            seconds_ago,
            seconds_ago,
        ),
    )
    return cur.fetchone()[0]


def admin_session(session_token):
    session = requests.Session()
    session.cookies.set("poool_session", session_token)
    response = session.get(f"{BASE_URL}/admin/marketplace/orders", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin marketplace orders page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_marketplace_orders_cancel_releases_hold_and_audits():
    created_user_ids = []
    created_order_ids = []
    asset_id = None

    conn = db_connect()
    conn.autocommit = False
    cur = conn.cursor()
    try:
        admin = create_user(cur, email_prefix="e2e-orders-admin", roles=("admin",))
        buyer = create_user(
            cur,
            email_prefix="e2e-orders-buyer",
            cash_balance_cents=10_000_000,
            held_balance_cents=ORDER_HOLD_CENTS,
        )
        created_user_ids.extend([admin["id"], buyer["id"]])
        asset_id = create_asset(cur)
        order_id = create_open_order(cur, user_id=buyer["id"], asset_id=asset_id)
        created_order_ids.append(order_id)
        conn.commit()

        session = admin_session(admin["session_token"])

        missing_reason = session.delete(
            f"{BASE_URL}/api/admin/marketplace/orders/{order_id}",
            json={"reason": "   "},
            timeout=10,
        )
        assert missing_reason.status_code == 400, missing_reason.text

        cancel_response = session.delete(
            f"{BASE_URL}/api/admin/marketplace/orders/{order_id}",
            json={"reason": "E2E admin cancellation releases held cash"},
            timeout=10,
        )
        assert cancel_response.status_code == 200, cancel_response.text
        assert cancel_response.json()["success"] is True

        duplicate_response = session.delete(
            f"{BASE_URL}/api/admin/marketplace/orders/{order_id}",
            json={"reason": "E2E duplicate cancellation attempt"},
            timeout=10,
        )
        assert duplicate_response.status_code == 400, duplicate_response.text

        cur.execute("SELECT status, cancel_reason FROM market_orders WHERE id = %s", (order_id,))
        assert cur.fetchone() == ("admin_cancelled", "E2E admin cancellation releases held cash")
        cur.execute(
            """
            SELECT balance_cents, held_balance_cents
            FROM wallets
            WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
            """,
            (buyer["id"],),
        )
        assert cur.fetchone() == (10_000_000, 0)
        cur.execute(
            """
            SELECT previous_state->>'status', new_state->>'status', new_state->>'reason'
            FROM audit_logs
            WHERE entity_type = 'market_order'
              AND entity_id = %s
              AND action = 'marketplace.order.admin_cancelled'
            """,
            (order_id,),
        )
        assert cur.fetchone() == (
            "open",
            "admin_cancelled",
            "E2E admin cancellation releases held cash",
        )
    finally:
        conn.rollback()
        if created_order_ids:
            cur.execute("DELETE FROM audit_logs WHERE entity_id = ANY(%s::uuid[])", ([str(v) for v in created_order_ids],))
            cur.execute("DELETE FROM market_orders WHERE id = ANY(%s::uuid[])", ([str(v) for v in created_order_ids],))
        if asset_id is not None:
            cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
        if created_user_ids:
            cur.execute("DELETE FROM user_sessions WHERE user_id = ANY(%s::uuid[])", ([str(v) for v in created_user_ids],))
            cur.execute("DELETE FROM wallets WHERE user_id = ANY(%s::uuid[])", ([str(v) for v in created_user_ids],))
            cur.execute("DELETE FROM user_roles WHERE user_id = ANY(%s::uuid[])", ([str(v) for v in created_user_ids],))
            cur.execute("DELETE FROM users WHERE id = ANY(%s::uuid[])", ([str(v) for v in created_user_ids],))
        conn.commit()
        cur.close()
        conn.close()


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_marketplace_orders_safe_rendering_and_pagination(admin_page):
    page, tracker = admin_page
    created_user_ids = []
    created_order_ids = []
    asset_id = None

    conn = db_connect()
    conn.autocommit = False
    cur = conn.cursor()
    try:
        user = create_user(cur, email_prefix="e2e-orders-render")
        created_user_ids.append(user["id"])
        asset_id = create_asset(cur, title="E2E <img src=x onerror=alert(1)> Orders Asset")
        for idx in range(26):
            created_order_ids.append(
                create_open_order(
                    cur,
                    user_id=user["id"],
                    asset_id=asset_id,
                    quantity=ORDER_QUANTITY + idx,
                    seconds_ago=idx,
                )
            )
        conn.commit()

        page.goto(f"{BASE_URL}/admin/marketplace/orders", wait_until="networkidle")
        tracker.assert_page_loaded()

        expect(page.get_by_text("E2E <img src=x onerror=alert(1)> Orders Asset").first).to_be_visible()
        expect(page.locator("img[src='x']")).to_have_count(0)
        expect(page.locator("#orders-next-page")).to_be_enabled()

        page.locator("#orders-next-page").click()
        expect(page.locator("#orders-page-info")).to_contain_text("page 2")
        expect(page.locator("#orders-prev-page")).to_be_enabled()
    finally:
        conn.rollback()
        if created_order_ids:
            cur.execute("DELETE FROM market_orders WHERE id = ANY(%s::uuid[])", ([str(v) for v in created_order_ids],))
        if asset_id is not None:
            cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
        if created_user_ids:
            cur.execute("DELETE FROM user_sessions WHERE user_id = ANY(%s::uuid[])", ([str(v) for v in created_user_ids],))
            cur.execute("DELETE FROM wallets WHERE user_id = ANY(%s::uuid[])", ([str(v) for v in created_user_ids],))
            cur.execute("DELETE FROM user_roles WHERE user_id = ANY(%s::uuid[])", ([str(v) for v in created_user_ids],))
            cur.execute("DELETE FROM users WHERE id = ANY(%s::uuid[])", ([str(v) for v in created_user_ids],))
        conn.commit()
        cur.close()
        conn.close()
