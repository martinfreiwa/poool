"""Playwright UI smoke for the admin Orderbook page.

Covers the new top-5/next-10/next-10 UX surface:

- Combobox + searchable asset picker
- Toolbar (side chip, tick group, tz)
- Live indicator visible after data loads
- Match-preview popover on row hover
- CSV export downloads a file
- Settings drawer opens + Esc closes
- Pause/resume of auto-refresh
"""

import os
import re
import uuid

import psycopg2
import pytest
from playwright.sync_api import expect


BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
DB_URL = os.environ.get("DATABASE_URL", "postgres://martin@localhost/poool")
ORDER_PRICE_CENTS = 7_500
ORDER_QUANTITY = 88


def _seed_orderbook(cur):
    """Insert an asset + 1 bid + 1 ask so the page has something to render."""
    slug = f"e2e-ob-ui-{uuid.uuid4().hex[:8]}"
    cur.execute(
        """
        INSERT INTO assets (
            title, slug, short_description, description, asset_type,
            total_value_cents, token_price_cents, tokens_total, tokens_available,
            funding_status, published
        )
        VALUES (
            'E2E UI Orderbook Asset', %s, 'fixture',
            'Seeded by orderbook UI E2E', 'real_estate',
            100000000, 10000, 10000, 9000, 'funded', TRUE
        )
        RETURNING id
        """,
        (slug,),
    )
    asset_id = cur.fetchone()[0]

    user_ids = []
    for prefix, side in (("e2e-ob-buy", "buy"), ("e2e-ob-sell", "sell")):
        email = f"{prefix}-{uuid.uuid4().hex[:8]}@poool.app"
        cur.execute(
            """INSERT INTO users (email, email_verified, status)
               VALUES (%s, TRUE, 'active') RETURNING id""",
            (email,),
        )
        uid = cur.fetchone()[0]
        user_ids.append(uid)
        cur.execute(
            """
            INSERT INTO market_orders (
                user_id, asset_id, side, order_type, price_cents, quantity,
                status, idempotency_key, expires_at
            )
            VALUES (%s, %s, %s, 'limit', %s, %s, 'open', %s, NOW() + INTERVAL '90 days')
            """,
            (
                uid,
                asset_id,
                side,
                ORDER_PRICE_CENTS + (500 if side == "sell" else 0),
                ORDER_QUANTITY,
                str(uuid.uuid4()),
            ),
        )
    return asset_id, user_ids


def _cleanup(cur, asset_id, user_ids):
    cur.execute("DELETE FROM market_orders WHERE asset_id = %s", (asset_id,))
    cur.execute("DELETE FROM investments WHERE asset_id = %s", (asset_id,))
    cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
    if user_ids:
        ids = [str(u) for u in user_ids]
        cur.execute(
            "DELETE FROM user_sessions WHERE user_id = ANY(%s::uuid[])", (ids,)
        )
        cur.execute("DELETE FROM wallets WHERE user_id = ANY(%s::uuid[])", (ids,))
        cur.execute("DELETE FROM users WHERE id = ANY(%s::uuid[])", (ids,))


@pytest.mark.admin
@pytest.mark.marketplace
def test_orderbook_ui_smoke(admin_page):
    page, _tracker = admin_page

    conn = psycopg2.connect(DB_URL)
    conn.autocommit = False
    cur = conn.cursor()
    asset_id, user_ids = _seed_orderbook(cur)
    conn.commit()

    try:
        page.goto(f"{BASE_URL}/admin/marketplace/orderbook.html")

        # Combobox renders w/ active count once assets load.
        page.wait_for_selector("#asset-combobox button", timeout=5000)

        # Switch to seeded asset via combobox (open + filter + click).
        page.keyboard.press("/")
        page.wait_for_selector("#mp-ob-combo-search", timeout=5000)
        page.fill("#mp-ob-combo-search", "E2E UI Orderbook")
        page.click(".mp-ob-combo-item:has-text('E2E UI Orderbook Asset')")

        # Live indicator + KPI strip both populate.
        page.wait_for_selector(".mp-ob-live-dot", timeout=5000)
        assert page.locator("#ob-stats .mp-ob-stat").count() >= 4

        # Side filter chip works.
        page.click(".mp-ob-chip[data-side='buy']")
        assert page.locator(".mp-ob-chip[data-side='buy'].is-active").count() == 1

        # Hovering a level row shows the match-preview popover.
        page.hover("#bids-body .mp-ob-level-row")
        page.wait_for_selector("#mp-ob-popover:not([hidden])", timeout=2000)

        # CSV export triggers a download.
        with page.expect_download() as info:
            page.click("#btn-export-csv")
        download = info.value
        assert re.search(r"orderbook_.*\.csv$", download.suggested_filename)

        # Settings drawer opens + Esc closes.
        page.click("#btn-asset-settings")
        page.wait_for_selector("#mp-ob-settings-drawer:not([hidden])", timeout=2000)
        page.keyboard.press("Escape")
        expect(page.locator("#mp-ob-settings-drawer")).to_be_hidden()

        # Pause auto-refresh toggles label/aria-pressed.
        pause = page.locator("#btn-pause-refresh")
        pause.click()
        assert pause.get_attribute("aria-pressed") == "true"
        pause.click()
        assert pause.get_attribute("aria-pressed") == "false"

        # Audit dropdown surfaces full timeline link.
        page.click("#btn-rebuild-history")
        page.wait_for_selector("#mp-ob-rebuild-history:not([hidden])", timeout=3000)
        assert (
            page.locator(".mp-ob-history-link").count()
            + page.locator(".mp-ob-history-empty").count()
            >= 1
        )

        # Depth-curve sparkline renders when the local orderbook service has
        # depth data; no-Redis local runs still verify the visible level rows.
        if page.locator("#mp-ob-depth-chart:not([hidden])").count():
            assert page.locator("#mp-ob-depth-chart path").count() >= 1
        else:
            assert page.locator("#bids-body .mp-ob-level-row").count() >= 1

        # Open level drilldown, click cancel to surface reason overlay.
        page.click("#bids-body .mp-ob-level-row")
        page.wait_for_selector(".mp-ob-level-table tbody .admin-btn--danger", timeout=4000)
        page.click(".mp-ob-level-table tbody .admin-btn--danger")
        page.wait_for_selector("#mp-ob-reason-overlay:not([hidden])", timeout=2000)
        # Cancel out — don't actually delete the seeded order.
        page.click("#mp-ob-reason-cancel")
        expect(page.locator("#mp-ob-reason-overlay")).to_be_hidden()
    finally:
        _cleanup(cur, asset_id, user_ids)
        conn.commit()
        cur.close()
        conn.close()
