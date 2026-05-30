/**
 * Marketplace Orderbook — admin view.
 *
 * Renders aggregated depth, market context (last/24h/spread), expandable level
 * drilldown with cancel, searchable asset combobox, [/] keyboard navigation,
 * live auto-refresh, tick-size aggregation, side/min-qty filter,
 * match-preview tooltips, and a CSV-snapshot exporter.
 */
(function () {
  "use strict";

  const API = "/api/admin/marketplace/orderbook";
  const CANCEL_API = "/api/admin/marketplace/orders";
  const AUDIT_API = "/api/admin/audit-logs";
  const REFRESH_MS = 5000;
  const PREFS_KEY = "poool.admin.orderbook.prefs";
  const AGE_AMBER_S = 3600;     // 1h
  const AGE_RED_S = 86400;      // 24h

  const RECENT_KEY = "poool.admin.orderbook.recent";
  const RECENT_MAX = 5;

  const persisted = loadPrefs();

  const state = {
    assets: [],
    selectedAssetId: "",
    loading: false,
    lastData: null,
    refreshTimer: null,
    refreshPaused: persisted.refreshPaused === true,
    expanded: new Set(),
    pickerOpen: false,
    pickerFilter: "",
    pickerCursor: -1,
    sideFilter: persisted.sideFilter || "both",
    minQty: persisted.minQty || 0,
    tickCents: persisted.tickCents || 1,
    tz: persisted.tz || "utc",
    rebuildHistoryOpen: false,
    rebuildHistory: null,
    settingsOpen: false,
    matchPreview: null, // { left, top, html }
    wsState: "closed",
    recentAssetIds: loadRecent(),
    bulkSelected: new Set(), // order_ids picked in drilldown
    fetchFailures: 0,
    pageSize: 50, // combobox page size
    pageMore: 0,  // extra pages clicked open
  };

  function loadRecent() {
    try {
      const arr = JSON.parse(localStorage.getItem(RECENT_KEY) || "[]");
      return Array.isArray(arr) ? arr.slice(0, RECENT_MAX) : [];
    } catch (_) {
      return [];
    }
  }

  function pushRecent(assetId) {
    if (!assetId) return;
    const next = [assetId, ...state.recentAssetIds.filter((id) => id !== assetId)].slice(
      0,
      RECENT_MAX,
    );
    state.recentAssetIds = next;
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(next));
    } catch (_) {
      /* ignore */
    }
  }

  function loadPrefs() {
    try {
      return JSON.parse(localStorage.getItem(PREFS_KEY) || "{}") || {};
    } catch (_) {
      return {};
    }
  }

  function savePrefs() {
    try {
      localStorage.setItem(
        PREFS_KEY,
        JSON.stringify({
          refreshPaused: state.refreshPaused,
          sideFilter: state.sideFilter,
          minQty: state.minQty,
          tickCents: state.tickCents,
          tz: state.tz,
        }),
      );
    } catch (_) {
      /* ignore */
    }
  }

  function reportError(scope, err) {
    const msg = err && err.message ? err.message : String(err);
    // Sentry breadcrumb if SDK present, else structured console.error.
    if (window.Sentry && typeof window.Sentry.captureException === "function") {
      try {
        window.Sentry.withScope((s) => {
          s.setTag("orderbook.scope", scope);
          s.setTag("orderbook.asset_id", state.selectedAssetId || "none");
          window.Sentry.captureException(err);
        });
      } catch (_) {
        /* ignore */
      }
    }
    console.error(`[orderbook:${scope}]`, { asset: state.selectedAssetId, error: msg });
  }

  // ─── helpers ─────────────────────────────────────────────────────

  function getCsrfToken() {
    if (typeof window.getCsrfToken === "function") return window.getCsrfToken();
    const value = `; ${document.cookie}`;
    const parts = value.split("; csrf_token=");
    return parts.length === 2 ? parts.pop().split(";").shift() : "";
  }

  function csrfHeaders(headers = {}) {
    const token = getCsrfToken();
    return token ? { ...headers, "X-CSRF-Token": token } : headers;
  }

  function formatUsd(cents) {
    if (cents == null) return "—";
    return `$${(Number(cents) / 100).toLocaleString(undefined, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })}`;
  }

  function formatQty(value) {
    return Number(value || 0).toLocaleString();
  }

  function formatPct(pct) {
    if (pct == null || !isFinite(pct)) return "—";
    const sign = pct > 0 ? "+" : "";
    return `${sign}${pct.toFixed(2)}%`;
  }

  function formatRelative(iso) {
    if (!iso) return "never";
    const diff = (Date.now() - new Date(iso).getTime()) / 1000;
    if (diff < 5) return "just now";
    if (diff < 60) return `${Math.floor(diff)}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  function formatTime(iso) {
    if (!iso) return "—";
    const d = new Date(iso);
    if (state.tz === "utc") {
      return d.toISOString().replace("T", " ").slice(0, 19) + " UTC";
    }
    return d.toLocaleString();
  }

  function clearNode(node) {
    if (!node) return;
    while (node.firstChild) node.removeChild(node.firstChild);
  }

  function el(tag, props = {}, ...children) {
    const node = document.createElement(tag);
    Object.entries(props).forEach(([k, v]) => {
      if (v == null) return;
      if (k === "class") node.className = v;
      else if (k === "dataset") Object.assign(node.dataset, v);
      else if (k.startsWith("on") && typeof v === "function")
        node.addEventListener(k.slice(2).toLowerCase(), v);
      else if (k in node) node[k] = v;
      else node.setAttribute(k, v);
    });
    children.flat().forEach((c) => {
      if (c == null || c === false) return;
      node.appendChild(typeof c === "string" ? document.createTextNode(c) : c);
    });
    return node;
  }

  function setStatus(message, type = "info") {
    const node = document.getElementById("orderbook-status");
    if (!node) return;
    if (!message) {
      node.hidden = true;
      node.textContent = "";
      node.className = "admin-alert";
      return;
    }
    node.hidden = false;
    node.className = `admin-alert admin-alert--${type}`;
    node.textContent = message;
  }

  function setBusy(isBusy) {
    state.loading = isBusy;
    const rebuildBtn = document.getElementById("btn-rebuild-orderbook");
    if (rebuildBtn) {
      rebuildBtn.disabled = isBusy;
      rebuildBtn.setAttribute("aria-busy", isBusy ? "true" : "false");
    }
  }

  async function fetchJSON(url, options = {}) {
    const response = await fetch(url, { credentials: "same-origin", ...options });
    if (!response.ok) {
      let message = `HTTP ${response.status}`;
      try {
        const body = await response.json();
        message = body.error || body.message || message;
      } catch (_) {
        /* keep fallback */
      }
      throw new Error(message);
    }
    return response.json();
  }

  // ─── tick-size aggregation + filters ─────────────────────────────

  function aggregateLevels(levels, side) {
    const tick = state.tickCents;
    if (!Array.isArray(levels)) return [];
    let items = levels.slice();

    if (state.minQty > 0) {
      items = items.filter((l) => Number(l.total_quantity || 0) >= state.minQty);
    }

    if (tick > 1) {
      const buckets = new Map();
      items.forEach((l) => {
        // Floor for bids (worse), ceil for asks (worse), so cumulative makes sense.
        const price = Number(l.price_cents);
        const bucket =
          side === "buy"
            ? Math.floor(price / tick) * tick
            : Math.ceil(price / tick) * tick;
        const cur = buckets.get(bucket) || {
          price_cents: bucket,
          total_quantity: 0,
          order_count: 0,
          // Aggregation across price levels can't dedupe users without per-row
          // user IDs (we only have aggregated counts). max() is the safest
          // upper bound — a user with orders at both $500 and $501 would be
          // counted once after grouping. Approximation only; admin drilldown
          // shows exact users.
          unique_users: 0,
        };
        cur.total_quantity += Number(l.total_quantity || 0);
        cur.order_count += Number(l.order_count || 0);
        cur.unique_users = Math.max(cur.unique_users, Number(l.unique_users || 0));
        buckets.set(bucket, cur);
      });
      items = Array.from(buckets.values());
      items.sort((a, b) =>
        side === "buy" ? b.price_cents - a.price_cents : a.price_cents - b.price_cents,
      );
    }
    return items;
  }

  // ─── header KPIs ─────────────────────────────────────────────────

  function renderHeaderKpis(data) {
    const stats = document.getElementById("ob-stats");
    if (!stats) return;
    clearNode(stats);

    const midLabel = data.mid_price_is_fallback ? "Mid (fallback)" : "Mid";
    const midTooltip = data.mid_price_is_fallback
      ? "One-sided book. Mid = best bid or best ask."
      : "(best bid + best ask) / 2";

    const changeColor =
      data.change_24h_pct == null
        ? ""
        : data.change_24h_pct >= 0
          ? "mp-ob-stat-value--up"
          : "mp-ob-stat-value--down";

    const items = [
      {
        label: "Last",
        value: formatUsd(data.last_trade_cents),
        sub: formatRelative(data.last_trade_at),
      },
      {
        label: "24h Δ",
        value: formatPct(data.change_24h_pct),
        valueClass: changeColor,
      },
      { label: "24h Vol", value: formatUsd(data.volume_24h_cents) },
      { label: "24h Trades", value: formatQty(data.trades_24h) },
      { label: midLabel, value: formatUsd(data.mid_price_cents), tooltip: midTooltip },
      {
        label: "Spread",
        value:
          data.spread_cents == null
            ? "—"
            : `${formatUsd(data.spread_cents)}${
                data.best_bid_cents
                  ? ` (${((data.spread_cents / data.best_bid_cents) * 100).toFixed(2)}%)`
                  : ""
              }`,
      },
    ];

    items.forEach((item) => {
      const wrapper = el(
        "div",
        { class: "mp-ob-stat", title: item.tooltip || "" },
        el("div", { class: "mp-ob-stat-label" }, item.label),
        el(
          "div",
          { class: `mp-ob-stat-value ${item.valueClass || ""}` },
          item.value,
        ),
        item.sub ? el("div", { class: "mp-ob-stat-sub" }, item.sub) : null,
      );
      stats.appendChild(wrapper);
    });
  }

  // ─── spread bar ──────────────────────────────────────────────────

  function renderSpread(data) {
    const bar = document.getElementById("spread-bar");
    if (!bar) return;
    clearNode(bar);

    const status = data.market_status;
    // Split "one_sided" into bids-only / asks-only at render so admins see
    // which side is missing liquidity. "crossed" = invariant violation
    // (matching engine missed a match) and renders as an error pill.
    let statusLabel;
    if (status === "live") {
      statusLabel = "Live two-sided";
    } else if (status === "crossed") {
      statusLabel = "⚠ Crossed book — investigate";
    } else if (status === "one_sided") {
      const bidsOnly = data.best_bid_cents != null && data.best_ask_cents == null;
      statusLabel = bidsOnly
        ? "One-sided · bids only (no sellers)"
        : "One-sided · asks only (no buyers)";
    } else if (status === "no_orders") {
      statusLabel = "No active orders";
    } else {
      statusLabel = status;
    }
    const statusClass =
      status === "live"
        ? "mp-ob-status-pill--ok"
        : status === "crossed"
          ? "mp-ob-status-pill--error"
          : status === "one_sided"
            ? "mp-ob-status-pill--warn"
            : "mp-ob-status-pill--muted";

    bar.appendChild(
      el(
        "div",
        { class: "mp-ob-spread-item" },
        el("span", { class: `mp-ob-status-pill ${statusClass}` }, statusLabel),
      ),
    );

    [
      [
        "Best Bid",
        formatUsd(data.best_bid_cents),
        "mp-ob-spread-value mp-ob-spread-value--bid",
        "▲",
      ],
      ["Bid Vol", formatQty(data.bid_volume), "mp-ob-spread-value mp-ob-spread-value--bid", "▲"],
      [
        "Best Ask",
        formatUsd(data.best_ask_cents),
        "mp-ob-spread-value mp-ob-spread-value--ask",
        "▼",
      ],
      ["Ask Vol", formatQty(data.ask_volume), "mp-ob-spread-value mp-ob-spread-value--ask", "▼"],
    ].forEach(([label, value, valueClass, glyph]) => {
      bar.appendChild(
        el(
          "div",
          { class: "mp-ob-spread-item" },
          el("span", { class: "mp-ob-spread-label" }, label),
          el(
            "span",
            { class: valueClass },
            el("span", { class: "mp-ob-glyph", "aria-hidden": "true" }, glyph),
            value,
          ),
        ),
      );
    });
  }

  // ─── depth rows ──────────────────────────────────────────────────

  function levelKey(side, priceCents) {
    return `${side}:${priceCents}`;
  }

  function renderEmptyRow(tbody, message, colspan = 5, withCta = false) {
    clearNode(tbody);
    const cell = el(
      "td",
      { colSpan: colspan, style: "text-align:center;padding:24px" },
      el("div", { class: "mp-ob-empty-msg" }, message),
    );
    if (withCta) {
      cell.appendChild(
        el(
          "div",
          { class: "mp-ob-empty-cta" },
          "No liquidity on this side. ",
          el(
            "a",
            {
              href: state.selectedAssetId
                ? `/admin/marketplace/p2p.html?asset_id=${state.selectedAssetId}`
                : "/admin/marketplace/p2p.html",
            },
            "Open P2P",
          ),
          " · ",
          el(
            "a",
            {
              href: state.selectedAssetId
                ? `/admin/marketplace/orders.html?asset_id=${state.selectedAssetId}`
                : "/admin/marketplace/orders.html",
            },
            "View orders",
          ),
        ),
      );
    }
    tbody.appendChild(el("tr", { class: "mp-ob-empty-row" }, cell));
  }

  function renderDepthCell(side, widthPct, value, glyph) {
    return el(
      "td",
      { style: "position:relative; text-align: " + (side === "bid" ? "right" : "left") },
      el("div", {
        class: `mp-ob-depth mp-ob-depth--${side}`,
        style: `width:${Math.max(0, Math.min(100, widthPct))}%`,
      }),
      el(
        "span",
        { style: "position:relative; display:inline-flex; gap:4px; align-items:center;" },
        glyph
          ? el("span", { class: "mp-ob-glyph", "aria-hidden": "true" }, glyph)
          : null,
        value,
      ),
    );
  }

  async function loadLevelOrders(side, priceCents, container) {
    container.textContent = "Loading…";
    try {
      const orders = await fetchJSON(
        `${API}/${state.selectedAssetId}/level?side=${side}&price_cents=${priceCents}`,
      );
      clearNode(container);
      if (!orders.length) {
        container.appendChild(el("div", {}, "No orders at this level."));
        return;
      }
      let bulkBtn;
      const selectedIds = () =>
        orders.filter((o) => state.bulkSelected.has(o.id)).map((o) => o.id);
      const updateBulkBtn = () => {
        if (!bulkBtn) return;
        const hasSelection = selectedIds().length > 0;
        bulkBtn.disabled = !hasSelection;
        bulkBtn.setAttribute("aria-disabled", hasSelection ? "false" : "true");
      };
      const headerCheckbox = el("input", {
        type: "checkbox",
        "aria-label": "Select all orders at level",
        onClick: (ev) => {
          const checked = ev.target.checked;
          orders.forEach((o) =>
            checked ? state.bulkSelected.add(o.id) : state.bulkSelected.delete(o.id),
          );
          container.querySelectorAll('input[type="checkbox"][data-oid]').forEach((cb) => {
            cb.checked = checked;
          });
          updateBulkBtn();
          renderBulkBar();
        },
      });
      bulkBtn = el(
        "button",
        {
          class: "admin-btn admin-btn--danger admin-btn--sm",
          type: "button",
          disabled: true,
          "aria-disabled": "true",
          onClick: (ev) => {
            ev.stopPropagation();
            const ids = selectedIds();
            if (!ids.length) return;
            bulkCancel(ids);
          },
        },
        "Cancel selected",
      );
      const toolbar = el(
        "div",
        { class: "mp-ob-level-toolbar" },
        headerCheckbox,
        el("span", {}, `${orders.length} orders at this level`),
        bulkBtn,
      );
      container.appendChild(toolbar);
      const table = el(
        "table",
        { class: "admin-table mp-ob-level-table" },
        el(
          "thead",
          {},
          el(
            "tr",
            {},
            el("th", { style: "width:24px" }, ""),
            el("th", {}, "User"),
            el("th", { style: "text-align:right" }, "Qty"),
            el("th", { style: "text-align:right" }, "Filled"),
            el("th", { style: "text-align:right" }, "Created"),
            el("th", { style: "text-align:right" }, "Action"),
          ),
        ),
      );
      const tbody = el("tbody", {});
      orders.forEach((o) => {
        const remaining = o.quantity - o.quantity_filled;
        const ageSec = (Date.now() - new Date(o.created_at).getTime()) / 1000;
        const ageClass =
          ageSec >= AGE_RED_S
            ? "mp-ob-age--red"
            : ageSec >= AGE_AMBER_S
              ? "mp-ob-age--amber"
              : "";
        tbody.appendChild(
          el(
            "tr",
            { class: ageClass },
            el(
              "td",
              {},
              el("input", {
                type: "checkbox",
                dataset: { oid: o.id },
                "aria-label": `Select order ${o.id.slice(0, 8)}`,
                checked: state.bulkSelected.has(o.id),
                onClick: (ev) => {
                  ev.stopPropagation();
                  if (ev.target.checked) state.bulkSelected.add(o.id);
                  else state.bulkSelected.delete(o.id);
                  updateBulkBtn();
                  renderBulkBar();
                },
              }),
            ),
            el(
              "td",
              {},
              el("span", { class: "mp-ob-uid" }, o.user_email || o.user_id.slice(0, 8)),
            ),
            el("td", { style: "text-align:right" }, formatQty(remaining)),
            el(
              "td",
              { style: "text-align:right" },
              `${o.quantity_filled}/${o.quantity}`,
            ),
            el(
              "td",
              { style: "text-align:right", title: formatTime(o.created_at) },
              formatRelative(o.created_at),
            ),
            el(
              "td",
              { style: "text-align:right" },
              el(
                "button",
                {
                  class: "admin-btn admin-btn--danger admin-btn--sm",
                  type: "button",
                  onClick: (ev) => {
                    ev.stopPropagation();
                    cancelOrder(o.id);
                  },
                },
                "Cancel",
              ),
            ),
          ),
        );
      });
      table.appendChild(tbody);
      container.appendChild(table);
      updateBulkBtn();
    } catch (err) {
      clearNode(container);
      container.appendChild(
        el("div", { class: "admin-alert admin-alert--error" }, `Error: ${err.message}`),
      );
    }
  }

  async function cancelOrder(orderId) {
    const reason = await reasonPrompt("Reason for cancelling this order?");
    if (!reason || !reason.trim()) return;
    pushReason(reason.trim());
    try {
      await fetchJSON(`${CANCEL_API}/${orderId}`, {
        method: "DELETE",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({ reason: reason.trim() }),
      });
      if (typeof window.mpToast === "function") window.mpToast("Order cancelled", "success");
      await loadOrderbook({ silent: true });
      state.expanded.forEach((key) => {
        const [side, price] = key.split(":");
        const row = document.querySelector(
          `tr[data-level-key="${key}"] + tr .mp-ob-level-detail`,
        );
        if (row) loadLevelOrders(side, Number(price), row);
      });
    } catch (err) {
      reportError("cancelOrder", err);
      if (typeof window.mpToast === "function") window.mpToast(err.message, "error");
    }
  }

  const REASON_KEY = "poool.admin.orderbook.reasons";

  function loadReasons() {
    try {
      const arr = JSON.parse(localStorage.getItem(REASON_KEY) || "[]");
      return Array.isArray(arr) ? arr.slice(0, 20) : [];
    } catch (_) {
      return [];
    }
  }

  function pushReason(reason) {
    const list = loadReasons().filter((r) => r !== reason);
    list.unshift(reason);
    try {
      localStorage.setItem(REASON_KEY, JSON.stringify(list.slice(0, 20)));
    } catch (_) {
      /* ignore */
    }
    syncReasonDatalist();
  }

  function syncReasonDatalist() {
    const dl = document.getElementById("mp-ob-reason-list");
    if (!dl) return;
    clearNode(dl);
    loadReasons().forEach((r) => dl.appendChild(el("option", { value: r })));
  }

  function closeReasonPrompt() {
    const overlay = document.getElementById("mp-ob-reason-overlay");
    if (!overlay) return;
    overlay.hidden = true;
    overlay.setAttribute("aria-hidden", "true");
  }

  /**
   * Custom prompt that wires a <datalist> autocomplete from past reasons.
   * Falls back to window.prompt() if our overlay can't render.
   */
  function reasonPrompt(message, defaultValue = "") {
    return new Promise((resolve) => {
      const overlay = document.getElementById("mp-ob-reason-overlay");
      if (!overlay) {
        resolve(window.prompt(message, defaultValue));
        return;
      }
      const titleEl = overlay.querySelector(".mp-ob-reason-title");
      const input = overlay.querySelector("#mp-ob-reason-input");
      const ok = overlay.querySelector("#mp-ob-reason-ok");
      const cancel = overlay.querySelector("#mp-ob-reason-cancel");
      if (titleEl) titleEl.textContent = message;
      input.value = defaultValue;
      syncReasonDatalist();
      overlay.hidden = false;
      overlay.setAttribute("aria-hidden", "false");
      requestAnimationFrame(() => input.focus());

      const cleanup = (val) => {
        overlay.hidden = true;
        overlay.setAttribute("aria-hidden", "true");
        ok.removeEventListener("click", onOk);
        cancel.removeEventListener("click", onCancel);
        input.removeEventListener("keydown", onKey);
        overlay.removeEventListener("click", onScrim);
        document.removeEventListener("keydown", onDocKey);
        resolve(val);
      };
      const onOk = () => cleanup(input.value);
      const onCancel = () => cleanup(null);
      const onKey = (e) => {
        if (e.key === "Enter") onOk();
        else if (e.key === "Escape") onCancel();
      };
      ok.addEventListener("click", onOk);
      cancel.addEventListener("click", onCancel);
      input.addEventListener("keydown", onKey);
      const onScrim = (ev) => {
        if (ev.target === overlay) onCancel();
      };
      overlay.addEventListener("click", onScrim);
      const onDocKey = (ev) => {
        if (ev.key === "Escape") onCancel();
      };
      document.addEventListener("keydown", onDocKey);
    });
  }

  async function bulkCancel(orderIds) {
    if (!orderIds.length) return;
    const reason = await reasonPrompt(
      `Reason for cancelling ${orderIds.length} order(s)?`,
    );
    if (!reason || !reason.trim()) return;
    const trimmed = reason.trim();
    pushReason(trimmed);
    const results = await Promise.allSettled(
      orderIds.map((id) =>
        fetchJSON(`${CANCEL_API}/${id}`, {
          method: "DELETE",
          headers: csrfHeaders({ "Content-Type": "application/json" }),
          body: JSON.stringify({ reason: trimmed }),
        }),
      ),
    );
    const ok = results.filter((r) => r.status === "fulfilled").length;
    const fail = results.length - ok;
    if (typeof window.mpToast === "function") {
      window.mpToast(
        `${ok} cancelled${fail ? `, ${fail} failed` : ""}`,
        fail ? "warning" : "success",
      );
    }
    state.bulkSelected.clear();
    renderBulkBar();
    await loadOrderbook({ silent: true });
    state.expanded.forEach((key) => {
      const [side, price] = key.split(":");
      const detail = document.querySelector(
        `tr[data-level-key="${key}"] + tr .mp-ob-level-detail`,
      );
      if (detail) loadLevelOrders(side, Number(price), detail);
    });
  }

  async function bulkCancelSide(apiSide) {
    if (!state.lastData) return;
    const levels = apiSide === "buy" ? state.lastData.bids : state.lastData.asks;
    if (!levels || !levels.length) return;
    if (
      !window.confirm(
        `Fetch and cancel ALL ${apiSide === "buy" ? "bids" : "asks"} on the book? ` +
          `This will hit the level endpoint for each price level (${levels.length}).`,
      )
    )
      return;
    const reason = await reasonPrompt(
      `Reason for cancelling all ${apiSide === "buy" ? "bids" : "asks"}?`,
    );
    if (!reason || !reason.trim()) return;
    pushReason(reason.trim());
    const allIds = [];
    for (const lvl of levels) {
      try {
        const orders = await fetchJSON(
          `${API}/${state.selectedAssetId}/level?side=${apiSide}&price_cents=${lvl.price_cents}`,
        );
        orders.forEach((o) => allIds.push(o.id));
      } catch (err) {
        reportError("bulkCancelSide.fetchLevel", err);
      }
    }
    if (!allIds.length) return;
    state.bulkSelected = new Set(allIds);
    renderBulkBar();
    await bulkCancel(allIds);
  }

  function renderBulkBar() {
    const bar = document.getElementById("mp-ob-bulk-bar");
    if (!bar) return;
    const count = state.bulkSelected.size;
    clearNode(bar);
    bar.hidden = false;
    bar.appendChild(
      el("span", { class: "mp-ob-bulk-side" }, `Bulk:`),
    );
    bar.appendChild(
      el(
        "button",
        {
          class: "admin-btn admin-btn--ghost admin-btn--sm",
          type: "button",
          onClick: () => bulkCancelSide("buy"),
        },
        "Select all bids",
      ),
    );
    bar.appendChild(
      el(
        "button",
        {
          class: "admin-btn admin-btn--ghost admin-btn--sm",
          type: "button",
          onClick: () => bulkCancelSide("sell"),
        },
        "Select all asks",
      ),
    );
    if (count) {
      bar.appendChild(el("span", { class: "mp-ob-bulk-count" }, `${count} selected`));
      bar.appendChild(
        el(
          "button",
          {
            class: "admin-btn admin-btn--danger admin-btn--sm",
            type: "button",
            onClick: () => bulkCancel([...state.bulkSelected]),
          },
          "Cancel selected",
        ),
      );
      bar.appendChild(
        el(
          "button",
          {
            class: "admin-btn admin-btn--ghost admin-btn--sm",
            type: "button",
            onClick: () => {
              state.bulkSelected.clear();
              document.querySelectorAll('input[type="checkbox"][data-oid]').forEach((cb) => {
                cb.checked = false;
              });
              renderBulkBar();
            },
          },
          "Clear",
        ),
      );
    }
  }

  // ─── match preview popover ───────────────────────────────────────

  function describeMatch(level, cumQty, cumUsd, side) {
    if (side === "buy") {
      return `Sell ${formatQty(cumQty)} units at ≥ ${formatUsd(level.price_cents)} → ${formatUsd(
        cumUsd,
      )} gross.`;
    }
    return `Buy ${formatQty(cumQty)} units at ≤ ${formatUsd(level.price_cents)} → ${formatUsd(
      cumUsd,
    )} gross.`;
  }

  async function _trySimulateMatch(level, cumQty, apiSide) {
    // Optional: backend may expose POST /api/admin/marketplace/match-preview.
    // Render a richer popover when it does; otherwise the cumulative
    // estimate is good enough.
    if (!state.selectedAssetId) return null;
    try {
      return await fetchJSON("/api/admin/marketplace/match-preview", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({
          asset_id: state.selectedAssetId,
          side: apiSide === "buy" ? "sell" : "buy",
          quantity: cumQty,
          limit_price_cents: level.price_cents,
        }),
      });
    } catch (_) {
      return null;
    }
  }

  let _matchPopToken = 0;

  function _placePopover(pop, target) {
    pop.hidden = false;
    const rect = target.getBoundingClientRect();
    const popRect = pop.getBoundingClientRect();
    const margin = 8;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    let left = rect.left + 12;
    let top = rect.bottom + 6;
    if (left + popRect.width + margin > vw) left = vw - popRect.width - margin;
    if (left < margin) left = margin;
    if (top + popRect.height + margin > vh) top = rect.top - popRect.height - 6;
    if (top < margin) top = margin;
    pop.style.left = `${left + window.scrollX}px`;
    pop.style.top = `${top + window.scrollY}px`;
  }

  function showMatchPopover(target, level, cumQty, cumUsd, apiSide) {
    const pop = document.getElementById("mp-ob-popover");
    if (!pop) return;
    const token = ++_matchPopToken;
    const avgPrice = cumQty > 0 ? cumUsd / cumQty : 0;
    const slipBps =
      state.lastData && state.lastData.mid_price_cents
        ? Math.round(
            (Math.abs(level.price_cents - state.lastData.mid_price_cents) /
              state.lastData.mid_price_cents) *
              10000,
          )
        : null;
    clearNode(pop);
    pop.appendChild(
      el(
        "div",
        { class: "mp-ob-pop-title" },
        apiSide === "buy" ? "Match SELL into bids" : "Match BUY into asks",
      ),
    );
    pop.appendChild(
      el(
        "div",
        {},
        cumQty > 0
          ? describeMatch(level, cumQty, cumUsd, apiSide)
          : "No fillable depth at this level after current filters.",
      ),
    );
    pop.appendChild(
      el(
        "div",
        { class: "mp-ob-pop-row" },
        el("span", {}, "Worst price:"),
        el("strong", {}, formatUsd(level.price_cents)),
      ),
    );
    pop.appendChild(
      el(
        "div",
        { class: "mp-ob-pop-row" },
        el("span", {}, "Avg fill (cumulative):"),
        el("strong", {}, formatUsd(avgPrice)),
      ),
    );
    if (slipBps != null) {
      pop.appendChild(
        el(
          "div",
          { class: "mp-ob-pop-row" },
          el("span", {}, "Distance from mid:"),
          el("strong", {}, `${slipBps} bps`),
        ),
      );
    }
    _placePopover(pop, target);
    // Augment with server-side simulation if available — only patch DOM if
    // this is still the most recent hover (prevents flicker race).
    _trySimulateMatch(level, cumQty, apiSide).then((sim) => {
      if (token !== _matchPopToken || !sim || pop.hidden) return;
      const block = el(
        "div",
        { class: "mp-ob-pop-sim" },
        el("div", { class: "mp-ob-pop-sim-title" }, "Server simulation"),
        el(
          "div",
          { class: "mp-ob-pop-row" },
          el("span", {}, "Filled:"),
          el(
            "strong",
            {},
            `${formatQty(sim.filled_qty)}${sim.partial ? " (partial)" : ""}`,
          ),
        ),
        sim.avg_price_cents != null
          ? el(
              "div",
              { class: "mp-ob-pop-row" },
              el("span", {}, "Avg price:"),
              el("strong", {}, formatUsd(sim.avg_price_cents)),
            )
          : null,
        el(
          "div",
          { class: "mp-ob-pop-row" },
          el("span", {}, "Total cost:"),
          el("strong", {}, formatUsd(sim.total_cost_cents)),
        ),
      );
      pop.appendChild(block);
      _placePopover(pop, target);
    });
  }

  function hideMatchPopover() {
    const pop = document.getElementById("mp-ob-popover");
    if (pop) pop.hidden = true;
  }

  // ─── side tables ─────────────────────────────────────────────────

  function renderSideTable(tbody, levels, side) {
    if (!tbody) return;
    if (!levels.length) {
      renderEmptyRow(tbody, side === "bid" ? "No bids" : "No asks", 5, true);
      return;
    }
    clearNode(tbody);

    const maxQty = Math.max(...levels.map((l) => Number(l.total_quantity || 0)), 1);
    let cumQty = 0;
    let cumUsd = 0;
    const apiSide = side === "bid" ? "buy" : "sell";

    levels.forEach((level) => {
      const qty = Number(level.total_quantity || 0);
      cumQty += qty;
      cumUsd += qty * Number(level.price_cents || 0);

      const key = levelKey(apiSide, level.price_cents);
      const isExpanded = state.expanded.has(key);
      const widthPct = (qty / maxQty) * 100;
      const matchPreview = describeMatch(level, cumQty, cumUsd, apiSide);

      const row = el("tr", {
        class: `mp-ob-level-row ${isExpanded ? "is-expanded" : ""}`,
        dataset: { levelKey: key },
        title: matchPreview,
        onClick: () => toggleLevel(apiSide, level.price_cents),
        onMouseenter: (ev) =>
          showMatchPopover(ev.currentTarget, level, cumQty, cumUsd, apiSide),
        onMouseleave: hideMatchPopover,
        tabIndex: 0,
        onFocus: (ev) =>
          showMatchPopover(ev.currentTarget, level, cumQty, cumUsd, apiSide),
        onBlur: hideMatchPopover,
        onKeydown: (e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            toggleLevel(apiSide, level.price_cents);
          }
        },
      });

      // "5 orders / 3 traders" — orders is COUNT(*), unique_users is COUNT(DISTINCT
      // user_id). When unique_users < order_count it means at least one user has
      // multiple orders at this level (legal — common with iceberg/laddering).
      const ordersTxt = `${level.order_count} order${Number(level.order_count) === 1 ? "" : "s"}`;
      const usersN = Number(level.unique_users || 0);
      const usersTxt = usersN > 0 && usersN < Number(level.order_count)
        ? ` · ${usersN} trader${usersN === 1 ? "" : "s"}`
        : "";
      const cellText = ordersTxt + usersTxt;

      if (side === "bid") {
        row.appendChild(el("td", {}, cellText));
        row.appendChild(el("td", { style: "text-align:right" }, formatQty(qty)));
        row.appendChild(el("td", { style: "text-align:right" }, formatQty(cumQty)));
        row.appendChild(el("td", { style: "text-align:right" }, formatUsd(cumUsd)));
        row.appendChild(renderDepthCell("bid", widthPct, formatUsd(level.price_cents), "▲"));
      } else {
        row.appendChild(renderDepthCell("ask", widthPct, formatUsd(level.price_cents), "▼"));
        row.appendChild(el("td", {}, formatUsd(cumUsd)));
        row.appendChild(el("td", {}, formatQty(cumQty)));
        row.appendChild(el("td", {}, formatQty(qty)));
        row.appendChild(el("td", { style: "text-align:right" }, cellText));
      }

      tbody.appendChild(row);

      if (isExpanded) {
        const detailRow = el(
          "tr",
          { class: "mp-ob-detail-row" },
          el(
            "td",
            { colSpan: 5 },
            el("div", { class: "mp-ob-level-detail" }, "Loading…"),
          ),
        );
        tbody.appendChild(detailRow);
        const container = detailRow.querySelector(".mp-ob-level-detail");
        loadLevelOrders(apiSide, level.price_cents, container);
      }
    });
  }

  function toggleLevel(side, priceCents) {
    const key = levelKey(side, priceCents);
    if (state.expanded.has(key)) state.expanded.delete(key);
    else state.expanded.add(key);
    if (state.lastData) renderOrderbook(state.lastData, { keepStatus: true });
  }

  // ─── live indicator ──────────────────────────────────────────────

  function renderLive(data) {
    const live = document.getElementById("ob-live-indicator");
    if (!live) return;
    clearNode(live);
    const ageMs = data.generated_at ? Date.now() - new Date(data.generated_at).getTime() : 0;
    const stale = ageMs > 30000;
    live.appendChild(
      el("span", {
        class: `mp-ob-live-dot ${stale ? "mp-ob-live-dot--stale" : ""}`,
        title: stale ? "No update in >30s" : "Live data",
      }),
    );
    live.appendChild(
      el(
        "span",
        { title: formatTime(data.generated_at) },
        `Live · updated ${formatRelative(data.generated_at)}`,
      ),
    );
    if (data.last_rebuild_at) {
      live.appendChild(
        el(
          "span",
          { class: "mp-ob-live-sep", title: formatTime(data.last_rebuild_at) },
          ` · last rebuild ${formatRelative(data.last_rebuild_at)}`,
        ),
      );
    }
    const wsLabel = {
      open: "WS connected",
      connecting: "WS connecting…",
      reconnecting: "WS reconnecting…",
      closed: "WS offline",
    }[state.wsState] || "";
    if (wsLabel) {
      const wsClass =
        state.wsState === "open"
          ? "mp-ob-ws-pill--ok"
          : state.wsState === "closed"
            ? "mp-ob-ws-pill--err"
            : "mp-ob-ws-pill--warn";
      const pill = el(
        "button",
        {
          type: "button",
          class: `mp-ob-ws-pill ${wsClass}`,
          title: "Click to force reconnect",
          onClick: reconnectWebSocket,
        },
        wsLabel,
      );
      live.appendChild(pill);
    }
  }

  function tickRelativeTimes() {
    if (state.lastData) renderLive(state.lastData);
  }

  // ─── render orchestration ────────────────────────────────────────

  function renderOrderbook(data, opts = {}) {
    state.lastData = data;
    const nameEl = document.getElementById("selected-asset-name");
    if (nameEl) nameEl.textContent = data.asset_title || `Asset ${data.asset_id}`;

    // Sync the asset-selector "(N active)" badge with the live orderbook so
    // the count never lags. Derives total open orders from the same response
    // we just rendered. Avoids extra fetch + eliminates the 7-vs-10 mismatch.
    const totalActive =
      (data.bids || []).reduce((s, l) => s + Number(l.order_count || 0), 0) +
      (data.asks || []).reduce((s, l) => s + Number(l.order_count || 0), 0);
    const assetEntry = state.assets.find((a) => a.id === data.asset_id);
    if (assetEntry && assetEntry.active_orders !== totalActive) {
      assetEntry.active_orders = totalActive;
      // Re-render the combobox label only if the picker isn't open (avoids
      // closing it mid-search). The list re-renders next time it opens.
      if (!state.pickerOpen) renderCombobox();
    }

    renderHeaderKpis(data);
    renderSpread(data);

    const bidsBody = document.getElementById("bids-body");
    const asksBody = document.getElementById("asks-body");

    const bidLevels =
      state.sideFilter === "sell" ? [] : aggregateLevels(data.bids || [], "buy");
    const askLevels =
      state.sideFilter === "buy" ? [] : aggregateLevels(data.asks || [], "sell");

    renderSideTable(bidsBody, bidLevels, "bid");
    renderSideTable(asksBody, askLevels, "ask");
    renderDepthChart(bidLevels, askLevels);
    renderLive(data);

    const tzNote = document.getElementById("mp-ob-tz-note");
    if (tzNote) {
      tzNote.textContent =
        state.tz === "utc" ? "Times shown in UTC" : "Times shown in local timezone";
    }

    if (!opts.keepStatus) setStatus("");
  }

  function clearOrderbook(message) {
    const stats = document.getElementById("ob-stats");
    const spreadBar = document.getElementById("spread-bar");
    clearNode(stats);
    clearNode(spreadBar);
    if (spreadBar) spreadBar.appendChild(el("div", { class: "mp-ob-spread-item" }, message));
    renderEmptyRow(document.getElementById("bids-body"), "No bids");
    renderEmptyRow(document.getElementById("asks-body"), "No asks");
  }

  // ─── combobox ────────────────────────────────────────────────────

  function renderCombobox() {
    const wrapper = document.getElementById("asset-combobox");
    if (!wrapper) return;
    clearNode(wrapper);

    const current = state.assets.find((a) => a.id === state.selectedAssetId);
    const button = el(
      "button",
      {
        type: "button",
        class: "mp-ob-combo-btn",
        "aria-haspopup": "listbox",
        "aria-expanded": state.pickerOpen ? "true" : "false",
        onClick: () => {
          state.pickerOpen = !state.pickerOpen;
          renderCombobox();
          if (state.pickerOpen) {
            const input = document.getElementById("mp-ob-combo-search");
            if (input) input.focus();
          }
        },
      },
      el(
        "span",
        { class: "mp-ob-combo-current" },
        current ? current.title : "No tradable assets",
      ),
      current
        ? el(
            "span",
            {
              class: "mp-ob-combo-meta",
              title: "Number of currently open or partially-filled orders for this asset",
            },
            `${current.active_orders} open`,
          )
        : null,
      el("span", { class: "mp-ob-combo-caret", "aria-hidden": "true" }, "▾"),
    );
    wrapper.appendChild(button);

    if (!state.pickerOpen) return;

    const panel = el(
      "div",
      { class: "mp-ob-combo-panel", role: "listbox" },
      el("input", {
        id: "mp-ob-combo-search",
        type: "search",
        class: "mp-ob-combo-search",
        placeholder: "Search assets… (press / anywhere)",
        value: state.pickerFilter,
        onInput: (e) => {
          state.pickerFilter = e.target.value;
          state.pageMore = 0;
          const list = document.getElementById("mp-ob-combo-list");
          if (list) renderComboList(list, e.target.value);
        },
        onKeydown: (e) => {
          if (e.key === "Escape") {
            state.pickerOpen = false;
            state.pickerCursor = -1;
            renderCombobox();
          } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
            e.preventDefault();
            const filtered = filteredAssets();
            if (!filtered.length) return;
            state.pickerCursor =
              (state.pickerCursor + (e.key === "ArrowDown" ? 1 : -1) + filtered.length) %
              filtered.length;
            highlightCursor();
          } else if (e.key === "Enter") {
            const filtered = filteredAssets();
            const target =
              state.pickerCursor >= 0 ? filtered[state.pickerCursor] : filtered[0];
            if (target) selectAsset(target.id);
          }
        },
      }),
    );

    const list = el("ul", { id: "mp-ob-combo-list", class: "mp-ob-combo-list" });
    panel.appendChild(list);
    renderComboList(list, state.pickerFilter);
    wrapper.appendChild(panel);
  }

  function filteredAssets() {
    const filter = (state.pickerFilter || "").toLowerCase();
    return state.assets
      .filter(
        (a) =>
          !filter ||
          a.title.toLowerCase().includes(filter) ||
          a.slug.toLowerCase().includes(filter),
      )
      .slice(0, 50);
  }

  function highlightCursor() {
    const items = document.querySelectorAll("#mp-ob-combo-list .mp-ob-combo-item");
    items.forEach((node, i) =>
      node.classList.toggle("is-cursor", i === state.pickerCursor),
    );
    const target = items[state.pickerCursor];
    if (target && typeof target.scrollIntoView === "function") {
      target.scrollIntoView({ block: "nearest" });
    }
  }

  function renderComboList(list, filterText) {
    clearNode(list);
    const filter = (filterText || "").toLowerCase();
    const showRecent = !filter && state.recentAssetIds.length > 0;

    const matches = state.assets.filter(
      (a) =>
        !filter ||
        a.title.toLowerCase().includes(filter) ||
        a.slug.toLowerCase().includes(filter),
    );
    if (!matches.length) {
      list.appendChild(el("li", { class: "mp-ob-combo-empty" }, "No matches"));
      return;
    }

    const renderItem = (asset, isPinned) => {
      list.appendChild(
        el(
          "li",
          {
            class: `mp-ob-combo-item ${
              asset.id === state.selectedAssetId ? "is-selected" : ""
            }`,
            role: "option",
            tabIndex: 0,
            "aria-selected": asset.id === state.selectedAssetId ? "true" : "false",
            onClick: () => selectAsset(asset.id),
            onKeydown: (e) => {
              if (e.key === "Enter") selectAsset(asset.id);
            },
          },
          el(
            "span",
            { class: "mp-ob-combo-item-title" },
            isPinned
              ? el("span", { class: "mp-ob-combo-pin", "aria-hidden": "true" }, "📌 ")
              : null,
            asset.title,
          ),
          el(
            "span",
            { class: "mp-ob-combo-item-meta" },
            `${asset.active_orders} open`,
          ),
        ),
      );
    };

    const window = state.pageSize + state.pageMore * state.pageSize;

    if (showRecent) {
      const recentObjs = state.recentAssetIds
        .map((id) => matches.find((a) => a.id === id))
        .filter(Boolean);
      if (recentObjs.length) {
        list.appendChild(el("li", { class: "mp-ob-combo-section" }, "Recent"));
        recentObjs.forEach((a) => renderItem(a, true));
        list.appendChild(el("li", { class: "mp-ob-combo-section" }, "All assets"));
      }
      const rest = matches.filter((a) => !state.recentAssetIds.includes(a.id));
      rest.slice(0, window).forEach((a) => renderItem(a, false));
      if (rest.length > window) {
        list.appendChild(_renderShowMore(rest.length - window));
      }
    } else {
      matches.slice(0, window).forEach((a) => renderItem(a, false));
      if (matches.length > window) {
        list.appendChild(_renderShowMore(matches.length - window));
      }
    }
  }

  function _ensureComboObserver() {
    if (state._comboObserver) return state._comboObserver;
    state._comboObserver = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (e.isIntersecting) {
            state.pageMore += 1;
            const list = document.getElementById("mp-ob-combo-list");
            if (list) renderComboList(list, state.pickerFilter);
          }
        }
      },
      { root: document.getElementById("mp-ob-combo-list"), rootMargin: "60px" },
    );
    return state._comboObserver;
  }

  function _renderShowMore(remaining) {
    const node = el(
      "li",
      {
        class: "mp-ob-combo-more",
        role: "option",
        tabIndex: 0,
        onClick: (ev) => {
          ev.stopPropagation();
          state.pageMore += 1;
          const list = document.getElementById("mp-ob-combo-list");
          if (list) renderComboList(list, state.pickerFilter);
        },
        onKeydown: (e) => {
          if (e.key === "Enter") {
            state.pageMore += 1;
            const list = document.getElementById("mp-ob-combo-list");
            if (list) renderComboList(list, state.pickerFilter);
          }
        },
      },
      `Show ${Math.min(remaining, state.pageSize)} more (${remaining} remaining)`,
    );
    try {
      _ensureComboObserver().observe(node);
    } catch (_) {
      /* IntersectionObserver unsupported — keep manual click */
    }
    return node;
  }

  function selectAsset(assetId) {
    state.selectedAssetId = assetId;
    pushRecent(assetId);
    state.pickerOpen = false;
    state.pickerFilter = "";
    state.pickerCursor = -1;
    state.expanded.clear();
    state.bulkSelected.clear();
    renderBulkBar();
    renderCombobox();
    updateSettingsLink();
    if (window.MarketWS) {
      try {
        window.MarketWS.connect(assetId);
      } catch (err) {
        reportError("ws.connect", err);
      }
    }
    loadOrderbook();
  }

  function updateSettingsLink() {
    const link = document.getElementById("btn-asset-settings");
    if (!link) return;
    if (state.selectedAssetId) {
      link.href = `/admin/marketplace/settings.html?asset_id=${state.selectedAssetId}`;
      link.removeAttribute("aria-disabled");
    } else {
      link.href = "/admin/marketplace/settings.html";
    }
  }

  function shiftAsset(direction) {
    if (!state.assets.length) return;
    const idx = state.assets.findIndex((a) => a.id === state.selectedAssetId);
    const next =
      (idx + (direction === "next" ? 1 : -1) + state.assets.length) % state.assets.length;
    selectAsset(state.assets[next].id);
  }

  // ─── load + refresh ──────────────────────────────────────────────

  async function loadAssets() {
    setBusy(true);
    setStatus("Loading orderbook assets…");
    try {
      state.assets = await fetchJSON(`${API}/assets`);
      if (state.assets.length && !state.selectedAssetId) {
        state.selectedAssetId = state.assets[0].id;
      }
      renderCombobox();
      setStatus("");
      if (state.assets.length) await loadOrderbook();
      else clearOrderbook("No tradable assets");
    } catch (err) {
      setStatus(`Unable to load assets: ${err.message}`, "error");
      clearOrderbook("Orderbook unavailable");
    } finally {
      setBusy(false);
    }
  }

  async function loadOrderbook(opts = {}) {
    if (!state.selectedAssetId) {
      clearOrderbook("Select an asset");
      return;
    }
    if (!opts.silent) {
      setBusy(true);
      setStatus("Loading orderbook…");
    }
    try {
      const data = await fetchJSON(`${API}/${state.selectedAssetId}`);
      state.fetchFailures = 0;
      renderOfflineBanner();
      renderOrderbook(data);
    } catch (err) {
      state.fetchFailures += 1;
      renderOfflineBanner();
      clearOrderbook("Orderbook unavailable");
      setStatus(`Unable to load orderbook: ${err.message}`, "error");
    } finally {
      if (!opts.silent) setBusy(false);
    }
  }

  function renderOfflineBanner() {
    const banner = document.getElementById("mp-ob-offline-banner");
    if (!banner) return;
    // Show when both polling has failed >= 3x AND ws is closed/reconnecting/never-open.
    const wsBad = state.wsState !== "open";
    const pollBad = state.fetchFailures >= 3;
    if (wsBad && pollBad) {
      banner.hidden = false;
      banner.textContent =
        `Connection lost. Live updates paused; ${state.fetchFailures} failed reloads. ` +
        `Click WS pill or hit "r" to retry.`;
    } else {
      banner.hidden = true;
      banner.textContent = "";
    }
  }

  function setupWebSocket() {
    if (!window.MarketWS || !window.MarketBus) return;
    window.MarketBus.on("orderbook:update", (msg) => {
      // Only refresh when message is for current asset.
      if (!state.selectedAssetId) return;
      if (msg && msg.asset_id && msg.asset_id !== state.selectedAssetId) return;
      loadOrderbook({ silent: true });
    });
    window.MarketBus.on("ws:state", ({ state: wsState }) => {
      state.wsState = wsState;
      renderOfflineBanner();
      if (state.lastData) renderLive(state.lastData);
    });
    if (state.selectedAssetId) {
      try {
        window.MarketWS.connect(state.selectedAssetId);
      } catch (err) {
        reportError("ws.connect", err);
      }
    }
  }

  function reconnectWebSocket() {
    if (!window.MarketWS || !state.selectedAssetId) return;
    try {
      window.MarketWS.disconnect();
      window.MarketWS.connect(state.selectedAssetId);
    } catch (err) {
      reportError("ws.reconnect", err);
    }
  }

  function startAutoRefresh() {
    stopAutoRefresh();
    if (state.refreshPaused) return;
    let tick = 0;
    state.refreshTimer = setInterval(() => {
      if (document.hidden) return;
      loadOrderbook({ silent: true });
      // Every 6th tick (~30s at 5s cadence) also refresh the asset list so
      // counts for OTHER assets (not the currently selected one) stay fresh.
      tick = (tick + 1) % 6;
      if (tick === 0) refreshAssetList();
    }, REFRESH_MS);
  }

  // Background-refresh the asset list without disturbing UI state. Used to
  // keep the combobox counts honest across all assets, not just the active one.
  async function refreshAssetList() {
    try {
      const fresh = await fetchJSON(`${API}/assets`);
      if (!Array.isArray(fresh)) return;
      // Preserve selection; merge counts only.
      const byId = new Map(fresh.map((a) => [a.id, a]));
      state.assets = state.assets.map((a) => byId.get(a.id) || a);
      // Add any newly-tradable assets that weren't in the list before.
      fresh.forEach((a) => {
        if (!state.assets.some((x) => x.id === a.id)) state.assets.push(a);
      });
      if (!state.pickerOpen) renderCombobox();
    } catch (_) {
      /* silent — non-critical background refresh */
    }
  }

  function stopAutoRefresh() {
    if (state.refreshTimer) clearInterval(state.refreshTimer);
    state.refreshTimer = null;
  }

  async function rebuildOrderbook() {
    if (
      !window.confirm(
        "Rebuild the Redis orderbook from PostgreSQL? This is a destructive operation.",
      )
    )
      return;
    setBusy(true);
    setStatus("Rebuilding orderbook…");
    try {
      const body = await fetchJSON(`${API}/rebuild`, {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
      });
      if (typeof window.mpToast === "function")
        window.mpToast(body.message || "Orderbook rebuilt", "success");
      await loadOrderbook();
      setStatus(body.message || "Orderbook rebuilt successfully", "success");
    } catch (err) {
      reportError("rebuildOrderbook", err);
      setStatus(`Unable to rebuild orderbook: ${err.message}`, "error");
      if (typeof window.mpToast === "function") window.mpToast(err.message, "error");
    } finally {
      setBusy(false);
    }
  }

  // ─── rebuild history dropdown ────────────────────────────────────

  async function loadRebuildHistory() {
    try {
      const params = new URLSearchParams({
        action: "marketplace.orderbook.rebuilt",
        per_page: "5",
      });
      state.rebuildHistory = await fetchJSON(`${AUDIT_API}?${params.toString()}`);
    } catch (err) {
      reportError("loadRebuildHistory", err);
      state.rebuildHistory = { error: err.message };
    }
    renderRebuildHistory();
  }

  function renderRebuildHistory() {
    const panel = document.getElementById("mp-ob-rebuild-history");
    if (!panel) return;
    clearNode(panel);
    if (!state.rebuildHistoryOpen) {
      panel.hidden = true;
      return;
    }
    panel.hidden = false;
    if (!state.rebuildHistory) {
      panel.appendChild(el("div", { class: "mp-ob-history-empty" }, "Loading…"));
      return;
    }
    if (state.rebuildHistory.error) {
      panel.appendChild(
        el(
          "div",
          { class: "mp-ob-history-empty" },
          `Could not load history: ${state.rebuildHistory.error}`,
        ),
      );
      return;
    }
    const rows = Array.isArray(state.rebuildHistory)
      ? state.rebuildHistory
      : state.rebuildHistory.data || [];
    if (!rows.length) {
      panel.appendChild(el("div", { class: "mp-ob-history-empty" }, "No rebuilds recorded."));
      return;
    }
    panel.appendChild(
      el(
        "div",
        { class: "mp-ob-history-footer" },
        el(
          "a",
          {
            href: "/admin/audit-logs.html?action=marketplace.orderbook.rebuilt",
            target: "_blank",
            rel: "noopener",
            class: "mp-ob-history-link",
          },
          "Open full audit timeline ↗",
        ),
      ),
    );
    rows.slice(0, 5).forEach((entry) => {
      const ts = entry.created_at || entry.timestamp;
      const actor = entry.actor_email || entry.actor_user_id || "system";
      const newState = entry.new_state || {};
      const restored = newState.orders_restored != null ? `${newState.orders_restored} orders` : "";
      panel.appendChild(
        el(
          "div",
          { class: "mp-ob-history-item" },
          el(
            "div",
            { class: "mp-ob-history-row" },
            el("span", { class: "mp-ob-history-when", title: formatTime(ts) }, formatRelative(ts)),
            el("span", { class: "mp-ob-history-meta" }, restored || "—"),
          ),
          el("div", { class: "mp-ob-history-actor" }, actor),
        ),
      );
    });
  }

  function toggleRebuildHistory() {
    state.rebuildHistoryOpen = !state.rebuildHistoryOpen;
    renderRebuildHistory();
    if (state.rebuildHistoryOpen && state.rebuildHistory == null) loadRebuildHistory();
  }

  // ─── CSV export ──────────────────────────────────────────────────

  function csvEscape(value) {
    const s = String(value ?? "");
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  }

  function renderDepthChart(bids, asks) {
    const svg = document.getElementById("mp-ob-depth-chart");
    if (!svg) return;
    if ((!bids || !bids.length) && (!asks || !asks.length)) {
      svg.hidden = true;
      svg.replaceChildren();
      return;
    }
    svg.hidden = false;

    // Build cumulative curves on shared price axis.
    const W = 800;
    const H = 80;
    const allPrices = [...bids.map((l) => l.price_cents), ...asks.map((l) => l.price_cents)];
    if (!allPrices.length) {
      svg.replaceChildren();
      return;
    }
    const minP = Math.min(...allPrices);
    const maxP = Math.max(...allPrices);
    const span = Math.max(1, maxP - minP);

    const buildCurve = (levels, ascending) => {
      const sorted = levels
        .slice()
        .sort((a, b) => (ascending ? a.price_cents - b.price_cents : b.price_cents - a.price_cents));
      let cum = 0;
      return sorted.map((l) => {
        cum += Number(l.total_quantity || 0);
        return { x: ((l.price_cents - minP) / span) * W, qty: cum };
      });
    };
    const bidPts = buildCurve(bids, false); // descending → cumulative right→left
    const askPts = buildCurve(asks, true);
    const maxQty = Math.max(
      bidPts.reduce((m, p) => Math.max(m, p.qty), 0),
      askPts.reduce((m, p) => Math.max(m, p.qty), 0),
      1,
    );
    const toY = (q) => H - (q / maxQty) * (H - 4) - 2;

    const toPath = (pts, leftEdge) => {
      if (!pts.length) return "";
      // pts come ordered along their natural side; we draw a step-curve.
      let d = `M ${leftEdge} ${H} L ${pts[0].x} ${H}`;
      pts.forEach((p) => {
        d += ` L ${p.x} ${toY(p.qty)}`;
      });
      d += ` L ${pts[pts.length - 1].x} ${H} Z`;
      return d;
    };

    const ns = "http://www.w3.org/2000/svg";
    const gradId = "mp-ob-depth-grad";
    svg.replaceChildren();
    const defs = document.createElementNS(ns, "defs");
    // Explicit hex stops so light-mode gradient does not collapse to grey
    // when `currentColor` is muted on the parent SVG.
    defs.innerHTML =
      `<linearGradient id="bid-fill" x1="0" x2="0" y1="0" y2="1">
        <stop offset="0%" stop-color="#22c55e" stop-opacity="0.45"/>
        <stop offset="100%" stop-color="#22c55e" stop-opacity="0.05"/>
      </linearGradient>` +
      `<linearGradient id="ask-fill" x1="0" x2="0" y1="0" y2="1">
        <stop offset="0%" stop-color="#ef4444" stop-opacity="0.45"/>
        <stop offset="100%" stop-color="#ef4444" stop-opacity="0.05"/>
      </linearGradient>`;
    svg.appendChild(defs);

    if (bidPts.length) {
      const bidPath = document.createElementNS(ns, "path");
      bidPath.setAttribute("d", toPath(bidPts, 0));
      bidPath.setAttribute("class", "mp-ob-depth-bid");
      bidPath.setAttribute("fill", "url(#bid-fill)");
      bidPath.setAttribute("stroke", "#16a34a");
      bidPath.setAttribute("stroke-width", "1");
      svg.appendChild(bidPath);
    }
    if (askPts.length) {
      const askPath = document.createElementNS(ns, "path");
      askPath.setAttribute("d", toPath(askPts, W));
      askPath.setAttribute("class", "mp-ob-depth-ask");
      askPath.setAttribute("fill", "url(#ask-fill)");
      askPath.setAttribute("stroke", "#dc2626");
      askPath.setAttribute("stroke-width", "1");
      svg.appendChild(askPath);
    }
    // Mid-line marker
    if (state.lastData && state.lastData.mid_price_cents != null) {
      const x = ((state.lastData.mid_price_cents - minP) / span) * W;
      const line = document.createElementNS(ns, "line");
      line.setAttribute("x1", x);
      line.setAttribute("x2", x);
      line.setAttribute("y1", 0);
      line.setAttribute("y2", H);
      line.setAttribute("class", "mp-ob-depth-mid");
      svg.appendChild(line);
    }
    void gradId;
  }

  function exportCsv() {
    const data = state.lastData;
    if (!data) return;
    const lines = ["side,price_cents,total_quantity,order_count,unique_users"];
    (data.bids || []).forEach((l) =>
      lines.push(
        ["buy", l.price_cents, l.total_quantity, l.order_count, l.unique_users || 0]
          .map(csvEscape)
          .join(","),
      ),
    );
    (data.asks || []).forEach((l) =>
      lines.push(
        ["sell", l.price_cents, l.total_quantity, l.order_count, l.unique_users || 0]
          .map(csvEscape)
          .join(","),
      ),
    );
    const blob = new Blob([lines.join("\n")], { type: "text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = el("a", {
      href: url,
      download: `orderbook_${data.asset_slug || data.asset_id}_${new Date()
        .toISOString()
        .replace(/[:.]/g, "-")}.csv`,
    });
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  function exportPdf() {
    // Use browser print dialog with print-only stylesheet hooks.
    document.body.classList.add("mp-ob-printing");
    const cleanup = () => document.body.classList.remove("mp-ob-printing");
    window.addEventListener("afterprint", cleanup, { once: true });
    setTimeout(() => window.print(), 50);
    setTimeout(cleanup, 4000); // safety fallback
  }


  function _applyOverrides(overrides) {
    if (!overrides) return;
    if (overrides.tick_size_cents != null && state.tickCents === 1) {
      state.tickCents = Number(overrides.tick_size_cents) || 1;
      const tick = document.getElementById("mp-ob-tick");
      if (tick) tick.value = String(state.tickCents);
    }
    if (overrides.min_order_size != null && state.minQty === 0) {
      state.minQty = Number(overrides.min_order_size) || 0;
      const minQ = document.getElementById("mp-ob-min-qty");
      if (minQ) minQ.value = String(state.minQty);
    }
    if (state.lastData) renderOrderbook(state.lastData, { keepStatus: true });
  }

  // ─── per-asset settings drawer ───────────────────────────────────

  const DRAWER_SCROLL_KEY = "poool.admin.orderbook.drawerScroll";
  let _drawerPrevFocus = null;

  function openSettingsDrawer() {
    const drawer = document.getElementById("mp-ob-settings-drawer");
    if (!drawer) return;
    state.settingsOpen = true;
    _drawerPrevFocus = document.activeElement;
    drawer.hidden = false;
    drawer.setAttribute("aria-hidden", "false");
    document.body.classList.add("mp-ob-drawer-open");
    drawer.addEventListener("keydown", _drawerKeydown);
    loadAssetSettings().then(() => {
      const body = document.getElementById("mp-ob-settings-body");
      const saved = Number(localStorage.getItem(DRAWER_SCROLL_KEY) || "0");
      if (body && Number.isFinite(saved)) body.scrollTop = saved;
      if (body) body.addEventListener("scroll", _drawerScrollSave);
    });
    requestAnimationFrame(() => {
      const close = document.getElementById("mp-ob-drawer-close");
      if (close) close.focus();
    });
  }

  function closeSettingsDrawer() {
    const drawer = document.getElementById("mp-ob-settings-drawer");
    if (!drawer) return;
    if (state.assetSettingsDirty) {
      const proceed = window.confirm("Discard unsaved settings changes?");
      if (!proceed) return;
    }
    state.assetSettingsDirty = false;
    state.settingsOpen = false;
    drawer.hidden = true;
    drawer.setAttribute("aria-hidden", "true");
    drawer.removeEventListener("keydown", _drawerKeydown);
    const body = document.getElementById("mp-ob-settings-body");
    if (body) body.removeEventListener("scroll", _drawerScrollSave);
    document.body.classList.remove("mp-ob-drawer-open");
    if (_drawerPrevFocus && typeof _drawerPrevFocus.focus === "function") {
      _drawerPrevFocus.focus();
    }
    _drawerPrevFocus = null;
  }

  function _drawerScrollSave(ev) {
    try {
      localStorage.setItem(DRAWER_SCROLL_KEY, String(ev.target.scrollTop || 0));
    } catch (_) {
      /* ignore */
    }
  }

  function _drawerKeydown(e) {
    if (e.key !== "Tab") return;
    const drawer = document.getElementById("mp-ob-settings-drawer");
    if (!drawer) return;
    const focusable = drawer.querySelectorAll(
      'a[href], button:not([disabled]), input:not([disabled]), select, textarea, [tabindex]:not([tabindex="-1"])',
    );
    if (!focusable.length) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }

  async function loadAssetSettings() {
    const body = document.getElementById("mp-ob-settings-body");
    if (!body) return;
    clearNode(body);
    body.appendChild(el("div", {}, "Loading…"));
    try {
      // Try the per-asset endpoint first; fall back to global if backend
      // is older or this asset has no record. Both return MarketplaceSettings-
      // shaped JSON: per-asset wraps it as { global, asset_overrides, ... }.
      let payload = null;
      if (state.selectedAssetId) {
        try {
          payload = await fetchJSON(
            `/api/admin/marketplace/settings/asset/${state.selectedAssetId}`,
          );
        } catch (_) {
          payload = null;
        }
      }
      const settings = payload
        ? Object.assign({}, payload.global || {}, payload.asset_overrides || {})
        : await fetchJSON("/api/admin/marketplace/settings");
      const hasOverride = !!(payload && payload.has_override);
      state.assetSettingsEtag = payload && payload.etag ? payload.etag : null;
      _applyOverrides(payload ? payload.asset_overrides : null);
      clearNode(body);
      if (hasOverride) {
        body.appendChild(
          el(
            "div",
            { class: "mp-ob-settings-banner" },
            "Per-asset overrides active (read-only here \u2014 use full settings page to edit).",
          ),
        );
      }
      const tickField = el(
        "label",
        { class: "mp-ob-settings-field" },
        el("span", {}, "Tick size (cents)"),
        el("input", {
          id: "mp-ob-settings-tick",
          type: "number",
          min: "1",
          value: settings.tick_size_cents != null ? String(settings.tick_size_cents) : "1",
        }),
      );
      const minQty = el(
        "label",
        { class: "mp-ob-settings-field" },
        el("span", {}, "Min order size"),
        el("input", {
          id: "mp-ob-settings-minqty",
          type: "number",
          min: "1",
          value: settings.min_order_size != null ? String(settings.min_order_size) : "1",
        }),
      );
      const maxSize = el(
        "label",
        { class: "mp-ob-settings-field" },
        el("span", {}, "Max order size"),
        el("input", {
          id: "mp-ob-settings-maxsize",
          type: "number",
          min: "1",
          value: settings.max_order_size != null ? String(settings.max_order_size) : "10000",
        }),
      );
      const matchingAlgo = el(
        "label",
        { class: "mp-ob-settings-field" },
        el("span", {}, "Matching algorithm"),
        (() => {
          const sel = el("select", { id: "mp-ob-settings-algo" });
          ["price-time", "pro-rata"].forEach((v) => {
            const o = el("option", { value: v }, v);
            if (settings.matching_algorithm === v) o.selected = true;
            sel.appendChild(o);
          });
          return sel;
        })(),
      );
      const tradingEnabled = el(
        "label",
        { class: "mp-ob-settings-field mp-ob-settings-field--row" },
        el("input", {
          id: "mp-ob-settings-trading",
          type: "checkbox",
          checked: settings.trading_enabled !== false,
        }),
        el("span", {}, "Trading enabled (kill-switch)"),
      );
      const weekend = el(
        "label",
        { class: "mp-ob-settings-field mp-ob-settings-field--row" },
        el("input", {
          id: "mp-ob-settings-weekend",
          type: "checkbox",
          checked: settings.weekend_trading === true,
        }),
        el("span", {}, "Weekend trading"),
      );
      body.appendChild(tickField);
      body.appendChild(minQty);
      body.appendChild(maxSize);
      body.appendChild(matchingAlgo);
      body.appendChild(tradingEnabled);
      body.appendChild(weekend);
      // Mark dirty on any change.
      [tickField, minQty, maxSize, matchingAlgo, tradingEnabled, weekend].forEach((wrap) => {
        wrap.querySelectorAll("input,select").forEach((field) => {
          field.addEventListener("change", () => (state.assetSettingsDirty = true));
          field.addEventListener("input", () => (state.assetSettingsDirty = true));
        });
      });
      state.assetSettingsDirty = false;
      const recentBox = el("div", { id: "mp-ob-settings-audit", class: "mp-ob-settings-audit" });
      body.appendChild(recentBox);
      loadAssetSettingsAudit(recentBox);
      body.appendChild(
        el(
          "div",
          { class: "mp-ob-settings-actions" },
          el(
            "button",
            {
              type: "button",
              class: "admin-btn admin-btn--primary admin-btn--sm",
              onClick: saveAssetSettings,
            },
            "Save as default for asset",
          ),
          el(
            "a",
            {
              class: "admin-btn admin-btn--ghost admin-btn--sm",
              href: `/admin/marketplace/settings.html?asset_id=${state.selectedAssetId}`,
              target: "_blank",
              rel: "noopener",
            },
            "Open full settings ↗",
          ),
        ),
      );
    } catch (err) {
      reportError("loadAssetSettings", err);
      clearNode(body);
      body.appendChild(
        el(
          "div",
          { class: "admin-alert admin-alert--error" },
          `Settings unavailable: ${err.message}`,
        ),
      );
    }
  }


  async function saveAssetSettings() {
    if (!state.selectedAssetId) return;
    const tick = Number(document.getElementById("mp-ob-settings-tick")?.value || 0) | 0;
    const minSize = Number(document.getElementById("mp-ob-settings-minqty")?.value || 0) | 0;
    const maxSize = Number(document.getElementById("mp-ob-settings-maxsize")?.value || 0) | 0;
    const algo = document.getElementById("mp-ob-settings-algo")?.value || null;
    const trading = document.getElementById("mp-ob-settings-trading");
    const weekend = document.getElementById("mp-ob-settings-weekend");
    const body = {
      tick_size_cents: tick > 0 ? tick : null,
      min_order_size: minSize > 0 ? minSize : null,
      max_order_size: maxSize > 0 ? maxSize : null,
      matching_algorithm: algo,
      trading_enabled: trading ? trading.checked : null,
      weekend_trading: weekend ? weekend.checked : null,
    };
    try {
      const headers = csrfHeaders({ "Content-Type": "application/json" });
      if (state.assetSettingsEtag) headers["If-Match"] = `"${state.assetSettingsEtag}"`;
      const resp = await fetchJSON(
        `/api/admin/marketplace/settings/asset/${state.selectedAssetId}`,
        { method: "POST", headers, body: JSON.stringify(body) },
      );
      state.assetSettingsEtag = resp && resp.etag ? resp.etag : state.assetSettingsEtag;
      state.assetSettingsDirty = false;
      _applyOverrides(body);
      if (typeof window.mpToast === "function") window.mpToast("Asset overrides saved", "success");
      const audit = document.getElementById("mp-ob-settings-audit");
      if (audit) loadAssetSettingsAudit(audit);
    } catch (err) {
      reportError("saveAssetSettings", err);
      if (typeof window.mpToast === "function")
        window.mpToast(
          /409|conflict|etag/i.test(err.message)
            ? "Settings were updated by someone else. Reload the drawer to see the latest."
            : err.message,
          "error",
        );
    }
  }

  async function loadAssetSettingsAudit(container) {
    if (!container || !state.selectedAssetId) return;
    clearNode(container);
    container.appendChild(el("div", { class: "mp-ob-settings-audit-title" }, "Recent overrides"));
    try {
      const data = await fetchJSON(
        `/api/admin/audit-logs?action=marketplace.asset_settings.saved&per_page=5`,
      );
      const rows = Array.isArray(data) ? data : data.logs || [];
      const matching = rows.filter((r) => String(r.entity_id) === String(state.selectedAssetId));
      if (!matching.length) {
        container.appendChild(
          el("div", { class: "mp-ob-settings-audit-empty" }, "No overrides recorded for this asset."),
        );
        return;
      }
      matching.slice(0, 5).forEach((r) => {
        container.appendChild(
          el(
            "div",
            { class: "mp-ob-settings-audit-row" },
            el(
              "span",
              { class: "mp-ob-settings-audit-when", title: formatTime(r.created_at) },
              formatRelative(r.created_at),
            ),
            el("span", { class: "mp-ob-settings-audit-actor" }, r.actor_email || "system"),
          ),
        );
      });
      container.appendChild(
        el(
          "a",
          {
            class: "mp-ob-history-link",
            target: "_blank",
            rel: "noopener",
            href: `/admin/audit-logs.html?action=marketplace.asset_settings.saved&entity_id=${state.selectedAssetId}`,
          },
          "Open full audit timeline ↗",
        ),
      );
    } catch (err) {
      reportError("loadAssetSettingsAudit", err);
      container.appendChild(
        el("div", { class: "mp-ob-settings-audit-empty" }, "Audit unavailable."),
      );
    }
  }

  // ─── toolbar wiring ──────────────────────────────────────────────

  function wireToolbar() {
    // Restore persisted control values into the toolbar
    document.querySelectorAll(".mp-ob-chip[data-side]").forEach((b) =>
      b.classList.toggle("is-active", b.dataset.side === state.sideFilter),
    );
    const tickEl = document.getElementById("mp-ob-tick");
    if (tickEl) tickEl.value = String(state.tickCents);
    const minQtyEl = document.getElementById("mp-ob-min-qty");
    if (minQtyEl && state.minQty) minQtyEl.value = String(state.minQty);
    const tzEl = document.getElementById("mp-ob-tz");
    if (tzEl) tzEl.value = state.tz;

    document.querySelectorAll(".mp-ob-chip[data-side]").forEach((btn) => {
      btn.addEventListener("click", () => {
        state.sideFilter = btn.dataset.side;
        document
          .querySelectorAll(".mp-ob-chip[data-side]")
          .forEach((b) => b.classList.toggle("is-active", b === btn));
        savePrefs();
        if (state.lastData) renderOrderbook(state.lastData, { keepStatus: true });
      });
    });

    if (tickEl) {
      tickEl.addEventListener("change", () => {
        state.tickCents = Number(tickEl.value) || 1;
        savePrefs();
        if (state.lastData) renderOrderbook(state.lastData, { keepStatus: true });
      });
    }

    if (minQtyEl) {
      minQtyEl.addEventListener("change", () => {
        state.minQty = Math.max(0, Number(minQtyEl.value) || 0);
        savePrefs();
        if (state.lastData) renderOrderbook(state.lastData, { keepStatus: true });
      });
    }

    if (tzEl) {
      tzEl.addEventListener("change", () => {
        state.tz = tzEl.value === "local" ? "local" : "utc";
        savePrefs();
        if (state.lastData) renderOrderbook(state.lastData, { keepStatus: true });
      });
    }

    const pauseBtn = document.getElementById("btn-pause-refresh");
    if (pauseBtn) {
      const sync = () => {
        pauseBtn.textContent = state.refreshPaused ? "▶ Resume" : "⏸ Pause";
        pauseBtn.setAttribute("aria-pressed", state.refreshPaused ? "true" : "false");
        pauseBtn.title = state.refreshPaused
          ? "Auto-refresh is paused — click to resume"
          : "Auto-refresh every 5s — click to pause";
      };
      sync();
      pauseBtn.addEventListener("click", () => {
        state.refreshPaused = !state.refreshPaused;
        savePrefs();
        sync();
        if (state.refreshPaused) stopAutoRefresh();
        else startAutoRefresh();
      });
    }
  }

  // ─── keyboard ────────────────────────────────────────────────────

  function handleKey(e) {
    const tag = (e.target.tagName || "").toLowerCase();
    if (tag === "input" || tag === "textarea" || tag === "select") return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === "[") {
      e.preventDefault();
      shiftAsset("prev");
    } else if (e.key === "]") {
      e.preventDefault();
      shiftAsset("next");
    } else if (e.key === "/") {
      e.preventDefault();
      state.pickerOpen = true;
      renderCombobox();
      const input = document.getElementById("mp-ob-combo-search");
      if (input) input.focus();
    } else if (e.key === "r" && !e.shiftKey) {
      e.preventDefault();
      loadOrderbook();
    } else if (e.key === "?") {
      e.preventDefault();
      toggleShortcutHelp();
    } else if (e.key === "Escape") {
      if (state.helpOpen) {
        toggleShortcutHelp(false);
        return;
      }
      if (state.settingsOpen) {
        closeSettingsDrawer();
        return;
      }
      if (state.pickerOpen) {
        state.pickerOpen = false;
        renderCombobox();
      }
    }
  }

  function toggleShortcutHelp(force) {
    const overlay = document.getElementById("mp-ob-help-overlay");
    if (!overlay) return;
    const next = typeof force === "boolean" ? force : overlay.hidden;
    overlay.hidden = !next;
    overlay.setAttribute("aria-hidden", String(!next));
    state.helpOpen = next;
  }

  function handleOutsideClick(e) {
    if (!state.pickerOpen) return;
    const wrapper = document.getElementById("asset-combobox");
    if (wrapper && !wrapper.contains(e.target)) {
      state.pickerOpen = false;
      renderCombobox();
    }
  }

  document.addEventListener("DOMContentLoaded", () => {
    closeReasonPrompt();
    wireToolbar();
    setupWebSocket();
    loadAssets();
    startAutoRefresh();
    setInterval(tickRelativeTimes, 1000);

    const rebuildBtn = document.getElementById("btn-rebuild-orderbook");
    if (rebuildBtn) rebuildBtn.addEventListener("click", rebuildOrderbook);

    const historyBtn = document.getElementById("btn-rebuild-history");
    if (historyBtn) historyBtn.addEventListener("click", toggleRebuildHistory);

    const csvBtn = document.getElementById("btn-export-csv");
    if (csvBtn) csvBtn.addEventListener("click", exportCsv);

    const helpBtn = document.getElementById("mp-ob-help-close");
    if (helpBtn) helpBtn.addEventListener("click", () => toggleShortcutHelp(false));
    const helpOverlay = document.getElementById("mp-ob-help-overlay");
    if (helpOverlay) {
      helpOverlay.hidden = true;
      helpOverlay.setAttribute("aria-hidden", "true");
      helpOverlay.addEventListener("click", (ev) => {
        if (ev.target === helpOverlay) toggleShortcutHelp(false);
      });
    }
    document.querySelectorAll(".mp-ob-shortcut[data-shortcut]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const action = btn.dataset.shortcut;
        if (action === "prev") shiftAsset("prev");
        else if (action === "next") shiftAsset("next");
        else if (action === "reload") loadOrderbook();
        else if (action === "search") {
          state.pickerOpen = true;
          renderCombobox();
          const input = document.getElementById("mp-ob-combo-search");
          if (input) input.focus();
        }
      });
    });

    const pdfBtn = document.getElementById("btn-export-pdf");
    if (pdfBtn) pdfBtn.addEventListener("click", exportPdf);

    const settingsBtn = document.getElementById("btn-asset-settings");
    if (settingsBtn) {
      settingsBtn.addEventListener("click", (ev) => {
        // Open in-page drawer; let cmd/ctrl-click open new tab.
        if (ev.metaKey || ev.ctrlKey || ev.shiftKey) return;
        ev.preventDefault();
        openSettingsDrawer();
      });
    }
    const drawerClose = document.getElementById("mp-ob-drawer-close");
    if (drawerClose) drawerClose.addEventListener("click", closeSettingsDrawer);
    const drawerScrim = document.getElementById("mp-ob-drawer-scrim");
    if (drawerScrim) drawerScrim.addEventListener("click", closeSettingsDrawer);

    document.addEventListener("keydown", handleKey);
    document.addEventListener("click", handleOutsideClick);
  });
})();
