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


def admin_session(session_token, path="/admin/deposits"):
    session = requests.Session()
    session.cookies.set("poool_session", session_token, domain="localhost", path="/")
    session.cookies.set("poool_session", session_token, path="/")
    response = session.get(f"{BASE_URL}{path}", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin deposits page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def seed_deposit(cur, user_id, marker, *, amount_cents=12_345, status="pending"):
    provider_reference = f"{marker}-deposit-{uuid.uuid4().hex[:8]}"
    cur.execute(
        """
        INSERT INTO deposit_requests (
            user_id, currency, amount_cents, provider, provider_reference, status, expires_at
        )
        VALUES (%s, 'USD', %s, 'manual', %s, %s, NOW() + INTERVAL '24 hours')
        RETURNING id
        """,
        (user_id, amount_cents, provider_reference, status),
    )
    return cur.fetchone()[0], provider_reference


def seed_dispute(cur, user_id, marker):
    provider_dispute_id = f"{marker}-dispute-{uuid.uuid4().hex[:8]}"
    cur.execute(
        """
        INSERT INTO payment_disputes (
            user_id, provider, provider_dispute_id, amount_cents, currency, reason, status
        )
        VALUES (%s, 'stripe', %s, 7000, 'USD', 'E2E dispute', 'open')
        RETURNING id
        """,
        (user_id, provider_dispute_id),
    )
    return cur.fetchone()[0], provider_dispute_id


def cleanup_marker(marker):
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            DELETE FROM audit_logs
            WHERE action IN (
                'deposit.confirmed',
                'admin.deposit_cancel',
                'admin.deposit_extend_expiry',
                'admin.dispute_status_update',
                'admin.dispute_evidence_bundle_generated'
            )
            AND (
                new_state::text LIKE %s
                OR previous_state::text LIKE %s
                OR entity_id IN (
                    SELECT id FROM deposit_requests WHERE provider_reference LIKE %s
                    UNION
                    SELECT id FROM payment_disputes WHERE provider_dispute_id LIKE %s
                )
            )
            """,
            (f"%{marker}%", f"%{marker}%", f"%{marker}%", f"%{marker}%"),
        )
        cur.execute(
            "DELETE FROM payment_disputes WHERE provider_dispute_id LIKE %s",
            (f"%{marker}%",),
        )
        cur.execute(
            "DELETE FROM deposit_requests WHERE provider_reference LIKE %s",
            (f"%{marker}%",),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()


def wallet_balance(cur, user_id):
    cur.execute(
        """
        SELECT balance_cents
        FROM wallets
        WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
        """,
        (user_id,),
    )
    return cur.fetchone()[0]


@pytest.mark.admin
@pytest.mark.financial
def test_admin_deposits_permissions_mutations_audit_and_ui(quality_page):
    marker = f"e2e-deposits-{uuid.uuid4().hex[:10]}"
    admin = create_e2e_user(email_prefix="e2e-deposits-admin", roles=("admin",))
    denied_admin = create_e2e_user(email_prefix="e2e-deposits-denied", roles=("admin",))
    deposit_user = create_e2e_user(
        email_prefix="e2e-deposit-user",
        display_name=f"E2E Deposit User {marker}",
        cash_balance_cents=0,
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
        denied_page = denied_session.get(f"{BASE_URL}/admin/deposits", timeout=10)
        assert denied_page.url.endswith("/admin/")
        denied_api = denied_session.get(f"{BASE_URL}/api/admin/deposits", timeout=10)
        assert denied_api.status_code == 403

        allowed_permissions = sorted(set(original_permissions) | {"deposits.read", "deposits.write"})
        set_admin_permissions(cur, allowed_permissions)
        confirm_deposit_id, _ = seed_deposit(cur, deposit_user["user_id"], marker)
        csrf_deposit_id, _ = seed_deposit(cur, deposit_user["user_id"], marker)
        cancel_deposit_id, _ = seed_deposit(cur, deposit_user["user_id"], marker)
        extend_deposit_id, _ = seed_deposit(cur, deposit_user["user_id"], marker)
        dispute_id, provider_dispute_id = seed_dispute(cur, deposit_user["user_id"], marker)
        starting_balance = wallet_balance(cur, deposit_user["user_id"])
        conn.commit()

        session = admin_session(admin["session_token"])
        deposits_response = session.get(f"{BASE_URL}/api/admin/deposits", timeout=10)
        assert deposits_response.status_code == 200, deposits_response.text
        deposits = deposits_response.json()["deposits"]
        assert any(item["id"] == str(confirm_deposit_id) for item in deposits)
        assert all("provider" in item for item in deposits)

        no_csrf = requests.Session()
        no_csrf.cookies.set("poool_session", admin["session_token"])
        no_csrf.cookies.set("csrf_token", session.cookies.get("csrf_token"))
        csrf_denied = no_csrf.post(
            f"{BASE_URL}/api/admin/deposits/{csrf_deposit_id}/confirm",
            json={"notes": f"{marker} missing csrf"},
            timeout=10,
        )
        assert csrf_denied.status_code == 403
        cur.execute("SELECT status FROM deposit_requests WHERE id = %s", (csrf_deposit_id,))
        assert cur.fetchone()[0] == "pending"

        confirm_notes = f"{marker} admin confirmed"
        confirm = session.post(
            f"{BASE_URL}/api/admin/deposits/{confirm_deposit_id}/confirm",
            json={"notes": confirm_notes},
            timeout=20,
        )
        assert confirm.status_code == 200, confirm.text
        assert confirm.json()["status"] == "confirmed"

        cur.execute(
            """
            SELECT d.status, d.paid_at IS NOT NULL, w.balance_cents
            FROM deposit_requests d
            JOIN wallets w ON w.user_id = d.user_id
            WHERE d.id = %s AND w.wallet_type = 'cash' AND w.currency = 'USD'
            """,
            (confirm_deposit_id,),
        )
        status, has_paid_at, balance_after_confirm = cur.fetchone()
        assert status == "paid"
        assert has_paid_at is True
        assert balance_after_confirm == starting_balance + 12_345

        cur.execute(
            """
            SELECT COUNT(*)
            FROM audit_logs
            WHERE actor_user_id = %s
              AND action = 'deposit.confirmed'
              AND entity_type = 'deposit_request'
              AND entity_id = %s
              AND new_state->>'admin_notes' = %s
            """,
            (admin["user_id"], confirm_deposit_id, confirm_notes),
        )
        assert cur.fetchone()[0] == 1

        cancel_reason = f"{marker} cancelled by test"
        cancel = session.post(
            f"{BASE_URL}/api/admin/deposits/{cancel_deposit_id}/cancel",
            json={"reason": cancel_reason},
            timeout=10,
        )
        assert cancel.status_code == 200, cancel.text
        assert cancel.json()["status"] == "cancelled"
        cur.execute("SELECT status FROM deposit_requests WHERE id = %s", (cancel_deposit_id,))
        assert cur.fetchone()[0] == "cancelled"
        cur.execute(
            """
            SELECT COUNT(*)
            FROM audit_logs
            WHERE actor_user_id = %s
              AND action = 'admin.deposit_cancel'
              AND entity_id = %s
              AND previous_state->>'status' = 'pending'
              AND new_state->>'status' = 'cancelled'
              AND new_state->>'reason' = %s
            """,
            (admin["user_id"], cancel_deposit_id, cancel_reason),
        )
        assert cur.fetchone()[0] == 1

        extend = session.post(
            f"{BASE_URL}/api/admin/deposits/{extend_deposit_id}/extend",
            timeout=10,
        )
        assert extend.status_code == 200, extend.text
        assert extend.json()["extended_by_hours"] == 48
        cur.execute("SELECT status FROM deposit_requests WHERE id = %s", (extend_deposit_id,))
        assert cur.fetchone()[0] == "pending"
        cur.execute(
            """
            SELECT COUNT(*)
            FROM audit_logs
            WHERE actor_user_id = %s
              AND action = 'admin.deposit_extend_expiry'
              AND entity_id = %s
              AND previous_state->>'status' = 'pending'
              AND new_state->>'extended_by_hours' = '48'
            """,
            (admin["user_id"], extend_deposit_id),
        )
        assert cur.fetchone()[0] == 1

        disputes_response = session.get(f"{BASE_URL}/api/admin/disputes", timeout=10)
        assert disputes_response.status_code == 200, disputes_response.text
        disputes = disputes_response.json()["disputes"]
        assert any(item["provider_dispute_id"] == provider_dispute_id for item in disputes)

        no_csrf_evidence = no_csrf.post(
            f"{BASE_URL}/api/admin/disputes/{dispute_id}/evidence",
            timeout=10,
        )
        assert no_csrf_evidence.status_code == 403

        dispute_update = session.put(
            f"{BASE_URL}/api/admin/disputes/{dispute_id}/status",
            json={"status": "under_review"},
            timeout=10,
        )
        assert dispute_update.status_code == 200, dispute_update.text
        cur.execute("SELECT status FROM payment_disputes WHERE id = %s", (dispute_id,))
        assert cur.fetchone()[0] == "under_review"
        cur.execute(
            """
            SELECT COUNT(*)
            FROM audit_logs
            WHERE actor_user_id = %s
              AND action = 'admin.dispute_status_update'
              AND entity_id = %s
              AND previous_state->>'status' = 'open'
              AND new_state->>'new_status' = 'under_review'
            """,
            (admin["user_id"], dispute_id),
        )
        assert cur.fetchone()[0] == 1

        page, tracker = quality_page
        page.context.add_cookies(
            [{"name": "poool_session", "value": admin["session_token"], "url": BASE_URL}]
        )
        with page.expect_response(
            lambda response: "/api/admin/deposits" in response.url and response.status == 200
        ):
            tracker.navigate_and_check(f"{BASE_URL}/admin/deposits")

        expect(page.locator("#deposits-table-body")).to_contain_text(marker)
        sort_header = page.locator("th[data-sort='provider']")
        expect(sort_header).to_have_attribute("role", "button")
        sort_header.focus()
        expect(sort_header).to_be_focused()
        page.keyboard.press("Enter")
        expect(sort_header).to_have_attribute("aria-sort", "ascending")
        expect(page.locator("#confirm-modal")).to_have_attribute("role", "dialog")
        with page.expect_response(
            lambda response: "/api/admin/disputes" in response.url and response.status == 200
        ):
            page.locator(".admin-tab[data-tab='disputes']").click()
        expect(page.locator("#disputes-table-body")).to_contain_text(deposit_user["email"])
        expect(page.locator("#disputes-table-body")).to_contain_text("under_review")
        dispute_row = page.locator("#disputes-table-body tr").filter(has_text=deposit_user["email"])
        expect(dispute_row.get_by_role("button", name="Build Bundle")).to_be_visible()
        with page.expect_response(
            lambda response: response.url.endswith(f"/api/admin/disputes/{dispute_id}/evidence")
            and response.request.method == "POST",
            timeout=20_000,
        ) as evidence_response:
            dispute_row.get_by_role("button", name="Build Bundle").click()
        assert evidence_response.value.status == 200
        expect(dispute_row.get_by_role("link", name="View Bundle")).to_be_visible()
        evidence_url = f"/api/admin/disputes/{dispute_id}/evidence"
        cur.execute(
            """
            SELECT evidence_url
            FROM payment_disputes
            WHERE id = %s
            """,
            (dispute_id,),
        )
        assert cur.fetchone()[0] == evidence_url
        cur.execute(
            """
            SELECT COUNT(*)
            FROM audit_logs
            WHERE actor_user_id = %s
              AND action = 'admin.dispute_evidence_bundle_generated'
              AND entity_id = %s
              AND new_state->>'evidence_url' = %s
            """,
            (admin["user_id"], dispute_id, evidence_url),
        )
        assert cur.fetchone()[0] == 1
        evidence_bundle = session.get(f"{BASE_URL}{evidence_url}", timeout=10)
        assert evidence_bundle.status_code == 200, evidence_bundle.text
        bundle = evidence_bundle.json()
        assert bundle["bundle_type"] == "payment_dispute_evidence"
        assert bundle["dispute"]["provider_dispute_id"] == provider_dispute_id
        assert bundle["dispute"]["amount_cents"] == 7000
        expect(page.locator("script[src^='https://unpkg.com/htmx']")).to_have_count(0)

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[404])
    finally:
        conn.rollback()
        cur.close()
        conn.close()
        cleanup_marker(marker)
        if original_permissions is not None:
            restore_admin_permissions(original_permissions)
        cleanup_test_user(admin["user_id"])
        cleanup_test_user(denied_admin["user_id"])
        cleanup_test_user(deposit_user["user_id"])
