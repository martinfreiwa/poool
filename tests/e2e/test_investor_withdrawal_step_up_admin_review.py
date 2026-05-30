import base64
import hashlib
import hmac
import os
import struct
import time
import uuid

import psycopg2
import requests
from playwright.sync_api import expect

from tests.e2e.conftest import BASE_URL, DB_URL, cleanup_test_user, create_e2e_user


TOTP_SECRET = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP"


def _db():
    return psycopg2.connect(DB_URL)


def _totp_code(secret=TOTP_SECRET, for_time=None):
    counter = int((for_time or time.time()) // 30)
    key = base64.b32decode(secret, casefold=True)
    msg = struct.pack(">Q", counter)
    digest = hmac.new(key, msg, hashlib.sha1).digest()
    offset = digest[-1] & 0x0F
    code = struct.unpack(">I", digest[offset : offset + 4])[0] & 0x7FFFFFFF
    return f"{code % 1_000_000:06d}"


def _session_with_csrf(user, path):
    session = requests.Session()
    session.cookies.set("poool_session", user["session_token"])
    response = session.get(f"{BASE_URL}{path}", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf = session.cookies.get("csrf_token")
    assert csrf, f"Expected CSRF cookie from {path}"
    session.headers.update({"X-CSRF-Token": csrf})
    return session


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
    conn = _db()
    cur = conn.cursor()
    try:
        _set_admin_permissions(cur, permissions)
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _enable_totp_and_seed_bank(cur, user, marker):
    cur.execute(
        """
        INSERT INTO user_settings (user_id, totp_secret, totp_enabled)
        VALUES (%s, %s, TRUE)
        ON CONFLICT (user_id)
        DO UPDATE SET totp_secret = EXCLUDED.totp_secret, totp_enabled = TRUE
        """,
        (user["user_id"], TOTP_SECRET),
    )
    cur.execute(
        """
        UPDATE user_sessions
        SET is_2fa_verified = TRUE
        WHERE session_token = %s
        """,
        (user["session_token"],),
    )
    cur.execute(
        """
        INSERT INTO payment_methods (
            user_id, method_type, provider_token, provider_name, last4, brand,
            account_name, currency, is_default, status, bank_country,
            routing_number, bank_system
        )
        VALUES (
            %s, 'bank_account', %s, 'manual', '6789', 'Workflow Bank',
            'Workflow Investor', 'USD', TRUE, 'active', 'US', '110000000', 'ach'
        )
        RETURNING id
        """,
        (user["user_id"], f"workflow-withdraw-{marker}"),
    )
    return cur.fetchone()[0]


def _wallet_snapshot(cur, user_id):
    cur.execute(
        """
        SELECT id, balance_cents, held_balance_cents
        FROM wallets
        WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
        """,
        (user_id,),
    )
    return cur.fetchone()


def _latest_withdrawal(cur, user_id):
    cur.execute(
        """
        SELECT id, amount_cents, fee_cents, status, payment_method_id
        FROM withdrawal_requests
        WHERE user_id = %s
        ORDER BY created_at DESC
        LIMIT 1
        """,
        (user_id,),
    )
    return cur.fetchone()


def _audit_count(cur, action, entity_id, actor_id):
    cur.execute(
        """
        SELECT COUNT(*)
        FROM audit_logs
        WHERE action = %s
          AND entity_type = 'withdrawal_request'
          AND entity_id = %s
          AND actor_user_id = %s
        """,
        (action, entity_id, actor_id),
    )
    return cur.fetchone()[0]


def _cleanup(marker, users):
    conn = _db()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            DELETE FROM audit_logs
            WHERE entity_id IN (
                SELECT id FROM withdrawal_requests
                WHERE payment_method_id IN (
                    SELECT id FROM payment_methods WHERE provider_token = %s
                )
            )
            OR actor_user_id = ANY(%s::uuid[])
            """,
            (f"workflow-withdraw-{marker}", [str(u["user_id"]) for u in users]),
        )
        cur.execute(
            """
            DELETE FROM withdrawal_requests
            WHERE payment_method_id IN (
                SELECT id FROM payment_methods WHERE provider_token = %s
            )
            """,
            (f"workflow-withdraw-{marker}",),
        )
        cur.execute("DELETE FROM payment_methods WHERE provider_token = %s", (f"workflow-withdraw-{marker}",))
        conn.commit()
    finally:
        cur.close()
        conn.close()
    for user in users:
        cleanup_test_user(user["user_id"])


def test_investor_withdrawal_step_up_admin_reject_and_approve(quality_page):
    page, tracker = quality_page
    marker = uuid.uuid4().hex[:10]
    investor = create_e2e_user(
        email_prefix="e2e-withdraw-investor",
        display_name="Workflow Withdraw Investor",
        cash_balance_cents=200_000,
        kyc_status="approved",
    )
    admin = create_e2e_user(
        email_prefix="e2e-withdraw-admin",
        display_name="Workflow Withdraw Admin",
        roles=("admin", "super_admin"),
        cash_balance_cents=0,
    )
    original_permissions = None

    conn = _db()
    cur = conn.cursor()
    try:
        original_permissions = _snapshot_admin_permissions(cur)
        _set_admin_permissions(
            cur,
            sorted(set(original_permissions) | {"withdrawals.read", "withdrawals.write"}),
        )
        payment_method_id = _enable_totp_and_seed_bank(cur, investor, marker)
        conn.commit()

        page.context.add_cookies(
            [{"name": "poool_session", "value": investor["session_token"], "url": BASE_URL}]
        )
        tracker.navigate_and_check(f"{BASE_URL}/wallet")
        expect(page.locator("body")).to_contain_text("Withdraw")
        expect(page.locator("body")).to_contain_text("Workflow Bank")

        investor_session = _session_with_csrf(investor, "/wallet")
        invalid_step_up = investor_session.post(
            f"{BASE_URL}/api/wallet/step-up/verify",
            json={"code": "000000", "action": "withdrawal"},
            timeout=10,
        )
        assert invalid_step_up.status_code == 401
        assert invalid_step_up.json()["error"] == "invalid_code"

        high_without_step_up = investor_session.post(
            f"{BASE_URL}/wallet/withdraw",
            data={"amount": "600.00", "payment_method_id": str(payment_method_id)},
            allow_redirects=False,
            timeout=10,
        )
        assert high_without_step_up.status_code in (302, 303)
        assert "error=withdraw_2fa_required" in high_without_step_up.headers["Location"]

        valid_step_up = None
        for skew_seconds in (-60, -30, 0, 30, 60):
            valid_step_up = investor_session.post(
                f"{BASE_URL}/api/wallet/step-up/verify",
                json={
                    "code": _totp_code(for_time=time.time() + skew_seconds),
                    "action": "withdrawal",
                },
                timeout=10,
            )
            if valid_step_up.status_code == 200:
                break
        assert valid_step_up is not None
        assert valid_step_up.status_code == 200, valid_step_up.text
        assert valid_step_up.json()["status"] == "verified"

        start_wallet_id, start_balance, start_held = _wallet_snapshot(cur, investor["user_id"])
        first_withdraw = investor_session.post(
            f"{BASE_URL}/wallet/withdraw",
            data={"amount": "600.00", "payment_method_id": str(payment_method_id)},
            allow_redirects=False,
            timeout=10,
        )
        assert first_withdraw.status_code in (302, 303), first_withdraw.text
        assert "withdraw_requested=true" in first_withdraw.headers["Location"]
        conn.commit()
        first_id, first_amount, first_fee, first_status, first_pm = _latest_withdrawal(
            cur, investor["user_id"]
        )
        assert first_amount == 60_000
        assert first_status == "pending"
        assert first_pm == payment_method_id
        _, balance_after_first, held_after_first = _wallet_snapshot(cur, investor["user_id"])
        assert balance_after_first == start_balance - first_amount - first_fee
        assert held_after_first == start_held

        page.reload(wait_until="domcontentloaded")
        expect(page.locator("body")).to_contain_text("Pending")

        page.context.add_cookies(
            [{"name": "poool_session", "value": admin["session_token"], "url": BASE_URL}]
        )
        tracker.navigate_and_check(f"{BASE_URL}/admin/treasury")
        admin_session = _session_with_csrf(admin, "/admin/treasury")
        withdrawal_list = admin_session.get(f"{BASE_URL}/api/admin/withdrawals", timeout=10)
        assert withdrawal_list.status_code == 200, withdrawal_list.text
        admin_rows = withdrawal_list.json()
        assert any(
            row["id"] == str(first_id)
            and row["amount_cents"] == 60_000
            and row["payment_method_id"] == str(payment_method_id)
            for row in admin_rows
        )

        reject = admin_session.post(
            f"{BASE_URL}/api/admin/withdrawals/{first_id}/reject",
            json={"reason": "Workflow test rejection"},
            timeout=10,
        )
        assert reject.status_code == 200, reject.text
        conn.commit()
        cur.execute("SELECT status, admin_notes FROM withdrawal_requests WHERE id = %s", (first_id,))
        assert cur.fetchone() == ("rejected", "Workflow test rejection")
        _, balance_after_reject, _ = _wallet_snapshot(cur, investor["user_id"])
        assert balance_after_reject == start_balance
        assert _audit_count(cur, "withdrawal.rejected", first_id, admin["user_id"]) == 1

        second_withdraw = investor_session.post(
            f"{BASE_URL}/wallet/withdraw",
            data={"amount": "550.00", "payment_method_id": str(payment_method_id)},
            allow_redirects=False,
            timeout=10,
        )
        assert second_withdraw.status_code in (302, 303), second_withdraw.text
        conn.commit()
        second_id, second_amount, second_fee, second_status, _ = _latest_withdrawal(
            cur, investor["user_id"]
        )
        assert second_amount == 55_000
        assert second_status == "pending"

        approve = admin_session.post(
            f"{BASE_URL}/api/admin/withdrawals/{second_id}/approve",
            timeout=10,
        )
        assert approve.status_code == 200, approve.text
        conn.commit()
        cur.execute("SELECT status, approved_at IS NOT NULL FROM withdrawal_requests WHERE id = %s", (second_id,))
        assert cur.fetchone() == ("approved", True)
        cur.execute(
            """
            SELECT id, status, amount_cents
            FROM wallet_transactions
            WHERE wallet_id = %s AND external_ref_id = %s AND type = 'withdrawal'
            """,
            (start_wallet_id, str(second_id)),
        )
        second_tx_id, second_tx_status, second_tx_amount = cur.fetchone()
        assert (second_tx_status, second_tx_amount) == ("completed", -55_000)
        _, final_balance, _ = _wallet_snapshot(cur, investor["user_id"])
        assert final_balance == start_balance - second_amount - second_fee
        assert _audit_count(cur, "withdrawal.approved", second_id, admin["user_id"]) == 1

        page.context.add_cookies(
            [{"name": "poool_session", "value": investor["session_token"], "url": BASE_URL}]
        )
        tracker.navigate_and_check(f"{BASE_URL}/transactions")
        expect(page.locator("body")).to_contain_text("Withdrawal")
        tracker.navigate_and_check(f"{BASE_URL}/transactions/{second_tx_id}")
        expect(page.locator("body")).to_contain_text("Withdrawal")
        expect(page.locator("body")).to_contain_text("Completed")
        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures(ignore_status=[404])
    finally:
        cur.close()
        conn.close()
        if original_permissions is not None:
            _restore_admin_permissions(original_permissions)
        _cleanup(marker, [investor, admin])
