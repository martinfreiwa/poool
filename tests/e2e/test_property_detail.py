"""
POOOL E2E Tests - Property Detail
=================================
Authenticated browser coverage for the property detail investment, documents,
contact, and mobile gallery flows.
"""

import uuid

import psycopg2
import pytest
import requests
from playwright.sync_api import expect

from tests.e2e.conftest import (
    BASE_URL,
    DB_URL,
    _create_context_and_page,
    _teardown_context,
    attach_session_cookie,
    cleanup_test_user,
    create_e2e_user,
)


@pytest.fixture(scope="function")
def mobile_authenticated_user_page(playwright_session, request):
    """Create an authenticated investor session in a mobile viewport."""
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    user = create_e2e_user(
        email_prefix="e2e-property-mobile",
        display_name="E2E Property Mobile",
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker, user

    _teardown_context(context, page, tracker, request)
    cleanup_test_user(user["user_id"])


@pytest.fixture
def property_asset():
    """Create a published property with public and private documents."""
    unique = uuid.uuid4().hex[:8]
    slug = f"e2e-property-detail-{unique}"
    title = f"E2E Property Detail {unique}"

    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO assets (
                title, slug, short_description, description, asset_type,
                location_city, location_country, total_value_cents, token_price_cents,
                tokens_total, tokens_available, annual_yield_bps,
                capital_appreciation_bps, funding_status, published, featured,
                term_months, land_size_sqm, operator_name, fixed_roi_bps,
                revenue_min_cents, revenue_max_cents, expenses_cents,
                net_profit_min_cents, net_profit_max_cents, investor_payout_cents,
                operator_split_pct, poool_split_pct, min_funding_tokens
            )
            VALUES (
                %s, %s, 'E2E property short description', 'E2E property long description',
                'real_estate', 'Bali', 'ID', 24000000, 10000, 2400, 1200, 1250,
                300, 'funding_open', TRUE, TRUE, 12, 600, 'E2E Operator',
                1250, 2800000, 3200000, 600000, 2200000, 2600000, 2400000,
                20, 5, 1200
            )
            RETURNING id
            """,
            (title, slug),
        )
        asset_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO asset_images (asset_id, image_url, alt_text, sort_order, is_cover)
            VALUES (%s, '/static/images/seed/villa1.webp', %s, 0, TRUE),
                   (%s, '/static/images/seed/villa2.webp', %s, 1, FALSE)
            """,
            (asset_id, title, asset_id, title),
        )
        cur.execute(
            """
            INSERT INTO asset_documents (asset_id, document_type, title, file_url, file_size_bytes)
            VALUES
                (%s, 'expose', 'E2E Investment Expose', '/static/images/seed/villa1.webp', 12345),
                (%s, 'appraisal', 'E2E Appraisal Report', '/static/images/seed/villa2.webp', 23456),
                (%s, 'tax_npwp', 'E2E Private Tax Document', '/docs/e2e-private-tax.pdf', 34567)
            RETURNING id, document_type
            """,
            (asset_id, asset_id, asset_id),
        )
        documents = {doc_type: doc_id for doc_id, doc_type in cur.fetchall()}
        conn.commit()

        yield {
            "id": asset_id,
            "slug": slug,
            "title": title,
            "public_doc_id": documents["expose"],
            "private_doc_id": documents["tax_npwp"],
        }
    finally:
        conn.rollback()
        cur.execute(
            "DELETE FROM cart_items WHERE asset_id IN (SELECT id FROM assets WHERE slug = %s)",
            (slug,),
        )
        cur.execute("DELETE FROM assets WHERE slug = %s", (slug,))
        conn.commit()
        cur.close()
        conn.close()


def _cart_quantity_for_user(user_id, asset_id):
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    try:
        cur.execute(
            """
            SELECT COALESCE(SUM(tokens_quantity), 0)
            FROM cart_items
            WHERE user_id = %s AND asset_id = %s
            """,
            (user_id, asset_id),
        )
        return cur.fetchone()[0]
    finally:
        cur.close()
        conn.close()


def _session_token_for_user(user_id):
    conn = psycopg2.connect(DB_URL)
    cur = conn.cursor()
    try:
        cur.execute(
            "SELECT session_token FROM user_sessions WHERE user_id = %s ORDER BY created_at DESC LIMIT 1",
            (user_id,),
        )
        row = cur.fetchone()
        assert row, "Expected authenticated test user session"
        return row[0]
    finally:
        cur.close()
        conn.close()


def test_property_detail_cart_documents_and_contact(authenticated_user_page, property_asset):
    page, tracker, user = authenticated_user_page
    page.goto(f"{BASE_URL}/property/{property_asset['slug']}", wait_until="domcontentloaded")

    expect(page.locator("#property-title")).to_contain_text(property_asset["title"])

    page.locator("#investment-amount-input").fill("100")
    with page.expect_navigation(url=f"{BASE_URL}/cart"):
        page.locator("#add-to-cart-main-btn").click()
    assert _cart_quantity_for_user(user["user_id"], property_asset["id"]) == 1

    page.goto(f"{BASE_URL}/property/{property_asset['slug']}", wait_until="domcontentloaded")
    expect(page.locator("#documents-section .document-item")).to_have_count(2)
    expect(page.locator("#documents-section")).to_contain_text("E2E Investment Expose")
    expect(page.locator("#documents-section")).not_to_contain_text("E2E Private Tax Document")

    session = requests.Session()
    session.cookies.set("poool_session", _session_token_for_user(user["user_id"]), path="/")
    public_download = session.get(
        f"{BASE_URL}/api/documents/{property_asset['public_doc_id']}/download",
        allow_redirects=False,
        timeout=10,
    )
    assert public_download.status_code in (200, 302, 303, 307, 308)
    private_download = session.get(
        f"{BASE_URL}/api/documents/{property_asset['private_doc_id']}/download",
        allow_redirects=False,
        timeout=10,
    )
    assert private_download.status_code == 403

    contact = page.locator(".contact-button.chat-button").first
    expect(contact).to_be_visible()
    assert contact.get_attribute("href")

    tracker.assert_no_critical_errors()


def test_property_detail_add_to_cart_error_and_mobile_gallery(
    authenticated_user_page, mobile_authenticated_user_page, property_asset
):
    page, tracker, _user = authenticated_user_page
    page.goto(f"{BASE_URL}/property/{property_asset['slug']}", wait_until="domcontentloaded")
    page.route("**/cart/add", lambda route: route.fulfill(status=403, body="Forbidden"))
    page.locator("#add-to-cart-main-btn").click()
    expect(page.locator("#property-cart-error")).to_be_visible()
    expect(page.locator("#add-to-cart-main-btn")).to_be_enabled()
    tracker.assert_no_critical_errors()

    mobile_page, mobile_tracker, _mobile_user = mobile_authenticated_user_page
    mobile_page.goto(
        f"{BASE_URL}/property/{property_asset['slug']}", wait_until="domcontentloaded"
    )
    dots = mobile_page.locator(".gallery-dot-clickable")
    expect(dots).to_have_count(2)
    dots.nth(1).click()
    assert "active" in (dots.nth(1).get_attribute("class") or "")
    mobile_tracker.assert_no_critical_errors()
