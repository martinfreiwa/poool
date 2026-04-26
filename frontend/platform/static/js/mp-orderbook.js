/**
 * Marketplace Orderbook
 * Renders live, aggregated admin orderbook data from backend APIs.
 */
(function () {
  "use strict";

  const API = "/api/admin/marketplace/orderbook";

  const state = {
    assets: [],
    selectedAssetId: "",
    loading: false,
  };

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
    return `$${(Number(cents || 0) / 100).toFixed(2)}`;
  }

  function formatQty(value) {
    return Number(value || 0).toLocaleString();
  }

  function setStatus(message, type = "info") {
    const el = document.getElementById("orderbook-status");
    if (!el) return;
    if (!message) {
      el.hidden = true;
      el.textContent = "";
      el.className = "admin-alert";
      return;
    }
    el.hidden = false;
    el.className = `admin-alert admin-alert--${type}`;
    el.textContent = message;
  }

  function setBusy(isBusy) {
    state.loading = isBusy;
    const selector = document.getElementById("asset-selector");
    const rebuildBtn = document.getElementById("btn-rebuild-orderbook");
    if (selector) selector.disabled = isBusy;
    if (rebuildBtn) {
      rebuildBtn.disabled = isBusy;
      rebuildBtn.setAttribute("aria-busy", isBusy ? "true" : "false");
    }
  }

  function clearNode(node) {
    if (!node) return;
    while (node.firstChild) node.removeChild(node.firstChild);
  }

  function appendText(parent, tagName, className, text) {
    const el = document.createElement(tagName);
    if (className) el.className = className;
    el.textContent = text;
    parent.appendChild(el);
    return el;
  }

  function renderStat(label, value) {
    const wrapper = document.createElement("div");
    wrapper.className = "mp-ob-stat";
    appendText(wrapper, "div", "mp-ob-stat-label", label);
    appendText(wrapper, "div", "mp-ob-stat-value", value);
    return wrapper;
  }

  function renderStats(data) {
    const stats = document.getElementById("ob-stats");
    if (!stats) return;
    clearNode(stats);

    const totalOrders =
      data.bids.reduce((sum, level) => sum + Number(level.order_count || 0), 0) +
      data.asks.reduce((sum, level) => sum + Number(level.order_count || 0), 0);
    stats.append(
      renderStat("Mid Price", data.mid_price_cents == null ? "N/A" : formatUsd(data.mid_price_cents)),
      renderStat("Orders", totalOrders.toLocaleString()),
    );
  }

  function renderEmptyRow(tbody, message) {
    clearNode(tbody);
    const row = document.createElement("tr");
    const cell = document.createElement("td");
    cell.colSpan = 3;
    cell.style.textAlign = "center";
    cell.style.color = "var(--admin-text-muted)";
    cell.textContent = message;
    row.appendChild(cell);
    tbody.appendChild(row);
  }

  function renderDepthCell(row, side, widthPct, value) {
    const cell = document.createElement("td");
    cell.style.position = "relative";
    const depth = document.createElement("div");
    depth.className = `mp-ob-depth mp-ob-depth--${side}`;
    depth.style.width = `${Math.max(0, Math.min(100, widthPct))}%`;
    const text = document.createElement("span");
    text.style.position = "relative";
    text.textContent = value;
    cell.append(depth, text);
    row.appendChild(cell);
  }

  function renderBids(bids) {
    const tbody = document.getElementById("bids-body");
    if (!tbody) return;
    if (!bids.length) {
      renderEmptyRow(tbody, "No bids");
      return;
    }

    clearNode(tbody);
    const maxQty = Math.max(...bids.map((level) => Number(level.total_quantity || 0)), 1);
    bids.forEach((level) => {
      const row = document.createElement("tr");
      row.style.position = "relative";
      appendText(row, "td", null, `${level.order_count} order${Number(level.order_count) === 1 ? "" : "s"}`);

      const qty = document.createElement("td");
      qty.style.textAlign = "right";
      qty.textContent = formatQty(level.total_quantity);
      row.appendChild(qty);

      renderDepthCell(row, "bid", (Number(level.total_quantity || 0) / maxQty) * 100, formatUsd(level.price_cents));
      tbody.appendChild(row);
    });
  }

  function renderAsks(asks) {
    const tbody = document.getElementById("asks-body");
    if (!tbody) return;
    if (!asks.length) {
      renderEmptyRow(tbody, "No asks");
      return;
    }

    clearNode(tbody);
    const maxQty = Math.max(...asks.map((level) => Number(level.total_quantity || 0)), 1);
    asks.forEach((level) => {
      const row = document.createElement("tr");
      row.style.position = "relative";
      renderDepthCell(row, "ask", (Number(level.total_quantity || 0) / maxQty) * 100, formatUsd(level.price_cents));
      appendText(row, "td", null, formatQty(level.total_quantity));

      const orders = document.createElement("td");
      orders.style.textAlign = "right";
      orders.textContent = `${level.order_count} order${Number(level.order_count) === 1 ? "" : "s"}`;
      row.appendChild(orders);
      tbody.appendChild(row);
    });
  }

  function renderSpread(data) {
    const spreadBar = document.getElementById("spread-bar");
    if (!spreadBar) return;
    clearNode(spreadBar);

    const bestBid = data.bids.length ? Number(data.bids[0].price_cents) : null;
    const bestAsk = data.asks.length ? Number(data.asks[0].price_cents) : null;
    if (bestBid == null || bestAsk == null) {
      const item = document.createElement("div");
      item.className = "mp-ob-spread-item";
      appendText(item, "span", "mp-ob-spread-label", "No active market");
      spreadBar.appendChild(item);
      return;
    }

    const spread = bestAsk - bestBid;
    const spreadPct = bestBid > 0 ? ((spread / bestBid) * 100).toFixed(2) : "0.00";
    const totalBidVol = data.bids.reduce((sum, level) => sum + Number(level.total_quantity || 0), 0);
    const totalAskVol = data.asks.reduce((sum, level) => sum + Number(level.total_quantity || 0), 0);

    [
      ["Best Bid", formatUsd(bestBid), "mp-ob-spread-value mp-ob-spread-value--bid"],
      ["Spread", `${formatUsd(spread)} (${spreadPct}%)`, "mp-ob-spread-value"],
      ["Best Ask", formatUsd(bestAsk), "mp-ob-spread-value mp-ob-spread-value--ask"],
      ["Bid Vol", totalBidVol.toLocaleString(), "mp-ob-spread-value"],
      ["Ask Vol", totalAskVol.toLocaleString(), "mp-ob-spread-value"],
    ].forEach(([label, value, valueClass]) => {
      const item = document.createElement("div");
      item.className = "mp-ob-spread-item";
      appendText(item, "span", "mp-ob-spread-label", label);
      appendText(item, "span", valueClass, value);
      spreadBar.appendChild(item);
    });
  }

  function renderOrderbook(data) {
    const nameEl = document.getElementById("selected-asset-name");
    if (nameEl) nameEl.textContent = data.asset_title || `Asset ${data.asset_id}`;
    renderStats(data);
    renderSpread(data);
    renderBids(data.bids || []);
    renderAsks(data.asks || []);
  }

  function renderSelector() {
    const selector = document.getElementById("asset-selector");
    if (!selector) return;
    clearNode(selector);

    if (!state.assets.length) {
      const option = document.createElement("option");
      option.value = "";
      option.textContent = "No tradable assets";
      selector.appendChild(option);
      selector.disabled = true;
      return;
    }

    state.assets.forEach((asset) => {
      const option = document.createElement("option");
      option.value = asset.id;
      option.textContent = `${asset.title} (${asset.active_orders} active)`;
      selector.appendChild(option);
    });

    if (!state.selectedAssetId) state.selectedAssetId = state.assets[0].id;
    selector.value = state.selectedAssetId;
  }

  async function fetchJSON(url, options = {}) {
    const response = await fetch(url, { credentials: "same-origin", ...options });
    if (!response.ok) {
      let message = `HTTP ${response.status}`;
      try {
        const body = await response.json();
        message = body.error || body.message || message;
      } catch (_err) {
        /* keep HTTP fallback */
      }
      throw new Error(message);
    }
    return response.json();
  }

  async function loadAssets() {
    setBusy(true);
    setStatus("Loading orderbook assets...");
    try {
      state.assets = await fetchJSON(`${API}/assets`);
      renderSelector();
      setStatus("");
      if (state.assets.length) await loadOrderbook();
    } catch (err) {
      renderSelector();
      setStatus(`Unable to load assets: ${err.message}`, "error");
      clearOrderbook("Orderbook unavailable");
    } finally {
      setBusy(false);
    }
  }

  function clearOrderbook(message) {
    const stats = document.getElementById("ob-stats");
    const spreadBar = document.getElementById("spread-bar");
    clearNode(stats);
    clearNode(spreadBar);
    if (spreadBar) appendText(spreadBar, "div", "mp-ob-spread-item", message);
    const bidsBody = document.getElementById("bids-body");
    const asksBody = document.getElementById("asks-body");
    if (bidsBody) renderEmptyRow(bidsBody, "No bids");
    if (asksBody) renderEmptyRow(asksBody, "No asks");
  }

  async function loadOrderbook() {
    const selector = document.getElementById("asset-selector");
    const assetId = selector ? selector.value : state.selectedAssetId;
    if (!assetId) {
      clearOrderbook("Select an asset");
      return;
    }

    state.selectedAssetId = assetId;
    setBusy(true);
    setStatus("Loading orderbook...");
    try {
      const data = await fetchJSON(`${API}/${assetId}`);
      renderOrderbook(data);
      setStatus("");
    } catch (err) {
      clearOrderbook("Orderbook unavailable");
      setStatus(`Unable to load orderbook: ${err.message}`, "error");
    } finally {
      setBusy(false);
    }
  }

  async function rebuildOrderbook() {
    if (!window.confirm("Rebuild the Redis orderbook from PostgreSQL now?")) return;

    setBusy(true);
    setStatus("Rebuilding orderbook...");
    try {
      const body = await fetchJSON(`${API}/rebuild`, {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
      });
      if (typeof window.mpToast === "function") {
        window.mpToast(body.message || "Orderbook rebuilt successfully", "success");
      }
      await loadOrderbook();
      setStatus(body.message || "Orderbook rebuilt successfully", "success");
    } catch (err) {
      setStatus(`Unable to rebuild orderbook: ${err.message}`, "error");
      if (typeof window.mpToast === "function") window.mpToast(err.message, "error");
    } finally {
      setBusy(false);
    }
  }

  document.addEventListener("DOMContentLoaded", () => {
    loadAssets();

    const selector = document.getElementById("asset-selector");
    if (selector) selector.addEventListener("change", loadOrderbook);

    const rebuildBtn = document.getElementById("btn-rebuild-orderbook");
    if (rebuildBtn) rebuildBtn.addEventListener("click", rebuildOrderbook);
  });
})();
