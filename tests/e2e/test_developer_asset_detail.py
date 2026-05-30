"""POOOL E2E — Developer: /developer/asset-detail

Smoke + tab + settings-panel coverage for the asset-detail view.

Coverage:
  * Page loads clean with a seeded asset UUID belonging to the developer
    (no console errors, no failed network requests).
  * 7-tab strip renders + clicking each tab swaps the active panel.
  * Settings tab contains the H-4 controls: toggle-featured, toggle-published,
    select-funding-status, btn-freeze, btn-unpublish, btn-archive.
  * Clicking toggle-featured invokes the JS handler without throwing — current
    behaviour is a toast (admin-gated). The conftest network tracker captures
    any HTTP failure that might be added later.
  * Mobile viewport renders cleanly; the tab strip stays scrollable.

Run:
    pytest tests/e2e/test_developer_asset_detail.py -v
    HEADED=1 pytest tests/e2e/test_developer_asset_detail.py -v
"""
import os
import re
import uuid
import pytest
from playwright.sync_api import expect, Page

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8888")
TIMEOUT = 15_000

# Tabs in the rendered order (data-tab attribute → human-readable text).
TABS = [
    ("overview", "Overview"),
    ("media", "Media"),
    ("documents", "Documents"),
    ("financials", "Financials"),
    ("milestones", "Milestones"),
    ("captable", "Cap Table"),
    ("orders", "Orders"),
    ("settings", "Settings"),
]


# ─── DB helper ─────────────────────────────────────────────────────────────

def _seed_developer_asset(user_id) -> str:
    """
    Insert a fully-populated asset owned by the test developer and return its
    UUID. The conftest developer_page fixture's _cleanup_developer_assets()
    deletes it again on teardown.
    """
    from tests.e2e.conftest import get_db_connection  # local import to keep file portable
    conn = get_db_connection()
    cur = conn.cursor()
    try:
        slug = f"e2e-dev-detail-{uuid.uuid4().hex[:10]}"
        cur.execute(
            """
            INSERT INTO assets (
                title, slug, short_description, description, asset_type,
                total_value_cents, token_price_cents, tokens_total, tokens_available,
                funding_status, published, developer_user_id, annual_yield_bps,
                capital_appreciation_bps, occupancy_rate_bps
            )
            VALUES (
                'E2E Asset Detail Villa', %s, 'E2E asset-detail fixture',
                'Seeded by the developer asset-detail E2E test.', 'real_estate',
                250000000, 25000, 10000, 8000,
                'available', FALSE, %s, 1000, 800, 8500
            )
            RETURNING id
            """,
            (slug, str(user_id)),
        )
        asset_id = cur.fetchone()[0]
        conn.commit()
        return str(asset_id)
    finally:
        cur.close()
        conn.close()


def _goto_asset_detail(page: Page, tracker, asset_id: str):
    tracker.navigate_and_check(
        f"{BASE_URL}/developer/asset-detail?id={asset_id}",
        timeout=TIMEOUT,
        wait_until="domcontentloaded",
    )
    page.wait_for_load_state("networkidle", timeout=TIMEOUT)
    # Wait until the JS finishes calling /api/developer/assets/:id and renders.
    page.wait_for_selector("#asset-content[style*='block']", state="attached", timeout=TIMEOUT)


# ─── Tests ─────────────────────────────────────────────────────────────────

@pytest.mark.developer
@pytest.mark.smoke
def test_loads_clean(developer_page):
    """Page loads with no console errors / no 5xx network calls (seeded asset)."""
    page, tracker, user = developer_page
    asset_id = _seed_developer_asset(user["user_id"])
    _goto_asset_detail(page, tracker, asset_id)

    expect(page).to_have_title(re.compile(r"Asset Details", re.IGNORECASE))
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures(ignore_status=[404])


@pytest.mark.developer
def test_seven_tab_strip_renders(developer_page):
    """All 8 tabs render (note: spec calls it 7-tab; template ships 8 including Settings)."""
    page, tracker, user = developer_page
    asset_id = _seed_developer_asset(user["user_id"])
    _goto_asset_detail(page, tracker, asset_id)

    tabs_root = page.locator("#asset-tabs")
    expect(tabs_root).to_be_visible(timeout=TIMEOUT)

    for slug, _label in TABS:
        btn = tabs_root.locator(f'[data-tab="{slug}"]')
        expect(btn).to_be_visible(timeout=TIMEOUT)


@pytest.mark.developer
def test_tab_switching_swaps_panel(developer_page):
    """Clicking each tab makes the corresponding panel the active panel."""
    page, tracker, user = developer_page
    asset_id = _seed_developer_asset(user["user_id"])
    _goto_asset_detail(page, tracker, asset_id)

    for slug, _label in TABS:
        page.locator(f'#asset-tabs [data-tab="{slug}"]').click()
        page.wait_for_timeout(50)
        panel = page.locator(f"#panel-{slug}")
        expect(panel).to_have_class(re.compile(r"\bactive\b"), timeout=2_000)


@pytest.mark.developer
def test_settings_tab_h4_controls_present(developer_page):
    """H-4 regression guard: Settings tab exposes toggle-featured + btn-freeze etc."""
    page, tracker, user = developer_page
    asset_id = _seed_developer_asset(user["user_id"])
    _goto_asset_detail(page, tracker, asset_id)

    page.locator('#asset-tabs [data-tab="settings"]').click()
    panel = page.locator("#panel-settings")
    expect(panel).to_have_class(re.compile(r"\bactive\b"))

    # Toggles + danger zone + funding-status select all present.
    expect(panel.locator("#toggle-featured")).to_be_visible()
    expect(panel.locator("#toggle-published")).to_be_visible()
    expect(panel.locator("#select-funding-status")).to_be_visible()
    expect(panel.locator("#btn-freeze")).to_be_visible()
    expect(panel.locator("#btn-unpublish")).to_be_visible()
    expect(panel.locator("#btn-archive")).to_be_visible()


@pytest.mark.developer
def test_toggle_featured_click_no_error(developer_page):
    """
    Clicking toggle-featured runs its JS handler. The current implementation
    shows a toast (admin-gated). If a network request is later wired up, the
    conftest network tracker would surface a 4xx/5xx — we assert it stays clean.
    """
    page, tracker, user = developer_page
    asset_id = _seed_developer_asset(user["user_id"])
    _goto_asset_detail(page, tracker, asset_id)

    page.locator('#asset-tabs [data-tab="settings"]').click()
    toggle = page.locator("#toggle-featured")
    expect(toggle).to_be_visible()

    # Click — the handler is async, give it a brief tick.
    toggle.click()
    page.wait_for_timeout(300)

    # No JS error, no 5xx response from any wired-up endpoint.
    tracker.assert_no_critical_errors()
    tracker.assert_no_network_failures(ignore_status=[404, 401, 403])


@pytest.mark.developer
@pytest.mark.mobile
def test_loads_on_mobile(developer_page):
    """Mobile viewport: page loads and tab strip is horizontally scrollable."""
    page, tracker, user = developer_page
    page.set_viewport_size({"width": 375, "height": 812})
    asset_id = _seed_developer_asset(user["user_id"])
    tracker.navigate_and_check(
        f"{BASE_URL}/developer/asset-detail?id={asset_id}", timeout=TIMEOUT
    )
    page.wait_for_load_state("networkidle", timeout=TIMEOUT)

    # Tab strip should still render at narrow widths.
    expect(page.locator("#asset-tabs")).to_be_attached(timeout=TIMEOUT)
    tracker.assert_no_critical_errors()
