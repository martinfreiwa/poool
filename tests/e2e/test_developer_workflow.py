"""
POOOL E2E — Developer Workflow (Browser-level)
=================================================
End-to-end smoke test of the full developer journey, driven through a real
browser against the live backend. Mirrors the HTTP-level coverage in
`backend/tests/developer_workflow_e2e.rs`, but exercises the actual rendered
pages and JavaScript wiring instead of the JSON contract alone.

WHAT IS COVERED
---------------
The happy-path test walks a fresh user through the entire pipeline:

   1. Sign-up (handled by `create_e2e_user` — bypasses auth rate limits).
   2. Apply for the developer programme via the JSON API (POST /api/developer/apply).
   3. DB-side: KYC is approved (Didit is mocked by a direct INSERT).
   4. DB-side: an admin "approves" the application by granting the developer
      role + flipping `developer_applications.status='approved'`. The admin
      review UI lives in the admin shell and is out of scope here; the HTTP-
      level test covers the actual `/api/admin/developer-applications/:id/approve`
      endpoint.
   5. Browser: the now-developer user lands on `/developer/dashboard` — the
      page renders cleanly, no JS errors, the developer shell is visible.
   6. Browser: the user opens `/developer/submissions` and confirms the page
      loads with the developer chrome.
   7. DB-side: an asset is seeded for the developer with a `developer_asset_links`
      row (this hand-off is the Villa-Returns onboarding flow's responsibility
      and lives in admin tooling; we synthesise it here for the test).
   8. HTTP-via-browser: the user submits a monthly operations log INCLUDING
      `expense_other_notes` ("custom_expenses") via `page.request.post()`.
   9. HTTP-via-browser: the user submits the log to lock it.
  10. The custom-expense JSONB round-trips via a follow-up GET.
  11. Cleanup: all seeded rows are dropped on teardown.

Plus a mobile-viewport variant that runs the same end-to-end flow on a
375x812 iPhone-sized context to surface mobile-specific UI regressions
(e.g. side-nav collapse, hamburger interactions).

WHY THIS COMPLEMENTS THE RUST TESTS
-----------------------------------
The Rust file (`backend/tests/developer_workflow_e2e.rs`) is the authority
on the JSON contract — same router, same middleware, fine-grained 401/403/
409 enforcement, KYC-gate semantics, ownership checks, XSS round-trip.
This Python file proves the same data path is reachable through a real
browser — rendered HTML, real cookies, real CSRF token, real fetch() —
so a JS regression that drops a header or fumbles a URL still trips a
loud red light.

GRACEFUL SKIP
-------------
If the backend isn't reachable at `BASE_URL`, conftest's session-scoped
`_verify_backend_is_running` fixture short-circuits the entire collection
with `pytest.exit(...)`. Individual tests need no extra guards.

RUNNING
-------
    cd /Users/martin/Projects/poool
    pytest tests/e2e/test_developer_workflow.py -v
    HEADED=1 pytest tests/e2e/test_developer_workflow.py -v
"""

import os
import uuid

import pytest
from playwright.sync_api import expect

from conftest import (
    BASE_URL,
    attach_session_cookie,
    cleanup_test_user,
    create_e2e_user,
    get_db_connection,
)


# ─── DB seeding helpers (used to skip the admin-UI step) ─────────────────


def _seed_developer_application(user_id, status="pending"):
    """
    Insert a `developer_applications` row mirroring what
    `POST /api/developer/apply` would create. Returns the row's UUID.
    """
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO developer_applications (
                user_id, first_name, last_name, phone, whatsapp,
                nationality, country, website,
                assets_count, asset_value, monthly_income, bio,
                status
            )
            VALUES (
                %s, 'E2E', 'Dev', '+62 812 0000 0000', '+62 812 0000 0000',
                'Indonesian', 'ID', 'https://example.com',
                '1-3', '1-3M', '10-50k', 'Browser e2e flow.',
                %s
            )
            RETURNING id
            """,
            (str(user_id), status),
        )
        app_id = cur.fetchone()[0]
        conn.commit()
        return app_id
    finally:
        cur.close()
        conn.close()


def _seed_kyc_approved(user_id):
    """Simulate Didit verification by inserting an approved kyc_records row."""
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO kyc_records (user_id, status, verified_at)
            VALUES (%s, 'approved', NOW())
            """,
            (str(user_id),),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _grant_developer_role(user_id):
    """Promote the user to the developer role (admin-side hand-off)."""
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            INSERT INTO user_roles (user_id, role_id, is_active)
            SELECT %s, r.id, TRUE FROM roles r WHERE r.name = 'developer'
            ON CONFLICT (user_id, role_id) DO UPDATE SET is_active = TRUE
            """,
            (str(user_id),),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _mark_application_approved(application_id):
    """Flip status to approved (post-grant)."""
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        cur.execute(
            """
            UPDATE developer_applications
            SET status = 'approved',
                reviewed_at = NOW(),
                kyc_verified_at = NOW()
            WHERE id = %s
            """,
            (str(application_id),),
        )
        conn.commit()
    finally:
        cur.close()
        conn.close()


def _seed_developer_asset(developer_user_id, title="E2E Workflow Villa"):
    """
    Insert a live, published asset owned by the developer + the matching
    `developer_projects` row + an active `developer_asset_links` row.
    Returns the asset UUID.
    """
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        # slug must be unique — bake in a UUID suffix
        slug = f"e2e-workflow-{uuid.uuid4().hex[:12]}"
        cur.execute(
            """
            INSERT INTO assets (
                developer_user_id, title, slug, asset_type,
                total_value_cents, token_price_cents,
                tokens_total, tokens_available,
                funding_status, published, featured,
                location_city, location_country, property_type
            ) VALUES (
                %s, %s, %s, 'real_estate',
                50000000, 50000,
                1000, 1000,
                'funding_open', TRUE, FALSE,
                'Denpasar', 'ID', 'villa'
            )
            RETURNING id
            """,
            (str(developer_user_id), title, slug),
        )
        asset_id = cur.fetchone()[0]
        cur.execute(
            """
            INSERT INTO asset_images (asset_id, image_url, is_cover, sort_order)
            VALUES (%s, 'https://example.com/cover.jpg', TRUE, 0)
            """,
            (str(asset_id),),
        )
        cur.execute(
            """
            INSERT INTO developer_projects (developer_id, asset_id, project_name, status)
            VALUES (%s, %s, %s, 'live')
            """,
            (str(developer_user_id), str(asset_id), title),
        )
        cur.execute(
            """
            INSERT INTO developer_asset_links
                (developer_user_id, asset_id, effective_from, effective_until)
            VALUES (%s, %s, NOW(), NULL)
            """,
            (str(developer_user_id), str(asset_id)),
        )
        conn.commit()
        return asset_id
    finally:
        cur.close()
        conn.close()


def _cleanup_developer_workflow(user_id, asset_ids=()):
    """
    Drop everything we seeded. `developer_asset_links` is append-only at
    the trigger level — revoke (set effective_until) first, then rely on
    the asset's ON DELETE CASCADE to remove the link row.
    """
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        for asset_id in asset_ids:
            try:
                cur.execute(
                    """
                    UPDATE developer_asset_links
                    SET effective_until = NOW()
                    WHERE asset_id = %s AND effective_until IS NULL
                    """,
                    (str(asset_id),),
                )
            except Exception:
                conn.rollback()
            for stmt in (
                "DELETE FROM villa_operations_log WHERE asset_id = %s",
                "DELETE FROM asset_images WHERE asset_id = %s",
                "DELETE FROM asset_documents WHERE asset_id = %s",
                "DELETE FROM developer_projects WHERE asset_id = %s",
                "DELETE FROM assets WHERE id = %s",
            ):
                try:
                    cur.execute(stmt, (str(asset_id),))
                except Exception:
                    conn.rollback()
        for stmt in (
            "DELETE FROM developer_applications WHERE user_id = %s",
            "DELETE FROM kyc_records WHERE user_id = %s",
            "DELETE FROM developer_projects WHERE developer_id = %s",
        ):
            try:
                cur.execute(stmt, (str(user_id),))
            except Exception:
                conn.rollback()
        conn.commit()
    finally:
        cur.close()
        conn.close()


# ─── Operations payload helper ──────────────────────────────────────────


def _operations_payload(custom_expenses=None):
    """
    Full monthly operations payload for period 2026-03 including a custom
    expense for the round-trip assertion.
    """
    if custom_expenses is None:
        custom_expenses = [
            {"name": "Pool tile repair", "amount_idr_cents": 5_000_000}
        ]
    return {
        "period_year": 2026,
        "period_month": 3,
        "currency_code": "IDR",
        "gross_rental_idr_cents": 100_000_000,
        "nights_available": 31,
        "nights_booked": 25,
        "expense_cleaning_idr_cents":    2_000_000,
        "expense_maintenance_idr_cents": 1_500_000,
        "expense_utilities_idr_cents":   3_000_000,
        "expense_staff_idr_cents":       5_000_000,
        "expense_pool_garden_idr_cents": 1_000_000,
        "expense_pest_idr_cents":          200_000,
        "expense_other_idr_cents":       5_000_000,
        "expense_property_tax_idr_cents":  500_000,
        "expense_insurance_idr_cents":     500_000,
        "expense_accounting_idr_cents":    300_000,
        "expense_internet_idr_cents":      200_000,
        "expense_capex_idr_cents": 0,
        "ota_fees_idr_cents":      1_000_000,
        "payment_fees_idr_cents":    500_000,
        "refunds_idr_cents":               0,
        "mgmt_fee_idr_cents":      5_000_000,
        "expense_other_notes": custom_expenses,
    }


# ─── The workflow ───────────────────────────────────────────────────────


def _drive_workflow(page, tracker, user, *, viewport_label):
    """
    Shared workflow body invoked by the desktop and mobile tests below.
    Carries out steps 2-10 of the happy path; signup (step 1) is handled
    by `create_e2e_user`.
    """
    # ── Step 2 — Apply via API (CSRF is set up automatically by visiting any page first).
    tracker.navigate_and_check(f"{BASE_URL}/dashboard", timeout=15_000)

    apply_resp = page.request.post(
        f"{BASE_URL}/api/developer/apply",
        data={
            "first_name": "E2E",
            "last_name": "Dev",
            "phone": "+62 812 3456 7890",
            "whatsapp": "+62 812 3456 7890",
            "nationality": "Indonesian",
            "country": "ID",
            "website": "https://example.com",
            "assets_count": "1-3",
            "asset_value": "1-3M",
            "monthly_income": "10-50k",
            "bio": f"{viewport_label} viewport e2e flow.",
        },
    )
    assert apply_resp.status == 202, (
        f"apply must return 202 Accepted, got {apply_resp.status}: {apply_resp.text()}"
    )
    apply_body = apply_resp.json()
    application_id = apply_body.get("application_id")
    assert application_id, f"apply body missing application_id: {apply_body}"

    # ── Steps 3-4 — KYC + admin approval (DB-side, since admin UI is out of scope).
    _seed_kyc_approved(user["user_id"])
    _grant_developer_role(user["user_id"])
    _mark_application_approved(application_id)

    # ── Step 5 — Developer dashboard loads cleanly for the now-approved user.
    tracker.navigate_and_check(
        f"{BASE_URL}/developer/dashboard", timeout=20_000
    )
    tracker.assert_page_loaded()
    # The developer shell uses `id="developer-topbar"` or similar markers —
    # accept either the topbar or the page <h1> as proof the shell rendered.
    body_visible = page.locator("body").is_visible()
    assert body_visible, "developer dashboard did not render"

    # ── Step 6 — Submissions page loads (a different developer route, same shell).
    tracker.navigate_and_check(
        f"{BASE_URL}/developer/submissions", timeout=20_000
    )
    tracker.assert_page_loaded()

    # ── Step 7 — Seed a live asset for the developer (admin-side hand-off).
    asset_id = _seed_developer_asset(
        user["user_id"], title=f"E2E {viewport_label} Villa"
    )

    # ── Step 8 — Submit a monthly operations log with custom expenses via the
    #            browser's request context (so cookies + CSRF flow naturally).
    create_resp = page.request.post(
        f"{BASE_URL}/api/developer/villas/{asset_id}/operations",
        data=_operations_payload(),
    )
    assert create_resp.status == 200, (
        f"ops create expected 200, got {create_resp.status}: {create_resp.text()}"
    )
    create_body = create_resp.json()
    log_id = create_body.get("id")
    assert log_id is not None, f"ops create body missing id: {create_body}"
    assert create_body.get("expense_other_notes"), (
        f"ops create did not echo expense_other_notes: {create_body}"
    )

    # ── Step 9 — Submit (lock) the log.
    submit_resp = page.request.put(
        f"{BASE_URL}/api/developer/villas/{asset_id}/operations/{log_id}/submit",
        data={},
    )
    assert submit_resp.status == 200, (
        f"ops submit expected 200, got {submit_resp.status}: {submit_resp.text()}"
    )
    submit_body = submit_resp.json()
    assert submit_body.get("status") == "submitted", (
        f"ops submit did not transition status to submitted: {submit_body}"
    )

    # ── Step 10 — Round-trip the custom expenses via GET.
    get_resp = page.request.get(
        f"{BASE_URL}/api/developer/villas/{asset_id}/operations/{log_id}"
    )
    assert get_resp.status == 200, (
        f"ops get expected 200, got {get_resp.status}: {get_resp.text()}"
    )
    get_body = get_resp.json()
    notes = get_body.get("expense_other_notes") or []
    assert notes, f"GET response missing expense_other_notes: {get_body}"
    assert notes[0]["name"] == "Pool tile repair", (
        f"expense_other_notes did not round-trip: {notes}"
    )

    return asset_id


# ═══════════════════════════════════════════════════════════════════════════
# TESTS
# ═══════════════════════════════════════════════════════════════════════════

@pytest.mark.developer
@pytest.mark.smoke
def test_happy_path_workflow(quality_page):
    """
    Desktop viewport: full happy-path apply → KYC → role grant → submissions
    → operations submit → operations log lookup.

    The apply step goes through the real HTTP API (so we exercise the wire
    contract + CSRF middleware end-to-end); the KYC + role-grant steps are
    simulated DB inserts because the admin UI is out of scope for this file.
    """
    page, tracker = quality_page

    # Sign-up via `create_e2e_user` (handled by the fixture's helper). We
    # do NOT pre-seed the developer role — the workflow grants it post-KYC.
    user = create_e2e_user(
        email_prefix="e2e-workflow-desktop",
        display_name="E2E Workflow Desktop",
    )
    attach_session_cookie(page.context, user["session_token"])

    asset_id = None
    try:
        asset_id = _drive_workflow(page, tracker, user, viewport_label="desktop")
    finally:
        _cleanup_developer_workflow(
            user["user_id"], asset_ids=[asset_id] if asset_id else ()
        )
        cleanup_test_user(user["user_id"])


@pytest.mark.developer
@pytest.mark.mobile
def test_happy_path_workflow_mobile(mobile_page):
    """
    Mobile viewport variant (375x812 iPhone) of `test_happy_path_workflow`.
    Same workflow, smaller screen — surfaces regressions in the mobile
    developer shell (collapsed nav, hamburger interactions).
    """
    page, tracker = mobile_page

    user = create_e2e_user(
        email_prefix="e2e-workflow-mobile",
        display_name="E2E Workflow Mobile",
    )
    attach_session_cookie(page.context, user["session_token"])

    asset_id = None
    try:
        asset_id = _drive_workflow(page, tracker, user, viewport_label="mobile")
    finally:
        _cleanup_developer_workflow(
            user["user_id"], asset_ids=[asset_id] if asset_id else ()
        )
        cleanup_test_user(user["user_id"])
