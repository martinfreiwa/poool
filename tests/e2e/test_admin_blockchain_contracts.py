import os
import uuid

import psycopg2
import pytest
import requests
from playwright.sync_api import expect

from tests.e2e.conftest import cleanup_test_user, create_e2e_user


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def db_connect():
    return psycopg2.connect(DB_URL)


def valid_contract_address():
    return "0x" + uuid.uuid4().hex + uuid.uuid4().hex[:8]


def valid_tx_hash():
    return "0x" + uuid.uuid4().hex + uuid.uuid4().hex


def snapshot_admin_permissions(cur):
    cur.execute(
        """
        SELECT permission
        FROM admin_permissions
        WHERE role_id = (SELECT id FROM roles WHERE name = 'admin')
        ORDER BY permission
        """
    )
    return [row[0] for row in cur.fetchall()]


def set_admin_permissions(cur, permissions):
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


def restore_admin_permissions(permissions):
    conn = db_connect()
    cur = conn.cursor()
    try:
        set_admin_permissions(cur, permissions)
        conn.commit()
    finally:
        cur.close()
        conn.close()


def create_tokenized_asset(cur, *, title, contract_address):
    slug = f"e2e-blockchain-contracts-{uuid.uuid4().hex[:8]}"
    tx_hash = valid_tx_hash()
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published, chain_token_id, chain_contract_address,
            chain_network, chain_tx_hash
        )
        VALUES (
            %s, %s, 'E2E blockchain contracts fixture',
            'Seeded by the admin blockchain contracts E2E test.', 'real_estate',
            100000000, 10000, 10000, 9600, 'funded', TRUE,
            '777', %s, 'polygon_amoy', %s
        )
        RETURNING id
        """,
        (title, slug, contract_address, tx_hash),
    )
    return cur.fetchone()[0], tx_hash


@pytest.mark.admin
@pytest.mark.blockchain
def test_admin_blockchain_contracts_auth_safe_rendering_and_copy_feedback(quality_page):
    marker = f"e2e-contracts-{uuid.uuid4().hex[:10]}"
    malicious_title = f"{marker} <img src=x onerror=alert(1)>"
    contract_address = valid_contract_address().lower()
    denied_admin = create_e2e_user(email_prefix="e2e-contracts-denied", roles=("admin",))
    allowed_admin = create_e2e_user(email_prefix="e2e-contracts-allowed", roles=("admin",))
    asset_id = None
    original_permissions = None

    conn = db_connect()
    cur = conn.cursor()
    try:
        original_permissions = snapshot_admin_permissions(cur)
        set_admin_permissions(cur, ())
        conn.commit()

        denied_session = requests.Session()
        denied_session.cookies.set("poool_session", denied_admin["session_token"])
        denied_page = denied_session.get(f"{BASE_URL}/admin/blockchain-contracts", timeout=10)
        assert denied_page.url.endswith("/admin/")
        denied_api = denied_session.get(f"{BASE_URL}/api/admin/blockchain/treasury", timeout=10)
        assert denied_api.status_code == 403

        unauth_page = requests.get(f"{BASE_URL}/admin/blockchain-contracts", timeout=10)
        assert unauth_page.status_code in (200, 303)
        assert unauth_page.url.endswith("/auth/login") or unauth_page.status_code == 303
        unauth_api = requests.get(f"{BASE_URL}/api/admin/blockchain/treasury", timeout=10)
        assert unauth_api.status_code == 401

        set_admin_permissions(cur, ("treasury.read",))
        asset_id, tx_hash = create_tokenized_asset(
            cur, title=malicious_title, contract_address=contract_address
        )
        conn.commit()

        allowed_session = requests.Session()
        allowed_session.cookies.set("poool_session", allowed_admin["session_token"])
        allowed_page = allowed_session.get(f"{BASE_URL}/admin/blockchain-contracts", timeout=10)
        assert allowed_page.status_code == 200, allowed_page.text[:500]
        api = allowed_session.get(f"{BASE_URL}/api/admin/blockchain/treasury", timeout=10)
        assert api.status_code == 200, api.text
        payload = api.json()
        seeded = [
            asset for asset in payload["tokenized_assets"]
            if asset["chain_contract_address"] == contract_address
        ]
        assert seeded, "Seeded tokenized asset missing from treasury payload"
        assert seeded[0]["title"] == malicious_title
        assert seeded[0]["chain_tx_hash"] == tx_hash

        page, tracker = quality_page
        page.context.add_cookies(
            [{"name": "poool_session", "value": allowed_admin["session_token"], "url": BASE_URL}]
        )
        page.add_init_script(
            """
            Object.defineProperty(navigator, 'clipboard', {
              configurable: true,
              value: { writeText: async (text) => { window.__copiedContract = text; } }
            });
            """
        )
        tracker.navigate_and_check(f"{BASE_URL}/admin/blockchain-contracts")
        expect(page.get_by_text(malicious_title, exact=True)).to_be_visible(timeout=10_000)
        expect(page.locator("#kpi-active-clones")).not_to_have_text("—")
        assert page.locator("img[src='x']").count() == 0
        expect(page.locator(f"a[href*='{contract_address}']").first).to_have_attribute(
            "rel", "noopener noreferrer"
        )
        expect(page.get_by_text("Tx History").first).to_have_attribute(
            "rel", "noopener noreferrer"
        )

        page.get_by_role("button", name=f"Copy contract address {contract_address}").click()
        expect(page.locator("#contracts-status")).to_contain_text(
            "Contract address copied.", timeout=5_000
        )
        assert page.evaluate("window.__copiedContract") == contract_address
        tracker.assert_no_critical_errors()
    finally:
        conn.rollback()
        if original_permissions is not None:
            restore_admin_permissions(original_permissions)
        if asset_id is not None:
            cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
            conn.commit()
        cur.close()
        conn.close()
        cleanup_test_user(denied_admin["user_id"])
        cleanup_test_user(allowed_admin["user_id"])
