import os
import uuid

import psycopg2
import pytest
import requests
from playwright.sync_api import expect


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
TOKEN_PRICE_CENTS = 10_000
TOKENS_SOLD = 600
TOKENS_TOTAL = 1_000
TOKENS_AVAILABLE = TOKENS_TOTAL - TOKENS_SOLD
SOFT_CAP_TOKENS = 500
ESCROW_BALANCE_CENTS = TOKEN_PRICE_CENTS * TOKENS_SOLD


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
            ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE
            """,
            (user_id, role),
        )
    return {"id": user_id, "email": email, "session_token": session_token}


def grant_role_permissions(cur, *, role, permissions):
    for permission in permissions:
        cur.execute(
            """
            INSERT INTO admin_permissions (role_id, permission)
            SELECT id, %s FROM roles WHERE name = %s
            ON CONFLICT (role_id, permission) DO NOTHING
            """,
            (permission, role),
        )


def create_primary_escrow_fixture(cur, *, investor_id):
    slug = f"e2e-primary-escrow-{uuid.uuid4().hex[:8]}"
    title = "E2E Primary Escrow Release Asset"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            min_funding_tokens, funding_status, published, escrow_agent, funding_end_at
        )
        VALUES (
            %s, %s, 'E2E primary escrow fixture',
            'Seeded by the admin primary escrow E2E test.', 'real_estate',
            %s, %s, %s, %s, %s, 'funding_in_progress', TRUE,
            'E2E escrow agent', NOW() + INTERVAL '30 days'
        )
        RETURNING id
        """,
        (
            title,
            slug,
            TOKEN_PRICE_CENTS * TOKENS_TOTAL,
            TOKEN_PRICE_CENTS,
            TOKENS_TOTAL,
            TOKENS_AVAILABLE,
            SOFT_CAP_TOKENS,
        ),
    )
    asset_id = cur.fetchone()[0]
    cur.execute(
        """
        INSERT INTO investments (
            user_id, asset_id, tokens_owned, purchase_value_cents,
            current_value_cents, status
        )
        VALUES (%s, %s, %s, %s, %s, 'funding_in_progress')
        """,
        (investor_id, asset_id, TOKENS_SOLD, ESCROW_BALANCE_CENTS, ESCROW_BALANCE_CENTS),
    )
    order_number = f"E2E-PE-{uuid.uuid4().hex[:10]}"
    cur.execute(
        """
        INSERT INTO orders (user_id, order_number, total_cents, status, payment_method)
        VALUES (%s, %s, %s, 'pending', 'bank_transfer')
        RETURNING id
        """,
        (investor_id, order_number, ESCROW_BALANCE_CENTS),
    )
    order_id = cur.fetchone()[0]
    cur.execute(
        """
        INSERT INTO order_items (
            order_id, asset_id, tokens_quantity, token_price_cents, subtotal_cents
        )
        VALUES (%s, %s, %s, %s, %s)
        """,
        (order_id, asset_id, TOKENS_SOLD, TOKEN_PRICE_CENTS, ESCROW_BALANCE_CENTS),
    )
    return asset_id, title, order_id


def authenticated_session(session_token, page_path):
    session = requests.Session()
    session.cookies.set("poool_session", session_token)
    response = session.get(f"{BASE_URL}{page_path}", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def approval_count(cur, *, asset_id, action):
    cur.execute(
        """
        SELECT COUNT(*)
        FROM audit_logs
        WHERE entity_id = %s
          AND action = %s
        """,
        (asset_id, action),
    )
    return cur.fetchone()[0]


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_primary_escrow_release_requires_permission_and_four_eyes_approval(quality_page):
    created_user_ids = []
    approval_id = None
    asset_id = None
    order_id = None

    conn = db_connect()
    conn.autocommit = False
    cur = conn.cursor()
    try:
        grant_role_permissions(
            cur,
            role="admin",
            permissions=("marketplace.view", "marketplace.manage", "approvals.manage"),
        )
        maker = create_user(cur, email_prefix="e2e-pe-maker", roles=("admin",))
        approver = create_user(cur, email_prefix="e2e-pe-approver", roles=("admin",))
        viewer = create_user(cur, email_prefix="e2e-pe-viewer")
        investor = create_user(cur, email_prefix="e2e-pe-investor")
        created_user_ids.extend([maker["id"], approver["id"], viewer["id"], investor["id"]])
        asset_id, asset_title, order_id = create_primary_escrow_fixture(
            cur, investor_id=investor["id"]
        )
        conn.commit()

        viewer_session = requests.Session()
        viewer_session.cookies.set("poool_session", viewer["session_token"])
        denied = viewer_session.get(f"{BASE_URL}/api/admin/primary-escrow", timeout=10)
        assert denied.status_code == 403

        maker_session = authenticated_session(
            maker["session_token"], "/admin/marketplace/primary-escrow"
        )
        list_response = maker_session.get(f"{BASE_URL}/api/admin/primary-escrow", timeout=10)
        assert list_response.status_code == 200, list_response.text
        seeded = [
            campaign
            for campaign in list_response.json()
            if campaign["asset_id"] == str(asset_id)
        ]
        assert seeded
        assert seeded[0]["current_escrow_cents"] == ESCROW_BALANCE_CENTS
        assert seeded[0]["release_ready"] is True

        page, tracker = quality_page
        page.context.add_cookies(
            [{"name": "poool_session", "value": maker["session_token"], "url": BASE_URL}]
        )
        tracker.navigate_and_check(f"{BASE_URL}/admin/marketplace/primary-escrow")
        tracker.assert_page_loaded()
        card = page.locator(".escrow-card").filter(has_text=asset_title).first
        expect(card).to_be_visible()
        card.locator('[name="notarization_reference"]').fill("E2E-NOTARY-001")
        card.locator('[name="reason"]').fill("E2E release after soft cap")
        with page.expect_response(
            lambda response: "/api/admin/primary-escrow/" in response.url
            and response.request.method == "POST"
        ) as release_response:
            card.get_by_role("button", name="Request Release").click()
        release_result = release_response.value
        assert release_result.status == 200
        release_body = release_result.json()
        approval_id = release_body["approval_id"]
        expect(card.locator(".escrow-release-feedback")).to_contain_text(
            "Approval request created"
        )
        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures()

        self_approve = maker_session.post(
            f"{BASE_URL}/api/admin/approvals/{approval_id}/approve",
            json={},
            timeout=10,
        )
        assert self_approve.status_code == 403

        approver_session = authenticated_session(approver["session_token"], "/admin/approvals")
        approve = approver_session.post(
            f"{BASE_URL}/api/admin/approvals/{approval_id}/approve",
            json={},
            timeout=10,
        )
        assert approve.status_code == 200, approve.text
        assert approve.json()["status"] == "approved"

        cur.execute("SELECT funding_status FROM assets WHERE id = %s", (asset_id,))
        assert cur.fetchone()[0] == "funded"
        cur.execute(
            "SELECT status FROM investments WHERE asset_id = %s AND user_id = %s",
            (asset_id, investor["id"]),
        )
        assert cur.fetchone()[0] == "active"
        cur.execute("SELECT status FROM orders WHERE id = %s", (order_id,))
        assert cur.fetchone()[0] == "completed"
        assert approval_count(cur, asset_id=asset_id, action="primary_escrow.release_requested") == 1
        assert approval_count(cur, asset_id=asset_id, action="primary_escrow.released") == 1
    finally:
        conn.rollback()
        if approval_id is not None:
            cur.execute("DELETE FROM audit_logs WHERE entity_id = %s", (approval_id,))
            cur.execute("DELETE FROM admin_approval_requests WHERE id = %s", (approval_id,))
        if asset_id is not None:
            cur.execute("DELETE FROM audit_logs WHERE entity_id = %s", (asset_id,))
            cur.execute("DELETE FROM order_items WHERE asset_id = %s", (asset_id,))
            cur.execute("DELETE FROM investments WHERE asset_id = %s", (asset_id,))
            cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
        if order_id is not None:
            cur.execute("DELETE FROM orders WHERE id = %s", (order_id,))
        if created_user_ids:
            ids = [str(uid) for uid in created_user_ids]
            cur.execute("DELETE FROM user_sessions WHERE user_id = ANY(%s::uuid[])", (ids,))
            cur.execute("DELETE FROM wallets WHERE user_id = ANY(%s::uuid[])", (ids,))
            cur.execute("DELETE FROM user_roles WHERE user_id = ANY(%s::uuid[])", (ids,))
            cur.execute("DELETE FROM users WHERE id = ANY(%s::uuid[])", (ids,))
        conn.commit()
        cur.close()
        conn.close()
