/**
 * Admin Live Assets JS
 * Manages published marketplace assets with funding progress.
 */

let allAssets = [];
let filteredAssets = [];
let currentPage = 1;
let PAGE_SIZE = 10;
const EXPLORER_BASE = "https://polygonscan.com";
const SNAPSHOT_KEY = "admin_assets_kpi_snapshot_v1";
const VIEWS_KEY = "admin_assets_saved_views_v1";
const REFRESH_INTERVAL_MS = 30000;
let refreshTimer = null;
let lastFetchAt = null;
let advancedFilters = {
  valueMin: null, valueMax: null,
  yieldMin: null, yieldMax: null,
  locations: new Set(),
};
let userActive = false;
let userActiveTimer = null;
let sortField = "title";
let sortOrder = "asc";
let assetStatusTimer = null;
const selectedIds = new Set();

const TERMINAL_STATUSES = new Set(["exited"]);
const ACTIVE_STATUSES = new Set(["funding_open", "funding_in_progress", "funded", "rented", "payout_pending"]);

function daysSince(iso) {
  if (!iso) return 0;
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return 0;
  return Math.max(0, Math.floor((Date.now() - t) / 86400000));
}

function fundingPct(a) {
  return a.tokens_total > 0
    ? Math.round(((a.tokens_total - a.tokens_available) / a.tokens_total) * 100)
    : 0;
}

function statusColorVar(s) {
  if (s === "funding_open") return "var(--admin-success)";
  if (s === "funding_in_progress" || s === "funded") return "var(--admin-info)";
  if (s === "payout_pending") return "var(--admin-warning)";
  if (s === "exited") return "var(--admin-text-muted)";
  if (s === "rented") return "var(--admin-success)";
  return "var(--admin-text-muted)";
}

document.addEventListener("DOMContentLoaded", () => {
  loadAssets();
  document
    .getElementById("asset-search")
    ?.addEventListener("input", debounce(applyFilters, 200));
  document
    .getElementById("filter-type")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("filter-status")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("filter-featured")
    ?.addEventListener("change", applyFilters);

  setupSorting();
  setupPagination();
  setupBulkActions();
  document.getElementById("export-csv-btn")?.addEventListener("click", () => exportCsv(filteredAssets, "assets-filtered.csv"));
  document.getElementById("page-size")?.addEventListener("change", (e) => {
    const v = parseInt(e.target.value, 10);
    if (Number.isFinite(v) && v > 0) {
      PAGE_SIZE = v;
      currentPage = 1;
      renderTable();
    }
  });
  document.querySelectorAll(".admin-kpi-card--clickable").forEach((card) => {
    card.addEventListener("click", () => {
      const sel = document.getElementById("filter-status");
      if (sel) {
        sel.value = card.dataset.filterStatus || "";
        applyFilters();
      }
    });
  });
  setupAdvancedFilters();
  setupSavedViews();
  setupAutoRefresh();
});

function setupAdvancedFilters() {
  document.getElementById("toggle-filters-btn")?.addEventListener("click", (e) => {
    const panel = document.getElementById("advanced-filter-panel");
    const btn = e.currentTarget;
    if (!panel) return;
    const open = panel.classList.toggle("open");
    btn.setAttribute("aria-expanded", open ? "true" : "false");
  });
  ["filter-value-min", "filter-value-max", "filter-yield-min", "filter-yield-max"].forEach((id) => {
    document.getElementById(id)?.addEventListener("input", debounce(() => {
      advancedFilters.valueMin = numOrNull(document.getElementById("filter-value-min")?.value);
      advancedFilters.valueMax = numOrNull(document.getElementById("filter-value-max")?.value);
      advancedFilters.yieldMin = numOrNull(document.getElementById("filter-yield-min")?.value);
      advancedFilters.yieldMax = numOrNull(document.getElementById("filter-yield-max")?.value);
      applyFilters();
    }, 200));
  });
  document.getElementById("filters-clear-btn")?.addEventListener("click", () => {
    ["filter-value-min", "filter-value-max", "filter-yield-min", "filter-yield-max"].forEach((id) => {
      const el = document.getElementById(id);
      if (el) el.value = "";
    });
    advancedFilters = { valueMin: null, valueMax: null, yieldMin: null, yieldMax: null, locations: new Set() };
    renderLocationChips();
    applyFilters();
  });
}

function numOrNull(v) {
  if (v == null || v === "") return null;
  const n = parseFloat(v);
  return Number.isFinite(n) ? n : null;
}

function renderLocationChips() {
  const host = document.getElementById("filter-locations");
  if (!host) return;
  const cities = [...new Set(allAssets.map((a) => a.location_city).filter(Boolean))].sort();
  host.innerHTML = cities
    .map((c) => `<button type="button" class="admin-location-chip${advancedFilters.locations.has(c) ? " active" : ""}" data-city="${esc(c)}">${esc(c)}</button>`)
    .join("") || `<span style="font-size:11px;color:var(--admin-text-muted)">No locations</span>`;
  host.querySelectorAll(".admin-location-chip").forEach((chip) => {
    chip.addEventListener("click", () => {
      const city = chip.dataset.city;
      if (advancedFilters.locations.has(city)) advancedFilters.locations.delete(city);
      else advancedFilters.locations.add(city);
      chip.classList.toggle("active");
      applyFilters();
    });
  });
}

function setupSavedViews() {
  refreshSavedViewsDropdown();
  document.getElementById("saved-views")?.addEventListener("change", (e) => {
    const name = e.target.value;
    if (!name) return;
    const views = readSavedViews();
    const view = views[name];
    if (!view) return;
    applyView(view);
  });
  document.getElementById("save-view-btn")?.addEventListener("click", () => {
    const name = window.prompt("Name this view:");
    if (!name) return;
    const views = readSavedViews();
    views[name] = captureView();
    writeSavedViews(views);
    refreshSavedViewsDropdown(name);
    showAssetStatus(`View "${name}" saved.`);
  });
}

function readSavedViews() {
  try { return JSON.parse(localStorage.getItem(VIEWS_KEY) || "{}"); } catch { return {}; }
}
function writeSavedViews(v) {
  try { localStorage.setItem(VIEWS_KEY, JSON.stringify(v)); } catch {}
}
function refreshSavedViewsDropdown(selected) {
  const sel = document.getElementById("saved-views");
  if (!sel) return;
  const views = readSavedViews();
  const names = Object.keys(views).sort();
  sel.innerHTML = `<option value="">— Saved views —</option>` +
    names.map((n) => `<option value="${esc(n)}"${n === selected ? " selected" : ""}>${esc(n)}</option>`).join("") +
    (names.length ? `<option disabled>──────</option><option value="__delete__">Delete current…</option>` : "");
  if (selected === "__delete__") {
    const name = window.prompt("Delete which view? Type exact name:");
    if (name && views[name]) {
      delete views[name];
      writeSavedViews(views);
      refreshSavedViewsDropdown();
      showAssetStatus(`View "${name}" deleted.`);
    } else {
      sel.value = "";
    }
  }
}
function captureView() {
  return {
    search: document.getElementById("asset-search")?.value || "",
    type: document.getElementById("filter-type")?.value || "",
    status: document.getElementById("filter-status")?.value || "",
    featured: !!document.getElementById("filter-featured")?.checked,
    valueMin: advancedFilters.valueMin,
    valueMax: advancedFilters.valueMax,
    yieldMin: advancedFilters.yieldMin,
    yieldMax: advancedFilters.yieldMax,
    locations: [...advancedFilters.locations],
    sortField, sortOrder,
  };
}
function applyView(v) {
  const set = (id, val) => { const el = document.getElementById(id); if (el) el.value = val ?? ""; };
  set("asset-search", v.search);
  set("filter-type", v.type);
  set("filter-status", v.status);
  const f = document.getElementById("filter-featured"); if (f) f.checked = !!v.featured;
  set("filter-value-min", v.valueMin ?? "");
  set("filter-value-max", v.valueMax ?? "");
  set("filter-yield-min", v.yieldMin ?? "");
  set("filter-yield-max", v.yieldMax ?? "");
  advancedFilters.valueMin = v.valueMin ?? null;
  advancedFilters.valueMax = v.valueMax ?? null;
  advancedFilters.yieldMin = v.yieldMin ?? null;
  advancedFilters.yieldMax = v.yieldMax ?? null;
  advancedFilters.locations = new Set(v.locations || []);
  if (v.sortField) sortField = v.sortField;
  if (v.sortOrder) sortOrder = v.sortOrder;
  renderLocationChips();
  applyFilters();
}

function setupAutoRefresh() {
  document.getElementById("refresh-btn")?.addEventListener("click", () => loadAssets({ silent: false }));
  document.getElementById("auto-refresh")?.addEventListener("change", (e) => {
    if (e.target.checked) startAutoRefresh();
    else stopAutoRefresh();
  });
  ["mousemove", "keydown", "scroll"].forEach((evt) => {
    document.addEventListener(evt, () => {
      userActive = true;
      clearTimeout(userActiveTimer);
      userActiveTimer = setTimeout(() => { userActive = false; }, 5000);
    }, { passive: true });
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) stopAutoRefresh();
    else if (document.getElementById("auto-refresh")?.checked) startAutoRefresh();
  });
  startAutoRefresh();
  setInterval(updateLastUpdatedLabel, 5000);
}
function startAutoRefresh() {
  stopAutoRefresh();
  refreshTimer = setInterval(() => {
    if (document.hidden) return;
    if (userActive) return;
    if (selectedIds.size > 0) return;
    loadAssets({ silent: true });
  }, REFRESH_INTERVAL_MS);
}
function stopAutoRefresh() {
  if (refreshTimer) { clearInterval(refreshTimer); refreshTimer = null; }
}
function updateLastUpdatedLabel() {
  const el = document.getElementById("last-updated-label");
  if (!el || !lastFetchAt) return;
  const sec = Math.floor((Date.now() - lastFetchAt) / 1000);
  el.textContent = sec < 5 ? "just now" : sec < 60 ? `${sec}s ago` : `${Math.floor(sec / 60)}m ago`;
  el.title = `Last fetched ${new Date(lastFetchAt).toLocaleTimeString()}`;
}

function setupBulkActions() {
  document.getElementById("select-all")?.addEventListener("change", (e) => {
    const start = (currentPage - 1) * PAGE_SIZE;
    const slice = filteredAssets.slice(start, start + PAGE_SIZE);
    if (e.target.checked) slice.forEach((a) => selectedIds.add(a.id));
    else slice.forEach((a) => selectedIds.delete(a.id));
    renderTable();
  });
  document.getElementById("bulk-clear")?.addEventListener("click", () => {
    selectedIds.clear();
    renderTable();
  });
  document.getElementById("bulk-feature")?.addEventListener("click", () => bulkSetFeatured(true));
  document.getElementById("bulk-unfeature")?.addEventListener("click", () => bulkSetFeatured(false));
  document.getElementById("bulk-export")?.addEventListener("click", () => {
    const list = allAssets.filter((a) => selectedIds.has(a.id));
    exportCsv(list, "assets-selected.csv");
  });
}

async function bulkSetFeatured(target) {
  const ids = [...selectedIds];
  if (ids.length === 0) return;
  const targets = ids
    .map((id) => allAssets.find((a) => a.id === id))
    .filter((a) => a && !!a.featured !== target);
  if (targets.length === 0) {
    showAssetStatus(`All selected already ${target ? "featured" : "unfeatured"}.`);
    return;
  }
  showAssetStatus(`Updating ${targets.length} asset(s)…`);
  const results = await Promise.allSettled(
    targets.map((a) => fetch(`/api/admin/assets/${a.id}/toggle-featured`, { method: "POST" })),
  );
  const failed = results.filter((r) => r.status === "rejected" || (r.value && !r.value.ok)).length;
  if (failed > 0) showAssetStatus(`${failed} update(s) failed.`, "error");
  else showAssetStatus(`${targets.length} asset(s) updated.`);
  selectedIds.clear();
  await loadAssets();
}

function exportCsv(rows, filename) {
  if (!rows || rows.length === 0) {
    showAssetStatus("Nothing to export.", "error");
    return;
  }
  const cols = [
    ["title", "Title"],
    ["slug", "Slug"],
    ["asset_type", "Type"],
    ["total_value_cents", "Value (USD)"],
    ["tokens_total", "Tokens Total"],
    ["tokens_available", "Tokens Available"],
    ["funding_pct", "Funding %"],
    ["annual_yield_bps", "Yield %"],
    ["location_city", "Location"],
    ["funding_status", "Status"],
    ["featured", "Featured"],
    ["age_days", "Age (days)"],
    ["created_at", "Created"],
    ["holders_count", "Holders"],
    ["pending_settlements", "Pending Settlements"],
    ["chain_tx_hash", "Chain TX"],
    ["chain_contract_address", "Contract"],
  ];
  const escCsv = (v) => {
    if (v == null) return "";
    const s = String(v);
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const lines = [cols.map(([, h]) => h).join(",")];
  rows.forEach((a) => {
    const enriched = {
      ...a,
      "total_value_cents": ((a.total_value_cents || 0) / 100).toFixed(2),
      "funding_pct": fundingPct(a),
      "annual_yield_bps": a.annual_yield_bps ? (a.annual_yield_bps / 100).toFixed(2) : "",
      "age_days": daysSince(a.created_at),
    };
    lines.push(cols.map(([k]) => escCsv(enriched[k])).join(","));
  });
  const blob = new Blob([lines.join("\n")], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

function setupSorting() {
  const table = document.querySelector(".admin-table");
  if (!table) return;
  const headers = table.querySelectorAll("th[data-sort]");
  headers.forEach((th) => {
    th.style.cursor = "pointer";
    th.setAttribute("role", "button");
    th.setAttribute("tabindex", "0");
    th.setAttribute("aria-label", `Sort by ${th.textContent.trim()}`);
    th.addEventListener("click", () => updateSort(th.dataset.sort));
    th.addEventListener("keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      updateSort(th.dataset.sort);
    });
  });
  updateSortHeaderState();
}

function updateSort(field) {
  if (!field) return;
  if (sortField === field) {
    sortOrder = sortOrder === "asc" ? "desc" : "asc";
  } else {
    sortField = field;
    sortOrder = "asc";
  }
  applyFilters();
}

function updateSortHeaderState() {
  document.querySelectorAll(".admin-table th[data-sort]").forEach((th) => {
    th.setAttribute(
      "aria-sort",
      th.dataset.sort === sortField
        ? sortOrder === "asc"
          ? "ascending"
          : "descending"
        : "none",
    );
  });
}

function setupPagination() {
  document.getElementById("prev-page")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      renderTable();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    const maxPage = Math.ceil(filteredAssets.length / PAGE_SIZE);
    if (currentPage < maxPage) {
      currentPage++;
      renderTable();
    }
  });
}

async function loadAssets(opts = {}) {
  const silent = !!opts.silent;
  try {
    const resp = await fetch("/api/admin/assets");
    if (!resp.ok) {
      const message = await responseMessage(
        resp,
        "Unable to load assets. Please try again.",
      );
      if (!silent) {
        allAssets = [];
        filteredAssets = [];
        resetStats();
        renderErrorState(message);
      }
      showAssetStatus(message, "error");
      return;
    }

    const data = await resp.json();
    const assets = data.assets || data;
    if (!Array.isArray(assets)) {
      throw new Error("Asset API returned an unexpected response shape.");
    }
    allAssets = assets;
    lastFetchAt = Date.now();
    updateLastUpdatedLabel();
    renderLocationChips();
    applyFilters();
    updateStats();
  } catch (e) {
    console.error("Error loading assets", e);
    if (typeof Sentry !== "undefined") Sentry.captureException(e);
    if (!silent) {
      allAssets = [];
      filteredAssets = [];
      resetStats();
      renderErrorState("Unable to load assets. Please refresh or try again.");
    }
    showAssetStatus("Unable to load assets. Please refresh or try again.", "error");
  }
}

function responseMessage(resp, fallback) {
  return resp
    .clone()
    .json()
    .then((data) => data.message || data.error || fallback)
    .catch(() => fallback);
}

function resetStats() {
  [
    "stat-total",
    "stat-funding",
    "stat-funded",
    "stat-aum",
    "stat-tokens-sold",
  ].forEach((id) => {
    const el = document.getElementById(id);
    if (el) el.textContent = "—";
  });
}

let yieldAnomalyIds = new Set();

function computeYieldAnomalies() {
  const ys = allAssets
    .filter((a) => a.annual_yield_bps && ACTIVE_STATUSES.has(a.funding_status))
    .map((a) => a.annual_yield_bps);
  if (ys.length < 4) { yieldAnomalyIds = new Set(); return; }
  const mean = ys.reduce((s, v) => s + v, 0) / ys.length;
  const variance = ys.reduce((s, v) => s + (v - mean) ** 2, 0) / ys.length;
  const std = Math.sqrt(variance);
  if (std === 0) { yieldAnomalyIds = new Set(); return; }
  yieldAnomalyIds = new Set(
    allAssets
      .filter((a) => a.annual_yield_bps && Math.abs(a.annual_yield_bps - mean) > 2 * std)
      .map((a) => a.id),
  );
}

function readSnapshot() {
  try { return JSON.parse(localStorage.getItem(SNAPSHOT_KEY) || "null"); } catch { return null; }
}
function writeSnapshot(s) {
  try { localStorage.setItem(SNAPSHOT_KEY, JSON.stringify(s)); } catch {}
}
function snapshotKpis() {
  return {
    ts: Date.now(),
    total: allAssets.length,
    funding: allAssets.filter((a) => a.funding_status === "funding_open" || a.funding_status === "funding_in_progress").length,
    funded: allAssets.filter((a) => ["funded", "rented", "exited"].includes(a.funding_status)).length,
    aum: allAssets.reduce((s, a) => s + (a.total_value_cents || 0), 0),
    sold: allAssets.reduce((s, a) => s + ((a.tokens_total || 0) - (a.tokens_available || 0)), 0),
  };
}
function trendArrow(curr, prev) {
  if (prev == null || curr == null) return "";
  const delta = curr - prev;
  if (delta === 0) return `<span class="admin-trend-flat" style="font-size:11px;margin-left:6px" title="No change vs last snapshot">→ 0</span>`;
  const cls = delta > 0 ? "admin-trend-up" : "admin-trend-down";
  const arrow = delta > 0 ? "▲" : "▼";
  return `<span class="${cls}" style="font-size:11px;margin-left:6px" title="vs snapshot">${arrow} ${Math.abs(delta).toLocaleString()}</span>`;
}

function updateStats() {
  const prev = readSnapshot();
  const curr = snapshotKpis();
  const showTrend = prev && Date.now() - prev.ts > 6 * 3600 * 1000; // ≥6h old

  const setStat = (id, val, prevVal, formatter) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.innerHTML = (formatter ? formatter(val) : val.toLocaleString()) +
      (showTrend ? trendArrow(val, prevVal) : "");
  };

  const totalEl = document.getElementById("stat-total");
  if (totalEl) totalEl.innerHTML = allAssets.length + (showTrend ? trendArrow(curr.total, prev.total) : "");

  const fundingEl = document.getElementById("stat-funding");
  if (fundingEl) fundingEl.innerHTML = curr.funding + (showTrend ? trendArrow(curr.funding, prev.funding) : "");

  const fundedEl = document.getElementById("stat-funded");
  if (fundedEl) fundedEl.innerHTML = curr.funded + (showTrend ? trendArrow(curr.funded, prev.funded) : "");

  const aumEl = document.getElementById("stat-aum");
  if (aumEl) aumEl.innerHTML = formatUSD(curr.aum) + (showTrend ? trendArrow(curr.aum, prev.aum) : "");

  const soldEl = document.getElementById("stat-tokens-sold");
  if (soldEl) {
    const lifetime = allAssets.reduce(
      (s, a) => s + ((a.tokens_total || 0) - (a.tokens_available || 0)),
      0,
    );
    const active = allAssets.reduce(
      (s, a) => TERMINAL_STATUSES.has(a.funding_status)
        ? s
        : s + ((a.tokens_total || 0) - (a.tokens_available || 0)),
      0,
    );
    soldEl.textContent = lifetime.toLocaleString();
    soldEl.title = `Lifetime: ${lifetime.toLocaleString()} (incl. exited)\nActive (locked): ${active.toLocaleString()}`;
    const sub = document.getElementById("stat-tokens-sold-sub");
    if (sub) sub.textContent = `${active.toLocaleString()} active`;
  }

  const featuredCountEl = document.getElementById("featured-count");
  if (featuredCountEl) {
    const fc = allAssets.filter((a) => a.featured).length;
    featuredCountEl.textContent = `(${fc})`;
  }

  computeYieldAnomalies();
  renderActionRequired();

  if (!prev || Date.now() - prev.ts > 24 * 3600 * 1000) writeSnapshot(curr);
}

function renderActionRequired() {
  const banner = document.getElementById("action-required-banner");
  if (!banner) return;
  const stalled = allAssets.filter(
    (a) => a.funding_status === "funding_in_progress" && fundingPct(a) < 50 && daysSince(a.created_at) > 30,
  );
  const slowOpen = allAssets.filter(
    (a) => a.funding_status === "funding_open" && daysSince(a.created_at) > 30 && fundingPct(a) < 10,
  );
  const fundedReady = allAssets.filter((a) => a.funding_status === "funded");
  const payoutPending = allAssets.filter((a) => a.funding_status === "payout_pending");
  const mintPending = allAssets.filter(
    (a) => ["funded", "rented", "payout_pending"].includes(a.funding_status) && !a.chain_tx_hash,
  );
  const settlementsPending = allAssets.filter((a) => (a.pending_settlements || 0) > 0);

  const items = [];
  if (mintPending.length) items.push({ label: "Mint pending (funded, no on-chain tx)", count: mintPending.length, status: "funded", color: "var(--admin-warning)" });
  if (settlementsPending.length) {
    const total = settlementsPending.reduce((s, a) => s + (a.pending_settlements || 0), 0);
    items.push({ label: `Trade settlements pending on-chain (${total})`, count: settlementsPending.length, status: "", color: "var(--admin-danger, #C2410C)" });
  }
  if (fundedReady.length) items.push({ label: "Funded · ready for distribution", count: fundedReady.length, status: "funded", color: "var(--admin-info)" });
  if (payoutPending.length) items.push({ label: "Payout pending", count: payoutPending.length, status: "payout_pending", color: "var(--admin-warning)" });
  if (stalled.length) items.push({ label: "Stalled funding (>30d, <50%)", count: stalled.length, status: "funding_in_progress", color: "var(--admin-danger, #C2410C)" });
  if (slowOpen.length) items.push({ label: "Slow start (>30d, <10%)", count: slowOpen.length, status: "funding_open", color: "var(--admin-warning)" });

  if (items.length === 0) {
    banner.style.display = "none";
    banner.innerHTML = "";
    return;
  }

  banner.style.display = "block";
  banner.innerHTML = `
    <div class="admin-card" style="padding:12px 16px;border-left:3px solid var(--admin-warning);display:flex;align-items:center;gap:14px;flex-wrap:wrap">
      <span style="font-weight:700;font-size:13px;color:var(--admin-text-primary);display:flex;align-items:center;gap:6px">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--admin-warning)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
        Action required
      </span>
      ${items
        .map(
          (i) => `
        <button type="button" class="admin-action-chip" data-status="${i.status}" style="border:1px solid var(--admin-border);background:transparent;border-radius:999px;padding:4px 10px;font-size:12px;cursor:pointer;display:inline-flex;align-items:center;gap:6px">
          <span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:${i.color}"></span>
          <span style="color:var(--admin-text-secondary)">${i.label}</span>
          <strong style="color:var(--admin-text-primary)">${i.count}</strong>
        </button>`,
        )
        .join("")}
    </div>`;
  banner.querySelectorAll(".admin-action-chip").forEach((btn) => {
    btn.addEventListener("click", () => {
      const sel = document.getElementById("filter-status");
      if (sel) {
        sel.value = btn.dataset.status;
        applyFilters();
      }
    });
  });
}

function applyFilters() {
  const search = (
    document.getElementById("asset-search")?.value || ""
  ).toLowerCase();
  const type = document.getElementById("filter-type")?.value || "";
  const status = document.getElementById("filter-status")?.value || "";
  const featured = document.getElementById("filter-featured")?.checked || false;

  const valueMinC = advancedFilters.valueMin != null ? advancedFilters.valueMin * 100 : null;
  const valueMaxC = advancedFilters.valueMax != null ? advancedFilters.valueMax * 100 : null;
  const yieldMinBps = advancedFilters.yieldMin != null ? advancedFilters.yieldMin * 100 : null;
  const yieldMaxBps = advancedFilters.yieldMax != null ? advancedFilters.yieldMax * 100 : null;

  let result = allAssets.filter((a) => {
    if (type && a.asset_type !== type) return false;
    if (status && a.funding_status !== status) return false;
    if (featured && !a.featured) return false;
    if (
      search &&
      !`${a.title} ${a.location_city} ${a.slug}`.toLowerCase().includes(search)
    )
      return false;
    if (valueMinC != null && (a.total_value_cents || 0) < valueMinC) return false;
    if (valueMaxC != null && (a.total_value_cents || 0) > valueMaxC) return false;
    if (yieldMinBps != null && (a.annual_yield_bps || 0) < yieldMinBps) return false;
    if (yieldMaxBps != null && (a.annual_yield_bps || 0) > yieldMaxBps) return false;
    if (advancedFilters.locations.size > 0 && !advancedFilters.locations.has(a.location_city)) return false;
    return true;
  });

  // Sort Result
  result.sort((a, b) => {
    let valA = a[sortField];
    let valB = b[sortField];

    if (sortField === "funding_progress") {
      valA = fundingPct(a);
      valB = fundingPct(b);
    } else if (sortField === "age_days") {
      valA = daysSince(a.created_at);
      valB = daysSince(b.created_at);
    } else if (sortField === "featured") {
      valA = a.featured ? 1 : 0;
      valB = b.featured ? 1 : 0;
    }

    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredAssets = result;
  currentPage = 1;
  document.getElementById("asset-count-label").textContent =
    `${filteredAssets.length} assets`;
  renderTable();
  updateSortHeaderState();
}

function renderTable() {
  const tbody = document.getElementById("assets-table-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(filteredAssets.length / PAGE_SIZE));
  currentPage = Math.min(currentPage, totalPages);
  const start = (currentPage - 1) * PAGE_SIZE;
  const slice = filteredAssets.slice(start, start + PAGE_SIZE);

  if (slice.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="10" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No assets match your filters.</td></tr>';
    updatePaginationState(totalPages);
    syncBulkBar();
    return;
  }

  updatePaginationState(totalPages);

  tbody.innerHTML = slice
    .map((a) => {
      const sold = (a.tokens_total || 0) - (a.tokens_available || 0);
      const pct = fundingPct(a);
      const progressColor = statusColorVar(a.funding_status);
      const age = daysSince(a.created_at);
      const isStalled = a.funding_status === "funding_in_progress" && pct < 50 && age > 30;
      const isSlowOpen = a.funding_status === "funding_open" && age > 30 && pct < 10;
      const ageBadge = isStalled || isSlowOpen
        ? `<span title="${isStalled ? "Stalled — >30d at <50% funded" : "Slow start — >30d at <10% funded"}" style="display:inline-block;margin-left:6px;font-size:10px;font-weight:700;color:var(--admin-danger, #C2410C);background:rgba(194,65,12,0.10);padding:1px 5px;border-radius:3px">${isStalled ? "STALLED" : "SLOW"}</span>`
        : "";
      const ageColor = age > 60 ? "var(--admin-danger, #C2410C)" : age > 30 ? "var(--admin-warning)" : "var(--admin-text-muted)";
      const checked = selectedIds.has(a.id) ? "checked" : "";
      const isExited = a.funding_status === "exited";
      const barInner = isExited
        ? `<div style="width:${pct}%;height:100%;background:repeating-linear-gradient(45deg,${progressColor},${progressColor} 4px,transparent 4px,transparent 7px);opacity:0.6;border-radius:3px"></div>`
        : `<div style="width:${pct}%;height:100%;background:${progressColor};border-radius:3px;transition:width 0.4s;"></div>`;
      const yieldText = a.annual_yield_bps
        ? (a.annual_yield_bps / 100).toFixed(1) + "%"
        : `<span title="Yield TBD or capital-gain only (no recurring yield)" style="cursor:help;color:var(--admin-text-muted)">—</span>`;
      const onChainLink = a.chain_contract_address
        ? `<a href="${EXPLORER_BASE}/token/${esc(a.chain_contract_address)}${a.chain_token_id ? "?a=" + esc(a.chain_token_id) : ""}" target="_blank" rel="noopener noreferrer" class="admin-btn admin-btn--secondary admin-btn--sm" title="View on-chain (Polygonscan)" aria-label="View ${esc(a.title)} on-chain"><svg aria-hidden="true" width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M5 8a3 3 0 003 3h2"/><path d="M11 8a3 3 0 00-3-3H6"/><circle cx="3.5" cy="8" r="1.5"/><circle cx="12.5" cy="8" r="1.5"/></svg></a>`
        : "";
      const rowBorder = (isStalled || isSlowOpen)
        ? "box-shadow: inset 3px 0 0 var(--admin-danger, #C2410C);"
        : "";
      const isYieldAnomaly = yieldAnomalyIds.has(a.id);
      const rowClass = isYieldAnomaly ? ' class="admin-anomaly-yield"' : "";
      const updatedTooltip = a.updated_at ? `Last updated: ${new Date(a.updated_at).toLocaleString()}` : "";

      const minted = !!a.chain_tx_hash && !!a.chain_token_id;
      const isFundedish = ["funded", "rented", "payout_pending", "exited"].includes(a.funding_status);
      let distBadge = "";
      if (minted) {
        distBadge = `<span title="On-chain: token minted (tx ${esc(a.chain_tx_hash)})" style="display:inline-flex;align-items:center;gap:3px;font-size:10px;font-weight:700;color:#065f46;background:#d1fae5;padding:1px 6px;border-radius:3px"><svg width="9" height="9" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 8 7 12 13 4"/></svg>ON-CHAIN</span>`;
      } else if (isFundedish) {
        distBadge = `<span title="Funded but not yet minted on-chain — distribution pending" style="display:inline-flex;align-items:center;font-size:10px;font-weight:700;color:#92400e;background:#fef3c7;padding:1px 6px;border-radius:3px">MINT PENDING</span>`;
      }
      if ((a.pending_settlements || 0) > 0) {
        distBadge += ` <span title="${a.pending_settlements} trade settlement(s) pending on-chain" style="display:inline-flex;align-items:center;font-size:10px;font-weight:700;color:#7f1d1d;background:#fee2e2;padding:1px 6px;border-radius:3px">⏳ ${a.pending_settlements}</span>`;
      }
      const holdersText = (a.holders_count || 0) > 0
        ? `<span title="Distinct holders (from settled trades)" style="font-size:11px;color:var(--admin-text-muted);margin-left:6px">· 👥 ${a.holders_count}</span>`
        : "";

      return `
        <tr${rowClass} style="${rowBorder}" title="${esc(updatedTooltip)}">
            <td style="text-align:center"><input type="checkbox" class="row-select" data-id="${esc(a.id)}" ${checked} aria-label="Select ${esc(a.title)}" style="accent-color:var(--admin-accent);cursor:pointer" /></td>
            <td>
                <div style="font-weight:600;color:var(--admin-text-primary);margin-bottom:2px;display:flex;align-items:center;gap:6px">
                  <button type="button" class="featured-toggle" data-id="${esc(a.id)}" title="${a.featured ? "Unfeature" : "Feature"}" aria-label="${a.featured ? "Unfeature" : "Feature"} ${esc(a.title)}" style="background:none;border:none;padding:0;cursor:pointer;line-height:0;color:${a.featured ? "var(--admin-warning)" : "var(--admin-text-muted)"}">
                    <svg aria-hidden="true" width="14" height="14" viewBox="0 0 20 20" fill="${a.featured ? "currentColor" : "none"}" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M10 2l2.4 4.8L18 7.6l-4 3.9.9 5.5L10 14.6 5.1 17l.9-5.5-4-3.9 5.6-.8L10 2z"/></svg>
                  </button>
                  <a href="/admin/asset-details.html?id=${esc(a.id)}" style="color:inherit;text-decoration:none" onmouseover="this.style.textDecoration='underline'" onmouseout="this.style.textDecoration='none'">${esc(a.title)}</a>
                </div>
                <div style="font-size:11px;color:var(--admin-text-muted);display:flex;align-items:center;gap:6px;flex-wrap:wrap">${esc(a.slug)}${distBadge ? `<span style="display:inline-flex;gap:4px">${distBadge}</span>` : ""}${holdersText}</div>
            </td>
            <td>${typeBadge(a.asset_type)}</td>
            <td style="font-weight:700;font-variant-numeric:tabular-nums;">${formatUSD(a.total_value_cents)}</td>
            <td style="min-width:140px;">
                <div style="display:flex;align-items:center;gap:8px;">
                    <div style="flex:1;height:6px;background:var(--admin-border);border-radius:3px;overflow:hidden;">
                        ${barInner}
                    </div>
                    <span style="font-size:11px;font-weight:600;color:var(--admin-text-secondary);width:36px;text-align:right;">${pct}%</span>
                </div>
                <div style="font-size:11px;color:var(--admin-text-muted);margin-top:2px;">${sold.toLocaleString()} / ${(a.tokens_total || 0).toLocaleString()} tokens</div>
            </td>
            <td style="font-size:12px;color:${ageColor};font-variant-numeric:tabular-nums;white-space:nowrap" title="Published ${esc(a.created_at || "")}">${age}d${ageBadge}</td>
            <td style="font-variant-numeric:tabular-nums;${isYieldAnomaly ? "color:var(--admin-danger, #C2410C);font-weight:700" : ""}" ${isYieldAnomaly ? `title="Yield outlier (>2σ from mean)"` : ""}>${yieldText}${isYieldAnomaly ? " ⚠" : ""}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${esc(a.location_city || "—")}</td>
            <td>${statusBadge(a.funding_status)}</td>
            <td>
                <div style="display:flex;gap:4px;">
                    ${onChainLink}
                    <a href="/admin/asset-details.html?id=${esc(a.id)}" class="admin-btn admin-btn--secondary admin-btn--sm" title="Manage asset" aria-label="Manage ${esc(a.title)}">
                        <svg aria-hidden="true" width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M11.3 2.7l2 2L5 13l-3 .5.5-3z"/></svg>
                    </a>
                    <a href="/property/${esc(a.slug)}" target="_blank" rel="noopener noreferrer" class="admin-btn admin-btn--secondary admin-btn--sm" title="View on marketplace" aria-label="View ${esc(a.title)} on marketplace">
                        <svg aria-hidden="true" width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M6 2H3a1 1 0 00-1 1v10a1 1 0 001 1h10a1 1 0 001-1v-3"/><path d="M9 2h5v5M14 2L7 9"/></svg>
                    </a>
                </div>
            </td>
        </tr>
        `;
    })
    .join("");

  tbody.querySelectorAll(".row-select").forEach((cb) => {
    cb.addEventListener("change", (e) => {
      const id = e.target.dataset.id;
      if (e.target.checked) selectedIds.add(id);
      else selectedIds.delete(id);
      syncBulkBar();
    });
  });
  tbody.querySelectorAll(".featured-toggle").forEach((btn) => {
    btn.addEventListener("click", (e) => toggleFeatured(btn.dataset.id, e));
  });
  syncBulkBar();
}

function syncBulkBar() {
  const bar = document.getElementById("bulk-action-bar");
  const count = document.getElementById("bulk-count");
  const selectAll = document.getElementById("select-all");
  if (count) count.textContent = `${selectedIds.size} selected`;
  if (bar) bar.style.display = selectedIds.size > 0 ? "flex" : "none";
  if (selectAll) {
    const start = (currentPage - 1) * PAGE_SIZE;
    const slice = filteredAssets.slice(start, start + PAGE_SIZE);
    const allSel = slice.length > 0 && slice.every((a) => selectedIds.has(a.id));
    const someSel = slice.some((a) => selectedIds.has(a.id));
    selectAll.checked = allSel;
    selectAll.indeterminate = !allSel && someSel;
  }
}

function updatePaginationState(totalPages) {
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${filteredAssets.length} total)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;
}

function renderErrorState(message) {
  const tbody = document.getElementById("assets-table-body");
  if (!tbody) return;
  tbody.innerHTML = `
    <tr>
      <td colspan="10" style="text-align:center;padding:40px;color:var(--admin-danger, #C2410C);">
        <div style="font-weight:600;margin-bottom:8px;">${esc(message)}</div>
        <button type="button" class="admin-btn admin-btn--secondary admin-btn--sm" onclick="loadAssets()">Retry</button>
      </td>
    </tr>`;
  const count = document.getElementById("asset-count-label");
  if (count) count.textContent = "0 assets";
  currentPage = 1;
  updatePaginationState(1);
}

function showAssetStatus(message, type = "info") {
  let region = document.getElementById("asset-status-message");
  if (!region) {
    const content = document.querySelector(".admin-content");
    region = document.createElement("div");
    region.id = "asset-status-message";
    region.setAttribute("role", type === "error" ? "alert" : "status");
    region.setAttribute("aria-live", type === "error" ? "assertive" : "polite");
    region.style.margin = "0 0 16px";
    region.style.padding = "10px 12px";
    region.style.borderRadius = "8px";
    region.style.fontSize = "13px";
    region.style.fontWeight = "600";
    const filters = document.querySelector(".admin-page-header")?.nextElementSibling;
    content?.insertBefore(region, filters || content.firstChild);
  }

  region.textContent = message;
  region.style.display = "block";
  region.style.color =
    type === "error" ? "var(--admin-danger, #C2410C)" : "var(--admin-text-primary)";
  region.style.border =
    type === "error"
      ? "1px solid rgba(194, 65, 12, 0.25)"
      : "1px solid var(--admin-border)";
  region.style.background =
    type === "error" ? "rgba(254, 242, 242, 0.95)" : "var(--admin-card-bg, #fff)";

  clearTimeout(assetStatusTimer);
  if (type !== "error") {
    assetStatusTimer = setTimeout(() => {
      region.style.display = "none";
    }, 4000);
  }
}

async function toggleFeatured(id, event) {
  const asset = allAssets.find((a) => a.id === id);
  if (!asset) return;
  const button = event?.currentTarget;
  if (button) {
    button.disabled = true;
    button.setAttribute("aria-busy", "true");
  }
  try {
    const resp = await fetch(`/api/admin/assets/${id}/toggle-featured`, {
      method: "POST",
    });
    if (resp.ok) {
      const data = await resp.json().catch(() => ({}));
      showAssetStatus(
        `${asset.title || "Asset"} is ${data.featured ? "now featured" : "no longer featured"}.`,
      );
      await loadAssets();
      return;
    }

    const message = await responseMessage(
      resp,
      "Failed to toggle featured status.",
    );
    showAssetStatus(message, "error");
  } catch (e) {
    console.error("Error toggling featured status", e);
    if (typeof Sentry !== "undefined") Sentry.captureException(e);
    showAssetStatus("Failed to toggle featured status. Please try again.", "error");
  } finally {
    if (button) {
      button.disabled = false;
      button.removeAttribute("aria-busy");
    }
  }
}

// ─── Helpers ────────────────────────────────────────────────────

function esc(s) {
  if (typeof s !== "string") return s || "";
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}
function formatUSD(c) {
  return (
    "$" +
    (Math.abs(c || 0) / 100).toLocaleString("en-US", {
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    })
  );
}
function debounce(fn, ms) {
  let t;
  return function (...a) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, a), ms);
  };
}

function typeBadge(t) {
  const m = {
    real_estate: "Real Estate",
    commercial_property: "Commercial",
    commodity: "Commodity",
    business: "Business",
    startup: "Startup",
    land_plot: "Land",
  };
  return `<span class="admin-badge admin-badge--neutral">${m[t] || t}</span>`;
}

function statusBadge(s) {
  const m = {
    upcoming: ["admin-badge--neutral", "Upcoming"],
    funding_open: ["admin-badge--success", "Funding Open"],
    funding_in_progress: ["admin-badge--info", "In Progress"],
    funded: ["admin-badge--info", "Funded"],
    rented: ["admin-badge--success", "Rented"],
    payout_pending: ["admin-badge--warning", "Payout"],
    exited: ["admin-badge--exited", "Exited"],
  };
  const [cls, label] = m[s] || ["admin-badge--neutral", s];
  return `<span class="admin-badge ${cls}">${label}</span>`;
}
