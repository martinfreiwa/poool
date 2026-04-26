import os
import uuid

import psycopg2
import pytest
import requests
from playwright.sync_api import expect


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def db_connect():
    return psycopg2.connect(DB_URL)


def valid_contract_address():
    return "0x" + uuid.uuid4().hex + uuid.uuid4().hex[:8]


def create_user(cur, *, email_prefix, roles=(), wallet_address=None):
    email = f"{email_prefix}-{uuid.uuid4().hex[:8]}@poool.app"
    session_token = str(uuid.uuid4())
    cur.execute(
        """
        INSERT INTO users (email, email_verified, status, chain_wallet_address)
        VALUES (%s, TRUE, 'active', %s)
        RETURNING id, email
        """,
        (email, wallet_address),
    )
    user_id, email = cur.fetchone()
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


def create_asset(cur, contract_address):
    slug = f"e2e-blockchain-contract-{uuid.uuid4().hex[:8]}"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published, chain_token_id, chain_contract_address,
            chain_network, chain_tx_hash
        )
        VALUES (
            'E2E Blockchain Contract Detail Asset', %s, 'E2E blockchain fixture',
            'Seeded by the admin blockchain contract detail E2E test.', 'real_estate',
            100000000, 10000, 10000, 9600, 'funded', TRUE,
            '1', %s, 'polygon_amoy',
            '0x1111111111111111111111111111111111111111111111111111111111111111'
        )
        RETURNING id
        """,
        (slug, contract_address),
    )
    return cur.fetchone()[0]


def authenticated_session(session_token, page_path):
    session = requests.Session()
    session.cookies.set("poool_session", session_token)
    response = session.get(f"{BASE_URL}{page_path}", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def assert_control_state(cur, *, asset_id, contract_address, is_paused):
    cur.execute(
        """
        SELECT asset_id, is_paused, last_action, last_tx_hash IS NOT NULL
        FROM chain_contract_controls
        WHERE contract_address = %s
        """,
        (contract_address.lower(),),
    )
    row = cur.fetchone()
    assert row is not None
    assert row[0] == asset_id
    assert row[1] is is_paused
    assert row[2] == ("pause" if is_paused else "unpause")
    assert row[3] is True


def audit_count(cur, *, asset_id, action):
    cur.execute(
        """
        SELECT COUNT(*)
        FROM audit_logs
        WHERE entity_type = 'contract'
          AND entity_id = %s
          AND action = %s
        """,
        (asset_id, action),
    )
    return cur.fetchone()[0]


@pytest.mark.admin
@pytest.mark.blockchain
def test_admin_blockchain_contract_detail_permissions_state_audit_and_ui(quality_page):
    created_user_ids = []
    asset_id = None
    holder_wallet = valid_contract_address()
    contract_address = valid_contract_address().lower()
    unmapped_address = valid_contract_address().lower()

    conn = db_connect()
    conn.autocommit = False
    cur = conn.cursor()
    try:
        normal_admin = create_user(cur, email_prefix="e2e-chain-admin", roles=("admin",))
        super_admin = create_user(
            cur,
            email_prefix="e2e-chain-super",
            roles=("admin", "super_admin"),
        )
        holder = create_user(
            cur,
            email_prefix="e2e-chain-holder",
            roles=("investor",),
            wallet_address=holder_wallet,
        )
        created_user_ids.extend([normal_admin["id"], super_admin["id"], holder["id"]])

        asset_id = create_asset(cur, contract_address)
        cur.execute(
            """
            INSERT INTO chain_contract_controls (
                contract_address, asset_id, is_paused, last_action, updated_by
            )
            VALUES (%s, %s, FALSE, 'sync', %s)
            """,
            (contract_address, asset_id, super_admin["id"]),
        )
        cur.execute(
            """
            INSERT INTO onchain_balances (
                user_id, asset_id, balance, last_synced_block, last_synced_at
            )
            VALUES (%s, %s, 250, 12345, NOW())
            """,
            (holder["id"], asset_id),
        )
        conn.commit()

        detail_path = f"/admin/blockchain-contract-detail?address={contract_address}"
        super_session = authenticated_session(super_admin["session_token"], detail_path)
        detail = super_session.get(
            f"{BASE_URL}/api/admin/blockchain/contracts/{contract_address}/detail",
            timeout=10,
        )
        assert detail.status_code == 200, detail.text
        payload = detail.json()
        assert payload["title"] == "E2E Blockchain Contract Detail Asset"
        assert payload["pause_state"] == "live"
        assert payload["is_paused"] is False
        assert payload["tokens_sold"] == 400
        assert len(payload["holders"]) == 1
        assert payload["holders"][0]["wallet_address"].lower() == holder_wallet.lower()

        normal_session = authenticated_session(normal_admin["session_token"], detail_path)
        denied = normal_session.post(
            f"{BASE_URL}/api/admin/blockchain/contracts/{contract_address}/pause",
            timeout=10,
        )
        assert denied.status_code == 403

        no_csrf = requests.Session()
        no_csrf.cookies.set("poool_session", super_admin["session_token"])
        csrf_denied = no_csrf.post(
            f"{BASE_URL}/api/admin/blockchain/contracts/{contract_address}/pause",
            timeout=10,
        )
        assert csrf_denied.status_code == 403

        invalid = super_session.post(
            f"{BASE_URL}/api/admin/blockchain/contracts/not-a-contract/pause",
            timeout=10,
        )
        assert invalid.status_code == 400

        unmapped = super_session.post(
            f"{BASE_URL}/api/admin/blockchain/contracts/{unmapped_address}/pause",
            timeout=10,
        )
        assert unmapped.status_code == 404

        paused = super_session.post(
            f"{BASE_URL}/api/admin/blockchain/contracts/{contract_address}/pause",
            timeout=10,
        )
        assert paused.status_code == 200, paused.text
        paused_payload = paused.json()
        assert paused_payload["mocked"] in (True, False)
        assert paused_payload["action"] == "paused"
        assert_control_state(cur, asset_id=asset_id, contract_address=contract_address, is_paused=True)
        assert audit_count(cur, asset_id=asset_id, action="blockchain.clone_pause") == 1

        unpaused = super_session.post(
            f"{BASE_URL}/api/admin/blockchain/contracts/{contract_address}/unpause",
            timeout=10,
        )
        assert unpaused.status_code == 200, unpaused.text
        unpaused_payload = unpaused.json()
        assert unpaused_payload["mocked"] in (True, False)
        assert unpaused_payload["action"] == "unpaused"
        assert_control_state(cur, asset_id=asset_id, contract_address=contract_address, is_paused=False)
        assert audit_count(cur, asset_id=asset_id, action="blockchain.clone_unpause") == 1

        page, tracker = quality_page
        page.context.add_cookies(
            [{"name": "poool_session", "value": super_admin["session_token"], "url": BASE_URL}]
        )
        tracker.navigate_and_check(f"{BASE_URL}{detail_path}")
        expect(page.locator("#page-asset-title")).to_have_text("E2E Blockchain Contract Detail Asset")
        expect(page.locator("#kpi-live-status")).to_contain_text("Live Clone")
        expect(page.locator("#holders-tbody")).to_contain_text(holder["email"])
        expect(page.locator("#btn-manual-netting")).to_be_disabled()
        expect(page.locator("#contract-link")).to_have_attribute("rel", "noopener noreferrer")
        expect(page.locator("#btn-freeze-transfers")).to_be_enabled()

        with page.expect_response(
            lambda response: response.url.endswith(f"/api/admin/blockchain/contracts/{contract_address}/pause")
            and response.request.method == "POST",
            timeout=10_000,
        ) as pause_response:
            page.locator("#btn-freeze-transfers").click()
            expect(page.locator(".pc-overlay")).to_be_visible()
            page.get_by_role("button", name="Freeze Transfers").click()
        assert pause_response.value.status == 200
        expect(page.locator("#kpi-live-status")).to_contain_text("Contract Paused", timeout=10_000)
        tracker.assert_no_critical_errors()
    finally:
        conn.rollback()
        if asset_id is not None:
            cur.execute("DELETE FROM audit_logs WHERE entity_id = %s", (asset_id,))
            cur.execute("DELETE FROM onchain_balances WHERE asset_id = %s", (asset_id,))
            cur.execute("DELETE FROM chain_contract_controls WHERE asset_id = %s", (asset_id,))
            cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
        if created_user_ids:
            cur.execute(
                "DELETE FROM audit_logs WHERE actor_user_id = ANY(%s::uuid[])",
                ([str(uid) for uid in created_user_ids],),
            )
            cur.execute(
                "DELETE FROM user_sessions WHERE user_id = ANY(%s::uuid[])",
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
