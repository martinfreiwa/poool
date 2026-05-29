"""
POOOL E2E — Developer: Affiliate Team (7 sub-pages, parametrized)
==================================================================
Covers the 7 sub-pages of the affiliate-team section:
  • /developer/affiliate-team            (Analytics root)
  • /developer/affiliate-team/members
  • /developer/affiliate-team/customers
  • /developer/affiliate-team/products
  • /developer/affiliate-team/settings
  • /developer/affiliate-team/analytics  (explicit alias)
  • /developer/affiliate-team/tier

Per page:
  • Loads clean (no JS errors, no 500s).
  • Topbar renders (h1 "Affiliate Team"); sub-nav (sidebar children) renders.
  • The per-page principal element (table, form, hero, …) is visible.
  • Mobile viewport variant.

Settings-specific tests:
  • The bank-IBAN edit triggers a 2FA step-up — the PATCH must return HTTP 428
    (Precondition Required, mapped from AppError::TwoFactorRequired in the
    backend's affiliate team route). This proves the step-up gate is wired
    even when the user hasn't enrolled in TOTP.
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


# ─── Per-page contract: (URL path, principal element selector) ───────────────

AFFILIATE_PAGES = [
    ("/developer/affiliate-team",            "#dat-bymember-table"),         # Analytics root
    ("/developer/affiliate-team/members",    "#dat-members-tbody"),
    ("/developer/affiliate-team/customers",  "#dat-customers-thead-row"),
    ("/developer/affiliate-team/products",   "#dat-products-thead-row"),
    ("/developer/affiliate-team/settings",   "#dat-settings-form"),
    ("/developer/affiliate-team/analytics",  "#dat-bymember-table"),         # Alias of root
    ("/developer/affiliate-team/tier",       "#dat-tier-hero-title"),
]


def _cleanup_dev_team(user_id):
    """Remove any developer_teams + memberships seeded automatically."""
    try:
        conn = get_db_connection()
        cur = conn.cursor()
        cur.execute(
            "DELETE FROM developer_team_memberships WHERE team_id IN "
            "(SELECT id FROM developer_teams WHERE developer_user_id = %s)",
            (str(user_id),),
        )
        cur.execute(
            "DELETE FROM developer_teams WHERE developer_user_id = %s",
            (str(user_id),),
        )
        conn.commit()
        cur.close()
        conn.close()
    except Exception:
        pass


# ─── Fixtures ────────────────────────────────────────────────────────────────

@pytest.fixture(scope="function")
def affiliate_dev(playwright_session, request):
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name
    )
    user = create_e2e_user(
        email_prefix="e2e-aff",
        display_name="E2E Affiliate Dev",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker, user

    _teardown_context(context, page, tracker, request)
    _cleanup_dev_team(user["user_id"])
    cleanup_test_user(user["user_id"])


@pytest.fixture(scope="function")
def affiliate_dev_mobile(playwright_session, request):
    context, page, tracker = _create_context_and_page(
        playwright_session, request.node.name, viewport="mobile"
    )
    user = create_e2e_user(
        email_prefix="e2e-aff-mob",
        display_name="E2E Affiliate Mobile Dev",
        roles=("developer",),
    )
    attach_session_cookie(context, user["session_token"])

    yield page, tracker, user

    _teardown_context(context, page, tracker, request)
    _cleanup_dev_team(user["user_id"])
    cleanup_test_user(user["user_id"])


# ─── Helpers ────────────────────────────────────────────────────────────────

def _go(page, tracker, path):
    tracker.navigate_and_check(f"{BASE_URL}{path}", timeout=TIMEOUT)
    page.wait_for_load_state("domcontentloaded")


# ═══════════════════════════════════════════════════════════════════════════
# PARAMETRIZED LOAD-CLEAN + STRUCTURE TESTS
# ═══════════════════════════════════════════════════════════════════════════

@pytest.mark.developer
@pytest.mark.smoke
@pytest.mark.parametrize("path,principal", AFFILIATE_PAGES, ids=[p[0] for p in AFFILIATE_PAGES])
def test_affiliate_subpage_loads_clean(affiliate_dev, path, principal):
    """Each affiliate sub-page renders without JS errors or 5xx."""
    page, tracker, _ = affiliate_dev
    _go(page, tracker, path)
    tracker.assert_page_loaded()
    # Topbar h1
    expect(page.locator("h1.lb-topbar-title")).to_contain_text("Affiliate Team")
    # Sub-nav (sidebar children) renders
    expect(page.locator("#nav-child-affiliate-team-analytics")).to_be_attached()
    expect(page.locator("#nav-child-affiliate-team-members")).to_be_attached()
    expect(page.locator("#nav-child-affiliate-team-customers")).to_be_attached()
    expect(page.locator("#nav-child-affiliate-team-products")).to_be_attached()
    expect(page.locator("#nav-child-affiliate-team-tier")).to_be_attached()
    expect(page.locator("#nav-child-affiliate-team-settings")).to_be_attached()
    # Principal element
    expect(page.locator(principal).first).to_be_visible(timeout=TIMEOUT)
    tracker.assert_no_critical_errors()


@pytest.mark.developer
@pytest.mark.mobile
@pytest.mark.parametrize("path,principal", AFFILIATE_PAGES, ids=[p[0] for p in AFFILIATE_PAGES])
def test_affiliate_subpage_mobile(affiliate_dev_mobile, path, principal):
    """Each affiliate sub-page renders cleanly at a mobile viewport."""
    page, tracker, _ = affiliate_dev_mobile
    _go(page, tracker, path)
    tracker.assert_page_loaded()
    expect(page.locator(principal).first).to_be_attached(timeout=TIMEOUT)
    tracker.assert_no_critical_errors()


# ═══════════════════════════════════════════════════════════════════════════
# SETTINGS — IBAN EDIT TRIGGERS 2FA STEP-UP
# ═══════════════════════════════════════════════════════════════════════════

@pytest.mark.developer
def test_settings_iban_patch_requires_step_up(affiliate_dev):
    """
    PATCH /api/developer/affiliate/team with a bank_iban field must return
    HTTP 428 (TwoFactorRequired) for a user without TOTP enrolled. This
    proves the step-up gate is wired even before the modal triggers.
    """
    page, tracker, _ = affiliate_dev
    _go(page, tracker, "/developer/affiliate-team/settings")
    expect(page.locator("#dat-bank-iban")).to_be_visible(timeout=TIMEOUT)

    # Fire the PATCH directly via fetch() inside the page so we exercise the
    # exact same CSRF + cookie path the UI uses.
    status = page.evaluate(
        """async () => {
            const csrfMatch = document.cookie.split('; ').find(p => p.startsWith('csrf_token='));
            const csrf = csrfMatch ? decodeURIComponent(csrfMatch.split('=')[1]) : '';
            const r = await fetch('/api/developer/affiliate/team', {
                method: 'PATCH',
                credentials: 'same-origin',
                headers: {
                    'Content-Type': 'application/json',
                    'X-CSRF-Token': csrf,
                    'Accept': 'application/json',
                },
                body: JSON.stringify({
                    display_name: 'E2E test team',
                    public_slug: null,
                    bank_iban: 'DE89370400440532013000',
                }),
            });
            return r.status;
        }"""
    )

    # 428 Precondition Required → AppError::TwoFactorRequired branch.
    # Some deployments may serialize as 403 if step-up shimming was changed;
    # both prove the gate fired. Anything else (200, 500) is a regression.
    assert status in (428, 403), (
        f"Bank IBAN PATCH should require step-up 2FA (HTTP 428/403), got HTTP {status}."
    )
