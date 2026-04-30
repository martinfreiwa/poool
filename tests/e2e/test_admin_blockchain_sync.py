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


def admin_session(session_token):
    session = requests.Session()
    session.cookies.set("poool_session", session_token, domain="localhost", path="/")
    session.cookies.set("poool_session", session_token, path="/")
    response = session.get(f"{BASE_URL}/admin/blockchain-sync", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin blockchain sync page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def wallet_address_for(user_id):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute("SELECT chain_wallet_address FROM users WHERE id = %s", (user_id,))
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def force_sync_audit_count(actor_id, target_id):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT COUNT(*)
            FROM audit_logs
            WHERE actor_user_id = %s
              AND action = 'blockchain.force_kyc_sync'
              AND entity_type = 'user'
              AND entity_id = %s
              AND new_state->>'triggered_by' = 'admin_force_sync'
            """,
            (actor_id, target_id),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def cleanup_sync_fixture(*user_ids):
    conn = db_connect()
    cur = conn.cursor()
    try:
        for user_id in user_ids:
            cur.execute(
                """
                DELETE FROM audit_logs
                WHERE action = 'blockchain.force_kyc_sync'
                  AND (actor_user_id = %s OR entity_id = %s)
                """,
                (user_id, user_id),
            )
        conn.commit()
    finally:
        cur.close()
        conn.close()
    for user_id in user_ids:
        cleanup_test_user(user_id)


@pytest.mark.admin
@pytest.mark.blockchain
def test_admin_blockchain_sync_auth_csrf_force_sync_and_ui(quality_page):
    marker = uuid.uuid4().hex[:10]
    denied_admin = create_e2e_user(email_prefix="e2e-sync-denied", roles=("admin",))
    read_only_admin = create_e2e_user(email_prefix="e2e-sync-readonly", roles=("admin",))
    manage_admin = create_e2e_user(email_prefix="e2e-sync-manage", roles=("admin",))
    pending_target = create_e2e_user(
        email_prefix=f"e2e-sync-pending-{marker}",
        kyc_status="pending",
    )
    eligible_target = create_e2e_user(
        email_prefix=f"e2e-sync-eligible-{marker}",
        kyc_status="approved",
    )
    original_permissions = None

    conn = db_connect()
    cur = conn.cursor()
    try:
        original_permissions = snapshot_admin_permissions(cur)
        set_admin_permissions(cur, ())
        conn.commit()

        denied_session = requests.Session()
        denied_session.cookies.set("poool_session", denied_admin["session_token"])
        denied_page = denied_session.get(f"{BASE_URL}/admin/blockchain-sync", timeout=10)
        assert denied_page.url.endswith("/admin/")
        denied_api = denied_session.get(f"{BASE_URL}/api/admin/blockchain/sync", timeout=10)
        assert denied_api.status_code == 403

        unauth_page = requests.get(f"{BASE_URL}/admin/blockchain-sync", timeout=10)
        assert unauth_page.status_code in (200, 303)
        assert unauth_page.url.endswith("/auth/login") or unauth_page.status_code == 303
        unauth_api = requests.get(f"{BASE_URL}/api/admin/blockchain/sync", timeout=10)
        assert unauth_api.status_code == 401

        set_admin_permissions(cur, ("treasury.read",))
        conn.commit()

        read_session = admin_session(read_only_admin["session_token"])
        sync_response = read_session.get(f"{BASE_URL}/api/admin/blockchain/sync", timeout=10)
        assert sync_response.status_code == 200, sync_response.text
        payload = sync_response.json()
        assert "indexer" in payload
        assert "settlement" in payload
        assert "whitelist_queue" in payload

        no_csrf = requests.Session()
        no_csrf.cookies.set("poool_session", read_only_admin["session_token"])
        no_csrf.cookies.set("csrf_token", read_session.cookies.get("csrf_token"))
        csrf_denied = no_csrf.post(
            f"{BASE_URL}/api/admin/blockchain/force-kyc-sync/{eligible_target['user_id']}",
            timeout=10,
        )
        assert csrf_denied.status_code == 403

        missing_manage = read_session.post(
            f"{BASE_URL}/api/admin/blockchain/force-kyc-sync/{eligible_target['user_id']}",
            timeout=10,
        )
        assert missing_manage.status_code == 403
        assert wallet_address_for(eligible_target["user_id"]) is None

        set_admin_permissions(cur, ("treasury.read", "blockchain.manage"))
        conn.commit()

        manage_session = admin_session(manage_admin["session_token"])
        ineligible = manage_session.post(
            f"{BASE_URL}/api/admin/blockchain/force-kyc-sync/{pending_target['user_id']}",
            timeout=10,
        )
        assert ineligible.status_code == 400
        assert "approved KYC" in ineligible.text
        assert wallet_address_for(pending_target["user_id"]) is None

        synced = manage_session.post(
            f"{BASE_URL}/api/admin/blockchain/force-kyc-sync/{eligible_target['user_id']}",
            timeout=10,
        )
        assert synced.status_code == 200, synced.text
        synced_payload = synced.json()
        assert synced_payload["success"] is True
        assert synced_payload["user_id"] == str(eligible_target["user_id"])
        assert synced_payload["wallet_address"].startswith("0x")
        assert wallet_address_for(eligible_target["user_id"]) == synced_payload["wallet_address"]
        assert force_sync_audit_count(manage_admin["user_id"], eligible_target["user_id"]) == 1

        repeat = manage_session.post(
            f"{BASE_URL}/api/admin/blockchain/force-kyc-sync/{eligible_target['user_id']}",
            timeout=10,
        )
        assert repeat.status_code == 400
        assert "already has a wallet address" in repeat.text

        page, tracker = quality_page
        page.context.add_cookies(
            [{"name": "poool_session", "value": manage_admin["session_token"], "url": BASE_URL}]
        )
        tracker.navigate_and_check(f"{BASE_URL}/admin/blockchain-sync")
        expect(page.locator("#kpi-indexer-status")).not_to_contain_text("Loading", timeout=10_000)
        expect(page.locator("#whitelist-count-badge")).to_contain_text("pending", timeout=10_000)
        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[403])
    finally:
        conn.rollback()
        if original_permissions is not None:
            restore_admin_permissions(original_permissions)
        cur.close()
        conn.close()
        cleanup_sync_fixture(
            denied_admin["user_id"],
            read_only_admin["user_id"],
            manage_admin["user_id"],
            pending_target["user_id"],
            eligible_target["user_id"],
        )
