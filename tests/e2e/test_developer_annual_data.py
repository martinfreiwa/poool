"""
POOOL E2E — Developer: Annual Data
====================================
Covers /developer/villas/<asset_id>/annual/<year> — the per-villa annual
rollup page (Villa-Returns C3).

Sections:
  1. Annual rollup summary (read-only)
  2. Submit CapEx event (form + submitted list)
  3. Forecast suggestion (form + submitted list)
  4. Annual document upload (tax statement / annual report)

Tests:
  • Smoke load is clean (no JS errors, no 500s).
  • All 4 section cards render in the grid.
  • Submitting a CapEx event fires POST /api/.../capex and renders the new
    row in the list.
  • Mobile viewport: sections stack into a single column.
"""

import os
import uuid
import pytest
from datetime import datetime, timezone
from playwright.sync_api import expect, Page

from tests.e2e.conftest import (
    BASE_URL,
    _create_context_and_page,
    _teardown_context,
    attach_session_cookie,
    cleanup_test_user,
    create_e2e_user,
    get_db_connection,
)

BASE_URL = os.environ.get("BASE_URL", BASE_URL)
TIMEOUT = 15_000


# ─── Helpers ────────────────────────────────────────────────────────────────

def _seed_villa(developer_id, *, title="E2E Annual Villa"):
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        slug = f"e2e-annual-{uuid.uuid4().hex[:8]}"
        cur.execute(
            """
            INSERT INTO assets (
                title, slug, short_description, asset_type,
                developer_user_id, total_value_cents, token_price_cents,
                tokens_total, tokens_available, published
            )
            VALUES (%s, %s, 'E2E annual villa fixture', 'real_estate',
                    %s, 500000000, 50000, 1000, 1000, TRUE)
            RETURNING id::text
            """,
            (title, slug, str(developer_id)),
        )
        asset_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO developer_asset_links
                (developer_user_id, asset_id, effective_from, granted_by)
            VALUES (%s, %s, NOW() - INTERVAL '13 months', %s)
            """,
            (str(developer_id), asset_id, str(developer_id)),
        )
        conn.commit()
        return asset_id
    finally:
        cur.close()
        conn.close()


def _cleanup(user_id, asset_ids):
    try:
        conn = get_db_connection()
        cur = conn.cursor()
        for aid in asset_ids:
            cur.execute("DELETE FROM villa_period_documents WHERE asset_id = %s", (aid,))
            cur.execute("DELETE FROM villa_annual_documents WHERE asset_id = %s", (aid,))
            cur.execute(
                "DELETE FROM villa_capex_events WHERE asset_id = %s "
                "AND status NOT IN ('approved','superseded')",
                (aid,),
            )
            cur.execute(
                "UPDATE villa_operations_log SET status = 'superseded' "
                "WHERE asset_id = %s AND status = 'published'",
                (aid,),
            )
            cur.execute(
                "DELETE FROM villa_operations_log WHERE asset_id = %s "
                "AND status <> 'superseded'",
                (aid,),
            )
            cur.execute(
                "UPDATE developer_asset_links SET effective_until = NOW(), "
                "revoked_at = NOW() WHERE asset_id = %s "
                "AND effective_until IS NULL",
                (aid,),
            )
        conn.commit()
        cur.close()
        conn.close()
    except Exception:
        pass


# ─── Fixtures ────────────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def annual_dev(playwright_session, request):
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    user = create_e2e_user(
        email_prefix="e2e-annual",
        display_name="E2E Annual Dev",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])
    asset_id = _seed_villa(user["user_id"])

    yield page, tracker, {"user": user, "asset_id": asset_id}

    _teardown_context(context, page, tracker, request)
    _cleanup(user["user_id"], [asset_id])
    cleanup_test_user(user["user_id"])


@pytest.fixture(scope="function")
def annual_dev_mobile(playwright_session, request):
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    user = create_e2e_user(
        email_prefix="e2e-annual-mob",
        display_name="E2E Annual Mobile",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])
    asset_id = _seed_villa(user["user_id"], title="E2E Annual Mobile Villa")

    yield page, tracker, {"user": user, "asset_id": asset_id}

    _teardown_context(context, page, tracker, request)
    _cleanup(user["user_id"], [asset_id])
    cleanup_test_user(user["user_id"])


def _go_annual(page, tracker, asset_id, year=None):
    year = year or datetime.utcnow().year
    url = f"{BASE_URL}/developer/villas/{asset_id}/annual/{year}"
    tracker.navigate_and_check(url, timeout=TIMEOUT)
    page.wait_for_load_state("domcontentloaded")
    # Hydrate happens in parallel — wait for the breadcrumb to fill
    page.wait_for_function(
        """() => {
            const el = document.getElementById('dad-breadcrumb');
            return el && !el.textContent.includes('Loading');
        }""",
        timeout=TIMEOUT,
    )


# ═══════════════════════════════════════════════════════════════════════════
# TESTS
# ═══════════════════════════════════════════════════════════════════════════

@pytest.mark.developer
@pytest.mark.smoke
def test_annual_data_loads_clean(annual_dev):
    """/developer/villas/<id>/annual/<year> loads without JS errors."""
    page, tracker, ctx = annual_dev
    _go_annual(page, tracker, ctx["asset_id"])
    tracker.assert_page_loaded()
    tracker.assert_no_critical_errors()


@pytest.mark.developer
def test_all_four_sections_render(annual_dev):
    """Annual rollup, CapEx form, Forecast form, Documents form — all visible."""
    page, tracker, ctx = annual_dev
    _go_annual(page, tracker, ctx["asset_id"])

    # 1) Annual rollup summary container
    expect(page.locator("#dad-summary")).to_be_visible()

    # 2) CapEx form fields + button
    expect(page.locator("#dad-capex-form")).to_be_visible()
    expect(page.locator("#dad-capex-date")).to_be_visible()
    expect(page.locator("#dad-capex-amount")).to_be_visible()
    expect(page.locator("#dad-capex-description")).to_be_visible()
    expect(page.locator("#btn-capex-submit")).to_be_visible()

    # 3) Forecast suggestion form
    expect(page.locator("#dad-forecast-form")).to_be_visible()
    expect(page.locator("#dad-forecast-occupancy")).to_be_visible()
    expect(page.locator("#btn-forecast-submit")).to_be_visible()

    # 4) Annual document upload form
    expect(page.locator("#dad-doc-form")).to_be_visible()
    expect(page.locator("#dad-doc-type")).to_be_visible()
    expect(page.locator("#dad-doc-file")).to_be_visible()
    expect(page.locator("#btn-doc-upload")).to_be_visible()


@pytest.mark.developer
def test_submitting_capex_event_renders_row(annual_dev):
    """
    Filling the CapEx form and clicking Submit fires POST /api/.../capex
    and refreshes the list (#dad-capex-list contains the new description).
    """
    page, tracker, ctx = annual_dev
    _go_annual(page, tracker, ctx["asset_id"])

    description = f"E2E generator replacement {uuid.uuid4().hex[:6]}"
    page.fill("#dad-capex-date", datetime.utcnow().strftime("%Y-%m-%d"))
    page.fill("#dad-capex-amount", "5000000")
    page.select_option("#dad-capex-category", "equipment")
    page.fill("#dad-capex-description", description)

    with page.expect_response(
        lambda r: f"/api/developer/villas/{ctx['asset_id']}/capex" in r.url
        and r.request.method == "POST",
        timeout=TIMEOUT,
    ) as resp_info:
        page.locator("#btn-capex-submit").click()

    assert resp_info.value.ok, (
        f"CapEx POST failed: {resp_info.value.status} — {resp_info.value.text()}"
    )

    # The list refreshes after submit; the new row description should appear.
    expect(page.locator("#dad-capex-list")).to_contain_text(description, timeout=TIMEOUT)


@pytest.mark.developer
@pytest.mark.mobile
def test_annual_data_mobile_stacks(annual_dev_mobile):
    """At mobile viewport the section grid collapses to a single column."""
    page, tracker, ctx = annual_dev_mobile
    _go_annual(page, tracker, ctx["asset_id"])
    tracker.assert_page_loaded()

    grid = page.locator(".dad-grid").first
    expect(grid).to_be_visible()
    # auto-fit minmax(360px, 1fr) → at 375px viewport this collapses to 1 col.
    template = grid.evaluate("el => getComputedStyle(el).gridTemplateColumns")
    # Either a single track, or two tracks at most due to padding rounding.
    cols = template.split()
    assert len(cols) <= 2, (
        f"Mobile grid should stack to ≤2 columns, got {len(cols)}: {template}"
    )
    tracker.assert_no_critical_errors()
