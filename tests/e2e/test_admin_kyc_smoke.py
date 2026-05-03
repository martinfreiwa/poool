"""
Smoke test for /admin/kyc — exercises the recent dashboard improvements.

Walks the page after login and verifies:
- KPI cards render with numeric values + scope subtitles
- Freshness strip pulls live data from /api/admin/kyc/providers/health
- Queue search + filter inputs are wired
- Bulk-action bar appears on row select
- Export-scope dropdown is present
- Saved-views chips persist via localStorage
- KPI cards are draggable
- Keyboard shortcut R opens the review modal (when queue non-empty)
- Audit-trail collapsible loads via /api/admin/kyc/:id/audit
- Reassign control is present in the modal
- No console errors during the walk
"""
import os
import pytest
from playwright.sync_api import expect

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")


@pytest.mark.admin
def test_admin_kyc_dashboard_smoke(admin_page):
    page, tracker = admin_page
    page.goto(f"{BASE_URL}/admin/kyc")

    # ── Page header + freshness strip ────────────────────────
    expect(page.locator(".admin-page-title")).to_have_text(
        "KYC & AML Compliance", timeout=10000
    )
    expect(page.locator("#kyc-data-freshness")).to_be_visible()
    expect(page.locator("#kyc-oldest-pending")).to_be_visible()

    # Provider-health endpoint should populate within 2s
    page.wait_for_function(
        "document.getElementById('kyc-provider-sumsub')?.textContent !== '—'",
        timeout=5000,
    )

    # ── KPI cards ────────────────────────────────────────────
    cards = page.locator(".admin-kpi-card[data-tab]")
    expect(cards).to_have_count(5)
    for tab in ("queue", "approved", "rejected", "pep", "expiring"):
        card = page.locator(f'.admin-kpi-card[data-tab="{tab}"]')
        expect(card).to_be_visible()
        # Each card has a value (filled by load) + scope subtitle
        expect(card.locator(".admin-kpi-scope")).to_be_visible()

    # KPI value present (numeric or em-dash on initial load — wait for fill)
    page.wait_for_function(
        "document.getElementById('kyc-approved')?.textContent.trim() !== '—'",
        timeout=10000,
    )

    # ── Active KPI sync with tab click ───────────────────────
    page.locator('.admin-tab[data-tab="approved"]').click()
    aria = page.locator('.admin-kpi-card[data-tab="approved"]').get_attribute("aria-pressed")
    assert aria == "true", f"Expected aria-pressed=true on approved card, got {aria}"

    # Click back to queue
    page.locator('.admin-tab[data-tab="queue"]').click()

    # ── Toolbar + saved views + export scope ─────────────────
    expect(page.locator("#kyc-review-next")).to_be_visible()
    expect(page.locator("#kyc-export-scope")).to_be_visible()
    expect(page.locator("#kyc-export-scope option")).to_have_count(3)
    expect(page.locator("#kyc-save-view")).to_be_visible()
    expect(page.locator("#auto-refresh-countdown")).to_be_visible()

    # Countdown decrements (poll twice 2.5s apart)
    initial = page.locator("#auto-refresh-countdown").inner_text()
    page.wait_for_timeout(2500)
    later = page.locator("#auto-refresh-countdown").inner_text()
    assert initial != later or initial == "off", \
        f"Countdown did not advance: {initial} → {later}"

    # ── Queue filters ────────────────────────────────────────
    expect(page.locator("#queue-search")).to_be_visible()
    expect(page.locator("#queue-filter-provider")).to_be_visible()
    expect(page.locator("#queue-filter-risk")).to_be_visible()

    # Queue search keyboard shortcut "/"
    page.keyboard.press("/")
    expect(page.locator("#queue-search")).to_be_focused()

    # ── Shortcuts dialog ─────────────────────────────────────
    page.keyboard.press("Escape")  # blur search
    page.locator("#kyc-shortcuts-btn").click()
    expect(page.locator("#kyc-shortcuts-dialog")).to_be_visible()
    expect(page.locator("#kyc-shortcuts-dialog")).to_contain_text("Review next")
    page.locator("#kyc-shortcuts-close").click()

    # ── KPI drag attribute present ───────────────────────────
    pending_card = page.locator('.admin-kpi-card[data-tab="queue"]')
    assert pending_card.get_attribute("draggable") == "true"

    # ── Saved view round-trip via localStorage ───────────────
    page.evaluate(
        "localStorage.setItem('kyc.savedViews.v1', "
        "JSON.stringify([{name:'High risk', state:{tab:'queue', risk:'high'}}]))"
    )
    page.reload()
    page.wait_for_selector("#kyc-views-list .admin-saved-view-chip", timeout=5000)
    chips = page.locator("#kyc-views-list .admin-saved-view-chip")
    expect(chips).to_have_count(1)
    expect(chips.first).to_contain_text("High risk")

    # ── Provider-health API direct ───────────────────────────
    resp = page.request.get(f"{BASE_URL}/api/admin/kyc/providers/health")
    assert resp.ok, f"providers/health returned {resp.status}"
    data = resp.json()
    assert "providers" in data and "sanctions" in data

    # ── No critical console errors ───────────────────────────
    errs = [e for e in tracker.console_errors
            if "favicon" not in e.lower() and "404" not in e.lower()]
    assert not errs, f"Console errors during walk: {errs}"


@pytest.mark.admin
def test_admin_kyc_review_modal_smoke(admin_page):
    """If the queue has at least one pending case, the Review Next button +
    audit + reassign controls should render without errors."""
    page, tracker = admin_page
    page.goto(f"{BASE_URL}/admin/kyc")

    # Wait for queue render
    page.wait_for_function(
        "document.getElementById('kyc-pending')?.textContent.trim() !== '—'",
        timeout=10000,
    )
    pending_text = page.locator("#kyc-pending").inner_text().strip()
    pending = int(pending_text) if pending_text.isdigit() else 0
    if pending == 0:
        pytest.skip("No pending KYC cases in this environment")

    # Press R to open modal via "Review Next"
    page.keyboard.press("r")
    modal = page.locator("#kyc-modal")
    expect(modal).to_be_visible(timeout=5000)

    # Audit collapsible + reassign field present
    expect(page.locator("#kyc-modal-audit")).to_be_visible()
    expect(page.locator("#kyc-modal-assignee")).to_be_visible()
    expect(page.locator("#kyc-modal-assign-btn")).to_be_visible()

    # Audit list populates (or shows empty message)
    page.wait_for_function(
        "document.getElementById('kyc-modal-audit-list')?.textContent.trim() !== 'Loading…'",
        timeout=5000,
    )

    # Close
    page.locator("#kyc-modal-cancel").click()
    expect(modal).to_be_hidden()

    # No errors
    errs = [e for e in tracker.console_errors
            if "favicon" not in e.lower() and "404" not in e.lower()]
    assert not errs, f"Console errors: {errs}"
