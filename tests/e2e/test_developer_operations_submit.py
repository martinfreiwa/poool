"""
POOOL E2E — Developer: Operations Submit (New + Edit Modes)
============================================================
Covers the monthly-operations submission form, shared by:
  • /developer/villas/<asset_id>/operations/new?year=…&month=…  (create mode)
  • /developer/villas/<asset_id>/operations/<log_id>             (edit mode, C-4 fix)

Tests:
  • New mode: form loads, all fields present, custom-expense add row works,
    save-draft and submit-for-approval fire the correct API calls.
  • Edit mode: single-log GET populates form fields (C-4 regression guard).
  • Custom-expense row names round-trip via expense_other_notes JSONB
    (C-5 regression guard: row name "Garbage Service" / amount 50000 must
    survive reload).
  • Mobile viewport: form is single-column.
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


# ─── Shared seeding helpers (parallel to operations_dashboard tests) ─────────

def _seed_villa(developer_id, *, title="E2E Submit Villa"):
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        slug = f"e2e-submit-villa-{uuid.uuid4().hex[:8]}"
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
        cur.execute(
            """
            INSERT INTO developer_asset_links
                (developer_user_id, asset_id, effective_from, granted_by)
            VALUES (%s, %s, NOW() - INTERVAL '6 months', %s)
            """,
            (str(developer_id), asset_id, str(developer_id)),
        )
        conn.commit()
        return asset_id
    finally:
        cur.close()
        conn.close()


def _seed_log(asset_id, *, year, month, status="draft",
              expense_other_notes=None, user_id=None,
              expense_other_idr_cents=0):
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO villa_operations_log (
                asset_id, period_year, period_month, status,
                gross_rental_idr_cents, expense_other_idr_cents,
                expense_other_notes, nights_available, nights_booked,
                submitted_by
            )
            VALUES (%s, %s, %s, %s, 0, %s, %s::jsonb, 0, 0, %s)
            RETURNING id
            """,
            (
                asset_id, year, month, status,
                expense_other_idr_cents,
                __import__("json").dumps(expense_other_notes) if expense_other_notes else None,
                str(user_id) if user_id else None,
            ),
        )
        log_id = cur.fetchone()[0]
        conn.commit()
        return log_id
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
def submit_dev(playwright_session, request):
    """Developer + 1 seeded villa, no logs yet (new-mode tests)."""
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    user = create_e2e_user(
        email_prefix="e2e-submit",
        display_name="E2E Submit Dev",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])
    asset_id = _seed_villa(user["user_id"])

    yield page, tracker, {"user": user, "asset_id": asset_id}

    _teardown_context(context, page, tracker, request)
    _cleanup(user["user_id"], [asset_id])
    cleanup_test_user(user["user_id"])


@pytest.fixture(scope="function")
def submit_dev_with_log(playwright_session, request):
    """Developer + 1 seeded villa + 1 draft log (edit-mode tests)."""
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    user = create_e2e_user(
        email_prefix="e2e-submit-edit",
        display_name="E2E Submit Edit",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])
    asset_id = _seed_villa(user["user_id"])
    # Last month so it's eligible for editing in the matrix view too
    now = datetime.utcnow()
    last_month = now.month - 1 if now.month > 1 else 12
    last_year = now.year if now.month > 1 else now.year - 1

    log_id = _seed_log(
        asset_id,
        year=last_year, month=last_month,
        status="draft",
        user_id=user["user_id"],
    )

    yield page, tracker, {
        "user": user,
        "asset_id": asset_id,
        "log_id": log_id,
        "year": last_year,
        "month": last_month,
    }

    _teardown_context(context, page, tracker, request)
    _cleanup(user["user_id"], [asset_id])
    cleanup_test_user(user["user_id"])


@pytest.fixture(scope="function")
def submit_dev_mobile(playwright_session, request):
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    user = create_e2e_user(
        email_prefix="e2e-submit-mob",
        display_name="E2E Submit Mobile",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])
    asset_id = _seed_villa(user["user_id"], title="E2E Mobile Submit Villa")

    yield page, tracker, {"user": user, "asset_id": asset_id}

    _teardown_context(context, page, tracker, request)
    _cleanup(user["user_id"], [asset_id])
    cleanup_test_user(user["user_id"])


# ─── Helpers ────────────────────────────────────────────────────────────────

def _go_new(page, tracker, asset_id, year=None, month=None):
    now = datetime.utcnow()
    year = year or now.year
    month = month or now.month
    url = (
        f"{BASE_URL}/developer/villas/{asset_id}/operations/new"
        f"?year={year}&month={month}"
    )
    tracker.navigate_and_check(url, timeout=TIMEOUT)
    page.wait_for_load_state("domcontentloaded")
    # Wait for the period header to be filled by JS
    page.wait_for_function(
        """() => {
            const el = document.getElementById('dops-period-text');
            return el && !el.textContent.includes('—');
        }""",
        timeout=TIMEOUT,
    )


def _go_edit(page, tracker, asset_id, log_id):
    url = f"{BASE_URL}/developer/villas/{asset_id}/operations/{log_id}"
    tracker.navigate_and_check(url, timeout=TIMEOUT)
    page.wait_for_load_state("domcontentloaded")
    # Edit mode → JS resolves the period from the API
    page.wait_for_function(
        """() => {
            const el = document.getElementById('dops-period-text');
            return el && !el.textContent.includes('—');
        }""",
        timeout=TIMEOUT,
    )


# ═══════════════════════════════════════════════════════════════════════════
# NEW MODE
# ═══════════════════════════════════════════════════════════════════════════

@pytest.mark.developer
@pytest.mark.smoke
def test_new_form_loads_clean(submit_dev):
    """New-mode URL renders the form skeleton without JS errors or 5xx."""
    page, tracker, ctx = submit_dev
    _go_new(page, tracker, ctx["asset_id"])
    tracker.assert_page_loaded()
    expect(page.locator("#dop-form")).to_be_visible()
    expect(page.locator("#btn-save-draft")).to_be_visible()
    expect(page.locator("#btn-submit")).to_be_visible()
    tracker.assert_no_critical_errors()


@pytest.mark.developer
def test_new_form_all_required_fields_present(submit_dev):
    """Every numeric input the form contract requires renders on the page."""
    page, tracker, ctx = submit_dev
    _go_new(page, tracker, ctx["asset_id"])

    required_inputs = [
        "#dop-gross-rental",
        "#dop-nights-available", "#dop-nights-booked",
        "#dop-expense-cleaning", "#dop-expense-maintenance",
        "#dop-expense-utilities", "#dop-expense-staff",
        "#dop-expense-pool-garden", "#dop-expense-pest",
        "#dop-expense-property-tax", "#dop-expense-insurance",
        "#dop-expense-accounting", "#dop-expense-internet",
        "#dop-expense-other", "#dop-expense-capex",
        "#dop-ota-fees", "#dop-payment-fees", "#dop-refunds",
        "#dop-mgmt-fee", "#dop-mgmt-reported-distributable",
    ]
    for sel in required_inputs:
        expect(page.locator(sel)).to_be_visible(timeout=TIMEOUT)


@pytest.mark.developer
def test_custom_expense_add_row(submit_dev):
    """Clicking 'Add custom expense' inserts an editable row in the list."""
    page, tracker, ctx = submit_dev
    _go_new(page, tracker, ctx["asset_id"])

    list_sel = "#dops-custom-expenses-list"
    before = page.locator(f"{list_sel} .dops-custom-expense-row").count()
    page.locator("#btn-add-expense").click()
    expect(page.locator(f"{list_sel} .dops-custom-expense-row")).to_have_count(before + 1)
    # Name + amount inputs exist
    expect(page.locator(
        f'{list_sel} .dops-custom-expense-row [data-role="expense-name"]'
    ).first).to_be_visible()
    expect(page.locator(
        f'{list_sel} .dops-custom-expense-row [data-role="expense-amount"]'
    ).first).to_be_visible()


@pytest.mark.developer
def test_save_draft_fires_post_then_submit_fires_put(submit_dev):
    """
    Save-draft creates the log via POST, then Submit-for-approval issues the
    PUT /submit endpoint. We watch both responses.
    """
    page, tracker, ctx = submit_dev
    _go_new(page, tracker, ctx["asset_id"])

    # Save draft → POST /api/developer/villas/<asset>/operations
    with page.expect_response(
        lambda r: f"/api/developer/villas/{ctx['asset_id']}/operations" in r.url
        and r.request.method == "POST",
        timeout=TIMEOUT,
    ) as save_info:
        page.locator("#btn-save-draft").click()
    assert save_info.value.ok, (
        f"Save draft failed: {save_info.value.status} — {save_info.value.text()}"
    )

    # Submit-for-approval → calls saveDraft() (PUT this time, log exists)
    # then PUT .../<log_id>/submit
    with page.expect_response(
        lambda r: "/operations/" in r.url and r.url.endswith("/submit")
        and r.request.method == "PUT",
        timeout=TIMEOUT,
    ) as submit_info:
        page.locator("#btn-submit").click()
    assert submit_info.value.ok, (
        f"Submit-for-approval failed: {submit_info.value.status} — {submit_info.value.text()}"
    )


# ═══════════════════════════════════════════════════════════════════════════
# EDIT MODE  (C-4 + C-5 regression guards)
# ═══════════════════════════════════════════════════════════════════════════

@pytest.mark.developer
def test_edit_mode_loads_existing_log(submit_dev_with_log):
    """
    C-4 regression guard. Hitting `/operations/<log_id>` directly must NOT
    404 and must populate the form via the single-log GET endpoint.
    """
    page, tracker, ctx = submit_dev_with_log

    # Watch for the single-log GET firing
    with page.expect_response(
        lambda r: f"/api/developer/villas/{ctx['asset_id']}/operations/{ctx['log_id']}"
        in r.url and r.request.method == "GET",
        timeout=TIMEOUT,
    ) as resp_info:
        _go_edit(page, tracker, ctx["asset_id"], ctx["log_id"])

    assert resp_info.value.ok, (
        f"Single-log GET failed: {resp_info.value.status} — C-4 regression!"
    )
    tracker.assert_page_loaded()
    # The form is enabled (draft status), gross rental input should be present
    expect(page.locator("#dop-gross-rental")).to_be_visible()


@pytest.mark.developer
def test_custom_expense_row_names_round_trip(submit_dev_with_log):
    """
    C-5 regression guard. Add a custom expense row with name "Garbage Service"
    and amount 50,000, save the draft, reload the page, confirm the name still
    appears in the rebuilt row (proves the JSONB persist + hydrate path works).
    """
    page, tracker, ctx = submit_dev_with_log
    _go_edit(page, tracker, ctx["asset_id"], ctx["log_id"])

    # Add a custom-expense row, fill name + amount
    page.locator("#btn-add-expense").click()
    row = page.locator("#dops-custom-expenses-list .dops-custom-expense-row").last
    row.locator('[data-role="expense-name"]').fill("Garbage Service")
    row.locator('[data-role="expense-amount"]').click()
    row.locator('[data-role="expense-amount"]').fill("50000")
    row.locator('[data-role="expense-amount"]').dispatch_event("input")

    # Save draft (PUT — log already exists)
    with page.expect_response(
        lambda r: f"/api/developer/villas/{ctx['asset_id']}/operations/{ctx['log_id']}"
        in r.url and r.request.method == "PUT",
        timeout=TIMEOUT,
    ) as save_info:
        page.locator("#btn-save-draft").click()
    assert save_info.value.ok, (
        f"Save draft for round-trip failed: {save_info.value.status}"
    )

    # Reload edit-mode URL
    _go_edit(page, tracker, ctx["asset_id"], ctx["log_id"])

    # The custom-expense row should be rebuilt with name + amount
    rebuilt = page.locator("#dops-custom-expenses-list .dops-custom-expense-row")
    expect(rebuilt.first).to_be_visible(timeout=TIMEOUT)
    name_input = rebuilt.first.locator('[data-role="expense-name"]')
    expect(name_input).to_have_value("Garbage Service")


@pytest.mark.developer
@pytest.mark.mobile
def test_submit_form_mobile_layout(submit_dev_mobile):
    """At mobile viewport the form column stacks vertically (single column)."""
    page, tracker, ctx = submit_dev_mobile
    _go_new(page, tracker, ctx["asset_id"])
    tracker.assert_page_loaded()

    shell = page.locator(".dops-shell")
    expect(shell).to_be_visible()
    # The summary panel should either wrap below the form or hide; check the
    # form column itself is visible and either spans full width or stacks.
    form_col = page.locator(".dops-form-col")
    expect(form_col).to_be_visible()
    width = form_col.evaluate("el => el.getBoundingClientRect().width")
    # At 375px viewport the form column should occupy roughly the full width
    assert width >= 300, (
        f"Mobile form column width {width}px should be near full viewport width"
    )
    tracker.assert_no_critical_errors()
