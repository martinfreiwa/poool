import os
import uuid

import psycopg2
import requests

from tests.e2e.conftest import cleanup_test_user, create_e2e_user


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_DSN = os.environ.get("DATABASE_URL", "dbname=poool user=martin host=localhost")


def _connect():
    return psycopg2.connect(DB_DSN)


def _session(session_token, warm_path="/cart"):
    session = requests.Session()
    session.cookies.set("poool_session", str(session_token), path="/")
    session.get(f"{BASE_URL}{warm_path}", timeout=10)
    for cookie in session.cookies:
        cookie.secure = False
    csrf = session.cookies.get("csrf_token")
    if csrf:
        session.headers.update({"X-CSRF-Token": csrf})
    return session


def _seed_asset(developer_user_id):
    conn = _connect()
    cur = conn.cursor()
    try:
        slug = f"workflow-kyc-limit-{uuid.uuid4().hex[:8]}"
        cur.execute(
            """
            INSERT INTO assets (
                developer_user_id, title, slug, short_description, description,
                asset_type, total_value_cents, token_price_cents,
                tokens_total, tokens_available, funding_status, published,
                featured, location_city, location_country, property_type
            )
            VALUES (
                %s, 'Workflow KYC Limit Villa', %s, 'Workflow checkout fixture',
                'Disposable fixture for KYC limit checkout workflow.',
                'real_estate', 50000000, 50000,
                1000, 1000, 'funding_open', TRUE,
                FALSE, 'Denpasar', 'Indonesia', 'villa'
            )
            RETURNING id
            """,
            (developer_user_id, slug),
        )
        asset_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO asset_images (asset_id, image_url, is_cover, sort_order)
            VALUES (%s, 'https://example.com/workflow-kyc-limit.jpg', TRUE, 0)
            """,
            (asset_id,),
        )
        conn.commit()
        return asset_id
    finally:
        cur.close()
        conn.close()


def _delete_asset(asset_id):
    conn = _connect()
    cur = conn.cursor()
    try:
        for sql in (
            "DELETE FROM asset_images WHERE asset_id = %s",
            "DELETE FROM cart_items WHERE asset_id = %s",
            "DELETE FROM order_items WHERE asset_id = %s",
            "DELETE FROM investments WHERE asset_id = %s",
            "DELETE FROM assets WHERE id = %s",
        ):
            cur.execute(sql, (asset_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _latest_kyc_id(user_id):
    conn = _connect()
    cur = conn.cursor()
    try:
        cur.execute(
            "SELECT id FROM kyc_records WHERE user_id = %s ORDER BY created_at DESC LIMIT 1",
            (user_id,),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def _insert_pending_kyc(user_id):
    conn = _connect()
    cur = conn.cursor()
    try:
        cur.execute(
            "INSERT INTO kyc_records (user_id, status) VALUES (%s, 'pending') RETURNING id",
            (user_id,),
        )
        kyc_id = cur.fetchone()[0]
        conn.commit()
        return kyc_id
    finally:
        cur.close()
        conn.close()


def _count_audit(action, entity_id):
    conn = _connect()
    cur = conn.cursor()
    try:
        cur.execute(
            "SELECT COUNT(*) FROM audit_logs WHERE action = %s AND entity_id = %s",
            (action, entity_id),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def _cleanup_user_orders(user_id):
    conn = _connect()
    cur = conn.cursor()
    try:
        cleanup_statements = (
            "DELETE FROM checkout_disclosures WHERE order_id IN (SELECT id FROM orders WHERE user_id = %s)",
            "DELETE FROM wallet_transactions WHERE related_order_id IN (SELECT id FROM orders WHERE user_id = %s)",
            "DELETE FROM order_items WHERE order_id IN (SELECT id FROM orders WHERE user_id = %s)",
            "DELETE FROM orders WHERE user_id = %s",
            "DELETE FROM cart_items WHERE user_id = %s",
            "DELETE FROM investments WHERE user_id = %s",
            "DELETE FROM transactional_email_outbox WHERE user_id = %s",
        )
        for sql in cleanup_statements:
            try:
                cur.execute(sql, (user_id,))
            except psycopg2.Error:
                conn.rollback()
        conn.commit()
    finally:
        cur.close()
        conn.close()


def test_kyc_admin_limit_checkout_eligibility_workflow():
    admin = create_e2e_user(
        email_prefix="e2e-kyc-limit-admin",
        roles=("admin", "super_admin"),
    )
    investor = create_e2e_user(
        email_prefix="e2e-kyc-limit-investor",
        cash_balance_cents=50_000_000,
        kyc_status="pending",
    )
    developer = create_e2e_user(
        email_prefix="e2e-kyc-limit-dev",
        roles=("developer",),
    )
    asset_id = _seed_asset(developer["user_id"])

    try:
        investor_session = _session(investor["session_token"], "/cart")
        admin_session = _session(admin["session_token"], "/admin/kyc")

        blocked_add = investor_session.post(
            f"{BASE_URL}/cart/add",
            data={"property_id": str(asset_id), "investment_amount": "1000"},
            allow_redirects=False,
            timeout=10,
        )
        assert blocked_add.status_code in {302, 303}
        assert "/kyc?reason=required" in blocked_add.headers.get("location", "")

        rejected_kyc_id = _latest_kyc_id(investor["user_id"])
        reject = admin_session.post(
            f"{BASE_URL}/api/admin/kyc/{rejected_kyc_id}/reject",
            json={"rejection_reason": "Workflow Test: clearer document required"},
            timeout=10,
        )
        assert reject.status_code == 200, reject.text
        assert _count_audit("admin.kyc_reject", rejected_kyc_id) == 1

        approved_kyc_id = _insert_pending_kyc(investor["user_id"])
        approve = admin_session.post(
            f"{BASE_URL}/api/admin/kyc/{approved_kyc_id}/approve",
            timeout=10,
        )
        assert approve.status_code == 200, approve.text
        assert _count_audit("admin.kyc_approve", approved_kyc_id) == 1

        settings = investor_session.get(f"{BASE_URL}/api/settings", timeout=10)
        assert settings.status_code == 200
        assert settings.json()["kyc_status"] == "approved"

        admin_readback = admin_session.get(
            f"{BASE_URL}/api/admin/users/{investor['user_id']}",
            timeout=10,
        )
        assert admin_readback.status_code == 200, admin_readback.text
        assert admin_readback.json()["kyc_status"] == "approved"

        low_limit = admin_session.post(
            f"{BASE_URL}/api/admin/users/{investor['user_id']}/investment-limit",
            json={"annual_limit_cents": 50_000},
            timeout=10,
        )
        assert low_limit.status_code == 200, low_limit.text
        assert _count_audit("admin.investment_limit_set", investor["user_id"]) >= 1

        add_allowed = investor_session.post(
            f"{BASE_URL}/cart/add",
            data={"property_id": str(asset_id), "investment_amount": "1000"},
            allow_redirects=False,
            timeout=10,
        )
        assert add_allowed.status_code in {200, 302, 303}

        over_limit = investor_session.post(
            f"{BASE_URL}/checkout",
            data={"payment_method": "wallet", "payment_currency": "USD"},
            timeout=10,
        )
        assert over_limit.status_code == 400
        assert "annual investment limit" in over_limit.text

        raised_limit = admin_session.post(
            f"{BASE_URL}/api/admin/users/{investor['user_id']}/investment-limit",
            json={"annual_limit_cents": 200_000},
            timeout=10,
        )
        assert raised_limit.status_code == 200, raised_limit.text

        checkout = investor_session.post(
            f"{BASE_URL}/checkout",
            data={"payment_method": "wallet", "payment_currency": "USD"},
            timeout=10,
        )
        assert checkout.status_code == 200, checkout.text
        assert checkout.json()["success"] is True

        conn = _connect()
        cur = conn.cursor()
        try:
            cur.execute(
                """
                SELECT il.invested_12m_cents, COUNT(i.id), COUNT(o.id)
                FROM investment_limits il
                LEFT JOIN investments i ON i.user_id = il.user_id AND i.asset_id = %s
                LEFT JOIN orders o ON o.user_id = il.user_id
                WHERE il.user_id = %s AND il.limit_year = EXTRACT(YEAR FROM NOW())::INTEGER
                GROUP BY il.invested_12m_cents
                """,
                (asset_id, investor["user_id"]),
            )
            invested_12m_cents, investment_count, order_count = cur.fetchone()
            assert invested_12m_cents >= 100_000
            assert investment_count >= 1
            assert order_count >= 1
        finally:
            cur.close()
            conn.close()
    finally:
        _cleanup_user_orders(investor["user_id"])
        _delete_asset(asset_id)
        cleanup_test_user(investor["user_id"])
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(developer["user_id"])
