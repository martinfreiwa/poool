/**
 * Admin — Asset Change Requests
 * Features: aging KPI, clickable KPI filters, sortable cols, multi-filter,
 * auto-refresh + last-updated, anomaly detection, bulk approve/reject,
 * diff side-panel, keyboard shortcuts, CSV export.
 */
(function () {
  "use strict";

  const REFRESH_MS = 30_000;
  const ANOMALY_WINDOW_MS = 60 * 60 * 1000; // 1h
  const ANOMALY_THRESHOLD = 8;
  const PAGE_SIZES = [25, 50, 100];

  const state = {
    items: [],
    selected: new Set(),
    sort: { key: "created_at", dir: "desc" },
    filters: { status: "all", developer: "all", range: "all", q: "", assignment: "all" },
    focusIdx: -1,
    lastFetched: null,
    autoRefresh: true,
    timer: null,
    anomalyDismissed: false,
    page: 1,
    pageSize: parseInt(localStorage.getItem("cr_page_size") || "25", 10),
    currentAdminId: null,
    currentAdminName: null,
  };

  document.addEventListener("DOMContentLoaded", init);

  function init() {
    bindUi();
    renderViews();
    fetchMe();
    load();
    startTimer();
    bindKeyboard();
  }

  async function fetchMe() {
    try {
      const r = await fetch("/api/me", { credentials: "same-origin" });
      if (!r.ok) return;
      const me = await r.json();
      state.currentAdminId = me.id;
      state.currentAdminName = me.full_name || me.first_name || me.email;
      // re-render to update "Me" labels
      if (state.items.length) render();
    } catch (e) { /* silent */ }
  }

  // ── Saved views ───────────────────────────────────────────────────────────
  const VIEWS_KEY = "cr_saved_views";
  const BUILTIN_VIEWS = [
    { id: "_pending_all", name: "All pending", builtin: true,
      filters: { status: "pending", developer: "all", range: "all", q: "", assignment: "all" }, sort: { key: "created_at", dir: "asc" } },
    { id: "_assigned_me", name: "Assigned to me", builtin: true,
      filters: { status: "pending", developer: "all", range: "all", q: "", assignment: "me" }, sort: { key: "created_at", dir: "asc" } },
    { id: "_unassigned", name: "Unassigned", builtin: true,
      filters: { status: "pending", developer: "all", range: "all", q: "", assignment: "unassigned" }, sort: { key: "created_at", dir: "asc" } },
    { id: "_old_pending", name: "Pending >24h", builtin: true,
      filters: { status: "pending", developer: "all", range: "all", q: "", assignment: "all" }, sort: { key: "created_at", dir: "asc" } },
    { id: "_recent_rejected", name: "Recent rejected (7d)", builtin: true,
      filters: { status: "rejected", developer: "all", range: "7d", q: "", assignment: "all" }, sort: { key: "created_at", dir: "desc" } },
  ];
  function readViews() {
    try { return JSON.parse(localStorage.getItem(VIEWS_KEY) || "[]"); }
    catch { return []; }
  }
  function writeViews(v) {
    localStorage.setItem(VIEWS_KEY, JSON.stringify(v));
  }
  function renderViews() {
    const wrap = $("cr-views");
    if (!wrap) return;
    const custom = readViews();
    const all = [...BUILTIN_VIEWS, ...custom];
    wrap.innerHTML = all.map((v) => `
      <button type="button" class="cr-view-chip${v.builtin ? " cr-view-chip--builtin" : ""}" data-view-id="${esc(v.id)}">
        ${esc(v.name)}
        ${v.builtin ? "" : `<span class="cr-view-chip-x" data-del="${esc(v.id)}" aria-label="Delete view">×</span>`}
      </button>`).join("") +
      `<button type="button" id="cr-view-save" class="cr-view-chip cr-view-chip--add" title="Save current filters as view">+ Save view</button>`;
    wrap.querySelectorAll(".cr-view-chip").forEach((b) => {
      const id = b.dataset.viewId;
      if (!id) return;
      b.addEventListener("click", (e) => {
        if (e.target.dataset.del) {
          e.stopPropagation();
          const cur = readViews().filter((v) => v.id !== e.target.dataset.del);
          writeViews(cur);
          renderViews();
          return;
        }
        applyView(id);
      });
    });
    $("cr-view-save")?.addEventListener("click", saveCurrentView);
  }
  function applyView(id) {
    const v = [...BUILTIN_VIEWS, ...readViews()].find((x) => x.id === id);
    if (!v) return;
    state.filters = { ...v.filters };
    if (v.sort) state.sort = { ...v.sort };
    state.appliedView = v;
    state.page = 1;
    $("search-input").value = state.filters.q || "";
    $("filter-status").value = state.filters.status;
    $("filter-range").value = state.filters.range;
    if ($("filter-assignment")) $("filter-assignment").value = state.filters.assignment || "all";
    if ([...$("filter-developer").options].some((o) => o.value === state.filters.developer)) {
      $("filter-developer").value = state.filters.developer;
    }
    syncKpiPressed();
    document.querySelectorAll(".cr-view-chip").forEach((c) =>
      c.classList.toggle("cr-view-chip--active", c.dataset.viewId === id));
    render();
  }
  function saveCurrentView() {
    const name = prompt("View name:");
    if (!name) return;
    const cur = readViews();
    const id = `v_${Date.now()}`;
    cur.push({ id, name, filters: { ...state.filters }, sort: { ...state.sort } });
    writeViews(cur);
    renderViews();
    toast("View saved", "success");
  }

  // ── DOM helpers ───────────────────────────────────────────────────────────
  const $ = (id) => document.getElementById(id);
  function esc(s) {
    if (s == null) return "";
    const d = document.createElement("div");
    d.textContent = String(s);
    return d.innerHTML;
  }

  // ── Wire UI ───────────────────────────────────────────────────────────────
  function bindUi() {
    $("search-input").addEventListener("input", debounce(() => {
      state.filters.q = $("search-input").value.trim().toLowerCase();
      state.page = 1;
      render();
    }, 120));

    $("filter-status").addEventListener("change", () => {
      state.filters.status = $("filter-status").value;
      state.page = 1;
      syncKpiPressed();
      render();
    });
    $("filter-developer").addEventListener("change", () => {
      state.filters.developer = $("filter-developer").value;
      state.page = 1;
      render();
    });
    $("filter-range").addEventListener("change", () => {
      state.filters.range = $("filter-range").value;
      state.page = 1;
      render();
    });
    $("filter-assignment")?.addEventListener("change", () => {
      state.filters.assignment = $("filter-assignment").value;
      state.page = 1;
      render();
    });
    $("filter-reset").addEventListener("click", resetFilters);

    document.querySelectorAll(".cr-kpi-card").forEach((card) => {
      card.addEventListener("click", () => {
        const f = card.dataset.filter;
        state.filters.status = f;
        $("filter-status").value = f;
        state.page = 1;
        syncKpiPressed();
        render();
      });
    });

    document.querySelectorAll("th.cr-sort").forEach((th) => {
      th.addEventListener("click", () => {
        const key = th.dataset.sort;
        if (state.sort.key === key) {
          state.sort.dir = state.sort.dir === "asc" ? "desc" : "asc";
        } else {
          state.sort.key = key;
          state.sort.dir = "asc";
        }
        state.page = 1;
        render();
      });
    });

    $("select-all").addEventListener("change", (e) => {
      const onPage = visibleItems();
      if (e.target.checked) onPage.forEach((i) => state.selected.add(i.id));
      else onPage.forEach((i) => state.selected.delete(i.id));
      render();
    });

    $("bulk-approve-btn").addEventListener("click", () => bulkAct("approve"));
    $("bulk-reject-btn").addEventListener("click", () => openRejectModal("bulk"));
    $("bulk-clear-btn").addEventListener("click", () => {
      state.selected.clear();
      render();
    });

    $("refresh-btn").addEventListener("click", () => load());
    $("export-btn").addEventListener("click", exportCsv);
    $("autorefresh-toggle").addEventListener("change", (e) => {
      state.autoRefresh = e.target.checked;
      if (state.autoRefresh) startTimer();
      else stopTimer();
    });

    $("anomaly-dismiss").addEventListener("click", () => {
      state.anomalyDismissed = true;
      $("anomaly-banner").hidden = true;
    });

    // Drawer
    document.querySelectorAll("[data-drawer-close]").forEach((el) =>
      el.addEventListener("click", closeDrawer)
    );
    $("drawer-prev").addEventListener("click", () => navDrawer(-1));
    $("drawer-next").addEventListener("click", () => navDrawer(1));
    $("drawer-approve").addEventListener("click", () => drawerAct("approve"));
    $("drawer-reject").addEventListener("click", () => openRejectModal("drawer"));
    $("drawer-claim")?.addEventListener("click", drawerClaim);

    // Reject modal
    document.querySelectorAll("[data-modal-close]").forEach((el) =>
      el.addEventListener("click", closeModals)
    );
    $("reject-confirm").addEventListener("click", confirmReject);
  }

  function resetFilters() {
    state.filters = { status: "all", developer: "all", range: "all", q: "", assignment: "all" };
    state.page = 1;
    $("search-input").value = "";
    $("filter-status").value = "all";
    $("filter-developer").value = "all";
    $("filter-range").value = "all";
    if ($("filter-assignment")) $("filter-assignment").value = "all";
    syncKpiPressed();
    render();
  }

  function syncKpiPressed() {
    document.querySelectorAll(".cr-kpi-card").forEach((c) => {
      c.setAttribute("aria-pressed", String(c.dataset.filter === state.filters.status));
    });
  }

  // ── Fetch ─────────────────────────────────────────────────────────────────
  async function load() {
    try {
      const resp = await fetch("/api/admin/change-requests");
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const data = await resp.json();
      state.items = data.items || [];
      state.lastFetched = new Date();
      populateDeveloperFilter();
      renderKpis(data);
      detectAnomalies();
      render();
      updateLastUpdated();
    } catch (err) {
      $("table-body").innerHTML =
        `<tr><td colspan="9" class="cr-state cr-state--error">Failed to load: ${esc(err.message)} <button class="admin-btn admin-btn--ghost admin-btn--sm" onclick="window.location.reload()">Retry</button></td></tr>`;
    }
  }

  function startTimer() {
    stopTimer();
    state.timer = setInterval(() => {
      if (state.autoRefresh && document.visibilityState === "visible") load();
      updateLastUpdated();
    }, REFRESH_MS);
  }
  function stopTimer() {
    if (state.timer) clearInterval(state.timer);
    state.timer = null;
  }

  function updateLastUpdated() {
    if (!state.lastFetched) return;
    const sec = Math.floor((Date.now() - state.lastFetched.getTime()) / 1000);
    const txt = sec < 5 ? "just now" :
                sec < 60 ? `${sec}s ago` :
                sec < 3600 ? `${Math.floor(sec / 60)}m ago` :
                `${Math.floor(sec / 3600)}h ago`;
    $("last-updated").textContent = `Updated ${txt}`;
  }

  // ── KPIs + aging ──────────────────────────────────────────────────────────
  function renderKpis(data) {
    $("kpi-pending").textContent = data.pending_count ?? 0;
    $("kpi-approved").textContent = data.approved_count ?? 0;
    $("kpi-rejected").textContent = data.rejected_count ?? 0;
    $("kpi-total").textContent = state.items.length;
    renderDeltas();

    const pending = state.items.filter((i) => i.status === "pending");
    if (pending.length === 0) {
      $("kpi-pending-sub").textContent = "Inbox zero ✓";
      $("sla-pill").hidden = true;
    } else {
      const oldest = pending.reduce((acc, i) =>
        new Date(i.created_at) < new Date(acc.created_at) ? i : acc, pending[0]);
      const ageMs = Date.now() - new Date(oldest.created_at).getTime();
      const ageH = ageMs / 3_600_000;
      let tier = "ok"; let label = "";
      if (ageH < 4) { tier = "ok"; label = `Oldest ${fmtAge(ageMs)}`; }
      else if (ageH < 24) { tier = "warn"; label = `Oldest ${fmtAge(ageMs)}`; }
      else { tier = "crit"; label = `SLA breach: ${fmtAge(ageMs)}`; }
      const pill = $("sla-pill");
      pill.hidden = false;
      pill.textContent = label;
      pill.className = `cr-sla-pill cr-sla-pill--${tier}`;
      $("kpi-pending-sub").textContent = `${pending.length} awaiting review`;
    }
  }

  function renderDeltas() {
    const now = Date.now();
    const w = 7 * 86_400_000;
    const last7 = (st) => state.items.filter((i) =>
      i.status === st && now - new Date(i.created_at).getTime() <= w).length;
    const prev7 = (st) => state.items.filter((i) => {
      const t = new Date(i.created_at).getTime();
      return i.status === st && now - t > w && now - t <= 2 * w;
    }).length;
    setDelta("kpi-approved-sub", last7("approved"), prev7("approved"), "approved");
    setDelta("kpi-rejected-sub", last7("rejected"), prev7("rejected"), "rejected");
  }
  function setDelta(elId, cur, prev, label) {
    const el = $(elId);
    if (!el) return;
    if (cur === 0 && prev === 0) {
      el.innerHTML = `No ${label} in 14d`;
      return;
    }
    let arrow = "→", cls = "neutral", pct = "";
    if (prev === 0) {
      arrow = "↑"; cls = "up"; pct = "new";
    } else {
      const d = ((cur - prev) / prev) * 100;
      if (Math.abs(d) < 1) { arrow = "→"; cls = "neutral"; pct = "0%"; }
      else if (d > 0) { arrow = "↑"; cls = "up"; pct = `${Math.round(d)}%`; }
      else { arrow = "↓"; cls = "down"; pct = `${Math.abs(Math.round(d))}%`; }
    }
    el.innerHTML = `<span class="cr-delta cr-delta--${cls}">${arrow} ${pct}</span> <span class="cr-delta-sub">${cur} in 7d vs ${prev}</span>`;
  }

  function fmtAge(ms) {
    const m = ms / 60000;
    if (m < 60) return `${Math.floor(m)}m`;
    const h = m / 60;
    if (h < 48) return `${Math.floor(h)}h`;
    return `${Math.floor(h / 24)}d`;
  }

  // ── Filter dropdown population ────────────────────────────────────────────
  function populateDeveloperFilter() {
    const sel = $("filter-developer");
    const cur = sel.value;
    const devs = [...new Set(state.items.map((i) => i.developer_name).filter(Boolean))].sort();
    sel.innerHTML = `<option value="all">All developers</option>` +
      devs.map((d) => `<option value="${esc(d)}">${esc(d)}</option>`).join("");
    if (devs.includes(cur)) sel.value = cur;
  }

  // ── Anomaly detection ─────────────────────────────────────────────────────
  function detectAnomalies() {
    if (state.anomalyDismissed) return;
    const now = Date.now();
    const recentByDev = new Map();
    state.items.forEach((i) => {
      const t = new Date(i.created_at).getTime();
      if (now - t > ANOMALY_WINDOW_MS) return;
      recentByDev.set(i.developer_name, (recentByDev.get(i.developer_name) || 0) + 1);
    });
    const offenders = [...recentByDev.entries()].filter(([, n]) => n >= ANOMALY_THRESHOLD);
    const banner = $("anomaly-banner");
    if (offenders.length === 0) {
      banner.hidden = true;
      return;
    }
    banner.hidden = false;
    $("anomaly-text").textContent =
      `Unusual activity: ${offenders.map(([d, n]) => `${d} (${n} changes/h)`).join(", ")}`;
  }

  // ── Filtering / sorting ───────────────────────────────────────────────────
  function filteredItems() {
    const { status, developer, range, q, assignment } = state.filters;
    const cutoff = rangeCutoff(range);
    return state.items
      .filter((i) => {
        if (status !== "all" && i.status !== status) return false;
        if (developer !== "all" && i.developer_name !== developer) return false;
        if (cutoff && new Date(i.created_at).getTime() < cutoff) return false;
        if (assignment === "me" && i.assigned_to !== state.currentAdminId) return false;
        if (assignment === "unassigned" && i.assigned_to) return false;
        if (assignment === "assigned" && !i.assigned_to) return false;
        if (q) {
          const hay = `${i.asset_title || ""} ${i.developer_name || ""}`.toLowerCase();
          if (!hay.includes(q)) return false;
        }
        return true;
      })
      .sort(sortFn);
  }

  // Items currently rendered on screen (after pagination).
  function visibleItems() {
    const all = filteredItems();
    const start = (state.page - 1) * state.pageSize;
    return all.slice(start, start + state.pageSize);
  }

  function rangeCutoff(r) {
    if (r === "24h") return Date.now() - 86_400_000;
    if (r === "7d") return Date.now() - 7 * 86_400_000;
    if (r === "30d") return Date.now() - 30 * 86_400_000;
    return null;
  }

  function sortFn(a, b) {
    const { key, dir } = state.sort;
    let va = a[key], vb = b[key];
    if (key === "created_at") { va = new Date(va).getTime(); vb = new Date(vb).getTime(); }
    if (typeof va === "string") va = va.toLowerCase();
    if (typeof vb === "string") vb = vb.toLowerCase();
    if (va < vb) return dir === "asc" ? -1 : 1;
    if (va > vb) return dir === "asc" ? 1 : -1;
    return 0;
  }

  // ── Render table ──────────────────────────────────────────────────────────
  function render() {
    syncKpiPressed();
    updateSortIndicators();
    const items = visibleItems();
    const tbody = $("table-body");

    if (items.length === 0) {
      tbody.innerHTML = renderEmpty();
      updateBulkBar();
      return;
    }

    const now = Date.now();
    tbody.innerHTML = items.map((it, idx) => {
      const age = now - new Date(it.created_at).getTime();
      const ageTier = it.status !== "pending" ? "" :
        age > 86_400_000 ? "crit" : age > 14_400_000 ? "warn" : "ok";
      const ageText = it.status === "pending" ? fmtAge(age) : "—";
      const statusClass =
        it.status === "pending" ? "admin-badge--warning" :
        it.status === "approved" ? "admin-badge--success" : "admin-badge--danger";
      const statusLabel = it.status[0].toUpperCase() + it.status.slice(1);
      const date = new Date(it.created_at);
      const dateStr = date.toLocaleString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
      const checked = state.selected.has(it.id) ? "checked" : "";
      const focused = idx === state.focusIdx ? "cr-row--focus" : "";
      return `
        <tr class="cr-row ${focused}" data-id="${esc(it.id)}" data-idx="${idx}" tabindex="0">
          <td class="cr-col-check"><input type="checkbox" class="cr-row-check" data-id="${esc(it.id)}" ${checked} aria-label="Select row"></td>
          <td><strong>${esc(it.asset_title || "Unknown Asset")}</strong></td>
          <td>${esc(it.developer_name || "Unknown")}</td>
          <td><span class="admin-badge admin-badge--info">${it.fields_changed} field${it.fields_changed !== 1 ? "s" : ""}</span></td>
          <td><span class="admin-badge ${statusClass}">${statusLabel}</span></td>
          <td class="cr-col-assigned">${renderAssignedCell(it)}</td>
          <td class="cr-col-date">${esc(dateStr)}</td>
          <td>${ageTier ? `<span class="cr-age-pill cr-age-pill--${ageTier}">${ageText}</span>` : `<span class="cr-muted">${ageText}</span>`}</td>
          <td class="cr-col-actions">
            <button type="button" class="admin-btn admin-btn--ghost admin-btn--sm cr-act-view" data-id="${esc(it.id)}" title="Diff preview">Diff</button>
            <a href="/admin/audit-logs.html?search=${esc(it.asset_id || it.id)}&entity=asset" class="admin-btn admin-btn--ghost admin-btn--sm" title="View audit history" onclick="event.stopPropagation()">History</a>
            <a href="/admin/asset-change-review.html?id=${esc(it.id)}" class="admin-btn admin-btn--primary admin-btn--sm" onclick="event.stopPropagation()">Review</a>
          </td>
        </tr>`;
    }).join("");

    // wire assignment actions
    tbody.querySelectorAll(".cr-act-claim").forEach((b) => {
      b.addEventListener("click", (e) => { e.stopPropagation(); assign(b.dataset.id); });
    });
    tbody.querySelectorAll(".cr-act-unclaim").forEach((b) => {
      b.addEventListener("click", (e) => { e.stopPropagation(); unassign(b.dataset.id); });
    });

    // wire row events
    tbody.querySelectorAll(".cr-row-check").forEach((cb) => {
      cb.addEventListener("change", (e) => {
        const id = e.target.dataset.id;
        if (e.target.checked) state.selected.add(id);
        else state.selected.delete(id);
        updateBulkBar();
      });
    });
    tbody.querySelectorAll(".cr-act-view").forEach((b) => {
      b.addEventListener("click", (e) => {
        e.stopPropagation();
        openDrawer(e.target.dataset.id);
      });
    });
    tbody.querySelectorAll(".cr-row").forEach((tr) => {
      tr.addEventListener("click", (e) => {
        if (e.target.closest("input,button,a")) return;
        openDrawer(tr.dataset.id);
      });
    });

    // select-all checkbox state
    const all = items.length > 0 && items.every((i) => state.selected.has(i.id));
    $("select-all").checked = all;
    updateBulkBar();
    renderPagination();
  }

  function renderPagination() {
    let bar = document.getElementById("cr-pagination");
    const total = filteredItems().length;
    const totalPages = Math.max(1, Math.ceil(total / state.pageSize));
    if (state.page > totalPages) state.page = totalPages;
    const start = total === 0 ? 0 : (state.page - 1) * state.pageSize + 1;
    const end = Math.min(state.page * state.pageSize, total);

    if (!bar) {
      bar = document.createElement("div");
      bar.id = "cr-pagination";
      bar.className = "cr-pagination";
      const tableCard = document.querySelector(".cr-table").closest(".admin-card");
      tableCard.appendChild(bar);
    }
    const sizeOpts = PAGE_SIZES.map((n) =>
      `<option value="${n}"${n === state.pageSize ? " selected" : ""}>${n}/page</option>`
    ).join("");
    bar.innerHTML = `
      <div class="cr-pagination-info">
        ${total === 0 ? "No results" : `Showing <strong>${start}–${end}</strong> of <strong>${total}</strong>`}
      </div>
      <div class="cr-pagination-controls">
        <select id="cr-page-size" class="admin-select admin-select--sm" aria-label="Page size">${sizeOpts}</select>
        <button type="button" class="admin-btn admin-btn--ghost admin-btn--sm" id="cr-page-first" ${state.page <= 1 ? "disabled" : ""} aria-label="First page">«</button>
        <button type="button" class="admin-btn admin-btn--ghost admin-btn--sm" id="cr-page-prev" ${state.page <= 1 ? "disabled" : ""} aria-label="Previous page">‹</button>
        <span class="cr-pagination-page">Page <strong>${state.page}</strong> / ${totalPages}</span>
        <button type="button" class="admin-btn admin-btn--ghost admin-btn--sm" id="cr-page-next" ${state.page >= totalPages ? "disabled" : ""} aria-label="Next page">›</button>
        <button type="button" class="admin-btn admin-btn--ghost admin-btn--sm" id="cr-page-last" ${state.page >= totalPages ? "disabled" : ""} aria-label="Last page">»</button>
      </div>`;
    $("cr-page-first").onclick = () => { state.page = 1; render(); };
    $("cr-page-prev").onclick = () => { state.page--; render(); };
    $("cr-page-next").onclick = () => { state.page++; render(); };
    $("cr-page-last").onclick = () => { state.page = totalPages; render(); };
    $("cr-page-size").onchange = (e) => {
      state.pageSize = parseInt(e.target.value, 10);
      localStorage.setItem("cr_page_size", String(state.pageSize));
      state.page = 1;
      render();
    };
  }

  function renderEmpty() {
    const filtered = state.filters.status !== "all" || state.filters.developer !== "all" ||
      state.filters.range !== "all" || state.filters.q !== "";
    if (filtered) {
      return `<tr><td colspan="9" class="cr-state">
        <div class="cr-empty">
          <div class="cr-empty-title">No matches</div>
          <div class="cr-empty-sub">Try resetting filters.</div>
          <button type="button" class="admin-btn admin-btn--ghost admin-btn--sm" onclick="document.getElementById('filter-reset').click()">Reset filters</button>
        </div></td></tr>`;
    }
    return `<tr><td colspan="9" class="cr-state">
      <div class="cr-empty">
        <div class="cr-empty-icon" aria-hidden="true">📭</div>
        <div class="cr-empty-title">No change requests</div>
        <div class="cr-empty-sub">Developers' edits to live assets appear here for review.</div>
        <a href="/admin/developer-submissions.html" class="admin-btn admin-btn--ghost admin-btn--sm">View submissions →</a>
      </div></td></tr>`;
  }

  function renderAssignedCell(it) {
    if (it.status !== "pending") {
      return `<span class="cr-muted">—</span>`;
    }
    if (!it.assigned_to) {
      return `<button type="button" class="cr-act-claim cr-assign-claim" data-id="${esc(it.id)}" title="Claim review">Claim</button>`;
    }
    const isMe = state.currentAdminId && it.assigned_to === state.currentAdminId;
    const label = isMe ? "Me" : (it.assigned_to_name || "Assigned");
    return `<span class="cr-assigned-chip ${isMe ? "cr-assigned-chip--me" : ""}">${esc(label)}</span>` +
      (isMe ? ` <button type="button" class="cr-act-unclaim cr-assign-x" data-id="${esc(it.id)}" title="Release">×</button>` : "");
  }

  async function assign(id) {
    try {
      const r = await fetch(`/api/admin/change-requests/${id}/assign`, { method: "POST" });
      if (!r.ok) throw new Error((await r.json()).error || "Assign failed");
      toast("Claimed", "success");
      await load();
    } catch (e) { toast(e.message, "error"); }
  }
  async function unassign(id) {
    try {
      const r = await fetch(`/api/admin/change-requests/${id}/unassign`, { method: "POST" });
      if (!r.ok) throw new Error((await r.json()).error || "Unassign failed");
      toast("Released", "success");
      await load();
    } catch (e) { toast(e.message, "error"); }
  }

  function updateSortIndicators() {
    document.querySelectorAll("th.cr-sort").forEach((th) => {
      th.classList.remove("cr-sort--active", "cr-sort--asc", "cr-sort--desc");
      const ind = th.querySelector(".cr-sort-ind");
      if (ind) ind.textContent = "";
      if (th.dataset.sort === state.sort.key) {
        th.classList.add("cr-sort--active", `cr-sort--${state.sort.dir}`);
        if (ind) ind.textContent = state.sort.dir === "asc" ? "↑" : "↓";
      }
    });
  }

  function updateBulkBar() {
    const n = state.selected.size;
    $("bulk-bar").hidden = n === 0;
    $("bulk-count").textContent = `${n} selected`;
  }

  // ── Bulk actions ──────────────────────────────────────────────────────────
  async function bulkAct(action) {
    const ids = [...state.selected];
    if (ids.length === 0) return;
    if (action === "approve") {
      if (!confirm(`Approve ${ids.length} change request${ids.length === 1 ? "" : "s"}? Changes will be applied to assets.`)) return;
      try {
        const resp = await fetch("/api/admin/change-requests/bulk-approve", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ ids }),
        });
        const data = await resp.json();
        if (!resp.ok) throw new Error(data.error || "Bulk approve failed");
        toast(`Approved ${data.approved}${data.failed?.length ? ` · ${data.failed.length} skipped` : ""}`, "success");
        state.selected.clear();
        await load();
      } catch (e) { toast(e.message, "error"); }
    }
  }

  // ── Reject modal flow ─────────────────────────────────────────────────────
  let rejectMode = null; // "bulk" | "drawer" | { id }
  function openRejectModal(mode) {
    rejectMode = mode;
    $("reject-modal").hidden = false;
    $("reject-reason").value = "";
    const sub = mode === "bulk"
      ? `Reject ${state.selected.size} request${state.selected.size === 1 ? "" : "s"}.`
      : "Reject this request.";
    $("reject-modal-sub").textContent = sub;
    setTimeout(() => $("reject-reason").focus(), 0);
  }
  async function confirmReject() {
    const reason = $("reject-reason").value.trim();
    if (!reason) { toast("Reason required", "error"); return; }
    try {
      if (rejectMode === "bulk") {
        const ids = [...state.selected];
        const resp = await fetch("/api/admin/change-requests/bulk-reject", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ ids, notes: reason }),
        });
        const data = await resp.json();
        if (!resp.ok) throw new Error(data.error || "Bulk reject failed");
        toast(`Rejected ${data.rejected}`, "success");
        state.selected.clear();
      } else if (rejectMode === "drawer" && drawerCurrentId) {
        const resp = await fetch(`/api/admin/change-requests/${drawerCurrentId}/reject`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ notes: reason }),
        });
        if (!resp.ok) throw new Error((await resp.json()).error || "Reject failed");
        toast("Rejected", "success");
        closeDrawer();
      }
      closeModals();
      await load();
    } catch (e) { toast(e.message, "error"); }
  }
  function closeModals() {
    $("reject-modal").hidden = true;
    $("help-modal").hidden = true;
    rejectMode = null;
  }

  // ── Diff drawer ───────────────────────────────────────────────────────────
  let drawerCurrentId = null;
  async function openDrawer(id) {
    drawerCurrentId = id;
    const meta = state.items.find((i) => i.id === id);
    $("diff-drawer").hidden = false;
    $("diff-drawer-body").innerHTML = `<div class="cr-state cr-state--loading">Loading…</div>`;
    $("diff-drawer-sub").textContent = "";
    $("drawer-fullview").href = `/admin/asset-change-review.html?id=${id}`;
    const hist = $("drawer-history");
    if (hist && meta) hist.href = `/admin/audit-logs.html?search=${meta.asset_id || id}&entity=asset`;
    try {
      const resp = await fetch(`/api/admin/change-requests/${id}`);
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const d = await resp.json();
      $("diff-drawer-title").textContent = d.asset_title || "Change Request";
      $("diff-drawer-sub").textContent = `${d.developer_name || "Unknown"} · ${new Date(d.created_at).toLocaleString()}`;
      $("diff-drawer-body").innerHTML = renderDiff(d.original_values, d.proposed_values, d.status, d.admin_notes);
      const isPending = d.status === "pending";
      $("drawer-approve").disabled = !isPending;
      $("drawer-reject").disabled = !isPending;
      // Claim button reflects current assignment
      const meta = state.items.find((i) => i.id === id);
      const claimBtn = $("drawer-claim");
      if (claimBtn) {
        if (!isPending) { claimBtn.hidden = true; }
        else if (meta && meta.assigned_to === state.currentAdminId) {
          claimBtn.hidden = false;
          claimBtn.textContent = "Release";
          claimBtn.dataset.mode = "release";
        } else if (meta && meta.assigned_to) {
          claimBtn.hidden = false;
          claimBtn.textContent = `Take over from ${meta.assigned_to_name || "reviewer"}`;
          claimBtn.dataset.mode = "claim";
        } else {
          claimBtn.hidden = false;
          claimBtn.textContent = "Claim";
          claimBtn.dataset.mode = "claim";
        }
      }
    } catch (e) {
      $("diff-drawer-body").innerHTML = `<div class="cr-state cr-state--error">Failed: ${esc(e.message)}</div>`;
    }
  }
  function closeDrawer() {
    $("diff-drawer").hidden = true;
    drawerCurrentId = null;
  }
  function renderDiff(orig, prop, status, notes) {
    const o = orig || {};
    const p = prop || {};
    const keys = [...new Set([...Object.keys(o), ...Object.keys(p)])].sort();
    if (keys.length === 0) return `<div class="cr-state">No fields changed.</div>`;
    const rows = keys.map((k) => `
      <div class="cr-diff-row">
        <div class="cr-diff-key">${esc(prettify(k))}</div>
        <div class="cr-diff-old">${esc(fmtVal(o[k]))}</div>
        <div class="cr-diff-arrow" aria-hidden="true">→</div>
        <div class="cr-diff-new">${esc(fmtVal(p[k]))}</div>
      </div>`).join("");
    const notesHtml = notes
      ? `<div class="cr-diff-notes"><strong>Admin notes:</strong> ${esc(notes)}</div>` : "";
    const statusHtml = status !== "pending"
      ? `<div class="cr-diff-status cr-diff-status--${status}">Status: ${status}</div>` : "";
    return `${statusHtml}<div class="cr-diff-grid">${rows}</div>${notesHtml}`;
  }
  function prettify(k) { return k.replace(/_/g, " ").replace(/\bbps\b/i, "(bps)"); }
  function fmtVal(v) {
    if (v == null || v === "") return "—";
    if (typeof v === "object") return JSON.stringify(v);
    return String(v);
  }
  function navDrawer(delta) {
    const items = filteredItems();
    const idx = items.findIndex((i) => i.id === drawerCurrentId);
    const next = items[idx + delta];
    if (next) openDrawer(next.id);
  }
  async function drawerClaim() {
    if (!drawerCurrentId) return;
    const mode = $("drawer-claim").dataset.mode;
    if (mode === "release") await unassign(drawerCurrentId);
    else await assign(drawerCurrentId);
    if (!$("diff-drawer").hidden) await openDrawer(drawerCurrentId);
  }

  async function drawerAct(action) {
    if (!drawerCurrentId) return;
    if (action === "approve") {
      if (!confirm("Approve and apply these changes?")) return;
      try {
        const resp = await fetch(`/api/admin/change-requests/${drawerCurrentId}/approve`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({}),
        });
        if (!resp.ok) throw new Error((await resp.json()).error || "Approve failed");
        toast("Approved", "success");
        closeDrawer();
        await load();
      } catch (e) { toast(e.message, "error"); }
    }
  }

  // ── Keyboard shortcuts ────────────────────────────────────────────────────
  function bindKeyboard() {
    document.addEventListener("keydown", (e) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const tag = (e.target.tagName || "").toLowerCase();
      const typing = tag === "input" || tag === "textarea" || e.target.isContentEditable;
      if (e.key === "Escape") {
        if (!$("diff-drawer").hidden) { closeDrawer(); return; }
        if (!$("reject-modal").hidden || !$("help-modal").hidden) { closeModals(); return; }
      }
      if (typing) return;
      switch (e.key) {
        case "/": e.preventDefault(); $("search-input").focus(); break;
        case "?": e.preventDefault(); $("help-modal").hidden = false; break;
        case "j": e.preventDefault(); moveFocus(1); break;
        case "k": e.preventDefault(); moveFocus(-1); break;
        case "x": e.preventDefault(); toggleFocusedSelect(); break;
        case "Enter": {
          const items = visibleItems();
          const it = items[state.focusIdx];
          if (it) { e.preventDefault(); openDrawer(it.id); }
          break;
        }
        case "a": {
          if (!$("diff-drawer").hidden) { e.preventDefault(); drawerAct("approve"); }
          break;
        }
        case "r": {
          if (!$("diff-drawer").hidden) { e.preventDefault(); openRejectModal("drawer"); }
          break;
        }
        case "c": {
          if (!$("diff-drawer").hidden) { e.preventDefault(); drawerClaim(); }
          break;
        }
        case "R": e.preventDefault(); load(); break;
      }
    });
  }
  function moveFocus(d) {
    const items = visibleItems();
    if (items.length === 0) return;
    state.focusIdx = Math.max(0, Math.min(items.length - 1, state.focusIdx + d));
    render();
    const row = document.querySelector(`.cr-row[data-idx="${state.focusIdx}"]`);
    if (row) row.scrollIntoView({ block: "nearest" });
  }
  function toggleFocusedSelect() {
    const items = visibleItems();
    const it = items[state.focusIdx];
    if (!it) return;
    if (state.selected.has(it.id)) state.selected.delete(it.id);
    else state.selected.add(it.id);
    render();
  }

  // ── Toast ─────────────────────────────────────────────────────────────────
  function toast(msg, kind = "info") {
    const t = document.createElement("div");
    t.className = `cr-toast cr-toast--${kind}`;
    t.textContent = msg;
    $("toast-region").appendChild(t);
    setTimeout(() => t.classList.add("cr-toast--in"), 10);
    setTimeout(() => { t.classList.remove("cr-toast--in"); setTimeout(() => t.remove(), 300); }, 3500);
  }

  // ── Export ────────────────────────────────────────────────────────────────
  function exportCsv() {
    const rows = [["id", "asset_title", "developer_name", "status", "fields_changed", "created_at"]];
    filteredItems().forEach((i) => {
      rows.push([i.id, i.asset_title || "", i.developer_name || "", i.status, i.fields_changed, i.created_at]);
    });
    const csv = rows.map((r) => r.map((v) => `"${String(v).replace(/"/g, '""')}"`).join(",")).join("\n");
    const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `change-requests-${new Date().toISOString().slice(0, 10)}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }

  // ── utils ────────────────────────────────────────────────────────────────
  function debounce(fn, ms) {
    let t;
    return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); };
  }
})();
