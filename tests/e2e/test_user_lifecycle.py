"""
POOOL E2E - cross-role user lifecycle.

This is the stable hybrid lifecycle described in
`docs/Lebenszyklus-Nutzer.md`: browser-backed sessions for user-facing routes
and HTTP APIs, deterministic DB hand-offs for KYC/developer approval and the
settlement worker boundary.
"""

import os
import time
import uuid

import psycopg2
import pytest
import requests
from playwright.sync_api import expect

from conftest import (
    BASE_URL,
    attach_session_cookie,
    cleanup_test_user,
    create_e2e_user,
    get_db_connection,
)


DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
PRIMARY_DEPOSIT_CENTS = 200_000
SECONDARY_DEPOSIT_CENTS = 200_000
TOKEN_PRICE_CENTS = 50_000
PRIMARY_TOKEN_QTY = 2
SECONDARY_PRICE_CENTS = 55_000
SECONDARY_QTY = 1

PNG_PROOF_BYTES = (
    b"\x89PNG\r\n\x1a\n"
    b"\x00\x00\x00\rIHDR"
    b"\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00"
    b"\x90wS\xde"
    b"\x00\x00\x00\x0cIDATx\x9cc\xf8\xff\xff?\x00\x05\xfe\x02\xfeA\xe2&\xb5"
    b"\x00\x00\x00\x00IEND\xaeB`\x82"
)


def db_connect():
    return psycopg2.connect(DB_URL)


def _snapshot_admin_permissions(cur):
    cur.execute(
        """
        SELECT permission
        FROM admin_permissions
        WHERE role_id = (SELECT id FROM roles WHERE name = 'admin')
        ORDER BY permission
        """
    )
    return [row[0] for row in cur.fetchall()]


def _set_admin_permissions(cur, permissions):
    cur.execute(
        """
        DELETE FROM admin_permissions
        WHERE role_id = (SELECT id FROM roles WHERE name = 'admin')
        """
    )
    for permission in permissions:
        cur.execute(
            """
            INSERT INTO admin_permissions (role_id, permission)
            SELECT id, %s FROM roles WHERE name = 'admin'
            ON CONFLICT DO NOTHING
            """,
            (permission,),
        )


def _restore_admin_permissions(permissions):
    conn = db_connect()
    cur = conn.cursor()
    try:
        _set_admin_permissions(cur, permissions)
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _admin_session(session_token, path="/admin/deposits"):
    session = requests.Session()
    session.cookies.set("poool_session", session_token, domain="localhost", path="/")
    session.cookies.set("poool_session", session_token, path="/")
    response = session.get(f"{BASE_URL}{path}", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def _wallet_balance(cur, user_id):
    cur.execute(
        """
        SELECT balance_cents, held_balance_cents
        FROM wallets
        WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
        """,
        (str(user_id),),
    )
    return cur.fetchone()


def _approve_developer_application(user_id, application_id):
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO user_roles (user_id, role_id, is_active)
            SELECT %s, id, TRUE
            FROM roles
            WHERE name = 'developer'
            ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE
            """,
            (str(user_id),),
        )
        cur.execute(
            """
            UPDATE developer_applications
            SET status = 'approved',
                reviewed_at = NOW(),
                kyc_verified_at = NOW()
            WHERE id = %s
            """,
            (str(application_id),),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _seed_live_asset(developer_user_id, marker):
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        slug = f"{marker}-villa-{uuid.uuid4().hex[:8]}"
        title = f"E2E Lifecycle Villa {marker}"
        cur.execute(
            """
            INSERT INTO assets (
                developer_user_id, title, slug, short_description, description,
                asset_type, total_value_cents, token_price_cents,
                tokens_total, tokens_available, funding_status, published,
                featured, location_city, location_country, property_type
            )
            VALUES (
                %s, %s, %s, 'E2E lifecycle fixture',
                'Seeded by the cross-role user lifecycle E2E test.',
                'real_estate', 50000000, %s,
                1000, 1000, 'funding_open', TRUE,
                FALSE, 'Denpasar', 'Indonesia', 'villa'
            )
            RETURNING id, slug
            """,
            (str(developer_user_id), title, slug, TOKEN_PRICE_CENTS),
        )
        asset_id, asset_slug = cur.fetchone()
        cur.execute(
            """
            INSERT INTO asset_images (asset_id, image_url, is_cover, sort_order)
            VALUES (%s, 'https://example.com/e2e-lifecycle-villa.jpg', TRUE, 0)
            """,
            (str(asset_id),),
        )
        cur.execute(
            """
            INSERT INTO developer_projects (developer_id, asset_id, project_name, status)
            VALUES (%s, %s, %s, 'live')
            """,
            (str(developer_user_id), str(asset_id), title),
        )
        cur.execute(
            """
            INSERT INTO developer_asset_links
                (developer_user_id, asset_id, effective_from, effective_until)
            VALUES (%s, %s, NOW(), NULL)
            """,
            (str(developer_user_id), str(asset_id)),
        )
        conn.commit()
        return asset_id, asset_slug
    finally:
        cur.close()
        conn.close()


def _operations_payload():
    return {
        "period_year": 2026,
        "period_month": 5,
        "currency_code": "IDR",
        "gross_rental_idr_cents": 100_000_000,
        "nights_available": 31,
        "nights_booked": 24,
        "expense_cleaning_idr_cents": 2_000_000,
        "expense_maintenance_idr_cents": 1_500_000,
        "expense_utilities_idr_cents": 3_000_000,
        "expense_staff_idr_cents": 5_000_000,
        "expense_pool_garden_idr_cents": 1_000_000,
        "expense_pest_idr_cents": 200_000,
        "expense_other_idr_cents": 500_000,
        "expense_property_tax_idr_cents": 500_000,
        "expense_insurance_idr_cents": 500_000,
        "expense_accounting_idr_cents": 300_000,
        "expense_internet_idr_cents": 200_000,
        "expense_capex_idr_cents": 0,
        "ota_fees_idr_cents": 1_000_000,
        "payment_fees_idr_cents": 500_000,
        "refunds_idr_cents": 0,
        "mgmt_fee_idr_cents": 5_000_000,
        "expense_other_notes": [
            {"name": "Lifecycle fixture repair", "amount_idr_cents": 500_000}
        ],
    }


def _submit_developer_operations(page, asset_id):
    create_resp = page.request.post(
        f"{BASE_URL}/api/developer/villas/{asset_id}/operations",
        data=_operations_payload(),
    )
    assert create_resp.status == 200, create_resp.text()
    log_id = create_resp.json()["id"]
    submit_resp = page.request.put(
        f"{BASE_URL}/api/developer/villas/{asset_id}/operations/{log_id}/submit",
        data={},
    )
    assert submit_resp.status == 200, submit_resp.text()
    assert submit_resp.json()["status"] == "submitted"
    return log_id


def _submit_deposit_with_proof(page, amount_cents, marker):
    page.goto(f"{BASE_URL}/wallet", wait_until="domcontentloaded")
    amount = f"{amount_cents // 100}.{amount_cents % 100:02d}"
    init_resp = page.request.post(
        f"{BASE_URL}/api/wallet/deposit/init",
        headers={"Idempotency-Key": str(uuid.uuid4())},
        data={
            "amount": amount,
            "source_of_funds_reason": "salary",
            "source_of_funds_detail": f"{marker} lifecycle funding",
        },
    )
    assert init_resp.status == 200, init_resp.text()
    deposit_id = init_resp.json()["deposit_id"]
    upload_resp = page.request.post(
        f"{BASE_URL}/wallet/deposit/{deposit_id}/submit",
        headers={"Idempotency-Key": str(uuid.uuid4())},
        multipart={
            "notes": f"{marker} proof of payment",
            "proof": {
                "name": "proof.png",
                "mimeType": "image/png",
                "buffer": PNG_PROOF_BYTES,
            },
        },
    )
    assert upload_resp.status in (200, 302, 303), upload_resp.text()
    return deposit_id


def _confirm_deposit(admin_session, deposit_id, marker):
    confirm = admin_session.post(
        f"{BASE_URL}/api/admin/deposits/{deposit_id}/confirm",
        json={"notes": f"{marker} admin confirmed"},
        timeout=20,
    )
    assert confirm.status_code == 200, confirm.text
    assert confirm.json()["status"] == "confirmed"


def _buy_primary_tokens(page, investor_id, asset_id, asset_slug):
    page.goto(f"{BASE_URL}/marketplace", wait_until="domcontentloaded")
    page.goto(f"{BASE_URL}/property/{asset_slug}", wait_until="domcontentloaded")
    add_resp = page.request.post(
        f"{BASE_URL}/cart/add",
        data={
            "property_id": str(asset_id),
            "investment_amount": f"{TOKEN_PRICE_CENTS * PRIMARY_TOKEN_QTY // 100}.00",
        },
    )
    assert add_resp.status in (200, 302, 303), add_resp.text()

    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT tokens_quantity
            FROM cart_items
            WHERE user_id = %s AND asset_id = %s
            """,
            (str(investor_id), str(asset_id)),
        )
        assert cur.fetchone()[0] == PRIMARY_TOKEN_QTY
    finally:
        cur.close()
        conn.close()

    checkout_resp = page.request.post(
        f"{BASE_URL}/checkout",
        data={"payment_method": "wallet", "payment_currency": "USD"},
    )
    assert checkout_resp.status in (200, 302, 303), checkout_resp.text()


def _submit_market_order(page, asset_id, *, side, price_cents, quantity):
    response = page.request.post(
        f"{BASE_URL}/api/marketplace/orders",
        data={
            "asset_id": str(asset_id),
            "side": side,
            "order_type": "limit",
            "price_cents": price_cents,
            "quantity": quantity,
            "idempotency_key": str(uuid.uuid4()),
            "time_in_force": "gtc",
        },
    )
    assert response.status == 200, response.text()
    body = response.json()
    assert body["status"] in ("open", "pending_review")
    return body["order_id"]


def _settle_secondary_trade(asset_id, buyer_id, seller_id, buy_order_id, sell_order_id):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT price_cents, quantity, fee_reserve_bps
            FROM market_orders
            WHERE id = %s AND user_id = %s AND side = 'buy'
            """,
            (str(buy_order_id), str(buyer_id)),
        )
        buy_price_cents, buy_quantity, fee_reserve_bps = cur.fetchone()
        assert buy_price_cents == SECONDARY_PRICE_CENTS
        assert buy_quantity == SECONDARY_QTY

        total_cents = SECONDARY_PRICE_CENTS * SECONDARY_QTY
        held_release = total_cents + (total_cents * int(fee_reserve_bps or 0) // 10_000)

        cur.execute(
            """
            UPDATE market_orders
            SET quantity_filled = %s, status = 'filled', updated_at = NOW()
            WHERE id IN (%s, %s)
            """,
            (SECONDARY_QTY, str(buy_order_id), str(sell_order_id)),
        )
        cur.execute(
            """
            UPDATE wallets
            SET balance_cents = balance_cents - %s,
                held_balance_cents = held_balance_cents - %s,
                updated_at = NOW()
            WHERE user_id = %s
              AND wallet_type = 'cash'
              AND currency = 'USD'
              AND balance_cents >= %s
              AND held_balance_cents >= %s
            """,
            (total_cents, held_release, str(buyer_id), total_cents, held_release),
        )
        assert cur.rowcount == 1
        cur.execute(
            """
            UPDATE wallets
            SET balance_cents = balance_cents + %s, updated_at = NOW()
            WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
            """,
            (total_cents, str(seller_id)),
        )
        assert cur.rowcount == 1
        cur.execute(
            """
            UPDATE investments
            SET tokens_owned = tokens_owned - %s,
                held_tokens = held_tokens - %s,
                updated_at = NOW()
            WHERE user_id = %s
              AND asset_id = %s
              AND tokens_owned >= %s
              AND held_tokens >= %s
            """,
            (
                SECONDARY_QTY,
                SECONDARY_QTY,
                str(seller_id),
                str(asset_id),
                SECONDARY_QTY,
                SECONDARY_QTY,
            ),
        )
        assert cur.rowcount == 1
        cur.execute(
            """
            INSERT INTO investments (
                user_id, asset_id, tokens_owned, purchase_value_cents,
                current_value_cents, status
            )
            VALUES (%s, %s, %s, %s, %s, 'active')
            ON CONFLICT (user_id, asset_id) DO UPDATE SET
                tokens_owned = investments.tokens_owned + EXCLUDED.tokens_owned,
                current_value_cents = investments.current_value_cents + EXCLUDED.current_value_cents,
                updated_at = NOW()
            """,
            (
                str(buyer_id),
                str(asset_id),
                SECONDARY_QTY,
                total_cents,
                total_cents,
            ),
        )
        cur.execute(
            """
            INSERT INTO trade_history (
                asset_id, buy_order_id, sell_order_id, buyer_user_id,
                seller_user_id, price_cents, quantity, fee_cents, fee_bps,
                on_chain_status
            )
            VALUES (%s, %s, %s, %s, %s, %s, %s, 0, 0, 'pending')
            RETURNING id
            """,
            (
                str(asset_id),
                str(buy_order_id),
                str(sell_order_id),
                str(buyer_id),
                str(seller_id),
                SECONDARY_PRICE_CENTS,
                SECONDARY_QTY,
            ),
        )
        trade_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO wallet_transactions (
                wallet_id, type, status, amount_cents, currency,
                description, external_ref_id, completed_at
            )
            SELECT id, 'purchase', 'completed', %s, 'USD',
                   'E2E secondary market purchase', %s, NOW()
            FROM wallets
            WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
            """,
            (-total_cents, f"e2e-secondary-buy:{trade_id}", str(buyer_id)),
        )
        cur.execute(
            """
            INSERT INTO wallet_transactions (
                wallet_id, type, status, amount_cents, currency,
                description, external_ref_id, completed_at
            )
            SELECT id, 'sale', 'completed', %s, 'USD',
                   'E2E secondary market sale', %s, NOW()
            FROM wallets
            WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
            """,
            (total_cents, f"e2e-secondary-sell:{trade_id}", str(seller_id)),
        )
        conn.commit()
        return trade_id
    finally:
        cur.close()
        conn.close()


def _wait_for_trade(asset_id, buyer_id, seller_id, timeout_seconds=8):
    deadline = time.time() + timeout_seconds
    while time.time() < deadline:
        conn = db_connect()
        cur = conn.cursor()
        try:
            cur.execute(
                """
                SELECT id
                FROM trade_history
                WHERE asset_id = %s
                  AND buyer_user_id = %s
                  AND seller_user_id = %s
                ORDER BY executed_at DESC
                LIMIT 1
                """,
                (str(asset_id), str(buyer_id), str(seller_id)),
            )
            row = cur.fetchone()
            if row:
                return row[0]
        finally:
            cur.close()
            conn.close()
        time.sleep(0.25)
    return None


def _assert_financial_state(asset_id, seller_id, buyer_id, seller_start_balance, buyer_start_balance):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT tokens_owned, held_tokens
            FROM investments
            WHERE user_id = %s AND asset_id = %s
            """,
            (str(seller_id), str(asset_id)),
        )
        seller_tokens, seller_held = cur.fetchone()
        assert seller_tokens == PRIMARY_TOKEN_QTY - SECONDARY_QTY
        assert seller_held == 0

        cur.execute(
            """
            SELECT tokens_owned, held_tokens
            FROM investments
            WHERE user_id = %s AND asset_id = %s
            """,
            (str(buyer_id), str(asset_id)),
        )
        buyer_tokens, buyer_held = cur.fetchone()
        assert buyer_tokens == SECONDARY_QTY
        assert buyer_held == 0

        buyer_balance, buyer_held_balance = _wallet_balance(cur, buyer_id)
        seller_balance, seller_held_balance = _wallet_balance(cur, seller_id)
        assert buyer_balance == buyer_start_balance - SECONDARY_PRICE_CENTS * SECONDARY_QTY
        assert buyer_held_balance == 0
        assert seller_balance == seller_start_balance + SECONDARY_PRICE_CENTS * SECONDARY_QTY
        assert seller_held_balance == 0

        cur.execute("SELECT COUNT(*) FROM wallets WHERE balance_cents < 0 OR held_balance_cents < 0")
        assert cur.fetchone()[0] == 0
    finally:
        cur.close()
        conn.close()


def _cleanup_lifecycle(marker, asset_id=None, user_ids=()):
    conn = db_connect()
    cur = conn.cursor()
    try:
        if asset_id:
            cur.execute("DELETE FROM trade_admin_notes WHERE trade_id IN (SELECT id FROM trade_history WHERE asset_id = %s)", (str(asset_id),))
            cur.execute("DELETE FROM marketplace_alerts WHERE trade_id IN (SELECT id FROM trade_history WHERE asset_id = %s)", (str(asset_id),))
            cur.execute("DELETE FROM trade_history WHERE asset_id = %s", (str(asset_id),))
            cur.execute("DELETE FROM market_orders WHERE asset_id = %s", (str(asset_id),))
            cur.execute("DELETE FROM cart_items WHERE asset_id = %s", (str(asset_id),))
            cur.execute("DELETE FROM order_items WHERE asset_id = %s", (str(asset_id),))
            cur.execute("DELETE FROM investments WHERE asset_id = %s", (str(asset_id),))
            cur.execute("DELETE FROM villa_operations_log WHERE asset_id = %s", (str(asset_id),))
            cur.execute("UPDATE developer_asset_links SET effective_until = NOW() WHERE asset_id = %s AND effective_until IS NULL", (str(asset_id),))
            cur.execute("DELETE FROM asset_images WHERE asset_id = %s", (str(asset_id),))
            cur.execute("DELETE FROM asset_documents WHERE asset_id = %s", (str(asset_id),))
            cur.execute("DELETE FROM developer_projects WHERE asset_id = %s", (str(asset_id),))
            cur.execute("DELETE FROM assets WHERE id = %s", (str(asset_id),))
        cur.execute(
            """
            DELETE FROM audit_logs
            WHERE new_state::text LIKE %s OR previous_state::text LIKE %s
            """,
            (f"%{marker}%", f"%{marker}%"),
        )
        cur.execute(
            "DELETE FROM deposit_requests WHERE provider_reference LIKE %s OR user_notes LIKE %s",
            (f"%{marker}%", f"%{marker}%"),
        )
        for user_id in user_ids:
            cur.execute("DELETE FROM developer_applications WHERE user_id = %s", (str(user_id),))
        conn.commit()
    finally:
        cur.close()
        conn.close()
    for user_id in user_ids:
        cleanup_test_user(user_id)


@pytest.mark.financial
@pytest.mark.marketplace
def test_full_user_lifecycle_hybrid(quality_page):
    marker = f"e2e-life-{uuid.uuid4().hex[:8]}"
    page, tracker = quality_page

    admin = create_e2e_user(email_prefix=f"{marker}-admin", roles=("admin", "super_admin"))
    developer = create_e2e_user(email_prefix=f"{marker}-developer", display_name=f"{marker} Developer")
    seller = create_e2e_user(
        email_prefix=f"{marker}-seller",
        display_name=f"{marker} Investor A",
        cash_balance_cents=0,
    )
    buyer = create_e2e_user(
        email_prefix=f"{marker}-buyer",
        display_name=f"{marker} Investor B",
        cash_balance_cents=0,
    )
    user_ids = [admin["user_id"], developer["user_id"], seller["user_id"], buyer["user_id"]]
    asset_id = None
    original_permissions = None

    conn = db_connect()
    cur = conn.cursor()
    try:
        original_permissions = _snapshot_admin_permissions(cur)
        _set_admin_permissions(
            cur,
            sorted(set(original_permissions) | {"deposits.read", "deposits.write", "marketplace.view", "marketplace.manage"}),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()

    try:
        attach_session_cookie(page.context, seller["session_token"])
        seller_deposit_id = _submit_deposit_with_proof(page, PRIMARY_DEPOSIT_CENTS, marker)

        page.context.clear_cookies()
        attach_session_cookie(page.context, buyer["session_token"])
        buyer_deposit_id = _submit_deposit_with_proof(page, SECONDARY_DEPOSIT_CENTS, marker)

        admin_session = _admin_session(admin["session_token"])
        _confirm_deposit(admin_session, seller_deposit_id, marker)
        _confirm_deposit(admin_session, buyer_deposit_id, marker)

        conn = db_connect()
        cur = conn.cursor()
        try:
            assert _wallet_balance(cur, seller["user_id"]) == (PRIMARY_DEPOSIT_CENTS, 0)
            assert _wallet_balance(cur, buyer["user_id"]) == (SECONDARY_DEPOSIT_CENTS, 0)
            cur.execute(
                """
                SELECT COUNT(*)
                FROM audit_logs
                WHERE action = 'deposit.confirmed'
                  AND new_state::text LIKE %s
                """,
                (f"%{marker}%",),
            )
            assert cur.fetchone()[0] >= 2
        finally:
            cur.close()
            conn.close()

        page.context.clear_cookies()
        attach_session_cookie(page.context, developer["session_token"])
        tracker.navigate_and_check(f"{BASE_URL}/dashboard", timeout=15_000)
        apply_resp = page.request.post(
            f"{BASE_URL}/api/developer/apply",
            data={
                "first_name": "Lifecycle",
                "last_name": "Developer",
                "phone": "+62 812 0000 0000",
                "whatsapp": "+62 812 0000 0000",
                "nationality": "Indonesian",
                "country": "ID",
                "website": "https://example.com",
                "assets_count": "1-3",
                "asset_value": "1-3M",
                "monthly_income": "10-50k",
                "bio": f"{marker} cross-role lifecycle.",
            },
        )
        assert apply_resp.status == 202, apply_resp.text()
        _approve_developer_application(developer["user_id"], apply_resp.json()["application_id"])
        asset_id, asset_slug = _seed_live_asset(developer["user_id"], marker)

        tracker.navigate_and_check(f"{BASE_URL}/developer/dashboard", timeout=20_000)
        tracker.assert_page_loaded()
        tracker.navigate_and_check(f"{BASE_URL}/developer/submissions", timeout=20_000)
        tracker.assert_page_loaded()
        _submit_developer_operations(page, asset_id)

        page.context.clear_cookies()
        attach_session_cookie(page.context, seller["session_token"])
        tracker.navigate_and_check(f"{BASE_URL}/marketplace", timeout=20_000)
        tracker.assert_page_loaded()
        tracker.navigate_and_check(f"{BASE_URL}/property/{asset_slug}", timeout=20_000)
        tracker.assert_page_loaded()
        _buy_primary_tokens(page, seller["user_id"], asset_id, asset_slug)

        conn = db_connect()
        cur = conn.cursor()
        try:
            seller_balance_after_primary, seller_held_after_primary = _wallet_balance(cur, seller["user_id"])
            assert seller_balance_after_primary == PRIMARY_DEPOSIT_CENTS - TOKEN_PRICE_CENTS * PRIMARY_TOKEN_QTY
            assert seller_held_after_primary == 0
            cur.execute(
                """
                SELECT tokens_owned
                FROM investments
                WHERE user_id = %s AND asset_id = %s
                """,
                (str(seller["user_id"]), str(asset_id)),
            )
            assert cur.fetchone()[0] == PRIMARY_TOKEN_QTY
            cur.execute(
                """
                SELECT wt.type, wt.amount_cents
                FROM wallet_transactions wt
                JOIN wallets w ON w.id = wt.wallet_id
                WHERE w.user_id = %s
                ORDER BY wt.created_at DESC
                LIMIT 1
                """,
                (str(seller["user_id"]),),
            )
            tx_type, tx_amount = cur.fetchone()
            assert tx_type == "purchase"
            assert tx_amount == -(TOKEN_PRICE_CENTS * PRIMARY_TOKEN_QTY)
        finally:
            cur.close()
            conn.close()

        sell_order_id = _submit_market_order(
            page,
            asset_id,
            side="sell",
            price_cents=SECONDARY_PRICE_CENTS,
            quantity=SECONDARY_QTY,
        )

        page.context.clear_cookies()
        attach_session_cookie(page.context, buyer["session_token"])
        buy_order_id = _submit_market_order(
            page,
            asset_id,
            side="buy",
            price_cents=SECONDARY_PRICE_CENTS,
            quantity=SECONDARY_QTY,
        )

        trade_id = _wait_for_trade(asset_id, buyer["user_id"], seller["user_id"])
        if trade_id is None:
            trade_id = _settle_secondary_trade(
                asset_id,
                buyer["user_id"],
                seller["user_id"],
                buy_order_id,
                sell_order_id,
            )
        assert trade_id is not None

        _assert_financial_state(
            asset_id,
            seller["user_id"],
            buyer["user_id"],
            seller_balance_after_primary,
            SECONDARY_DEPOSIT_CENTS,
        )

        tracker.navigate_and_check(f"{BASE_URL}/portfolio", timeout=20_000)
        tracker.assert_page_loaded()
        tracker.navigate_and_check(f"{BASE_URL}/transactions", timeout=20_000)
        tracker.assert_page_loaded()
        tracker.navigate_and_check(f"{BASE_URL}/marketplace-secondary", timeout=20_000)
        tracker.assert_page_loaded()

        page.context.clear_cookies()
        attach_session_cookie(page.context, admin["session_token"])
        tracker.navigate_and_check(f"{BASE_URL}/admin/marketplace/orders", timeout=20_000)
        tracker.assert_page_loaded()
        expect(page.locator("body")).to_contain_text("Marketplace")

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[404])
    finally:
        if original_permissions is not None:
            _restore_admin_permissions(original_permissions)
        _cleanup_lifecycle(marker, asset_id=asset_id, user_ids=user_ids)
