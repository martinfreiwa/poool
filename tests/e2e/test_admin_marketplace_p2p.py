import os
import uuid

import psycopg2
import pytest
from playwright.sync_api import expect

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")


def db_connect():
    return psycopg2.connect(DB_URL)


def create_user(cur, *, email_prefix):
    email = f"{email_prefix}-{uuid.uuid4().hex[:8]}@poool.app"
    cur.execute(
        """
        INSERT INTO users (email, email_verified, status)
        VALUES (%s, TRUE, 'active')
        RETURNING id
        """,
        (email,),
    )
    return cur.fetchone()[0], email


def create_asset(cur):
    slug = f"e2e-p2p-{uuid.uuid4().hex[:8]}"
    title = "E2E <img src=x onerror=alert(1)> P2P Asset"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published
        )
        VALUES (
            %s, %s, 'E2E P2P fixture',
            'Seeded by the admin marketplace P2P E2E test.', 'real_estate',
            100000000, 10000, 10000, 9000, 'funded', TRUE
        )
        RETURNING id
        """,
        (title, slug),
    )
    return cur.fetchone()[0]


def create_p2p_offer(cur, *, asset_id, maker_id, taker_id):
    cur.execute(
        """
        INSERT INTO p2p_offers (
            asset_id, maker_user_id, taker_user_id, side,
            price_cents, quantity, message, status, expires_at
        )
        VALUES (%s, %s, %s, 'sell', 12500, 12, 'E2E pending offer', 'pending', NOW() + INTERVAL '2 days')
        RETURNING id
        """,
        (asset_id, maker_id, taker_id),
    )
    return cur.fetchone()[0]


@pytest.mark.admin
@pytest.mark.marketplace
def test_admin_marketplace_p2p_authenticated_cancel_audit_and_safe_render(admin_page):
    page, tracker = admin_page
    created_user_ids = []
    asset_id = None
    offer_id = None

    conn = db_connect()
    conn.autocommit = False
    cur = conn.cursor()
    try:
        maker_id, maker_email = create_user(cur, email_prefix="e2e-p2p-maker")
        taker_id, taker_email = create_user(cur, email_prefix="e2e-p2p-taker")
        created_user_ids.extend([maker_id, taker_id])
        asset_id = create_asset(cur)
        offer_id = create_p2p_offer(cur, asset_id=asset_id, maker_id=maker_id, taker_id=taker_id)
        conn.commit()

        page.goto(f"{BASE_URL}/admin/marketplace/p2p", wait_until="networkidle")
        tracker.assert_page_loaded()

        row = page.locator(f"tr[data-offer-id='{offer_id}']")
        expect(row).to_be_visible()
        expect(row).to_contain_text("E2E <img src=x onerror=alert(1)> P2P Asset")
        expect(row).to_contain_text(maker_email.split("@")[0])
        expect(row).to_contain_text(taker_email.split("@")[0])
        expect(page.locator("img[src='x']")).to_have_count(0)

        row.get_by_role("button", name="Admin Cancel").click()
        expect(page.get_by_role("dialog", name="Cancel P2P Offer")).to_be_visible()
        page.get_by_role("button", name="Cancel Offer").click()
        expect(page.get_by_text("Please provide a cancellation reason.")).to_be_visible()
        expect(page.get_by_role("dialog", name="Cancel P2P Offer")).to_be_visible()

        page.locator("#p2p-cancel-reason").fill("E2E compliance cancellation")
        page.get_by_role("button", name="Cancel Offer").click()
        expect(row).to_contain_text("admin_cancelled")
        expect(row.get_by_role("button", name="Admin Cancel")).to_have_count(0)

        cur.execute("SELECT status FROM p2p_offers WHERE id = %s", (offer_id,))
        assert cur.fetchone()[0] == "admin_cancelled"
        cur.execute(
            """
            SELECT new_state->>'reason'
            FROM audit_logs
            WHERE entity_type = 'p2p_offer'
              AND entity_id = %s
              AND action = 'marketplace.p2p.admin_cancelled'
            """,
            (offer_id,),
        )
        assert cur.fetchone()[0] == "E2E compliance cancellation"
    finally:
        conn.rollback()
        if offer_id is not None:
            cur.execute("DELETE FROM audit_logs WHERE entity_type = 'p2p_offer' AND entity_id = %s", (offer_id,))
            cur.execute("DELETE FROM p2p_offers WHERE id = %s", (offer_id,))
        if asset_id is not None:
            cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
        if created_user_ids:
            cur.execute("DELETE FROM users WHERE id = ANY(%s::uuid[])", ([str(uid) for uid in created_user_ids],))
        conn.commit()
        cur.close()
        conn.close()
