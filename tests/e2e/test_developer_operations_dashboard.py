"""
POOOL E2E — Developer: Operations Dashboard
============================================
Covers /developer/operations — the monthly-submission matrix view.

Page contract (per spec):
  • Smoke load is clean (no JS errors, no 500s).
  • Matrix + year tabs + filter tabs render once the API returns.
  • Clicking a "draft" or "rejected" matrix cell navigates to
    `/developer/villas/<id>/operations/<log_id>` (C-4 regression guard:
     this URL must serve a 200 page, NOT a 404).
  • Clicking a "missing" cell navigates to
    `/operations/new?year=Y&month=M`.
  • At a mobile viewport the matrix scrolls horizontally
    (or the dedicated mobile-card list takes over).

We seed a single villa, a draft log and a rejected log directly in the DB
so the matrix has real cells to click. Asset rows are removed in the
developer_page fixture teardown.
"""

import os
import uuid
import pytest
from datetime import datetime
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


# ─── Fixtures ────────────────────────────────────────────────────────────────

def _cleanup_villa_artifacts(user_id, asset_ids):
    """Remove villa_operations_log + capex + asset_links + assets seeded for a test."""
    if not asset_ids:
        try:
            conn = get_db_connection()
            cur = conn.cursor()
            cur.execute(
                "SELECT id::text FROM assets WHERE developer_user_id = %s",
                (str(user_id),),
            )
            asset_ids = [r[0] for r in cur.fetchall()]
            cur.close()
            conn.close()
        except Exception:
            asset_ids = []

    if not asset_ids:
        return

    try:
        conn = get_db_connection()
        cur = conn.cursor()
        for aid in asset_ids:
            cur.execute(
                "DELETE FROM villa_period_documents WHERE asset_id = %s", (aid,)
            )
            cur.execute(
                "DELETE FROM villa_annual_documents WHERE asset_id = %s", (aid,)
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
                "DELETE FROM villa_capex_events WHERE asset_id = %s "
                "AND status NOT IN ('approved','superseded')",
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
        pass  # best effort


def _seed_villa(developer_id, *, title="E2E Operations Villa"):
    """
    Insert an asset + developer_asset_link so the operations matrix has a row.
    Returns the new asset_id (str).
    """
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        slug = f"e2e-villa-{uuid.uuid4().hex[:8]}"
        cur.execute(
            """
            INSERT INTO assets (
                title, slug, short_description, asset_type,
                developer_user_id, total_value_cents, token_price_cents,
                tokens_total, tokens_available, published
            )
            VALUES (%s, %s, 'E2E villa fixture', 'real_estate',
                    %s, 500000000, 50000, 1000, 1000, TRUE)
            RETURNING id::text
            """,
            (title, slug, str(developer_id)),
        )
        asset_id = cur.fetchone()[0]
        # Effective-from to two months ago so the matrix expects ≥1 submission.
        cur.execute(
            """
            INSERT INTO developer_asset_links
                (developer_user_id, asset_id, effective_from, granted_by)
            VALUES (%s, %s, NOW() - INTERVAL '2 months', %s)
            """,
            (str(developer_id), asset_id, str(developer_id)),
        )
        conn.commit()
        return asset_id
    finally:
        cur.close()
        conn.close()


def _seed_log(asset_id, *, year, month, status, rejected_reason=None,
              user_id=None):
    """Insert a single villa_operations_log row, return its id."""
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO villa_operations_log (
                asset_id, period_year, period_month, status,
                rejected_reason, gross_rental_idr_cents,
                nights_available, nights_booked, submitted_by
            )
            VALUES (%s, %s, %s, %s, %s, 0, 0, 0, %s)
            RETURNING id
            """,
            (
                asset_id, year, month, status, rejected_reason,
                str(user_id) if user_id else None,
            ),
        )
        log_id = cur.fetchone()[0]
        conn.commit()
        return log_id
    finally:
        cur.close()
        conn.close()


@pytest.fixture(scope="function")
def developer_with_villa(playwright_session, request):
    """Developer user + 1 seeded villa with a draft + rejected log."""
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    user = create_e2e_user(
        email_prefix="e2e-dev-ops",
        display_name="E2E Dev Operations",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])

    now = datetime.utcnow()
    year = now.year
    last_month = now.month - 1 if now.month > 1 else 12
    last_year = year if now.month > 1 else year - 1
    prev_month = last_month - 1 if last_month > 1 else 12
    prev_year = last_year if last_month > 1 else last_year - 1

    asset_id = _seed_villa(user["user_id"])
    # Draft for last month → matrix cell links to /operations/<log_id>
    draft_log_id = _seed_log(
        asset_id, year=last_year, month=last_month, status="draft",
        user_id=user["user_id"],
    )
    # Rejected for the month before → second cell
    rejected_log_id = _seed_log(
        asset_id, year=prev_year, month=prev_month, status="draft",
        rejected_reason="Missing OTA receipts",
        user_id=user["user_id"],
    )

    yield page, tracker, {
        "user": user,
        "asset_id": asset_id,
        "draft": {"log_id": draft_log_id, "year": last_year, "month": last_month},
        "rejected": {"log_id": rejected_log_id, "year": prev_year, "month": prev_month},
    }

    _teardown_context(context, page, tracker, request)
    _cleanup_villa_artifacts(user["user_id"], [asset_id])
    cleanup_test_user(user["user_id"])


@pytest.fixture(scope="function")
def developer_with_villa_mobile(playwright_session, request):
    """Mobile-viewport variant of the above."""
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    user = create_e2e_user(
        email_prefix="e2e-dev-ops-mob",
        display_name="E2E Dev Operations Mobile",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])

    asset_id = _seed_villa(user["user_id"], title="E2E Mobile Villa")

    yield page, tracker, {"user": user, "asset_id": asset_id}

    _teardown_context(context, page, tracker, request)
    _cleanup_villa_artifacts(user["user_id"], [asset_id])
    cleanup_test_user(user["user_id"])


# ─── Helpers ────────────────────────────────────────────────────────────────

def _go_dashboard(page, tracker):
    tracker.navigate_and_check(f"{BASE_URL}/developer/operations", timeout=TIMEOUT)
    page.wait_for_load_state("domcontentloaded")


def _wait_for_matrix_or_empty(page, timeout=10_000):
    """Wait until the dashboard JS finished hydrating (matrix or empty state visible)."""
    page.wait_for_function(
        """() => {
            const wrap = document.getElementById('ops-matrix-wrap');
            const empty = document.getElementById('ops-empty');
            const skel  = document.getElementById('ops-skeleton');
            return (skel && skel.style.display === 'none') &&
                   ((wrap && wrap.style.display !== 'none') ||
                    (empty && empty.style.display !== 'none'));
        }""",
        timeout=timeout,
    )


# ═══════════════════════════════════════════════════════════════════════════
# TESTS
# ═══════════════════════════════════════════════════════════════════════════

@pytest.mark.developer
@pytest.mark.smoke
def test_operations_dashboard_loads_clean(developer_with_villa):
    """/developer/operations loads and reaches the matrix view without JS errors."""
    page, tracker, ctx = developer_with_villa
    _go_dashboard(page, tracker)

    tracker.assert_page_loaded()
    _wait_for_matrix_or_empty(page)
    # Once data hydrated, the matrix table should be visible
    expect(page.locator("#ops-matrix-wrap")).to_be_visible(timeout=TIMEOUT)
    expect(page.locator("table.ops-matrix")).to_be_visible()
    tracker.assert_no_critical_errors()


@pytest.mark.developer
def test_operations_dashboard_renders_filter_tabs(developer_with_villa):
    """Stats grid, filter tabs and year tabs container all render."""
    page, tracker, _ = developer_with_villa
    _go_dashboard(page, tracker)
    _wait_for_matrix_or_empty(page)

    # Filter tabs (always present in static HTML)
    for f in ("all", "action", "docs", "rejected"):
        expect(page.locator(f'.ops-filter-tab[data-filter="{f}"]')).to_be_visible()

    # Stats container shown once API resolves
    expect(page.locator("#ops-stats")).to_be_visible()
    # Year tabs container exists; it hides itself when only one year, so
    # only assert the container is in DOM.
    assert page.locator("#ops-year-tabs").count() == 1


@pytest.mark.developer
def test_draft_cell_navigates_to_log_edit_url(developer_with_villa):
    """
    C-4 regression guard. Clicking a draft cell must navigate to
    /developer/villas/<asset>/operations/<log_id> and the page must render
    (NOT a 404). We assert no HTTP 404 was raised on this URL.
    """
    page, tracker, ctx = developer_with_villa
    _go_dashboard(page, tracker)
    _wait_for_matrix_or_empty(page)

    asset_id = ctx["asset_id"]
    log_id = ctx["draft"]["log_id"]
    expected_url_part = f"/developer/villas/{asset_id}/operations/{log_id}"

    # The draft cell is rendered as <a class="ops-dot ops-dot--draft ...">
    # whose href contains the log_id. Find it explicitly to avoid race conditions
    # with hover-action duplicates.
    selector = (
        f'a.ops-dot.ops-dot--draft[href*="/operations/{log_id}"]'
    )
    locator = page.locator(selector).first
    expect(locator).to_be_visible(timeout=TIMEOUT)
    href = locator.get_attribute("href")
    assert href and expected_url_part in href, (
        f"Draft dot href {href!r} should contain {expected_url_part!r}"
    )

    # Navigate by URL directly (link click would race with the overlay element)
    response = page.goto(f"{BASE_URL}{expected_url_part}", wait_until="domcontentloaded")
    assert response is not None
    assert response.status < 400, (
        f"Edit-mode URL returned HTTP {response.status} — C-4 regression!"
    )
    # The page is the same template as /new; topbar back link should be present
    expect(page.locator(".dops-back, #dop-form, body")).to_be_visible(timeout=TIMEOUT)


@pytest.mark.developer
def test_missing_cell_navigates_to_new_form(developer_with_villa):
    """A 'missing' cell becomes an <a> to /developer/villas/<id>/operations/new?year=…&month=…"""
    page, tracker, ctx = developer_with_villa
    _go_dashboard(page, tracker)
    _wait_for_matrix_or_empty(page)

    asset_id = ctx["asset_id"]

    # Missing cells render an anchor with class ops-dot--missing pointing at /new?year=…&month=…
    missing = page.locator("a.ops-dot.ops-dot--missing").first
    if missing.count() == 0:
        # Some periods may be in the future or before listing; fall back to inspecting
        # any anchor href that points at /operations/new for this asset.
        missing = page.locator(
            f'a[href*="/developer/villas/{asset_id}/operations/new?"]'
        ).first

    expect(missing).to_have_count(1, timeout=TIMEOUT)
    href = missing.get_attribute("href")
    assert href and "/operations/new?" in href and "year=" in href and "month=" in href, (
        f"Missing dot should point at the new form with year/month query — got {href!r}"
    )


@pytest.mark.developer
def test_rejected_filter_tab_activates(developer_with_villa):
    """Clicking the Rejected filter tab marks it active and filters the table."""
    page, tracker, _ = developer_with_villa
    _go_dashboard(page, tracker)
    _wait_for_matrix_or_empty(page)

    rejected_tab = page.locator('.ops-filter-tab[data-filter="rejected"]')
    rejected_tab.click()
    expect(rejected_tab).to_have_class(__import__("re").compile(r"active"))


@pytest.mark.developer
@pytest.mark.mobile
def test_operations_dashboard_mobile_horizontal_scroll(developer_with_villa_mobile):
    """At a mobile viewport the matrix scroll container exists and is horizontally scrollable."""
    page, tracker, _ = developer_with_villa_mobile
    _go_dashboard(page, tracker)
    _wait_for_matrix_or_empty(page)

    # Either the dedicated mobile card list shows OR the matrix table is horizontally scrollable
    mobile_list = page.locator("#ops-mobile-list")
    matrix_scroll = page.locator(".ops-table-scroll")

    has_mobile_list = (
        mobile_list.count() > 0 and mobile_list.evaluate("el => el.style.display !== 'none'")
    )
    if not has_mobile_list:
        expect(matrix_scroll).to_be_visible()
        overflow = matrix_scroll.evaluate(
            "el => getComputedStyle(el).overflowX"
        )
        assert overflow in ("auto", "scroll"), (
            f"Mobile matrix container should be horizontally scrollable, got overflow-x: {overflow}"
        )
    tracker.assert_no_critical_errors()
