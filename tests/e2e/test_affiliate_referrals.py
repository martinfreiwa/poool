import os
import uuid

import psycopg2
import pytest
from playwright.sync_api import expect

from tests.e2e.conftest import attach_session_cookie, cleanup_test_user, create_e2e_user


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def db_connect():
    return psycopg2.connect(DB_URL)


def _seed_affiliate_referrals_fixture():
    affiliate = create_e2e_user(
        email_prefix="e2e-affiliate-referrals",
        display_name="E2E Affiliate Referrals",
    )
    referred_holdback = create_e2e_user(
        email_prefix="e2e-affiliate-referrals-holdback",
        display_name="E2E Referral Holdback",
    )
    referred_payable = create_e2e_user(
        email_prefix="e2e-affiliate-referrals-payable",
        display_name="E2E Referral Payable",
    )
    referred_paid = create_e2e_user(
        email_prefix="e2e-affiliate-referrals-paid",
        display_name="E2E Referral Paid",
    )
    hostile_email = f'e2e-referral-{uuid.uuid4().hex[:8]}-<img src=x onerror=alert(1)>@poool.app'
    referral_code = f"E2E{uuid.uuid4().hex[:10].upper()}"

    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO affiliates (user_id, referral_code, status, current_tier, commission_rate_bps)
            VALUES (%s, %s, 'active', 'Access', 50)
            ON CONFLICT (user_id) DO UPDATE SET
                referral_code = EXCLUDED.referral_code,
                status = 'active',
                current_tier = 'Access',
                commission_rate_bps = 50
            """,
            (affiliate["user_id"], referral_code),
        )
        cur.execute(
            "UPDATE users SET email = %s WHERE id = %s",
            (hostile_email, referred_payable["user_id"]),
        )
        cur.execute(
            """
            INSERT INTO affiliate_links (
                code, link_type, attribution_user_id, payout_user_id, status
            )
            VALUES (%s, 'personal', %s, %s, 'active')
            RETURNING id
            """,
            (referral_code, affiliate["user_id"], affiliate["user_id"]),
        )
        link_id = cur.fetchone()[0]
        asset_slug = f"e2e-affiliate-referrals-{uuid.uuid4().hex[:10]}"
        cur.execute(
            """
            INSERT INTO assets (
                title, slug, asset_type, total_value_cents, token_price_cents,
                tokens_total, tokens_available, funding_status, published
            )
            VALUES (%s, %s, 'real_estate', 10000000, 10000, 1000, 900, 'funding_open', true)
            RETURNING id
            """,
            (f"E2E Affiliate Referrals {asset_slug}", asset_slug),
        )
        asset_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO investments (
                user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents
            )
            VALUES (%s, %s, 10, 100000, 100000)
            RETURNING id
            """,
            (referred_payable["user_id"], asset_id),
        )
        payable_investment_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO orders (user_id, order_number, total_cents, status, payment_method)
            VALUES (%s, %s, 100000, 'completed', 'wallet')
            RETURNING id
            """,
            (referred_payable["user_id"], f"E2E-AFF-PAYABLE-{uuid.uuid4().hex[:10]}"),
        )
        payable_order_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO order_items (
                order_id, asset_id, tokens_quantity, token_price_cents, subtotal_cents
            )
            VALUES (%s, %s, 10, 10000, 100000)
            """,
            (payable_order_id, asset_id),
        )
        cur.execute(
            """
            INSERT INTO investments (
                user_id, asset_id, tokens_owned, purchase_value_cents, current_value_cents
            )
            VALUES (%s, %s, 20, 200000, 200000)
            RETURNING id
            """,
            (referred_paid["user_id"], asset_id),
        )
        paid_investment_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO orders (user_id, order_number, total_cents, status, payment_method)
            VALUES (%s, %s, 200000, 'completed', 'wallet')
            RETURNING id
            """,
            (referred_paid["user_id"], f"E2E-AFF-PAID-{uuid.uuid4().hex[:10]}"),
        )
        paid_order_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO order_items (
                order_id, asset_id, tokens_quantity, token_price_cents, subtotal_cents
            )
            VALUES (%s, %s, 20, 10000, 200000)
            """,
            (paid_order_id, asset_id),
        )

        cur.execute(
            """
            INSERT INTO affiliate_referrals (
                affiliate_id, referred_user_id, link_id, attribution_user_id, payout_user_id,
                status, holdback_expires_at, sub_id, utm_source
            )
            VALUES (%s, %s, %s, %s, %s, 'under_holdback', NOW() + INTERVAL '14 days', 'holdback-campaign', 'e2e')
            RETURNING id
            """,
            (
                affiliate["user_id"],
                referred_holdback["user_id"],
                link_id,
                affiliate["user_id"],
                affiliate["user_id"],
            ),
        )
        holdback_referral_id = cur.fetchone()[0]

        cur.execute(
            """
            INSERT INTO affiliate_referrals (
                affiliate_id, referred_user_id, link_id, attribution_user_id, payout_user_id,
                status, sub_id, utm_source
            )
            VALUES (%s, %s, %s, %s, %s, 'qualified', 'payable,campaign', 'e2e')
            RETURNING id
            """,
            (
                affiliate["user_id"],
                referred_payable["user_id"],
                link_id,
                affiliate["user_id"],
                affiliate["user_id"],
            ),
        )
        payable_referral_id = cur.fetchone()[0]

        cur.execute(
            """
            INSERT INTO affiliate_referrals (
                affiliate_id, referred_user_id, link_id, attribution_user_id, payout_user_id,
                status, sub_id, utm_source
            )
            VALUES (%s, %s, %s, %s, %s, 'paid', 'paid-campaign', 'e2e')
            RETURNING id
            """,
            (
                affiliate["user_id"],
                referred_paid["user_id"],
                link_id,
                affiliate["user_id"],
                affiliate["user_id"],
            ),
        )
        paid_referral_id = cur.fetchone()[0]

        cur.execute(
            """
            INSERT INTO affiliate_commissions (
                referral_id, affiliate_id, link_id, attribution_user_id, payout_user_id,
                source_order_id, provisional_amount_cents, status, tier_at_execution
            )
            VALUES (%s, %s, %s, %s, %s, %s, 7500, 'payable', 'Access')
            RETURNING id
            """,
            (
                payable_referral_id,
                affiliate["user_id"],
                link_id,
                affiliate["user_id"],
                affiliate["user_id"],
                payable_order_id,
            ),
        )
        payable_commission_id = cur.fetchone()[0]

        cur.execute(
            """
            INSERT INTO affiliate_commissions (
                referral_id, affiliate_id, link_id, attribution_user_id, payout_user_id,
                source_order_id, provisional_amount_cents, status, tier_at_execution
            )
            VALUES (%s, %s, %s, %s, %s, %s, 12500, 'paid', 'Access')
            RETURNING id
            """,
            (
                paid_referral_id,
                affiliate["user_id"],
                link_id,
                affiliate["user_id"],
                affiliate["user_id"],
                paid_order_id,
            ),
        )
        paid_commission_id = cur.fetchone()[0]

        conn.commit()
        return {
            "affiliate": affiliate,
            "referred_users": [referred_holdback, referred_payable, referred_paid],
            "referral_ids": [holdback_referral_id, payable_referral_id, paid_referral_id],
            "commission_ids": [payable_commission_id, paid_commission_id],
            "investment_ids": [payable_investment_id, paid_investment_id],
            "order_ids": [payable_order_id, paid_order_id],
            "asset_id": asset_id,
            "link_id": link_id,
            "hostile_email": hostile_email,
        }
    finally:
        cur.close()
        conn.close()


def _cleanup_affiliate_referrals_fixture(fixture):
    conn = db_connect()
    cur = conn.cursor()
    try:
        affiliate_id = fixture["affiliate"]["user_id"]
        cur.execute("DELETE FROM affiliate_commissions WHERE affiliate_id = %s", (affiliate_id,))
        cur.execute("DELETE FROM affiliate_referrals WHERE affiliate_id = %s", (affiliate_id,))
        cur.execute("DELETE FROM investments WHERE id = ANY(%s::uuid[])", (fixture["investment_ids"],))
        cur.execute("DELETE FROM order_items WHERE order_id = ANY(%s::uuid[])", (fixture["order_ids"],))
        cur.execute("DELETE FROM orders WHERE id = ANY(%s::uuid[])", (fixture["order_ids"],))
        cur.execute("DELETE FROM assets WHERE id = %s", (fixture["asset_id"],))
        cur.execute("DELETE FROM affiliate_links WHERE id = %s", (fixture["link_id"],))
        cur.execute("DELETE FROM affiliates WHERE user_id = %s", (affiliate_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()

    cleanup_test_user(fixture["affiliate"]["user_id"])
    for user in fixture["referred_users"]:
        cleanup_test_user(user["user_id"])


@pytest.mark.financial
@pytest.mark.destructive
def test_affiliate_referrals_page_safe_render_filters_and_csv_export(quality_page):
    page, tracker = quality_page
    fixture = _seed_affiliate_referrals_fixture()
    try:
        attach_session_cookie(page.context, fixture["affiliate"]["session_token"])
        tracker.navigate_and_check(f"{BASE_URL}/affiliate/referrals")

        expect(page.locator("#referrals-content")).to_be_visible()
        expect(page.locator("#referrals-loading")).to_be_hidden()
        expect(page.locator("#referrals-table-body tr")).to_have_count(3)
        expect(page.locator("#referrals-table-body")).to_contain_text(fixture["hostile_email"])
        expect(page.locator("img[src='x']")).to_have_count(0)

        page.get_by_role("tab", name="Payable").click()
        expect(page.locator("#referrals-table-body tr")).to_have_count(1)
        expect(page.locator("#referrals-table-body")).to_contain_text("$75.00")
        expect(page.locator("#referrals-status")).to_contain_text("1 referral shown")

        page.locator("#referral-search").fill("no-match")
        expect(page.locator("#referrals-table-body")).to_contain_text("No referrals match")
        expect(page.locator("#referrals-status")).to_contain_text("No referrals match")

        page.locator("#referral-search").fill("")
        page.get_by_role("tab", name="All Referrals").press("ArrowRight")
        expect(page.get_by_role("tab", name="Under Holdback")).to_have_attribute("aria-selected", "true")
        expect(page.locator("#referrals-table-body tr")).to_have_count(1)

        with page.expect_response(
            lambda response: "/api/affiliate/commissions/export?format=csv" in response.url
            and response.status == 200
        ) as export_response:
            page.locator("#affiliate-referrals-export-btn").click()
        csv_response = export_response.value
        assert "text/csv" in csv_response.headers["content-type"]
        csv = csv_response.text()
        assert "payable,campaign" in csv
        assert '"payable,campaign"' in csv
        assert "75.00" in csv
        assert "125.00" in csv

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures()
    finally:
        _cleanup_affiliate_referrals_fixture(fixture)


@pytest.mark.destructive
def test_affiliate_referrals_redirects_inactive_affiliates(quality_page):
    page, tracker = quality_page
    user = create_e2e_user(
        email_prefix="e2e-affiliate-referrals-inactive",
        display_name="E2E Inactive Affiliate",
    )
    conn = db_connect()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO affiliates (user_id, referral_code, status)
            VALUES (%s, %s, 'pending_approval')
            ON CONFLICT (user_id) DO UPDATE SET status = 'pending_approval'
            """,
            (user["user_id"], f"PENDING{uuid.uuid4().hex[:8].upper()}"),
        )
        conn.commit()

        attach_session_cookie(page.context, user["session_token"])
        tracker.navigate_and_check(f"{BASE_URL}/affiliate/referrals")
        if "/affiliate/dashboard" not in page.url:
            page.wait_for_url("**/affiliate/dashboard", timeout=10000)
        assert "/affiliate/dashboard" in page.url
    finally:
        try:
            cur.execute("DELETE FROM affiliates WHERE user_id = %s", (user["user_id"],))
            conn.commit()
        except Exception:
            conn.rollback()
        cur.close()
        conn.close()
        cleanup_test_user(user["user_id"])
