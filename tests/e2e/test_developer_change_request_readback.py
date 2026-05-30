import uuid

import psycopg2
import requests
from playwright.sync_api import expect

from tests.e2e.conftest import BASE_URL, DB_URL, cleanup_test_user, create_e2e_user


def _session_with_csrf(user, path):
    session = requests.Session()
    session.cookies.set("poool_session", user["session_token"])
    response = session.get(f"{BASE_URL}{path}", timeout=10)
    assert response.status_code == 200, response.text[:500]
    csrf = session.cookies.get("csrf_token")
    assert csrf, f"Expected CSRF cookie from {path}"
    session.headers.update({"X-CSRF-Token": csrf})
    return session


def _seed_live_asset(developer_user_id, marker):
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    try:
        slug = f"workflow-change-{marker}"
        title = f"Workflow Change Original {marker}"
        cur.execute(
            """
            INSERT INTO assets (
                developer_user_id, title, slug, short_description, description,
                asset_type, location_city, location_country,
                total_value_cents, token_price_cents, tokens_total, tokens_available,
                annual_yield_bps, capital_appreciation_bps, occupancy_rate_bps,
                funding_status, published, featured, property_type, min_funding_tokens
            )
            VALUES (
                %s, %s, %s, 'Original investor summary',
                'Original investor-facing workflow description.',
                'real_estate', 'Bali', 'Indonesia',
                25000000, 10000, 2500, 2500,
                900, 250, 8500, 'funding_open', TRUE, FALSE, 'villa', 100
            )
            RETURNING id
            """,
            (developer_user_id, title, slug),
        )
        asset_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO asset_images (asset_id, image_url, alt_text, is_cover, sort_order)
            VALUES (%s, '/static/images/seed/villa1.webp', %s, TRUE, 0)
            """,
            (asset_id, title),
        )
        cur.execute(
            """
            INSERT INTO developer_projects (developer_id, asset_id, project_name, status)
            VALUES (%s, %s, %s, 'live')
            """,
            (developer_user_id, asset_id, title),
        )
        conn.commit()
        return asset_id, slug, title
    finally:
        cur.close()
        conn.close()


def _asset_title(asset_id):
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    try:
        cur.execute("SELECT title FROM assets WHERE id = %s", (asset_id,))
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def _change_request_status(change_request_id):
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    try:
        cur.execute(
            "SELECT status, admin_notes FROM asset_change_requests WHERE id = %s",
            (change_request_id,),
        )
        return cur.fetchone()
    finally:
        cur.close()
        conn.close()


def _count_audit(action, entity_id):
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT COUNT(*)
            FROM audit_logs
            WHERE action = %s
              AND entity_id = %s
            """,
            (action, entity_id),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def _cleanup(asset_id, users):
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    try:
        cur.execute("DELETE FROM audit_logs WHERE entity_id = %s", (asset_id,))
        cur.execute("DELETE FROM asset_change_requests WHERE asset_id = %s", (asset_id,))
        cur.execute("DELETE FROM developer_projects WHERE asset_id = %s", (asset_id,))
        cur.execute("DELETE FROM assets WHERE id = %s", (asset_id,))
        conn.commit()
    finally:
        cur.close()
        conn.close()
    for user in users:
        cleanup_test_user(user["user_id"])


def test_developer_change_request_reject_resubmit_approve_investor_readback(quality_page):
    page, tracker = quality_page
    marker = uuid.uuid4().hex[:8]
    developer = create_e2e_user(
        email_prefix="e2e-change-dev",
        display_name="Workflow Change Developer",
        roles=("developer",),
    )
    admin = create_e2e_user(
        email_prefix="e2e-change-admin",
        display_name="Workflow Change Admin",
        roles=("admin", "super_admin"),
    )
    investor = create_e2e_user(
        email_prefix="e2e-change-investor",
        display_name="Workflow Change Investor",
    )
    asset_id, slug, original_title = _seed_live_asset(developer["user_id"], marker)

    try:
        page.context.add_cookies(
            [{"name": "poool_session", "value": investor["session_token"], "url": BASE_URL}]
        )
        public_response = tracker.navigate_and_check(f"{BASE_URL}/property/{slug}")
        assert public_response is not None and public_response.status == 200
        expect(page.locator("body")).to_contain_text(original_title)

        developer_session = _session_with_csrf(developer, "/developer/assets")
        rejected_title = f"Workflow Change Rejected {marker}"
        submit_response = developer_session.put(
            f"{BASE_URL}/api/developer/assets/{asset_id}",
            json={
                "title": rejected_title,
                "short_description": "Rejected summary should never leak",
                "annual_yield_bps": 1111,
            },
            timeout=10,
        )
        assert submit_response.status_code == 200, submit_response.text
        body = submit_response.json()
        assert body["mode"] == "review"
        rejected_request_id = body["change_request_id"]
        assert _asset_title(asset_id) == original_title

        page.context.add_cookies(
            [{"name": "poool_session", "value": admin["session_token"], "url": BASE_URL}]
        )
        tracker.navigate_and_check(f"{BASE_URL}/admin/asset-change-requests")
        row = page.locator("#change-requests-table tr", has_text=original_title).first
        expect(row).to_be_visible(timeout=10_000)

        admin_session = _session_with_csrf(admin, "/admin/asset-change-requests")
        reject_response = admin_session.post(
            f"{BASE_URL}/api/admin/change-requests/{rejected_request_id}/reject",
            json={"notes": "Rejected by workflow test"},
            timeout=10,
        )
        assert reject_response.status_code == 200, reject_response.text
        assert _change_request_status(rejected_request_id) == (
            "rejected",
            "Rejected by workflow test",
        )
        assert _count_audit("asset.change_request.rejected", rejected_request_id) == 1

        investor_response = page.goto(f"{BASE_URL}/property/{slug}", wait_until="domcontentloaded")
        assert investor_response is not None and investor_response.status == 200
        expect(page.locator("body")).to_contain_text(original_title)
        expect(page.locator("body")).not_to_contain_text(rejected_title)

        approved_title = f"Workflow Change Approved {marker}"
        resubmit_response = developer_session.put(
            f"{BASE_URL}/api/developer/assets/{asset_id}",
            json={
                "title": approved_title,
                "short_description": "Approved summary visible after admin review",
                "annual_yield_bps": 1234,
            },
            timeout=10,
        )
        assert resubmit_response.status_code == 200, resubmit_response.text
        approved_request_id = resubmit_response.json()["change_request_id"]

        approve_response = admin_session.post(
            f"{BASE_URL}/api/admin/change-requests/{approved_request_id}/approve",
            json={"notes": "Approved by workflow test"},
            timeout=10,
        )
        assert approve_response.status_code == 200, approve_response.text
        assert _change_request_status(approved_request_id) == (
            "approved",
            "Approved by workflow test",
        )
        assert _asset_title(asset_id) == approved_title
        assert _count_audit("asset.change_request.approved", asset_id) == 1

        page.context.clear_cookies()
        page.context.add_cookies(
            [{"name": "poool_session", "value": investor["session_token"], "url": BASE_URL}]
        )
        investor_readback = tracker.navigate_and_check(f"{BASE_URL}/property/{slug}")
        assert investor_readback is not None and investor_readback.status == 200
        expect(page.locator("body")).to_contain_text(approved_title)
        expect(page.locator("body")).to_contain_text("Approved summary visible after admin review")

        tracker.assert_no_critical_errors()
        tracker.assert_no_network_failures()
    finally:
        _cleanup(asset_id, (developer, admin, investor))
