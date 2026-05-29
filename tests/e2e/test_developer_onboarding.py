"""
POOOL E2E — Developer: Onboarding
===================================
Covers /developer/onboarding — the "Become a Developer" application flow.

Key contract (post-2026-05-19 audit, C-1 + C-2 + H-15 fixes):
  • Page loads for ANY authed user (no developer role required).
  • The ToS link inside the review step points at /terms, NOT '#'
    (H-15 regression guard).
  • The page has a "Submit Application" CTA on the final step.
  • Filling + submitting the application form returns 202 Accepted and
    redirects to /marketplace (frontend redirect handles success).
  • After submission, the user does NOT acquire the `developer` role and
    so /developer/dashboard remains gated — confirmed via a follow-up
    GET that must redirect away from /dashboard (C-1 + C-2 regression guard).
"""

import os
import pytest
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

def _cleanup_application(user_id):
    try:
        conn = get_db_connection()
        cur = conn.cursor()
        cur.execute(
            "DELETE FROM developer_applications WHERE user_id = %s",
            (str(user_id),),
        )
        conn.commit()
        cur.close()
        conn.close()
    except Exception:
        pass


@pytest.fixture(scope="function")
def onboarding_user(playwright_session, request):
    """
    A *non-developer* authed user. We use the existing create_e2e_user helper
    but pass roles=() so the user holds no developer role.
    """
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    user = create_e2e_user(
        email_prefix="e2e-onb",
        display_name="E2E Onboarding User",
        roles=(),
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker, user

    _teardown_context(context, page, tracker, request)
    _cleanup_application(user["user_id"])
    cleanup_test_user(user["user_id"])


@pytest.fixture(scope="function")
def onboarding_user_mobile(playwright_session, request):
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    user = create_e2e_user(
        email_prefix="e2e-onb-mob",
        display_name="E2E Onboarding Mobile",
        roles=(),
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker, user

    _teardown_context(context, page, tracker, request)
    _cleanup_application(user["user_id"])
    cleanup_test_user(user["user_id"])


# ─── Helpers ────────────────────────────────────────────────────────────────

def _go_onboarding(page, tracker):
    tracker.navigate_and_check(f"{BASE_URL}/developer/onboarding", timeout=TIMEOUT)
    page.wait_for_load_state("domcontentloaded")


def _fill_step_1(page):
    page.fill("#ob-first-name", "E2E")
    page.fill("#ob-last-name", "Tester")
    page.fill("#ob-phone", "+62 812 555 0001")
    page.select_option("#ob-nationality", "DE")
    page.select_option("#ob-country", "ID")


def _fill_step_2(page):
    page.select_option("#ob-asset-value", "500k-1m")
    page.select_option("#ob-monthly-income", "5k-15k")


# ═══════════════════════════════════════════════════════════════════════════
# TESTS
# ═══════════════════════════════════════════════════════════════════════════

@pytest.mark.developer
@pytest.mark.smoke
def test_onboarding_loads_for_non_developer(onboarding_user):
    """A user without the developer role can reach the onboarding page."""
    page, tracker, _ = onboarding_user
    _go_onboarding(page, tracker)
    tracker.assert_page_loaded()
    expect(page.locator("#content-step-1")).to_be_visible()
    expect(page.locator("#ob-first-name")).to_be_visible()
    tracker.assert_no_critical_errors()


@pytest.mark.developer
def test_tos_link_points_to_terms_not_hash(onboarding_user):
    """
    H-15 regression guard. The Terms-of-Service link in step 3 must point
    at /terms (a real route), not at '#'.
    """
    page, tracker, _ = onboarding_user
    _go_onboarding(page, tracker)

    # Step 3 is hidden by default; advance the stepper via the global helper.
    page.evaluate("goToStep(3)")

    tos = page.locator('#content-step-3 a[href="/terms"]').first
    expect(tos).to_be_visible(timeout=TIMEOUT)
    href = tos.get_attribute("href")
    assert href == "/terms", (
        f"ToS link must be /terms (H-15 regression guard), got {href!r}"
    )
    # Defensive: confirm there is no stray href="#" inside the review section.
    assert page.locator('#content-step-3 a[href="#"]').count() == 0, (
        "Step 3 still has a placeholder href='#' link (H-15 regression!)"
    )


@pytest.mark.developer
def test_submit_application_button_present(onboarding_user):
    """
    Step 3 has a 'Submit Application' CTA. Verify the button is the standard
    one (#btn-submit) and the previous step contains a continue button leading
    to /developer/application-form? — actually onboarding submits via JS to
    /api/developer/apply, so only the CTA must exist.
    """
    page, tracker, _ = onboarding_user
    _go_onboarding(page, tracker)
    page.evaluate("goToStep(3)")

    expect(page.locator("#btn-submit")).to_be_visible(timeout=TIMEOUT)
    expect(page.locator("#btn-submit")).to_contain_text("Submit Application")


@pytest.mark.developer
def test_application_submit_returns_202_and_redirects(onboarding_user):
    """
    Filling the application form and clicking Submit must:
      • POST /api/developer/apply with status 202 Accepted
      • redirect the user to /marketplace (per onboarding-agent change).
    """
    page, tracker, _ = onboarding_user
    _go_onboarding(page, tracker)

    # Walk the stepper through 1 → 2 → 3
    _fill_step_1(page)
    page.evaluate("goToStep(2)")
    _fill_step_2(page)
    page.evaluate("goToStep(3)")

    # POST /api/developer/apply — backend returns 202 ACCEPTED on success
    with page.expect_response(
        lambda r: "/api/developer/apply" in r.url and r.request.method == "POST",
        timeout=TIMEOUT,
    ) as resp_info:
        page.locator("#btn-submit").click()

    resp = resp_info.value
    assert resp.status == 202, (
        f"Application submit should return 202 Accepted, got {resp.status}: {resp.text()}"
    )

    # The JS then redirects to /marketplace
    page.wait_for_url("**/marketplace**", timeout=TIMEOUT)


@pytest.mark.developer
def test_submitting_application_does_not_grant_developer_role(onboarding_user):
    """
    C-1 + C-2 regression guard. After submitting the application:
      • the user MUST NOT acquire the developer role automatically
      • /developer/dashboard must therefore redirect/403 them
      • The redirect target is /developer/application-form (per
        require_developer_page in backend/routes.rs).
    """
    page, tracker, _ = onboarding_user
    _go_onboarding(page, tracker)
    _fill_step_1(page)
    page.evaluate("goToStep(2)")
    _fill_step_2(page)
    page.evaluate("goToStep(3)")

    with page.expect_response(
        lambda r: "/api/developer/apply" in r.url and r.request.method == "POST",
        timeout=TIMEOUT,
    ) as resp_info:
        page.locator("#btn-submit").click()
    assert resp_info.value.status == 202

    page.wait_for_url("**/marketplace**", timeout=TIMEOUT)

    # Now attempt /developer/dashboard — must NOT serve the dashboard.
    response = page.goto(f"{BASE_URL}/developer/dashboard", wait_until="domcontentloaded")
    final_url = page.url
    # Either the response itself was a 4xx, OR the URL ended somewhere
    # outside /developer/dashboard (redirect to /developer/application-form
    # or /auth/login).
    landed_on_dashboard = final_url.rstrip("/").endswith("/developer/dashboard")
    assert not landed_on_dashboard, (
        f"Dashboard should NOT be reachable post-application — final URL: {final_url} "
        f"(C-1 + C-2 regression!)"
    )
    # Defensive: assert the redirect landed on a known onboarding/auth surface.
    assert any(s in final_url for s in (
        "/developer/application-form",
        "/developer/onboarding",
        "/auth/login",
        "/marketplace",
    )), f"Unexpected post-dashboard URL: {final_url}"


@pytest.mark.developer
@pytest.mark.mobile
def test_onboarding_mobile_loads(onboarding_user_mobile):
    """At mobile viewport the onboarding page renders cleanly."""
    page, tracker, _ = onboarding_user_mobile
    _go_onboarding(page, tracker)
    tracker.assert_page_loaded()
    expect(page.locator("#content-step-1")).to_be_visible()
    tracker.assert_no_critical_errors()
