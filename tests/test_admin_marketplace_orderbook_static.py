"""Static checks for admin Orderbook page wiring.

These guard against regressions in the orderbook page DOM, JS state, CSS
hooks, and backend handler that expose the enriched market context the UI
depends on.
"""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ADMIN_MARKETPLACE = ROOT / "backend/src/admin/marketplace.rs"
ORDERBOOK_HTML = ROOT / "frontend/platform/admin/marketplace/orderbook.html"
ORDERBOOK_JS = ROOT / "frontend/platform/static/js/mp-orderbook.js"
ORDERBOOK_CSS = ROOT / "frontend/platform/static/css/mp-orderbook.css"
ADMIN_MOD = ROOT / "backend/src/admin/mod.rs"


def test_admin_orderbook_response_exposes_market_context_fields():
    source = ADMIN_MARKETPLACE.read_text(encoding="utf-8")
    assert "pub struct AdminOrderbook {" in source
    for field in (
        "best_bid_cents",
        "best_ask_cents",
        "mid_price_is_fallback",
        "last_trade_cents",
        "last_trade_at",
        "volume_24h_cents",
        "trades_24h",
        "change_24h_pct",
        "bid_volume",
        "ask_volume",
        "market_status",
        "generated_at",
        "last_rebuild_at",
    ):
        assert f"pub {field}" in source, f"AdminOrderbook missing field {field}"


def test_admin_orderbook_mid_price_falls_back_to_one_sided_book():
    source = ADMIN_MARKETPLACE.read_text(encoding="utf-8")
    # Fallback covers (Some, None) and (None, Some).
    assert "(Some(bid), None) => (Some(bid), true)" in source
    assert "(None, Some(ask)) => (Some(ask), true)" in source


def test_admin_orderbook_level_endpoint_is_registered():
    source = ADMIN_MOD.read_text(encoding="utf-8")
    assert '"/api/admin/marketplace/orderbook/:asset_id/level"' in source
    assert "api_admin_marketplace_orderbook_level" in source


def test_orderbook_html_has_combobox_toolbar_and_footer():
    source = ORDERBOOK_HTML.read_text(encoding="utf-8")
    # Combobox + KPI strip + live indicator.
    assert 'id="asset-combobox"' in source
    assert 'id="ob-stats"' in source
    assert 'id="ob-live-indicator"' in source
    # Toolbar controls.
    assert 'class="mp-ob-toolbar"' in source
    assert 'id="mp-ob-tick"' in source
    assert 'id="mp-ob-min-qty"' in source
    assert 'id="mp-ob-tz"' in source
    # Action buttons.
    assert 'id="btn-pause-refresh"' in source
    assert 'id="btn-asset-settings"' in source
    assert 'id="btn-export-csv"' in source
    assert 'id="btn-rebuild-orderbook"' in source
    assert 'id="btn-rebuild-history"' in source
    # Bid/ask tables include cumulative columns.
    assert "Cum. Qty" in source
    assert "Cum. USD" in source
    # Footer + tz note.
    assert 'id="mp-ob-tz-note"' in source


def test_orderbook_js_persists_prefs_and_reports_errors():
    source = ORDERBOOK_JS.read_text(encoding="utf-8")
    # Aging thresholds.
    assert "AGE_AMBER_S = 3600" in source
    assert "AGE_RED_S = 86400" in source
    # localStorage prefs.
    assert "PREFS_KEY" in source
    assert "function loadPrefs" in source
    assert "function savePrefs" in source
    # Error breadcrumbs.
    assert "function reportError" in source
    assert 'reportError("cancelOrder"' in source
    assert 'reportError("rebuildOrderbook"' in source
    # Tick aggregation, side filter, tz state.
    assert "function aggregateLevels" in source
    assert "state.sideFilter" in source
    assert 'state.tz === "utc"' in source
    # CSV export.
    assert "function exportCsv" in source
    # Rebuild history dropdown.
    assert "function loadRebuildHistory" in source
    # Combobox arrow-key nav.
    assert "filteredAssets" in source
    assert "highlightCursor" in source


def test_orderbook_css_has_aging_status_pill_and_history_styles():
    source = ORDERBOOK_CSS.read_text(encoding="utf-8")
    assert ".mp-ob-age--amber" in source
    assert ".mp-ob-age--red" in source
    assert ".mp-ob-status-pill--ok" in source
    assert ".mp-ob-status-pill--warn" in source
    assert ".mp-ob-history-panel" in source
    assert ".mp-ob-empty-cta" in source
    assert ".mp-ob-combo-item.is-cursor" in source
