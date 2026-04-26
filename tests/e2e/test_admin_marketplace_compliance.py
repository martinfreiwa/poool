import os
import uuid

import psycopg2
import pytest
import requests


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


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
    cur.execute(
        """
        INSERT INTO user_profiles (user_id, display_name)
        VALUES (%s, %s)
        """,
        (user_id, email_prefix.replace("-", " ").title()),
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
    slug = f"e2e-marketplace-compliance-{uuid.uuid4().hex[:8]}"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published
        )
        VALUES (
            'E2E Marketplace Compliance Asset', %s, 'E2E compliance fixture',
            'Seeded by the admin marketplace compliance E2E test.', 'real_estate',
            100000000, 10000, 10000, 9000, 'funded', TRUE
        )
        RETURNING id
        """,
        (slug,),
    )
    return cur.fetchone()[0]


def create_filled_order(cur, *, user_id, asset_id, side):
    cur.execute(
        """
        INSERT INTO market_orders (
            user_id, asset_id, side, order_type, price_cents, quantity,
            quantity_filled, status, idempotency_key
        )
        VALUES (%s, %s, %s, 'limit', 12345, 4, 4, 'filled', %s)
        RETURNING id
        """,
        (user_id, asset_id, side, str(uuid.uuid4())),
    )
    return cur.fetchone()[0]


def compliance_session(session_token):
    session = requests.Session()
    session.cookies.set("poool_session", session_token)
    return session


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_marketplace_compliance_exports_auth_and_real_csv_data():
    created_user_ids = []
    created_order_ids = []
    trade_id = None
    asset_id = None
    tax_report_user_id = None

    conn = db_connect()
    conn.autocommit = False
    cur = conn.cursor()
    try:
        compliance = create_user(cur, email_prefix="e2e-mp-compliance", roles=("compliance",))
        finance = create_user(cur, email_prefix="e2e-mp-finance", roles=("finance",))
        buyer = create_user(cur, email_prefix="e2e-mp-travel-buyer")
        seller = create_user(cur, email_prefix="e2e-mp-travel-seller")
        tax_user = create_user(cur, email_prefix="e2e-mp-tax-user")
        created_user_ids.extend(
            [compliance["id"], finance["id"], buyer["id"], seller["id"], tax_user["id"]]
        )
        tax_report_user_id = tax_user["id"]

        asset_id = create_asset(cur)
        buy_order_id = create_filled_order(cur, user_id=buyer["id"], asset_id=asset_id, side="buy")
        sell_order_id = create_filled_order(cur, user_id=seller["id"], asset_id=asset_id, side="sell")
        created_order_ids.extend([buy_order_id, sell_order_id])

        cur.execute(
            """
            INSERT INTO trade_history (
                asset_id, buy_order_id, sell_order_id, buyer_user_id, seller_user_id,
                price_cents, quantity, fee_cents, fee_bps, executed_at
            )
            VALUES (%s, %s, %s, %s, %s, 12345, 4, 321, 65, '2026-02-15 12:00:00+00')
            RETURNING id
            """,
            (asset_id, buy_order_id, sell_order_id, buyer["id"], seller["id"]),
        )
        trade_id = cur.fetchone()[0]

        cur.execute(
            """
            INSERT INTO tax_reports (
                user_id, fiscal_year, total_investment_cents, total_dividends_cents,
                capital_gains_cents, withholding_tax_cents, status, generated_at
            )
            VALUES (%s, 2025, 1000000, 25000, 75000, 5000, 'generated', NOW())
            """,
            (tax_user["id"],),
        )
        conn.commit()

        session = compliance_session(compliance["session_token"])
        page_response = session.get(f"{BASE_URL}/admin/marketplace/compliance", timeout=10)
        assert page_response.status_code == 200, page_response.text[:500]
        assert "Compliance & OJK Reports" in page_response.text

        ojk_response = session.get(
            f"{BASE_URL}/api/admin/marketplace/compliance/ojk-report?quarter=2026-Q1",
            timeout=10,
        )
        assert ojk_response.status_code == 200, ojk_response.text
        assert ojk_response.headers["content-type"].startswith("text/csv")
        assert "Total Trade Volume (cents),49380,2026-Q1" in ojk_response.text
        assert "Total Trades,1,2026-Q1" in ojk_response.text

        travel_response = session.get(
            f"{BASE_URL}/api/admin/marketplace/compliance/travel-rule?from_date=2026-02-01&to_date=2026-02-28",
            timeout=10,
        )
        assert travel_response.status_code == 200, travel_response.text
        assert str(trade_id) in travel_response.text
        assert buyer["email"] in travel_response.text
        assert seller["email"] in travel_response.text
        assert "12345,4,49380" in travel_response.text

        invalid_range = session.get(
            f"{BASE_URL}/api/admin/marketplace/compliance/travel-rule?from_date=2026-03-01&to_date=2026-02-01",
            timeout=10,
        )
        assert invalid_range.status_code == 400

        tax_response = session.get(
            f"{BASE_URL}/api/admin/marketplace/compliance/tax-export?year=2025",
            timeout=10,
        )
        assert tax_response.status_code == 200, tax_response.text
        assert tax_user["email"] in tax_response.text
        assert "2025,1000000,25000,75000,5000,generated" in tax_response.text
        assert "user_placeholder@poool.app" not in tax_response.text

        finance_session = compliance_session(finance["session_token"])
        forbidden = finance_session.get(
            f"{BASE_URL}/api/admin/marketplace/compliance/ojk-report?quarter=2026-Q1",
            timeout=10,
        )
        assert forbidden.status_code == 403
    finally:
        conn.rollback()
        if trade_id is not None:
            cur.execute("DELETE FROM trade_history WHERE id = %s", (trade_id,))
        if created_order_ids:
            cur.execute(
                "DELETE FROM market_orders WHERE id = ANY(%s::uuid[])",
                ([str(oid) for oid in created_order_ids],),
            )
        if asset_id is not None:
            cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
        if tax_report_user_id is not None:
            cur.execute("DELETE FROM tax_reports WHERE user_id = %s", (tax_report_user_id,))
        if created_user_ids:
            cur.execute(
                "DELETE FROM user_sessions WHERE user_id = ANY(%s::uuid[])",
                ([str(uid) for uid in created_user_ids],),
            )
            cur.execute(
                "DELETE FROM user_profiles WHERE user_id = ANY(%s::uuid[])",
                ([str(uid) for uid in created_user_ids],),
            )
            cur.execute(
                "DELETE FROM user_roles WHERE user_id = ANY(%s::uuid[])",
                ([str(uid) for uid in created_user_ids],),
            )
            cur.execute(
                "DELETE FROM users WHERE id = ANY(%s::uuid[])",
                ([str(uid) for uid in created_user_ids],),
            )
        conn.commit()
        cur.close()
        conn.close()
