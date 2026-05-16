// State Management
let allDeposits = [];
let filteredDeposits = [];
let currentPage = 1;
let PAGE_SIZE = 15;
let sortField = "created_at";
let sortOrder = "desc";
let confirmModalReturnFocus = null;
const selectedIds = new Set();
const SNAPSHOT_KEY = "admin_deposits_kpi_snapshot_v1";
const VIEWS_KEY = "admin_deposits_saved_views_v1";
const REFRESH_INTERVAL_MS = 30000;
let refreshTimer = null;
let lastFetchAt = null;
let userActive = false;
let userActiveTimer = null;
let lastStats = null;
let advancedFilters = {
  amountMin: null, amountMax: null,
  dateFrom: null, dateTo: null,
  minAgeHours: null,
};

function ageSeconds(iso) {
  if (!iso) return 0;
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return 0;
  return Math.max(0, Math.floor((Date.now() - t) / 1000));
}
function formatAge(secs) {
  if (secs == null || secs <= 0) return "—";
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}
function ageColor(secs) {
  if (secs > 86400) return "var(--admin-danger, #C2410C)";
  if (secs > 14400) return "var(--admin-warning, #d97706)";
  return "var(--admin-text-muted)";
}
function numOrNull(v) {
  if (v == null || v === "") return null;
  const n = parseFloat(v);
  return Number.isFinite(n) ? n : null;
}

document.addEventListener("DOMContentLoaded", () => {
  loadDeposits();

  // Filters
  document
    .getElementById("deposit-search")
    ?.addEventListener("input", debounce(applyFilters, 250));
  ["filter-status", "filter-currency", "filter-provider"].forEach((id) => {
    document.getElementById(id)?.addEventListener("change", applyFilters);
  });

  // Pagination
  document.getElementById("prev-page")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      renderTable();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    const totalPages = Math.ceil(filteredDeposits.length / PAGE_SIZE);
    if (currentPage < totalPages) {
      currentPage++;
      renderTable();
    }
  });

  // Refresh
  document
    .getElementById("btn-refresh")
    ?.addEventListener("click", loadDeposits);

  // Sorting
  setupSorting();

  // Confirm modal
  document
    .getElementById("confirm-cancel")
    ?.addEventListener("click", closeConfirmModal);
  document.getElementById("confirm-modal")?.addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeConfirmModal();
  });
  document.addEventListener("keydown", (e) => {
    const modal = document.getElementById("confirm-modal");
    if (e.key === "Escape" && modal?.style.display === "flex") {
      closeConfirmModal();
    }
  });

  // M-features wiring
  setupAdvancedFilters();
  setupSavedViews();
  setupAutoRefresh();
  setupBulkActions();
  document.getElementById("export-csv-btn")?.addEventListener("click", () => exportCsv(filteredDeposits, "deposits-filtered.csv"));
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

  // Tabs
  document.querySelectorAll(".admin-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      document
        .querySelectorAll(".admin-tab")
        .forEach((t) => t.classList.remove("active"));
      document
        .querySelectorAll(".admin-tab-panel")
        .forEach((p) => (p.style.display = "none"));
      tab.classList.add("active");
      const panelId = `tab-${tab.dataset.tab}`;
      const panel = document.getElementById(panelId);
      if (panel) panel.style.display = "block";
      if (tab.dataset.tab === "disputes") loadDisputes();
    });
  });
});

function setupAdvancedFilters() {
  document.getElementById("toggle-filters-btn")?.addEventListener("click", (e) => {
    const panel = document.getElementById("advanced-filter-panel");
    const btn = e.currentTarget;
    if (!panel) return;
    const open = panel.classList.toggle("open");
    btn.setAttribute("aria-expanded", open ? "true" : "false");
  });
  ["filter-amount-min", "filter-amount-max", "filter-date-from", "filter-date-to", "filter-min-age"].forEach((id) => {
    document.getElementById(id)?.addEventListener("input", debounce(() => {
      advancedFilters.amountMin = numOrNull(document.getElementById("filter-amount-min")?.value);
      advancedFilters.amountMax = numOrNull(document.getElementById("filter-amount-max")?.value);
      advancedFilters.dateFrom = document.getElementById("filter-date-from")?.value || null;
      advancedFilters.dateTo = document.getElementById("filter-date-to")?.value || null;
      advancedFilters.minAgeHours = numOrNull(document.getElementById("filter-min-age")?.value);
      applyFilters();
    }, 200));
  });
  document.getElementById("filters-clear-btn")?.addEventListener("click", () => {
    ["filter-amount-min", "filter-amount-max", "filter-date-from", "filter-date-to", "filter-min-age"].forEach((id) => {
      const el = document.getElementById(id);
      if (el) el.value = "";
    });
    advancedFilters = { amountMin: null, amountMax: null, dateFrom: null, dateTo: null, minAgeHours: null };
    applyFilters();
  });
}

function setupSavedViews() {
  refreshSavedViewsDropdown();
  document.getElementById("saved-views")?.addEventListener("change", (e) => {
    const name = e.target.value;
    if (!name) return;
    if (name === "__delete__") {
      const target = window.prompt("Delete which view? Type exact name:");
      const views = readSavedViews();
      if (target && views[target]) {
        delete views[target];
        writeSavedViews(views);
        refreshSavedViewsDropdown();
        showToast(`View "${target}" deleted.`, "success");
      } else {
        e.target.value = "";
      }
      return;
    }
    const views = readSavedViews();
    if (views[name]) applyView(views[name]);
  });
  document.getElementById("save-view-btn")?.addEventListener("click", () => {
    const name = window.prompt("Name this view:");
    if (!name) return;
    const views = readSavedViews();
    views[name] = captureView();
    writeSavedViews(views);
    refreshSavedViewsDropdown(name);
    showToast(`View "${name}" saved.`, "success");
  });
}
function readSavedViews() { try { return JSON.parse(localStorage.getItem(VIEWS_KEY) || "{}"); } catch { return {}; } }
function writeSavedViews(v) { try { localStorage.setItem(VIEWS_KEY, JSON.stringify(v)); } catch {} }
function refreshSavedViewsDropdown(selected) {
  const sel = document.getElementById("saved-views");
  if (!sel) return;
  const views = readSavedViews();
  const names = Object.keys(views).sort();
  sel.innerHTML = `<option value="">— Saved views —</option>` +
    names.map((n) => `<option value="${esc(n)}"${n === selected ? " selected" : ""}>${esc(n)}</option>`).join("") +
    (names.length ? `<option disabled>──────</option><option value="__delete__">Delete view…</option>` : "");
}
function captureView() {
  return {
    search: document.getElementById("deposit-search")?.value || "",
    status: document.getElementById("filter-status")?.value || "",
    currency: document.getElementById("filter-currency")?.value || "",
    provider: document.getElementById("filter-provider")?.value || "",
    ...advancedFilters,
    sortField, sortOrder,
  };
}
function applyView(v) {
  const set = (id, val) => { const el = document.getElementById(id); if (el) el.value = val ?? ""; };
  set("deposit-search", v.search);
  set("filter-status", v.status);
  set("filter-currency", v.currency);
  set("filter-provider", v.provider);
  set("filter-amount-min", v.amountMin ?? "");
  set("filter-amount-max", v.amountMax ?? "");
  set("filter-date-from", v.dateFrom ?? "");
  set("filter-date-to", v.dateTo ?? "");
  set("filter-min-age", v.minAgeHours ?? "");
  advancedFilters.amountMin = v.amountMin ?? null;
  advancedFilters.amountMax = v.amountMax ?? null;
  advancedFilters.dateFrom = v.dateFrom ?? null;
  advancedFilters.dateTo = v.dateTo ?? null;
  advancedFilters.minAgeHours = v.minAgeHours ?? null;
  if (v.sortField) sortField = v.sortField;
  if (v.sortOrder) sortOrder = v.sortOrder;
  applyFilters();
}

function setupAutoRefresh() {
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
    if (document.getElementById("confirm-modal")?.style.display === "flex") return;
    loadDeposits({ silent: true });
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
    const slice = filteredDeposits.slice(start, start + PAGE_SIZE);
    if (e.target.checked) slice.forEach((d) => selectedIds.add(d.id));
    else slice.forEach((d) => selectedIds.delete(d.id));
    renderTable();
  });
  document.getElementById("bulk-clear")?.addEventListener("click", () => {
    selectedIds.clear();
    renderTable();
  });
  document.getElementById("bulk-confirm")?.addEventListener("click", () => bulkAction("confirm", "POST", "Confirmed"));
  document.getElementById("bulk-extend")?.addEventListener("click", () => bulkAction("extend", "POST", "Extended"));
  document.getElementById("bulk-cancel")?.addEventListener("click", async () => {
    if (!await confirmAction({ title: "Cancel selected deposits", message: `Cancel ${selectedIds.size} pending deposit(s)?`, confirmText: "Cancel deposits", type: "danger" })) return;
    bulkAction("cancel", "POST", "Cancelled");
  });
  document.getElementById("bulk-export")?.addEventListener("click", () => {
    const list = allDeposits.filter((d) => selectedIds.has(d.id));
    exportCsv(list, "deposits-selected.csv");
  });
}

async function bulkAction(endpoint, method, verbPast) {
  const ids = [...selectedIds];
  if (ids.length === 0) return;
  const body = endpoint === "cancel"
    ? { reason: "Bulk admin cancel" }
    : endpoint === "confirm"
      ? { notes: "Bulk admin confirm" }
      : null;
  showToast(`Updating ${ids.length} deposit(s)…`, "info");
  const results = await Promise.allSettled(
    ids.map((id) => fetch(`/api/admin/deposits/${id}/${endpoint}`, {
      method,
      headers: { "Content-Type": "application/json" },
      body: body ? JSON.stringify(body) : undefined,
    })),
  );
  const failed = results.filter((r) => r.status === "rejected" || (r.value && !r.value.ok)).length;
  if (failed > 0) showToast(`${failed} update(s) failed.`, "danger");
  else showToast(`${ids.length} deposit(s) ${verbPast.toLowerCase()}.`, "success");
  selectedIds.clear();
  await loadDeposits();
}

function syncBulkBar() {
  const bar = document.getElementById("bulk-action-bar");
  const count = document.getElementById("bulk-count");
  const selectAll = document.getElementById("select-all");
  if (count) count.textContent = `${selectedIds.size} selected`;
  if (bar) bar.style.display = selectedIds.size > 0 ? "flex" : "none";
  if (selectAll) {
    const start = (currentPage - 1) * PAGE_SIZE;
    const slice = filteredDeposits.slice(start, start + PAGE_SIZE);
    const allSel = slice.length > 0 && slice.every((d) => selectedIds.has(d.id));
    const someSel = slice.some((d) => selectedIds.has(d.id));
    selectAll.checked = allSel;
    selectAll.indeterminate = !allSel && someSel;
  }
}

function exportCsv(rows, filename) {
  if (!rows || rows.length === 0) {
    showToast("Nothing to export.", "danger");
    return;
  }
  const cols = [
    ["created_at", "Created"],
    ["user_name", "User"],
    ["user_email", "Email"],
    ["amount_cents", "Amount (major)"],
    ["currency", "Currency"],
    ["provider", "Provider"],
    ["external_ref_id", "Reference"],
    ["status", "Status"],
    ["age_seconds", "Age (sec)"],
    ["expires_at", "Expires"],
    ["updated_at", "Updated"],
    ["id", "Deposit ID"],
    ["user_id", "User ID"],
  ];
  const escCsv = (v) => {
    if (v == null) return "";
    const s = String(v);
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const lines = [cols.map(([, h]) => h).join(",")];
  rows.forEach((d) => {
    const enriched = {
      ...d,
      "amount_cents": ((d.amount_cents || 0) / 100).toFixed(2),
      "age_seconds": ageSeconds(d.created_at),
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
  const table = document.getElementById("deposits-table");
  if (!table) return;
  table.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    th.setAttribute("role", "button");
    th.setAttribute("tabindex", "0");
    th.setAttribute("aria-sort", getAriaSortValue(th.dataset.sort));
    th.addEventListener("click", () => {
      const field = th.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      applyFilters();
      updateSortIndicators();
    });
    th.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        th.click();
      }
    });
  });
}

function updateSortIndicators() {
  document.querySelectorAll("#deposits-table th[data-sort]").forEach((th) => {
    th.setAttribute("aria-sort", getAriaSortValue(th.dataset.sort));
  });
}

function getAriaSortValue(field) {
  if (field !== sortField) return "none";
  return sortOrder === "asc" ? "ascending" : "descending";
}

async function loadDeposits(opts = {}) {
  const silent = !!opts.silent;
  const btn = document.getElementById("btn-refresh");
  if (btn && !silent) btn.classList.add("admin-btn--loading");

  try {
    const resp = await fetch("/api/admin/deposits");
    if (resp.ok) {
      const data = await resp.json();
      allDeposits = data.deposits || data;
      lastFetchAt = Date.now();
      lastStats = data.stats || null;
      updateLastUpdatedLabel();
      applyFilters();
      updateStats(data.stats);
      renderActionRequired();
      if (btn) btn.classList.remove("admin-btn--loading");
    } else {
      if (!silent) {
        allDeposits = [];
        filteredDeposits = [];
        updateStats();
        renderDepositError("Could not load deposit requests.");
      }
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Could not load deposit requests.", "danger");
      if (btn) btn.classList.remove("admin-btn--loading");
    }
  } catch (e) {
    if (!silent) {
      allDeposits = [];
      filteredDeposits = [];
      updateStats();
      renderDepositError("Network error loading deposit requests.");
    }
    showToast("Network error loading deposit requests.", "danger");
    if (btn) btn.classList.remove("admin-btn--loading");
  }
}

function renderActionRequired() {
  const banner = document.getElementById("action-required-banner");
  if (!banner) return;
  const overSla = allDeposits.filter((d) => d.status === "pending" && ageSeconds(d.created_at) > 86400);
  const expiringSoon = allDeposits.filter((d) => {
    if (d.status !== "pending" || !d.expires_at) return false;
    const sec = (Date.parse(d.expires_at) - Date.now()) / 1000;
    return sec > 0 && sec < 7200;
  });
  const expiredUnconfirmed = allDeposits.filter((d) => d.status === "expired");
  const failed = allDeposits.filter((d) => d.status === "failed");

  const items = [];
  if (overSla.length) items.push({ label: "Pending >24h (SLA breach)", count: overSla.length, status: "pending", color: "var(--admin-danger, #C2410C)" });
  if (expiringSoon.length) items.push({ label: "Expiring in <2h", count: expiringSoon.length, status: "pending", color: "var(--admin-warning)" });
  if (failed.length) items.push({ label: "Failed deposits", count: failed.length, status: "failed", color: "var(--admin-danger, #C2410C)" });
  if (expiredUnconfirmed.length) items.push({ label: "Expired unconfirmed", count: expiredUnconfirmed.length, status: "expired", color: "var(--admin-text-muted)" });

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
      ${items.map((i) => `
        <button type="button" class="admin-action-chip" data-status="${i.status}">
          <span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:${i.color}"></span>
          <span style="color:var(--admin-text-secondary)">${i.label}</span>
          <strong style="color:var(--admin-text-primary)">${i.count}</strong>
        </button>`).join("")}
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
function readSnapshot() { try { return JSON.parse(localStorage.getItem(SNAPSHOT_KEY) || "null"); } catch { return null; } }
function writeSnapshot(s) { try { localStorage.setItem(SNAPSHOT_KEY, JSON.stringify(s)); } catch {} }
function trendArrow(curr, prev) {
  if (prev == null || curr == null) return "";
  const delta = curr - prev;
  if (delta === 0) return `<span class="admin-trend-flat" title="No change vs last snapshot">→ 0</span>`;
  const cls = delta > 0 ? "admin-trend-up" : "admin-trend-down";
  const arrow = delta > 0 ? "▲" : "▼";
  return `<span class="${cls}" title="vs snapshot">${arrow} ${Math.abs(delta).toLocaleString()}</span>`;
}

function updateStats(stats) {
  if (!stats) {
    stats = {
      pending_count: allDeposits.filter((d) => d.status === "pending").length,
      confirmed_24h: allDeposits.filter((d) => d.status === "paid").length,
      expired_count: allDeposits.filter((d) => d.status === "expired").length,
      pending_value_cents: allDeposits
        .filter((d) => d.status === "pending")
        .reduce((s, d) => s + (d.amount_cents || 0), 0),
      confirmed_24h_value_cents: allDeposits
        .filter((d) => d.status === "paid")
        .reduce((s, d) => s + (d.amount_cents || 0), 0),
      volume_30d_cents: allDeposits
        .filter((d) => d.status === "paid")
        .reduce((s, d) => s + (d.amount_cents || 0), 0),
      volume_30d_count: allDeposits.filter((d) => d.status === "paid").length,
      oldest_pending_age_seconds: Math.max(0, ...allDeposits
        .filter((d) => d.status === "pending")
        .map((d) => ageSeconds(d.created_at))),
    };
  }

  const prev = readSnapshot();
  const curr = {
    ts: Date.now(),
    pending: stats.pending_count ?? 0,
    confirmed: stats.confirmed_24h ?? 0,
    expired: stats.expired_count ?? 0,
    volume: stats.volume_30d_cents ?? 0,
  };
  const showTrend = prev && Date.now() - prev.ts > 6 * 3600 * 1000;

  const pendingEl = document.getElementById("stat-pending");
  const pendingSubEl = document.getElementById("stat-pending-value");
  if (pendingEl) pendingEl.innerHTML = (stats.pending_count ?? 0) + (showTrend ? trendArrow(curr.pending, prev.pending) : "");
  if (pendingSubEl)
    pendingSubEl.textContent = stats.pending_value_cents
      ? `${formatUSD(stats.pending_value_cents)} awaiting`
      : "Awaiting confirmation";

  const oldestEl = document.getElementById("stat-oldest-age");
  const oldestSubEl = document.getElementById("stat-oldest-sub");
  const oldestSec = stats.oldest_pending_age_seconds || 0;
  if (oldestEl) {
    if (oldestSec <= 0) {
      oldestEl.innerHTML = `<span style="color:var(--admin-success);">✓</span>`;
      if (oldestSubEl) oldestSubEl.textContent = "No pending";
    } else {
      const breach = oldestSec > 86400;
      oldestEl.innerHTML = `<span style="color:${breach ? 'var(--admin-danger, #C2410C)' : oldestSec > 14400 ? 'var(--admin-warning)' : 'var(--admin-text-primary)'}">${formatAge(oldestSec)}</span>`;
      if (oldestSubEl) oldestSubEl.textContent = breach ? "⚠ SLA breach" : oldestSec > 14400 ? "Approaching SLA" : "Within SLA";
    }
  }

  const confirmedEl = document.getElementById("stat-confirmed");
  const confirmedSubEl = document.getElementById("stat-confirmed-value");
  if (confirmedEl) confirmedEl.innerHTML = (stats.confirmed_24h ?? 0) + (showTrend ? trendArrow(curr.confirmed, prev.confirmed) : "");
  if (confirmedSubEl)
    confirmedSubEl.textContent = `${formatUSD(stats.confirmed_24h_value_cents || 0)} total`;

  const expiredEl = document.getElementById("stat-expired");
  if (expiredEl) expiredEl.innerHTML = (stats.expired_count ?? 0) + (showTrend ? trendArrow(curr.expired, prev.expired) : "");

  const volumeEl = document.getElementById("stat-volume");
  const volumeCountEl = document.getElementById("stat-volume-count");
  if (volumeEl) volumeEl.innerHTML = formatUSD(stats.volume_30d_cents || 0) + (showTrend ? trendArrow(curr.volume, prev.volume) : "");
  if (volumeCountEl)
    volumeCountEl.textContent = `${stats.volume_30d_count || 0} deposits`;

  if (!prev || Date.now() - prev.ts > 24 * 3600 * 1000) writeSnapshot(curr);
}

function applyFilters() {
  const search = (
    document.getElementById("deposit-search")?.value || ""
  ).toLowerCase();
  const status = document.getElementById("filter-status")?.value || "";
  const currency = document.getElementById("filter-currency")?.value || "";
  const provider = document.getElementById("filter-provider")?.value || "";

  const amtMinC = advancedFilters.amountMin != null ? advancedFilters.amountMin * 100 : null;
  const amtMaxC = advancedFilters.amountMax != null ? advancedFilters.amountMax * 100 : null;
  const dateFromMs = advancedFilters.dateFrom ? Date.parse(advancedFilters.dateFrom + "T00:00:00") : null;
  const dateToMs = advancedFilters.dateTo ? Date.parse(advancedFilters.dateTo + "T23:59:59") : null;
  const minAgeSecs = advancedFilters.minAgeHours != null ? advancedFilters.minAgeHours * 3600 : null;

  let result = allDeposits.filter((d) => {
    if (status && d.status !== status) return false;
    if (currency && d.currency !== currency) return false;
    if (provider && d.provider !== provider) return false;
    if (search) {
      const hay =
        `${d.user_name} ${d.user_email} ${d.external_ref_id || ""}`.toLowerCase();
      if (!hay.includes(search)) return false;
    }
    if (amtMinC != null && (d.amount_cents || 0) < amtMinC) return false;
    if (amtMaxC != null && (d.amount_cents || 0) > amtMaxC) return false;
    if (dateFromMs != null && Date.parse(d.created_at) < dateFromMs) return false;
    if (dateToMs != null && Date.parse(d.created_at) > dateToMs) return false;
    if (minAgeSecs != null && ageSeconds(d.created_at) < minAgeSecs) return false;
    return true;
  });

  // Sort Result
  result.sort((a, b) => {
    let valA = a[sortField];
    let valB = b[sortField];
    if (sortField === "age_seconds") {
      valA = ageSeconds(a.created_at);
      valB = ageSeconds(b.created_at);
    }
    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredDeposits = result;
  currentPage = 1;
  const depositCountEl = document.getElementById("deposit-count-label");
  if (depositCountEl)
    depositCountEl.textContent = `Showing ${filteredDeposits.length} deposits`;
  renderTable();
}

// ─── Render ─────────────────────────────────────────────────────

function renderTable() {
  const tbody = document.getElementById("deposits-table-body");
  if (!tbody) return;

  const totalPages = Math.max(
    1,
    Math.ceil(filteredDeposits.length / PAGE_SIZE),
  );
  currentPage = Math.min(currentPage, totalPages);
  const start = (currentPage - 1) * PAGE_SIZE;
  const slice = filteredDeposits.slice(start, start + PAGE_SIZE);

  if (slice.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="11" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No deposits match your filters.</td></tr>';
    updatePagination(totalPages);
    syncBulkBar();
    return;
  }

  updatePagination(totalPages);

  // Anomaly: amount > 5x median of confirmed deposits
  const confirmedAmounts = allDeposits.filter((d) => d.status === "paid").map((d) => d.amount_cents || 0).sort((a, b) => a - b);
  const median = confirmedAmounts.length ? confirmedAmounts[Math.floor(confirmedAmounts.length / 2)] : 0;
  const anomalyThreshold = median > 0 ? median * 5 : null;

  tbody.innerHTML = slice
    .map((d) => {
      const age = ageSeconds(d.created_at);
      const overSla = d.status === "pending" && age > 86400;
      const isAnomaly = anomalyThreshold && (d.amount_cents || 0) > anomalyThreshold;
      const checked = selectedIds.has(d.id) ? "checked" : "";
      const rowBorder = overSla ? "box-shadow: inset 3px 0 0 var(--admin-danger, #C2410C);" : "";
      const rowClass = isAnomaly ? ' class="admin-anomaly-row"' : "";
      const updatedTooltip = d.updated_at ? `Last updated: ${new Date(d.updated_at).toLocaleString()}` : "";
      return `
            <tr${rowClass} data-id="${esc(d.id)}" style="${rowBorder}" title="${esc(updatedTooltip)}">
                <td style="text-align:center"><input type="checkbox" class="row-select" data-id="${esc(d.id)}" ${checked} aria-label="Select deposit" style="accent-color:var(--admin-accent);cursor:pointer" /></td>
                <td>
                    <div class="admin-user-inline">
                        <div>
                            <div class="admin-user-inline-name">${d.user_id ? `<a href="/admin/user-details.html?id=${esc(d.user_id)}" style="color:inherit;text-decoration:none" onmouseover="this.style.textDecoration='underline'" onmouseout="this.style.textDecoration='none'">${esc(d.user_name)}</a>` : esc(d.user_name)}</div>
                            <div class="admin-user-inline-email">${esc(d.user_email)}</div>
                        </div>
                    </div>
                </td>
                <td style="font-weight:700;font-variant-numeric:tabular-nums;font-size:14px;${isAnomaly ? "color:var(--admin-danger, #C2410C);" : ""}" ${isAnomaly ? `title="Amount &gt;5× median confirmed deposit"` : ""}>${formatAmount(d.amount_cents, d.currency)}${isAnomaly ? " ⚠" : ""}</td>
                <td><span class="admin-badge admin-badge--neutral">${esc(d.currency)}</span></td>
                <td>${getProviderBadge(d.provider)}</td>
                <td style="font-family:monospace;font-size:11px;color:var(--admin-text-muted);max-width:120px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(d.external_ref_id || "—")}</td>
                <td>${getStatusBadge(d.status)}</td>
                <td style="font-size:12px;color:${ageColor(age)};white-space:nowrap;font-variant-numeric:tabular-nums" title="Created ${esc(d.created_at || "")}">${formatAge(age)}${overSla ? ' <span style="font-size:10px;font-weight:700;color:var(--admin-danger, #C2410C);background:rgba(194,65,12,0.10);padding:1px 5px;border-radius:3px;margin-left:4px">SLA</span>' : ""}</td>
                <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${d.expires_at ? formatDateTime(d.expires_at) : "—"}</td>
                <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDateTime(d.created_at)}</td>
                <td>
                    <div style="display:flex;gap:4px;flex-wrap:wrap;">
                        ${d.has_proof ? `
                            <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="viewDepositProof('${esc(d.id)}')" title="View proof of transfer" aria-label="View proof of transfer" style="color:var(--admin-accent);">
                                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M1 8s2.5-5 7-5 7 5 7 5-2.5 5-7 5-7-5-7-5z"/><circle cx="8" cy="8" r="2"/></svg>
                                Proof
                            </button>
                        ` : ""}
                        ${d.status === "pending"
          ? `
                            <button class="admin-btn admin-btn--primary admin-btn--sm" onclick="openConfirmModal('${esc(d.id)}')" title="Confirm deposit" aria-label="Confirm deposit">
                                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 8l3.5 3.5L13 4"/></svg>
                                Confirm
                            </button>
                            <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="extendDeposit('${esc(d.id)}')" title="Extend expiry by 48 hours" aria-label="Extend expiry" style="color:var(--admin-info);">
                                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><circle cx="8" cy="8" r="6.5"/><path d="M8 4.5v4l2.5 1.5"/></svg>
                            </button>
                            <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="cancelDeposit('${esc(d.id)}')" title="Cancel deposit" aria-label="Cancel deposit" style="color:var(--admin-danger);">
                                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg>
                            </button>
                            `
          : (d.has_proof ? "" : `<span style="font-size:12px;color:var(--admin-text-muted);">—</span>`)
        }
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
  syncBulkBar();
}

function updatePagination(totalPages) {
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${filteredDeposits.length} total)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;
}

function renderDepositError(message) {
  const tbody = document.getElementById("deposits-table-body");
  if (!tbody) return;
  const depositCountEl = document.getElementById("deposit-count-label");
  if (depositCountEl) depositCountEl.textContent = "Showing 0 deposits";
  tbody.innerHTML = `<tr><td colspan="11" role="alert" style="text-align:center;padding:40px;color:var(--admin-danger);">${esc(message)}</td></tr>`;
  updatePagination(1);
}

// ─── Confirm Modal ──────────────────────────────────────────────

let confirmingDepositId = null;

function openConfirmModal(depositId) {
  confirmingDepositId = depositId;
  confirmModalReturnFocus = document.activeElement;
  const dep = allDeposits.find((d) => d.id === depositId);
  if (!dep) return;
  document.getElementById("confirm-modal-text").textContent =
    `Confirm deposit of ${formatAmount(dep.amount_cents, dep.currency)} from ${dep.user_name} (${dep.user_email})? This will credit their wallet.`;
  document.getElementById("confirm-notes").value = "";
  const modal = document.getElementById("confirm-modal");
  modal.style.display = "flex";
  document.getElementById("confirm-notes")?.focus();

  document.getElementById("confirm-submit").onclick = () => {
    confirmDeposit(confirmingDepositId);
    closeConfirmModal();
  };
}

function closeConfirmModal() {
  document.getElementById("confirm-modal").style.display = "none";
  confirmingDepositId = null;
  if (confirmModalReturnFocus && typeof confirmModalReturnFocus.focus === "function") {
    confirmModalReturnFocus.focus();
  }
  confirmModalReturnFocus = null;
}

async function confirmDeposit(depositId) {
  const notes = document.getElementById("confirm-notes")?.value || "";
  try {
    const resp = await fetch(`/api/admin/deposits/${depositId}/confirm`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ notes }),
    });
    if (resp.ok) {
      loadDeposits();
      showToast("Deposit successfully confirmed!", "success");
      return;
    } else {
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Failed to confirm deposit.", "danger");
    }
  } catch (e) {
    showToast("Network error confirming deposit.", "danger");
  }
}

async function cancelDeposit(depositId) {
  const dep = allDeposits.find((d) => d.id === depositId);
  const label = dep
    ? `${dep.user_name} (${formatAmount(dep.amount_cents, dep.currency)})`
    : depositId;
  const confirmed = await confirmAction({
    title: "Cancel deposit",
    message: `Cancel the pending deposit from ${label}? This does not credit the wallet.`,
    confirmText: "Cancel deposit",
    type: "danger",
  });
  if (!confirmed) return;

  try {
    const resp = await fetch(`/api/admin/deposits/${depositId}/cancel`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ reason: "Admin cancelled from deposits board" }),
    });

    if (resp.ok) {
      loadDeposits();
      showToast("Deposit request cancelled.", "success");
    } else {
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Failed to cancel deposit.", "danger");
    }
  } catch (e) {
    showToast("Network error canceling deposit.", "danger");
  }
}

async function extendDeposit(depositId) {
  const dep = allDeposits.find((d) => d.id === depositId);
  const label = dep
    ? `${dep.user_name} (${formatAmount(dep.amount_cents, dep.currency)})`
    : depositId;
  if (!await confirmAction({ title: 'Extend deposit expiry', message: `Extend expiry by 48 hours for deposit from ${label}?`, confirmText: 'Extend', type: 'warning' })) return;
  try {
    const resp = await fetch(`/api/admin/deposits/${depositId}/extend`, {
      method: "POST",
    });
    if (resp.ok) {
      loadDeposits();
      showToast("Deposit expiry extended by 48 hours.", "success");
    } else {
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Failed to extend deposit.", "danger");
    }
  } catch (e) {
    showToast("Network error extending deposit.", "danger");
  }
}

async function loadDisputes() {
  const tbody = document.getElementById("disputes-table-body");
  if (!tbody) return;
  tbody.innerHTML =
    '<tr><td colspan="7" style="text-align:center;padding:40px;color:var(--admin-text-muted);">Loading disputes…</td></tr>';
  try {
    const resp = await fetch("/api/admin/disputes");
    if (resp.ok) {
      const data = await resp.json();
      const disputes = Array.isArray(data) ? data : data.disputes || [];
      const countEl = document.getElementById("dispute-count");
      if (countEl) countEl.textContent = `${disputes.length} active`;
      if (disputes.length === 0) {
        tbody.innerHTML =
          '<tr><td colspan="7" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No active disputes found.</td></tr>';
      } else {
        tbody.innerHTML = disputes
          .map(
            (d) => `
                    <tr>
	                        <td>
	                            <div class="admin-user-inline">
	                                <div>
	                                    <div class="admin-user-inline-name">${esc(d.user_email)}</div>
	                                </div>
	                            </div>
	                        </td>
	                        <td>${getProviderBadge(d.provider)}</td>
	                        <td style="font-weight:700;">${formatAmount(d.amount_cents, d.currency)}</td>
	                        <td>${getStatusBadge(d.status)}</td>
	                        <td style="font-size:12px;color:var(--admin-text-muted);">${formatDateTime(d.created_at)}</td>
	                        <td>
	                            ${renderEvidenceBundleControl(d)}
	                        </td>
	                        <td style="display:flex;gap:6px;align-items:center;">
	                            ${renderDisputeStatusSelect(d)}
	                            <button class="admin-btn admin-btn--primary admin-btn--sm" onclick="resolveDispute('${esc(d.id)}')">Save</button>
	                        </td>
	                    </tr>
	                `,
          )
          .join("");
      }
    } else {
        const err = await resp.json().catch(() => ({}));
        tbody.innerHTML = `<tr><td colspan="7" role="alert" style="text-align:center;padding:40px;color:var(--admin-danger);">${esc(err.error || "Could not load disputes.")}</td></tr>`;
        showToast(err.error || "Could not load disputes.", "danger");
    }
  } catch (e) {
    console.error("Error loading disputes", e);
    if (typeof Sentry !== 'undefined') Sentry.captureException(e);
    tbody.innerHTML =
      '<tr><td colspan="7" role="alert" style="text-align:center;padding:40px;color:var(--admin-danger);">Network error loading disputes.</td></tr>';
    showToast("Network error loading disputes.", "danger");
  }
}

function renderDisputeStatusSelect(dispute) {
  const id = esc(dispute.id);
  const current = dispute.status || "under_review";
  const statuses = ["under_review", "resolved", "escalated", "won", "lost"];
  return `<select id="dispute-status-${id}" class="admin-select" aria-label="Update dispute status">${statuses
    .map((status) => `<option value="${esc(status)}"${status === current ? " selected" : ""}>${esc(status.replace("_", " "))}</option>`)
    .join("")}</select>`;
}

function renderEvidenceBundleControl(dispute) {
  const evidenceUrl = dispute.evidence_url || "";
  if (isSafeEvidenceUrl(evidenceUrl)) {
    return `<a href="${esc(evidenceUrl)}" target="_blank" rel="noopener" class="admin-btn admin-btn--secondary admin-btn--sm">View Bundle</a>`;
  }
  return `<button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="buildEvidenceBundle('${esc(dispute.id)}')">Build Bundle</button>`;
}

function isSafeEvidenceUrl(url) {
  return url.startsWith("/api/admin/disputes/") || url.startsWith("https://") || url.startsWith("http://");
}

async function buildEvidenceBundle(disputeId) {
  try {
    const resp = await fetch(`/api/admin/disputes/${disputeId}/evidence`, {
      method: "POST",
      headers: { "X-CSRF-Token": getCsrfToken() },
    });
    if (resp.ok) {
      showToast("Evidence bundle generated.", "success");
      loadDisputes();
    } else {
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Failed to generate evidence bundle.", "danger");
    }
  } catch (e) {
    showToast("Network error generating evidence bundle.", "danger");
  }
}

async function resolveDispute(disputeId) {
  const newStatus = document.getElementById(`dispute-status-${disputeId}`)?.value;
  if (!newStatus) return;
  try {
    const resp = await fetch(`/api/admin/disputes/${disputeId}/status`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status: newStatus }),
    });
    if (resp.ok) {
      showToast("Dispute status updated.", "success");
      loadDisputes();
    } else {
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Failed to update dispute status.", "danger");
    }
  } catch (e) {
    showToast("Update failed.", "danger");
  }
}

function showToast(msg, type = "success") {
  if(window.showPooolToast) {
    window.showPooolToast(null, msg, type);
  }
}

async function confirmAction(options) {
  if (typeof window.pooolConfirm === "function") return window.pooolConfirm(options);
  return window.confirm(options.message || options.title || "Continue?");
}

// ─── Helpers ────────────────────────────────────────────────────

function esc(str) {
  if (typeof str !== "string") return str || "";
  const d = document.createElement("div");
  d.textContent = str;
  return d.innerHTML;
}

function formatUSD(cents) {
  if (typeof cents !== "number") return "$0.00";
  return (
    "$" +
    (Math.abs(cents) / 100).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
}

function formatAmount(cents, currency) {
  const abs = Math.abs(cents || 0);
  if (currency === "IDR") {
    return "Rp " + Math.round(abs / 100).toLocaleString("id-ID");
  }
  return (
    "$" +
    (abs / 100).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
}

function formatDateTime(iso) {
  if (!iso) return "—";
  const d = new Date(iso);
  return (
    d.toLocaleDateString("en-US", { month: "short", day: "numeric" }) +
    " " +
    d.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    })
  );
}

function getStatusBadge(status) {
  const map = {
    pending: ["admin-badge--warning", "Pending"],
    processing: ["admin-badge--info", "Processing"],
    completed: ["admin-badge--success", "Confirmed"],
    paid: ["admin-badge--success", "Confirmed"],
    failed: ["admin-badge--danger", "Failed"],
    cancelled: ["admin-badge--danger", "Cancelled"],
    expired: ["admin-badge--danger", "Expired"],
  };
  const [cls, label] = map[status] || ["admin-badge--neutral", status];
  return `<span class="admin-badge ${cls}"><span class="admin-badge-dot"></span>${esc(label)}</span>`;
}

function getProviderBadge(provider) {
  const map = {
    stripe: ["admin-badge--info", "Stripe"],
    ocbc: ["admin-badge--warning", "OCBC"],
    midtrans: ["admin-badge--warning", "Midtrans"],
    mangopay: ["admin-badge--info", "Mangopay"],
    manual: ["admin-badge--neutral", "Manual"],
  };
  const [cls, label] = map[provider] || ["admin-badge--neutral", provider];
  return `<span class="admin-badge ${cls}"><span class="admin-badge-dot"></span>${esc(label)}</span>`;
}

function debounce(fn, ms) {
  let t;
  return function (...args) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, args), ms);
  };
}

// ─── Proof Viewer ───────────────────────────────────────────────
//
// Fetches a 15-minute signed URL for the deposit's proof-of-transfer file
// and shows it in a lightweight modal. Images render inline; PDFs render
// in an iframe. Falls back to opening the URL in a new tab when the
// browser blocks inline preview.
async function viewDepositProof(depositId) {
  try {
    const resp = await fetch(
      `/api/admin/deposits/${encodeURIComponent(depositId)}/proof-url`,
      { credentials: "same-origin" },
    );
    if (!resp.ok) {
      const err = await resp.json().catch(() => ({}));
      alert(err.error || `Could not load proof (HTTP ${resp.status})`);
      return;
    }
    const data = await resp.json();
    const url = data.signed_url;
    if (!url) {
      alert("No proof URL returned");
      return;
    }

    // Build a simple overlay modal. Reuses admin-* styles for consistency.
    let overlay = document.getElementById("deposit-proof-overlay");
    if (overlay) overlay.remove();
    overlay = document.createElement("div");
    overlay.id = "deposit-proof-overlay";
    overlay.style.cssText = "position:fixed;inset:0;background:rgba(15,23,42,0.65);z-index:9999;display:flex;align-items:center;justify-content:center;padding:32px;";

    const dep = allDeposits.find((d) => d.id === depositId) || {};
    const uploaded = data.uploaded_at
      ? new Date(data.uploaded_at).toLocaleString()
      : "—";
    const userNotesHtml = data.user_notes
      ? `<div style="padding:10px 14px;background:#FFFBEB;border:1px solid #FED7AA;border-radius:6px;font-size:12px;color:#92400E;margin-bottom:12px;"><strong>User notes:</strong> ${esc(data.user_notes)}</div>`
      : "";
    const isPdf = /\.pdf(?:\?|$)/i.test(url) || url.includes("application%2Fpdf");

    overlay.innerHTML = `
      <div style="background:#fff;border-radius:12px;max-width:900px;width:100%;max-height:90vh;display:flex;flex-direction:column;overflow:hidden;">
        <div style="padding:16px 20px;border-bottom:1px solid #EAECF0;display:flex;justify-content:space-between;align-items:center;gap:12px;">
          <div>
            <div style="font-weight:700;font-size:15px;color:#101828;">Proof of transfer</div>
            <div style="font-size:12px;color:#667085;margin-top:2px;">
              ${esc(dep.user_name || dep.user_email || "Deposit")} ·
              ${esc(dep.external_ref_id || "—")} ·
              Uploaded ${esc(uploaded)}
            </div>
          </div>
          <div style="display:flex;gap:8px;">
            <a href="${esc(url)}" target="_blank" rel="noopener noreferrer" class="admin-btn admin-btn--secondary admin-btn--sm">Open in tab</a>
            <button class="admin-btn admin-btn--secondary admin-btn--sm" id="deposit-proof-close">Close</button>
          </div>
        </div>
        <div style="padding:16px 20px;flex:1;overflow:auto;background:#F8FAFC;">
          ${userNotesHtml}
          ${isPdf
            ? `<iframe src="${esc(url)}" style="width:100%;height:70vh;border:1px solid #EAECF0;border-radius:6px;background:#fff;" title="Deposit proof"></iframe>`
            : `<img src="${esc(url)}" alt="Deposit proof" style="max-width:100%;height:auto;border:1px solid #EAECF0;border-radius:6px;background:#fff;display:block;margin:0 auto;" />`
          }
          ${data.expires_in_minutes ? `<div style="margin-top:10px;font-size:11px;color:#98A2B3;text-align:center;">Signed URL expires in ${data.expires_in_minutes} minutes.</div>` : ""}
        </div>
      </div>
    `;
    document.body.appendChild(overlay);

    function close() {
      overlay.remove();
      document.removeEventListener("keydown", onEsc);
    }
    function onEsc(e) { if (e.key === "Escape") close(); }
    document.addEventListener("keydown", onEsc);
    overlay.addEventListener("click", (e) => { if (e.target === overlay) close(); });
    document.getElementById("deposit-proof-close").addEventListener("click", close);
  } catch (e) {
    console.error("viewDepositProof failed", e);
    alert("Network error loading proof");
  }
}

// Make the action accessible from inline `onclick` in the table cell.
window.viewDepositProof = viewDepositProof;
