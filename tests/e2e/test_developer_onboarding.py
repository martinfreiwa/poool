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
    redirects pending applicants to /marketplace.
  • After submission, the user does NOT acquire the `developer` role and
    developer-only APIs remain forbidden until admin approval.
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
    _set_select_value(page, "#ob-nationality", "DE")
    _set_select_value(page, "#ob-country", "ID")


def _fill_step_2(page):
    _set_select_value(page, "#ob-asset-value", "500k-1m")
    _set_select_value(page, "#ob-monthly-income", "5k-15k")


def _set_select_value(page: Page, selector: str, value: str):
    """Set POOOL-enhanced selects that keep the native <select> hidden."""
    page.locator(selector).evaluate(
        """(el, value) => {
            el.value = value;
            el.dispatchEvent(new Event('change', { bubbles: true }));
        }""",
        value,
    )


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
      • redirect the pending applicant to /marketplace.
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

    # The JS then redirects away from the protected developer area.
    page.wait_for_url("**/marketplace**", timeout=TIMEOUT)


@pytest.mark.developer
def test_submitting_application_does_not_grant_developer_role(onboarding_user):
    """
    C-1 + C-2 regression guard. After submitting the application:
      • the user MUST NOT acquire the developer role automatically
      • protected developer pages remain unavailable until admin approval
      • developer-only APIs must still reject them until admin approval.
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

    dashboard_response = page.goto(
        f"{BASE_URL}/developer/dashboard",
        wait_until="domcontentloaded",
        timeout=TIMEOUT,
    )
    assert dashboard_response is None or dashboard_response.status in {200, 302, 303, 403}
    assert not page.url.rstrip("/").endswith("/developer/dashboard"), (
        "Pending developer applicants must not reach the protected developer "
        "dashboard before admin approval"
    )

    api_resp = page.request.get(f"{BASE_URL}/api/developer/dashboard/stats")
    assert api_resp.status == 403, (
        "Pending developer applicants must not get developer API access before "
        f"admin approval; got HTTP {api_resp.status}"
    )


@pytest.mark.developer
@pytest.mark.mobile
def test_onboarding_mobile_loads(onboarding_user_mobile):
    """At mobile viewport the onboarding page renders cleanly."""
    page, tracker, _ = onboarding_user_mobile
    _go_onboarding(page, tracker)
    tracker.assert_page_loaded()
    expect(page.locator("#content-step-1")).to_be_visible()
    tracker.assert_no_critical_errors()
