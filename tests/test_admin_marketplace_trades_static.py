from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
ADMIN_MARKETPLACE = REPO_ROOT / "backend/src/admin/marketplace.rs"
TRADES_HTML = REPO_ROOT / "frontend/platform/admin/marketplace/trades.html"
TRADES_JS = REPO_ROOT / "frontend/platform/static/js/mp-trades.js"
ADMIN_MP_CSS = REPO_ROOT / "frontend/platform/static/css/admin-marketplace.css"


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


# ── Backend ──────────────────────────────────────────────────────────


def test_trade_filters_accept_search_and_sort_params():
    source = _read(ADMIN_MARKETPLACE)
    assert "pub q: Option<String>" in source
    assert "pub sort_by: Option<String>" in source
    assert "pub sort_dir: Option<String>" in source


def test_trade_sort_columns_are_allowlisted():
    source = _read(ADMIN_MARKETPLACE)
    # allowlist enum prevents SQL injection via sort_by
    assert "enum TradeSortColumn" in source
    assert "ExecutedAt" in source
    assert "TotalCents" in source
    assert "PriceCents" in source
    assert "Quantity" in source
    assert "FeeCents" in source
    # bad sort_by must error, not silently fallback
    assert "sort_by must be one of" in source
    assert "sort_dir must be asc or desc" in source


def test_trade_search_uses_bound_pattern_against_id_and_emails():
    source = _read(ADMIN_MARKETPLACE)
    assert "t.id::text ILIKE" in source
    assert "bu.email ILIKE" in source
    assert "su.email ILIKE" in source
    # uses push_bind, not string interpolation
    assert "query.push_bind(pattern" in source


def test_trades_response_includes_summary_aggregates():
    source = _read(ADMIN_MARKETPLACE)
    assert "pub struct AdminTradesSummary" in source
    assert "pub total_quantity: i64" in source
    assert "pub total_volume_cents: i64" in source
    assert "pub total_fee_cents: i64" in source
    assert "pub oldest_pending_age_seconds: Option<i64>" in source
    assert "pub over_sla_count: i64" in source
    assert "pub struct AdminTradesResponse" in source
    # handler returns the new response type with summary
    assert "Result<Json<AdminTradesResponse>" in source
    assert "summarize_admin_trades" in source


def test_trade_summary_query_flags_pending_over_one_hour():
    source = _read(ADMIN_MARKETPLACE)
    assert "WHERE t.on_chain_status = 'pending'" in source
    assert "NOW() - INTERVAL '1 hour'" in source
    assert "TRADE_PENDING_SLA_SECONDS: i64 = 3600" in source


# ── Frontend HTML ────────────────────────────────────────────────────


def test_trades_html_replaces_side_column_with_age():
    html = _read(TRADES_HTML)
    # Side column is gone; Age column is present
    assert "<th>Side</th>" not in html
    assert ">Age</th>" in html
    # Buyer/Seller headers are now plain "Buyer"/"Seller", not "Buyer ID"
    assert ">Buyer</th>" in html
    assert ">Seller</th>" in html


def test_trades_html_has_search_input_and_quick_range_chips():
    html = _read(TRADES_HTML)
    assert 'id="filter-search"' in html
    assert 'placeholder="Search trade ID or email' in html
    for r in ("today", "24h", "7d", "30d", "all"):
        assert f'data-range="{r}"' in html


def test_trades_html_has_sortable_headers_with_aria_sort():
    html = _read(TRADES_HTML)
    for key in ("executed_at", "quantity", "price", "fee", "total"):
        assert f'data-sort-key="{key}"' in html
    assert 'aria-sort="descending"' in html


def test_trades_html_has_sla_banner_and_totals_footer():
    html = _read(TRADES_HTML)
    assert 'id="trades-sla-banner"' in html
    assert 'id="trades-foot"' in html
    assert 'id="totals-qty"' in html
    assert 'id="totals-fee"' in html
    assert 'id="totals-volume"' in html


def test_trades_html_has_refresh_button_and_updated_stamp():
    html = _read(TRADES_HTML)
    assert 'id="btn-refresh-trades"' in html
    assert 'id="trades-updated-stamp"' in html


def test_trades_subtitle_no_longer_claims_only_executed():
    html = _read(TRADES_HTML)
    # old copy was misleading because pending trades are not yet executed onchain
    assert "All executed marketplace trades" not in html


# ── Frontend JS ──────────────────────────────────────────────────────


def test_trades_js_sends_search_and_sort_params():
    js = _read(TRADES_JS)
    assert "params.set('q', search)" in js
    assert "params.set('sort_by', sortBy)" in js
    assert "params.set('sort_dir', sortDir)" in js


def test_trades_js_has_sla_threshold_and_whale_threshold():
    js = _read(TRADES_JS)
    assert "WHALE_THRESHOLD_CENTS = 1_000_000" in js
    assert "SLA_SECONDS = 3600" in js


def test_trades_js_renders_age_with_over_sla_class_for_pending():
    js = _read(TRADES_JS)
    assert "function appendAgeCell" in js
    assert "mp-age-over-sla" in js
    # only flags pending trades, not confirmed
    assert "status === 'pending'" in js


def test_trades_js_renders_summary_and_sla_banner():
    js = _read(TRADES_JS)
    assert "renderSummary(data.summary)" in js
    assert "renderBanner(data.summary)" in js
    assert "over_sla_count" in js
    assert "oldest_pending_age_seconds" in js


def test_trades_js_search_input_is_debounced():
    js = _read(TRADES_JS)
    assert "SEARCH_DEBOUNCE_MS" in js
    assert "searchTimer" in js


def test_trades_js_auto_refreshes_when_tab_visible():
    js = _read(TRADES_JS)
    assert "AUTO_REFRESH_MS" in js
    assert "document.visibilityState === 'visible'" in js
    assert "loadTrades({ silent: true })" in js


def test_trades_js_quick_range_chips_set_date_range():
    js = _read(TRADES_JS)
    assert "function setQuickRange" in js
    for r in ("'today'", "'24h'", "'7d'", "'30d'", "'all'"):
        assert r in js


def test_trades_js_marks_whale_trades_above_threshold():
    js = _read(TRADES_JS)
    assert "mp-money-whale" in js
    assert "isWhale" in js


def test_trades_js_keeps_apply_button_for_dropdown_filters():
    """The E2E test relies on Apply triggering the request after dropdown change."""
    js = _read(TRADES_JS)
    assert "btn-apply-filter" in js
    # No change-listener on filter-asset / filter-status that would auto-fire
    # before the test clicks Apply.
    assert "filter-asset')?.addEventListener('change'" not in js
    assert "filter-status')?.addEventListener('change'" not in js


# ── Frontend CSS ─────────────────────────────────────────────────────


def test_trades_css_has_sla_banner_and_chip_styles():
    css = _read(ADMIN_MP_CSS)
    assert ".mp-sla-banner" in css
    assert ".mp-chip" in css
    assert ".mp-sortable" in css
    assert ".mp-age-over-sla" in css
    assert ".mp-money-whale" in css
    assert ".mp-totals-row" in css
    assert ".mp-alias-flag" in css


# ── Drill-down drawer ────────────────────────────────────────────────


def test_trades_html_has_detail_drawer_markup():
    html = _read(TRADES_HTML)
    assert 'id="trade-drawer"' in html
    assert 'id="trade-drawer-backdrop"' in html
    assert 'id="trade-drawer-body"' in html
    assert 'id="trade-drawer-close"' in html
    assert 'role="dialog"' in html
    assert 'aria-labelledby="trade-drawer-title"' in html


def test_trades_js_opens_drawer_on_row_click():
    js = _read(TRADES_JS)
    assert "function openTradeDrawer" in js
    assert "function closeTradeDrawer" in js
    assert "function bindRowDrilldown" in js
    # row click and Enter/Space keyboard activation both supported
    assert "tr.mp-trade-row" in js
    assert "e.key !== 'Enter' && e.key !== ' '" in js


def test_trades_js_drawer_closes_on_esc_and_backdrop():
    js = _read(TRADES_JS)
    assert "trade-drawer-backdrop" in js
    assert "e.key === 'Escape'" in js
    assert "closeTradeDrawer" in js


def test_trades_js_drawer_uses_full_uuids_with_copy_buttons():
    js = _read(TRADES_JS)
    # full UUIDs go into copyableId, not compactId
    assert "function copyableId" in js
    assert "function copyToClipboard" in js
    assert "navigator.clipboard?.writeText" in js
    # buyer/seller link out to user-details
    assert "/admin/user-details.html?id=" in js
    assert "/admin/asset-details.html?id=" in js


def test_trades_js_row_click_ignores_interactive_children():
    js = _read(TRADES_JS)
    # clicks on copy buttons / links inside the row must not also open the drawer
    assert "button, a, input, label, .mp-trade-copy-btn, .mp-check-wrap" in js


def test_trades_js_rows_are_keyboard_focusable_buttons():
    js = _read(TRADES_JS)
    assert "row.tabIndex = 0" in js
    assert "row.setAttribute('role', 'button')" in js


def test_trades_css_has_drawer_backdrop_and_row_hover_styles():
    css = _read(ADMIN_MP_CSS)
    assert ".mp-p2p-drawer-backdrop" in css
    assert ".mp-trade-row" in css
    assert ".mp-trade-copy-btn" in css


# ── Next-10 batch ────────────────────────────────────────────────────


# 1. Bulk actions
def test_trades_html_has_bulk_bar_and_checkbox_column():
    html = _read(TRADES_HTML)
    assert 'id="trades-bulk-bar"' in html
    assert 'id="trades-bulk-count"' in html
    assert 'id="select-all-trades"' in html
    assert 'id="btn-bulk-copy-ids"' in html
    assert 'id="btn-bulk-export"' in html
    assert 'id="btn-bulk-clear"' in html
    assert 'class="mp-th-checkbox"' in html


def test_trades_js_implements_bulk_actions():
    js = _read(TRADES_JS)
    assert "function appendCheckboxCell" in js
    assert "selectedTradeIds" in js
    assert "function renderBulkBar" in js
    assert "function syncSelectAllCheckbox" in js
    assert "function bindBulkActions" in js
    # selecting all visible / clearing
    assert "all.indeterminate" in js
    # client-side CSV export of just the selected rows
    assert "marketplace_trades_selected_" in js


# 2. Filter summary bar
def test_trades_html_has_filter_summary_bar():
    html = _read(TRADES_HTML)
    assert 'id="trades-filter-summary"' in html


def test_trades_js_renders_filter_summary_with_clear_all():
    js = _read(TRADES_JS)
    assert "function renderFilterSummary" in js
    assert "function clearAllFilters" in js
    assert "Showing" in js
    assert "Filters:" in js


# 3. Status preset chips
def test_trades_html_has_status_preset_chips():
    html = _read(TRADES_HTML)
    for preset in ("action_required", "pending", "failed", "all"):
        assert f'data-status-preset="{preset}"' in html


def test_trades_js_applies_status_preset_filter():
    js = _read(TRADES_JS)
    assert "function applyStatusPreset" in js
    assert "mp-status-preset" in js
    # presets toggle aria-pressed for visible state
    assert "aria-pressed" in js


# 4. Pagination — page-size + jump
def test_trades_js_pagination_has_page_size_and_jump():
    js = _read(TRADES_JS)
    assert "PAGE_SIZE_OPTIONS" in js
    assert "[15, 25, 50, 100]" in js
    assert "id = 'pg-size'" in js
    assert "id = 'pg-jump'" in js
    assert "writePageSize" in js
    # jump-to-page validates the bounds
    assert "v >= 1 && v <= totalPages" in js


def test_trades_js_persists_page_size_to_localstorage():
    js = _read(TRADES_JS)
    assert "PAGE_SIZE_KEY" in js
    assert "localStorage.getItem(PAGE_SIZE_KEY)" in js
    assert "localStorage.setItem(PAGE_SIZE_KEY" in js


# 5. Fee header tooltip
def test_trades_html_has_fee_tooltip_explaining_who_pays():
    html = _read(TRADES_HTML)
    assert 'class="mp-info-icon"' in html
    assert "Fee charged to buyer" in html


# 6. Reconciliation deep-link banner
def test_trades_html_has_recon_banner_zone():
    html = _read(TRADES_HTML)
    assert 'id="trades-recon-banner"' in html


def test_trades_js_renders_recon_banner_with_link():
    js = _read(TRADES_JS)
    assert "function renderReconBanner" in js
    assert "/admin/marketplace/reconciliation.html" in js
    # SLA banner and recon banner are mutually exclusive
    assert "oldest == null || overSla > 0" in js


# 7. Keyboard shortcuts
def test_trades_html_has_kbd_shortcut_hint():
    html = _read(TRADES_HTML)
    assert 'class="mp-kbd-hint"' in html
    assert "<kbd>/</kbd>" in html
    assert "<kbd>r</kbd>" in html
    assert "<kbd>Esc</kbd>" in html


def test_trades_js_binds_keyboard_shortcuts():
    js = _read(TRADES_JS)
    assert "function bindKeyboardShortcuts" in js
    # `/` focuses search, `r` refreshes, Esc clears search
    assert "e.key === '/'" in js
    assert "e.key === 'r'" in js
    # ignore shortcuts when typing in a form field (avoids stealing keys)
    assert "tag === 'input'" in js


# 8. Column visibility toggle
def test_trades_html_has_column_toggle_menu():
    html = _read(TRADES_HTML)
    assert 'id="btn-col-toggle"' in html
    assert 'id="col-toggle-menu"' in html
    # data-col attributes used to address columns
    for col in ("date", "asset", "age", "buyer", "seller", "qty", "price", "fee", "total"):
        assert f'data-col="{col}"' in html


def test_trades_js_column_toggle_persists_and_protects_required_cols():
    js = _read(TRADES_JS)
    assert "ALL_COLUMNS" in js
    assert "alwaysOn: true" in js
    assert "function buildColumnToggleMenu" in js
    assert "function applyColumnVisibility" in js
    # Trade ID and Status are mandatory and must not appear in the menu
    assert "key: 'id', label: 'Trade ID', alwaysOn: true" in js
    assert "key: 'status', label: 'Status', alwaysOn: true" in js
    # prefs round-trip via localStorage
    assert "COL_PREFS_KEY" in js
    assert "writeColumnPrefs" in js


def test_trades_css_has_col_hidden_helper():
    css = _read(ADMIN_MP_CSS)
    assert ".mp-col-hidden" in css
    assert "display: none" in css


# 9. Asset filter auto-hide
def test_trades_js_hides_asset_filter_when_only_one_traded_asset():
    js = _read(TRADES_JS)
    assert "function maybeHideAssetFilter" in js
    assert "assetCount <= 1" in js


# 10. Status SVG icons (a11y, not color-only)
def test_trades_js_status_cell_uses_svg_icon_per_status():
    js = _read(TRADES_JS)
    assert "function statusIconSvg" in js
    # status-specific shapes (check, clock, plane/arrow, X)
    assert "confirmed:" in js
    assert "pending:" in js
    assert "submitted:" in js
    assert "failed:" in js


def test_trades_css_has_summary_bulk_pagination_kbd_styles():
    css = _read(ADMIN_MP_CSS)
    assert ".mp-bulk-bar" in css
    assert ".mp-filter-summary" in css
    assert ".mp-pagination-size" in css
    assert ".mp-pagination-jump" in css
    assert ".mp-kbd-hint" in css
    assert ".mp-col-toggle-menu" in css
    assert ".mp-recon-banner" in css
    assert ".mp-status-icon" in css


# Colspan / table-shape consistency check
def test_trades_js_message_colspans_match_new_column_count():
    """Adding the checkbox column means full-row messages must span 12, not 11."""
    js = _read(TRADES_JS)
    assert "cell.colSpan = 12" in js
    assert "cell.colSpan = 11" not in js


# ── Batch 3 (next-10) ────────────────────────────────────────────────


# 1+2. Backend exposes order ids + tx hash + batch id
def test_admin_trade_struct_has_settlement_chain_fields():
    source = _read(ADMIN_MARKETPLACE)
    assert "pub buy_order_id: Option<Uuid>" in source
    assert "pub sell_order_id: Option<Uuid>" in source
    assert "pub on_chain_tx_hash: Option<String>" in source
    assert "pub on_chain_batch_id: Option<Uuid>" in source
    # SQL selects must include the new columns
    assert "t.buy_order_id" in source
    assert "t.sell_order_id" in source
    assert "t.on_chain_tx_hash" in source
    assert "t.on_chain_batch_id" in source


# 2. Drawer renders order/tx/batch fields
def test_trades_js_drawer_renders_order_tx_and_batch():
    js = _read(TRADES_JS)
    assert "function orderLink" in js
    assert "function txHashNode" in js
    assert "function batchLink" in js
    assert "/admin/marketplace/orders.html?order_id=" in js
    assert "/admin/marketplace/reconciliation.html?batch_id=" in js
    assert "Awaiting submission" in js  # placeholder for unsubmitted trades
    # all four fields appear in detail rows
    assert "detailRow('Buy order'" in js
    assert "detailRow('Sell order'" in js
    assert "detailRow('Onchain TX'" in js
    assert "detailRow('Settlement batch'" in js


# 3. Date/Time seconds + ISO tooltip
def test_trades_js_date_cell_shows_seconds_with_iso_tooltip():
    js = _read(TRADES_JS)
    # timeStyle: medium gives seconds; ISO tooltip on hover
    assert "timeStyle: 'medium'" in js
    assert "function formatIsoTooltip" in js
    assert "dateCell.title = formatIsoTooltip(trade.executed_at)" in js


# 4. Row kebab menu
def test_trades_js_row_kebab_menu_actions():
    js = _read(TRADES_JS)
    assert "function appendIdWithKebabCell" in js
    assert "function openKebabMenu" in js
    assert "function closeKebabMenu" in js
    assert "'View detail'" in js
    assert "'Copy trade ID'" in js
    assert "'Copy as JSON'" in js
    assert "'Open buyer'" in js
    assert "'Open seller'" in js
    assert "'Open buy order'" in js
    assert "'Open sell order'" in js
    # ARIA wiring
    assert "aria-haspopup" in js
    assert "role: 'menu'" in js or "'role', 'menu'" in js


# 5. Fee bps inline
def test_trades_js_fee_cell_shows_bps_subline():
    js = _read(TRADES_JS)
    assert "function feeBps" in js
    assert "function appendFeeCell" in js
    assert "mp-fee-bps" in js
    assert "bps" in js


# 6. Group-by-pair toggle + dim
def test_trades_html_has_group_by_pair_toggle():
    html = _read(TRADES_HTML)
    assert 'id="btn-group-by-pair"' in html
    assert 'aria-pressed="false"' in html


def test_trades_js_group_by_pair_marks_repeated_pairs():
    js = _read(TRADES_JS)
    assert "groupByPair" in js
    assert "GROUP_BY_PAIR_KEY" in js
    assert "mp-trade-row--repeat-pair" in js
    # only flagged when toggle is on AND consecutive rows share the buyer→seller pair
    assert "groupByPair && pairKey === prevPairKey" in js


def test_trades_css_dims_repeated_pair_rows():
    css = _read(ADMIN_MP_CSS)
    assert ".mp-trade-row--repeat-pair td:nth-child(6)" in css
    assert ".mp-trade-row--repeat-pair td:nth-child(7)" in css


# 7. Risk indicator on user chip
def test_trades_js_user_chip_has_risk_score_with_signals():
    js = _read(TRADES_JS)
    assert "function riskScore" in js
    assert "'plus-alias'" in js
    assert "'test-account'" in js
    assert "mp-risk-flag" in js
    # high vs low based on signal count (template literal builds the suffix)
    assert "'high' : 'low'" in js


def test_trades_css_has_risk_flag_severity_styles():
    css = _read(ADMIN_MP_CSS)
    assert ".mp-risk-flag--low" in css
    assert ".mp-risk-flag--high" in css


# 8. Apply-Filters dirty-state
def test_trades_js_marks_apply_button_dirty_on_filter_change():
    js = _read(TRADES_JS)
    assert "function markFiltersDirty" in js
    assert "function clearFiltersDirty" in js
    assert "mp-btn-dirty" in js
    # cleared automatically after a successful load
    assert "clearFiltersDirty()" in js


def test_trades_css_has_dirty_button_indicator():
    css = _read(ADMIN_MP_CSS)
    assert ".mp-btn-dirty" in css


# 9. Header capitalization consistency
def test_trades_html_uses_title_case_table_headers():
    """No more all-caps `BUYER ID` / `SELLER ID`. Title-case the visible labels."""
    html = _read(TRADES_HTML)
    # headers we care about
    for h in ('Trade ID', 'Date/Time', 'Asset', 'Age', 'Buyer', 'Seller', 'Qty', 'Price', 'Fee', 'Total', 'Status'):
        assert f">{h}</th>" in html or f">{h}<" in html


# 10. Sticky table head
def test_trades_css_has_sticky_thead():
    css = _read(ADMIN_MP_CSS)
    assert ".mp-trades-table thead th" in css
    # sticky positioning anchored to top of scroll container
    assert "position: sticky" in css


# Kebab CSS
def test_trades_css_has_kebab_menu_styles():
    css = _read(ADMIN_MP_CSS)
    assert ".mp-kebab-btn" in css
    assert ".mp-kebab-menu" in css
    assert ".mp-kebab-item" in css


# ── Critical #3a — Force-Settle / Cancel mutations ───────────────────


MIGRATION = REPO_ROOT / "database/111_trade_cancellation.sql"
ADMIN_MOD = REPO_ROOT / "backend/src/admin/mod.rs"


def test_migration_extends_status_check_with_cancelled():
    sql = _read(MIGRATION)
    assert "DROP CONSTRAINT IF EXISTS trade_history_on_chain_status_check" in sql
    assert "'pending', 'submitted', 'confirmed', 'failed', 'cancelled'" in sql
    assert "ADD COLUMN IF NOT EXISTS cancelled_at TIMESTAMPTZ" in sql
    assert "ADD COLUMN IF NOT EXISTS cancelled_by UUID REFERENCES users(id)" in sql
    assert "ADD COLUMN IF NOT EXISTS cancellation_reason TEXT" in sql


def test_backend_cancel_endpoint_guards_status_and_writes_audit():
    src = _read(ADMIN_MARKETPLACE)
    assert "pub async fn api_admin_marketplace_cancel_trade" in src
    # status guard prevents cancelling confirmed trades
    assert "matches!(prev_status.as_str(), \"pending\" | \"failed\")" in src
    # mandatory reason via shared helper
    assert "normalize_admin_cancel_reason(body.reason)" in src
    # audit entry
    assert "'marketplace.trade.cancelled'" in src


def test_backend_retry_endpoint_only_accepts_failed_status():
    src = _read(ADMIN_MARKETPLACE)
    assert "pub async fn api_admin_marketplace_retry_trade_settlement" in src
    assert "if prev_status != \"failed\"" in src
    assert "'marketplace.trade.retry_settlement'" in src


def test_backend_bulk_cancel_endpoint_caps_at_500_and_audits():
    src = _read(ADMIN_MARKETPLACE)
    assert "pub async fn api_admin_marketplace_trades_bulk_cancel" in src
    assert "trade_ids exceeds limit of 500" in src
    assert "'marketplace.trade.bulk_cancel'" in src


def test_backend_status_filter_now_accepts_cancelled():
    src = _read(ADMIN_MARKETPLACE)
    assert "\"pending\" | \"submitted\" | \"confirmed\" | \"failed\" | \"cancelled\"" in src


def test_routes_register_per_id_and_bulk_cancel_endpoints():
    src = _read(ADMIN_MOD)
    assert "/api/admin/marketplace/trades/:trade_id/cancel" in src
    assert "/api/admin/marketplace/trades/:trade_id/retry-settlement" in src
    assert "/api/admin/marketplace/trades/bulk-cancel" in src
    assert "api_admin_marketplace_cancel_trade" in src
    assert "api_admin_marketplace_retry_trade_settlement" in src
    assert "api_admin_marketplace_trades_bulk_cancel" in src


def test_drawer_renders_cancel_and_retry_buttons_for_eligible_status():
    js = _read(TRADES_JS)
    assert "function renderTradeMutationActions" in js
    # Cancel for pending or failed; Retry for failed only
    assert "canCancel = status === 'pending' || status === 'failed'" in js
    assert "canRetry = status === 'failed'" in js
    # confirmation prompts mention irreversibility / audit log
    assert "irreversible" in js
    assert "audit log" in js


def test_drawer_actions_post_to_per_id_endpoints():
    js = _read(TRADES_JS)
    assert "function onCancelTrade" in js
    assert "function onRetryTrade" in js
    assert "/cancel" in js
    assert "/retry-settlement" in js
    assert "method: 'POST'" in js


def test_bulk_bar_wires_cancel_and_retry_buttons():
    html = _read(TRADES_HTML)
    assert 'id="btn-bulk-retry"' in html
    assert 'id="btn-bulk-cancel"' in html
    js = _read(TRADES_JS)
    assert "function onBulkCancel" in js
    assert "function onBulkRetry" in js
    assert "/api/admin/marketplace/trades/bulk-cancel" in js
    assert "/api/admin/marketplace/trades/bulk-retry-onchain" in js


def test_drawer_action_styles_present():
    css = _read(ADMIN_MP_CSS)
    assert ".mp-trade-actions" in css
    assert ".mp-trade-actions-note" in css
    assert ".mp-trade-actions-row" in css
