import os
import uuid
from datetime import date, timedelta

import psycopg2
import psycopg2.extras
import pytest
import requests
from playwright.sync_api import expect

# Register UUID adapter so uuid.UUID objects can be passed as bind params.
psycopg2.extras.register_uuid()


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


def create_user(cur, *, email_prefix, roles=(), cash_balance_cents=0):
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
        VALUES (%s, 'cash', 'USD', %s, 0)
        ON CONFLICT (user_id, wallet_type, currency) DO UPDATE SET
            balance_cents = EXCLUDED.balance_cents,
            held_balance_cents = 0
        """,
        (user_id, cash_balance_cents),
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


def create_asset(cur, marker):
    slug = f"e2e-admin-dividends-{uuid.uuid4().hex[:8]}"
    title = f"E2E Dividends {marker} <img src=x onerror=alert(1)>"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published
        )
        VALUES (
            %s, %s, 'E2E dividends fixture',
            'Seeded by the admin dividends E2E test.', 'real_estate',
            100000000, 10000, 10000, 9900, 'funded', TRUE
        )
        RETURNING id
        """,
        (title, slug),
    )
    return cur.fetchone()[0], title


def create_investment(cur, *, user_id, asset_id, tokens_owned):
    cur.execute(
        """
        INSERT INTO investments (
            user_id, asset_id, tokens_owned, purchase_value_cents,
            current_value_cents, status, purchased_at
        )
        VALUES (
            %s, %s, %s, %s, %s, 'active', NOW() - INTERVAL '30 days'
        )
        """,
        (user_id, asset_id, tokens_owned, tokens_owned * 10000, tokens_owned * 10000),
    )


def admin_session(session_token):
    session = requests.Session()
    session.cookies.set("poool_session", session_token, domain="localhost", path="/")
    session.cookies.set("poool_session", session_token, path="/")
    response = session.get(f"{BASE_URL}/admin/dividends", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf_token = session.cookies.get("csrf_token")
    assert csrf_token, "Expected CSRF cookie from admin dividends page"
    session.headers.update({"X-CSRF-Token": csrf_token})
    return session


def create_distribution(session, *, asset_id, period_start, period_end, total_amount_cents=10000):
    response = session.post(
        f"{BASE_URL}/api/admin/dividends/distributions",
        json={
            "asset_id": str(asset_id),
            "period_start": period_start.isoformat(),
            "period_end": period_end.isoformat(),
            "total_amount_cents": total_amount_cents,
            "min_holding_days": 7,
        },
        timeout=10,
    )
    assert response.status_code == 200, response.text
    return response.json()["result"]


def fetch_cash_wallet(cur, user_id):
    cur.execute(
        """
        SELECT id, balance_cents
        FROM wallets
        WHERE user_id = %s AND wallet_type = 'cash' AND currency = 'USD'
        """,
        (user_id,),
    )
    return cur.fetchone()


def cleanup_fixture(cur, *, distribution_ids, asset_id, user_ids):
    if distribution_ids:
        cur.execute(
            "DELETE FROM audit_logs WHERE entity_id = ANY(%s::uuid[])",
            ([str(v) for v in distribution_ids],),
        )
        cur.execute(
            "DELETE FROM dividend_payouts WHERE distribution_id = ANY(%s::uuid[])",
            ([str(v) for v in distribution_ids],),
        )
        cur.execute(
            "DELETE FROM dividend_distributions WHERE id = ANY(%s::uuid[])",
            ([str(v) for v in distribution_ids],),
        )
    if asset_id is not None:
        cur.execute("DELETE FROM investments WHERE asset_id = %s", (asset_id,))
        cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
    if user_ids:
        cur.execute(
            """
            DELETE FROM wallet_transactions
            WHERE wallet_id IN (
                SELECT id FROM wallets WHERE user_id = ANY(%s::uuid[])
            )
            """,
            ([str(v) for v in user_ids],),
        )
        cur.execute(
            "DELETE FROM audit_logs WHERE actor_user_id = ANY(%s::uuid[])",
            ([str(v) for v in user_ids],),
        )
        cur.execute(
            "DELETE FROM user_sessions WHERE user_id = ANY(%s::uuid[])",
            ([str(v) for v in user_ids],),
        )
        cur.execute("DELETE FROM wallets WHERE user_id = ANY(%s::uuid[])", ([str(v) for v in user_ids],))
        cur.execute("DELETE FROM user_roles WHERE user_id = ANY(%s::uuid[])", ([str(v) for v in user_ids],))
        cur.execute("DELETE FROM users WHERE id = ANY(%s::uuid[])", ([str(v) for v in user_ids],))


@pytest.mark.admin
def test_admin_dividends_lifecycle_permissions_csrf_audit_and_browser(quality_page):
    marker = f"e2e-dividends-{uuid.uuid4().hex[:10]}"
    created_user_ids = []
    distribution_ids = []
    asset_id = None
    original_permissions = None

    conn = db_connect()
    conn.autocommit = False
    cur = conn.cursor()
    try:
        original_permissions = snapshot_admin_permissions(cur)
        set_admin_permissions(cur, ())
        denied_admin = create_user(cur, email_prefix="e2e-div-denied-admin", roles=("admin",))
        maker = create_user(cur, email_prefix="e2e-div-maker", roles=("admin",))
        approver = create_user(cur, email_prefix="e2e-div-approver", roles=("admin",))
        executor = create_user(cur, email_prefix="e2e-div-executor", roles=("admin",))
        investor_a = create_user(cur, email_prefix="e2e-div-investor-a", cash_balance_cents=0)
        investor_b = create_user(cur, email_prefix="e2e-div-investor-b", cash_balance_cents=0)
        created_user_ids.extend(
            [
                denied_admin["id"],
                maker["id"],
                approver["id"],
                executor["id"],
                investor_a["id"],
                investor_b["id"],
            ]
        )
        asset_id, asset_title = create_asset(cur, marker)
        create_investment(cur, user_id=investor_a["id"], asset_id=asset_id, tokens_owned=70)
        create_investment(cur, user_id=investor_b["id"], asset_id=asset_id, tokens_owned=30)
        conn.commit()

        denied_session = admin_session(denied_admin["session_token"])
        denied_create = denied_session.post(
            f"{BASE_URL}/api/admin/dividends/distributions",
            json={
                "asset_id": str(asset_id),
                "period_start": "2026-01-01",
                "period_end": "2026-02-01",
                "total_amount_cents": 10000,
                "min_holding_days": 7,
            },
            timeout=10,
        )
        assert denied_create.status_code == 403

        set_admin_permissions(cur, ("financials.payout.draft",))
        conn.commit()
        maker_session = admin_session(maker["session_token"])

        no_csrf = requests.Session()
        no_csrf.cookies.set("poool_session", maker["session_token"], domain="localhost", path="/")
        no_csrf.cookies.set("poool_session", maker["session_token"], path="/")
        no_csrf.cookies.set("csrf_token", maker_session.cookies.get("csrf_token"), path="/")
        missing_csrf = no_csrf.post(
            f"{BASE_URL}/api/admin/dividends/distributions",
            json={
                "asset_id": str(asset_id),
                "period_start": "2026-02-01",
                "period_end": "2026-03-01",
                "total_amount_cents": 10000,
                "min_holding_days": 7,
            },
            timeout=10,
        )
        assert missing_csrf.status_code == 403

        period_start = date.today().replace(day=1) + timedelta(days=370)
        period_end = period_start + timedelta(days=31)
        result = create_distribution(
            maker_session,
            asset_id=asset_id,
            period_start=period_start,
            period_end=period_end,
        )
        distribution_id = uuid.UUID(result["distribution_id"])
        distribution_ids.append(distribution_id)
        assert result["total_tokens"] == 100
        payouts = {item["user_id"]: item for item in result["payouts"]}
        assert payouts[str(investor_a["id"])]["payout_cents"] == 7000
        assert payouts[str(investor_b["id"])]["payout_cents"] == 3000

        self_approve = maker_session.post(
            f"{BASE_URL}/api/admin/dividends/distributions/{distribution_id}/approve",
            timeout=10,
        )
        assert self_approve.status_code == 403

        set_admin_permissions(
            cur,
            (
                "financials.payout.draft",
                "financials.payout.approve",
                "assets.view",
                "developer_projects.view",
                "marketplace.view",
                "marketplace.compliance",
                "notifications.view",
                "approvals.manage",
                "community.view",
                "affiliates.manage",
            ),
        )
        conn.commit()
        approver_session = admin_session(approver["session_token"])
        executor_session = admin_session(executor["session_token"])

        self_approve_with_permission = maker_session.post(
            f"{BASE_URL}/api/admin/dividends/distributions/{distribution_id}/approve",
            timeout=10,
        )
        assert self_approve_with_permission.status_code == 400

        approve = approver_session.post(
            f"{BASE_URL}/api/admin/dividends/distributions/{distribution_id}/approve",
            timeout=10,
        )
        assert approve.status_code == 200, approve.text
        assert approve.json()["status"] == "approved"

        creator_execute = maker_session.post(
            f"{BASE_URL}/api/admin/dividends/distributions/{distribution_id}/execute",
            timeout=10,
        )
        assert creator_execute.status_code == 400

        execute = executor_session.post(
            f"{BASE_URL}/api/admin/dividends/distributions/{distribution_id}/execute",
            timeout=20,
        )
        assert execute.status_code == 200, execute.text
        summary = execute.json()["summary"]
        assert summary["total_credited_cents"] == 10000
        assert summary["holders_credited"] == 2
        assert summary["holders_skipped"] == 0

        duplicate_execute = executor_session.post(
            f"{BASE_URL}/api/admin/dividends/distributions/{distribution_id}/execute",
            timeout=10,
        )
        assert duplicate_execute.status_code == 400

        cur.execute(
            """
            SELECT status, approved_by, distributed_by, eligible_holders
            FROM dividend_distributions
            WHERE id = %s
            """,
            (distribution_id,),
        )
        assert cur.fetchone() == ("distributed", approver["id"], executor["id"], 2)
        cur.execute(
            """
            SELECT user_id, amount_cents, wallet_credited, status
            FROM dividend_payouts
            WHERE distribution_id = %s
            ORDER BY amount_cents DESC
            """,
            (distribution_id,),
        )
        assert cur.fetchall() == [
            (investor_a["id"], 7000, True, "paid"),
            (investor_b["id"], 3000, True, "paid"),
        ]
        assert fetch_cash_wallet(cur, investor_a["id"])[1] == 7000
        assert fetch_cash_wallet(cur, investor_b["id"])[1] == 3000
        for user_id, expected_amount in ((investor_a["id"], 7000), (investor_b["id"], 3000)):
            wallet_id = fetch_cash_wallet(cur, user_id)[0]
            cur.execute(
                """
                SELECT amount_cents, status
                FROM wallet_transactions
                WHERE wallet_id = %s AND type = 'dividend'
                """,
                (wallet_id,),
            )
            assert cur.fetchone() == (expected_amount, "completed")
        cur.execute(
            """
            SELECT action, actor_user_id
            FROM audit_logs
            WHERE entity_type = 'dividend_distributions'
              AND entity_id = %s
            ORDER BY created_at
            """,
            (distribution_id,),
        )
        assert cur.fetchall() == [
            ("dividend_distribution.approved", approver["id"]),
            ("dividend_distribution.executed", executor["id"]),
        ]

        cancel_period_start = period_start + timedelta(days=40)
        cancel_period_end = cancel_period_start + timedelta(days=31)
        cancel_result = create_distribution(
            maker_session,
            asset_id=asset_id,
            period_start=cancel_period_start,
            period_end=cancel_period_end,
            total_amount_cents=5000,
        )
        cancel_distribution_id = uuid.UUID(cancel_result["distribution_id"])
        distribution_ids.append(cancel_distribution_id)
        cancel_response = maker_session.post(
            f"{BASE_URL}/api/admin/dividends/distributions/{cancel_distribution_id}/cancel",
            json={"reason": f"{marker} cancel before approval"},
            timeout=10,
        )
        assert cancel_response.status_code == 200, cancel_response.text
        cur.execute(
            "SELECT status, cancel_reason FROM dividend_distributions WHERE id = %s",
            (cancel_distribution_id,),
        )
        assert cur.fetchone() == ("cancelled", f"{marker} cancel before approval")
        cur.execute(
            """
            SELECT actor_user_id
            FROM audit_logs
            WHERE entity_type = 'dividend_distributions'
              AND entity_id = %s
              AND action = 'dividend_distribution.cancelled'
            """,
            (cancel_distribution_id,),
        )
        assert cur.fetchone() == (maker["id"],)

        page, tracker = quality_page
        page.context.add_cookies(
            [{"name": "poool_session", "value": executor["session_token"], "url": BASE_URL}]
        )
        with page.expect_response(
            lambda response: "/api/admin/dividends/distributions" in response.url
            and response.request.method == "GET"
            and response.status == 200
        ):
            tracker.navigate_and_check(f"{BASE_URL}/admin/dividends")
        tracker.assert_page_loaded()
        expect(page.locator("#distributions-history-body")).to_contain_text(marker)
        expect(page.locator("#distributions-history-body")).to_contain_text("Distributed")
        # PooolDropdown wraps the native <select>, hiding the original. Pick
        # any visible occurrence of the asset title (the rendered dropdown UI).
        expect(page.get_by_text(asset_title).locator("visible=true").first).to_be_visible()
        expect(page.locator("img[src='x']")).to_have_count(0)
        # Preview button is disabled by validation until required fields are
        # filled. Native <select> is hidden by PooolDropdown — set value via
        # JS and dispatch events to trigger validateForm.
        # Asset list is fetched async — re-call loadAssets in page context to
        # ensure it ran successfully (initial call may race with cookie attach).
        api_status = page.evaluate(
            "() => fetch('/api/admin/assets').then(r => r.status)"
        )
        if api_status != 200:
            raise AssertionError(f"/api/admin/assets returned {api_status}")
        # Trigger reload
        page.evaluate("() => typeof loadAssets === 'function' && loadAssets()")
        page.wait_for_function(
            "id => !!document.querySelector(`#asset-select option[value='${id}']`)",
            arg=str(asset_id),
            timeout=10000,
        )
        page.evaluate(
            """([assetIdValue]) => {
                const sel = document.getElementById('asset-select');
                sel.value = assetIdValue;
                sel.dispatchEvent(new Event('change', { bubbles: true }));
                const amt = document.getElementById('total-amount');
                amt.value = '100';
                amt.dispatchEvent(new Event('input', { bubbles: true }));
            }""",
            [str(asset_id)],
        )
        page.wait_for_function("!document.getElementById('btn-preview').disabled")
        page.locator("#btn-preview").focus()
        expect(page.locator("#btn-preview")).to_be_focused()
        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures()
    finally:
        conn.rollback()
        if original_permissions is not None:
            restore_admin_permissions(original_permissions)
        cleanup_fixture(
            cur,
            distribution_ids=distribution_ids,
            asset_id=asset_id,
            user_ids=created_user_ids,
        )
        conn.commit()
        cur.close()
        conn.close()
