/**
 * Admin Page Kit
 * Shared utilities for admin list pages: auto-refresh, action-required banner,
 * saved-views, CSV export, sticky columns, badge contrast, age helpers,
 * trend snapshots, KPI click-to-filter, bulk-action bar.
 *
 * Usage:
 *   const kit = AdminPageKit.init({ snapshotKey, viewsKey, refreshFn, ... });
 *   kit.renderActionRequired([{label, count, color, onClick}], "#banner");
 *   kit.exportCsv(rows, cols, "out.csv");
 */
(function () {
  "use strict";

  // ── Helpers ─────────────────────────────────────────────────────
  function escHtml(s) {
    if (typeof s !== "string") return s == null ? "" : String(s);
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }

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
    if (d > 0) return d + "d " + h + "h";
    if (h > 0) return h + "h " + m + "m";
    return m + "m";
  }

  function ageColor(secs, slaHours = 24) {
    const slaSec = slaHours * 3600;
    if (secs > slaSec) return "var(--admin-danger, #C2410C)";
    if (secs > slaSec / 2) return "var(--admin-warning, #d97706)";
    return "var(--admin-text-muted)";
  }

  function numOrNull(v) {
    if (v == null || v === "") return null;
    const n = parseFloat(v);
    return Number.isFinite(n) ? n : null;
  }

  function debounce(fn, ms) {
    let t;
    return function (...a) {
      clearTimeout(t);
      t = setTimeout(() => fn.apply(this, a), ms);
    };
  }

  // ── Scoped CSS ──────────────────────────────────────────────────
  let cssInjected = false;
  function injectScopedCss() {
    if (cssInjected) return;
    cssInjected = true;
    const css = `
      .admin-table .admin-badge { font-weight: 700; letter-spacing: 0.01em; }
      .admin-table .admin-badge--success { color: #065f46; background: #d1fae5; }
      .admin-table .admin-badge--info    { color: #1e3a8a; background: #dbeafe; }
      .admin-table .admin-badge--warning { color: #92400e; background: #fef3c7; }
      .admin-table .admin-badge--danger  { color: #7f1d1d; background: #fee2e2; }
      .admin-kpi-card--clickable { cursor: pointer; transition: all 0.15s; }
      .admin-kpi-card--clickable:hover { box-shadow: 0 1px 8px rgba(0,0,0,0.08); transform: translateY(-1px); }
      .apk-action-chip { border:1px solid var(--admin-border); background:transparent; border-radius:999px; padding:4px 10px; font-size:12px; cursor:pointer; display:inline-flex; align-items:center; gap:6px; }
      .apk-action-chip:hover { background: var(--admin-hover-overlay, #f1f5f9); }
      .apk-trend-up { color: var(--admin-success); font-size: 11px; margin-left: 6px; }
      .apk-trend-down { color: var(--admin-danger, #C2410C); font-size: 11px; margin-left: 6px; }
      .apk-trend-flat { color: var(--admin-text-muted); font-size: 11px; margin-left: 6px; }
      .apk-anomaly-row td { background: rgba(254, 243, 199, 0.4); }
      .apk-bulk-bar { display: none; padding: 10px 20px; border-top: 1px solid var(--admin-border); background: var(--admin-bg-subtle, #f8fafc); align-items: center; gap: 10px; font-size: 13px; }
      .apk-bulk-bar.open { display: flex; }
      .apk-filter-panel { display: none; padding: 14px 20px; border-top: 1px solid var(--admin-border); background: var(--admin-bg-subtle, #f8fafc); gap: 16px; flex-wrap: wrap; align-items: flex-start; }
      .apk-filter-panel.open { display: flex; }
      .apk-filter-group { display: flex; flex-direction: column; gap: 4px; min-width: 140px; }
      .apk-filter-group > label { font-size: 11px; font-weight: 600; color: var(--admin-text-muted); text-transform: uppercase; letter-spacing: 0.04em; }
      .apk-filter-group input { padding: 4px 6px; border: 1px solid var(--admin-border); border-radius: 4px; font-size: 12px; }
      .apk-filter-range { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--admin-text-muted); }
      .apk-sla-badge { display:inline-block; margin-left:6px; font-size:10px; font-weight:700; color:var(--admin-danger, #C2410C); background:rgba(194,65,12,0.10); padding:1px 5px; border-radius:3px; }
      .apk-sla-row { box-shadow: inset 3px 0 0 var(--admin-danger, #C2410C); }
      @media (max-width: 1280px) {
        .apk-table-scroll { overflow-x: auto; }
        .apk-table-scroll table { min-width: 1180px; }
        .apk-sticky-col-1 { position: sticky; left: 0; background: var(--admin-bg-card, #fff); z-index: 2; }
        .apk-sticky-col-2 { position: sticky; left: 36px; background: var(--admin-bg-card, #fff); z-index: 2; box-shadow: 1px 0 0 var(--admin-border); }
      }
    `;
    const style = document.createElement("style");
    style.id = "admin-page-kit-css";
    style.textContent = css;
    document.head.appendChild(style);
  }

  // ── Action-Required Banner ──────────────────────────────────────
  function renderActionRequired(items, container, onChipClick) {
    const el = typeof container === "string" ? document.querySelector(container) : container;
    if (!el) return;
    if (!items || items.length === 0) {
      el.style.display = "none";
      el.innerHTML = "";
      return;
    }
    el.style.display = "block";
    el.innerHTML = `
      <div class="admin-card" style="padding:12px 16px;border-left:3px solid var(--admin-warning);display:flex;align-items:center;gap:14px;flex-wrap:wrap">
        <span style="font-weight:700;font-size:13px;color:var(--admin-text-primary);display:flex;align-items:center;gap:6px">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="var(--admin-warning)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
          Action required
        </span>
        ${items.map((i, idx) => `
          <button type="button" class="apk-action-chip" data-idx="${idx}">
            <span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:${i.color || 'var(--admin-warning)'}"></span>
            <span style="color:var(--admin-text-secondary)">${escHtml(i.label)}</span>
            <strong style="color:var(--admin-text-primary)">${i.count}</strong>
          </button>`).join("")}
      </div>`;
    el.querySelectorAll(".apk-action-chip").forEach((chip) => {
      chip.addEventListener("click", () => {
        const idx = parseInt(chip.dataset.idx, 10);
        const item = items[idx];
        if (item && typeof item.onClick === "function") item.onClick();
        else if (typeof onChipClick === "function") onChipClick(item);
      });
    });
  }

  // ── KPI Click-to-Filter ─────────────────────────────────────────
  function wireKpiClicks(handler) {
    document.querySelectorAll(".admin-kpi-card--clickable").forEach((card) => {
      if (card.dataset.apkWired) return;
      card.dataset.apkWired = "1";
      card.addEventListener("click", () => handler(card));
    });
  }

  // ── Saved Views ─────────────────────────────────────────────────
  function readViews(key) { try { return JSON.parse(localStorage.getItem(key) || "{}"); } catch { return {}; } }
  function writeViews(key, v) { try { localStorage.setItem(key, JSON.stringify(v)); } catch {} }
  function setupSavedViews({ selector, key, capture, apply, saveBtn }) {
    const sel = typeof selector === "string" ? document.querySelector(selector) : selector;
    const btn = typeof saveBtn === "string" ? document.querySelector(saveBtn) : saveBtn;
    function refresh(selected) {
      if (!sel) return;
      const views = readViews(key);
      const names = Object.keys(views).sort();
      sel.innerHTML = `<option value="">— Saved views —</option>` +
        names.map((n) => `<option value="${escHtml(n)}"${n === selected ? " selected" : ""}>${escHtml(n)}</option>`).join("") +
        (names.length ? `<option disabled>──────</option><option value="__delete__">Delete view…</option>` : "");
    }
    refresh();
    sel?.addEventListener("change", (e) => {
      const name = e.target.value;
      if (!name) return;
      if (name === "__delete__") {
        const target = window.prompt("Delete which view? Type exact name:");
        const views = readViews(key);
        if (target && views[target]) {
          delete views[target];
          writeViews(key, views);
          refresh();
        } else {
          e.target.value = "";
        }
        return;
      }
      const views = readViews(key);
      if (views[name]) apply(views[name]);
    });
    btn?.addEventListener("click", () => {
      const name = window.prompt("Name this view:");
      if (!name) return;
      const views = readViews(key);
      views[name] = capture();
      writeViews(key, views);
      refresh(name);
    });
    return { refresh };
  }

  // ── Auto-Refresh ────────────────────────────────────────────────
  function setupAutoRefresh({ refreshFn, toggleSelector, lastUpdatedSelector, intervalMs = 30000, isBusy }) {
    let timer = null;
    let userActive = false;
    let userActiveTimer = null;
    let lastFetchAt = null;

    function start() {
      stop();
      timer = setInterval(() => {
        if (document.hidden) return;
        if (userActive) return;
        if (typeof isBusy === "function" && isBusy()) return;
        refreshFn({ silent: true });
      }, intervalMs);
    }
    function stop() {
      if (timer) { clearInterval(timer); timer = null; }
    }
    function markFetched() {
      lastFetchAt = Date.now();
      updateLabel();
    }
    function updateLabel() {
      const el = typeof lastUpdatedSelector === "string" ? document.querySelector(lastUpdatedSelector) : lastUpdatedSelector;
      if (!el || !lastFetchAt) return;
      const sec = Math.floor((Date.now() - lastFetchAt) / 1000);
      el.textContent = sec < 5 ? "just now" : sec < 60 ? sec + "s ago" : Math.floor(sec / 60) + "m ago";
      el.title = "Last fetched " + new Date(lastFetchAt).toLocaleTimeString();
    }

    const toggle = typeof toggleSelector === "string" ? document.querySelector(toggleSelector) : toggleSelector;
    toggle?.addEventListener("change", (e) => { e.target.checked ? start() : stop(); });
    ["mousemove", "keydown", "scroll"].forEach((evt) => {
      document.addEventListener(evt, () => {
        userActive = true;
        clearTimeout(userActiveTimer);
        userActiveTimer = setTimeout(() => { userActive = false; }, 5000);
      }, { passive: true });
    });
    document.addEventListener("visibilitychange", () => {
      if (document.hidden) stop();
      else if (toggle?.checked !== false) start();
    });
    if (toggle?.checked !== false) start();
    setInterval(updateLabel, 5000);

    return { start, stop, markFetched };
  }

  // ── Snapshots / Trends ──────────────────────────────────────────
  function readSnapshot(key) { try { return JSON.parse(localStorage.getItem(key) || "null"); } catch { return null; } }
  function writeSnapshot(key, s) { try { localStorage.setItem(key, JSON.stringify(s)); } catch {} }
  function trendArrow(curr, prev) {
    if (prev == null || curr == null) return "";
    const delta = curr - prev;
    if (delta === 0) return `<span class="apk-trend-flat" title="No change vs last snapshot">→ 0</span>`;
    const cls = delta > 0 ? "apk-trend-up" : "apk-trend-down";
    const arrow = delta > 0 ? "▲" : "▼";
    return `<span class="${cls}" title="vs snapshot">${arrow} ${Math.abs(delta).toLocaleString()}</span>`;
  }
  function maybeTrend(snapshotKey, computeFn) {
    const prev = readSnapshot(snapshotKey);
    const curr = { ts: Date.now(), ...computeFn() };
    const showTrend = prev && Date.now() - prev.ts > 6 * 3600 * 1000;
    if (!prev || Date.now() - prev.ts > 24 * 3600 * 1000) writeSnapshot(snapshotKey, curr);
    return { prev, curr, showTrend };
  }

  // ── CSV Export ──────────────────────────────────────────────────
  function exportCsv(rows, cols, filename) {
    if (!rows || rows.length === 0) {
      if (window.showPooolToast) window.showPooolToast(null, "Nothing to export.", "danger");
      return;
    }
    const escCsv = (v) => {
      if (v == null) return "";
      const s = String(v);
      return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
    };
    const lines = [cols.map(([, h]) => h).join(",")];
    rows.forEach((row) => {
      lines.push(cols.map(([k]) => escCsv(typeof k === "function" ? k(row) : row[k])).join(","));
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

  // ── Bulk Action Bar ─────────────────────────────────────────────
  function setupBulkBar({ barSelector, countSelector, selectAllSelector, selectedIds, onClear, getPageItems, syncOnRender = true }) {
    function sync() {
      const bar = typeof barSelector === "string" ? document.querySelector(barSelector) : barSelector;
      const count = typeof countSelector === "string" ? document.querySelector(countSelector) : countSelector;
      const selectAll = typeof selectAllSelector === "string" ? document.querySelector(selectAllSelector) : selectAllSelector;
      if (count) count.textContent = `${selectedIds.size} selected`;
      if (bar) bar.classList.toggle("open", selectedIds.size > 0);
      if (selectAll && getPageItems) {
        const slice = getPageItems();
        const allSel = slice.length > 0 && slice.every((d) => selectedIds.has(d.id));
        const someSel = slice.some((d) => selectedIds.has(d.id));
        selectAll.checked = allSel;
        selectAll.indeterminate = !allSel && someSel;
      }
    }
    const sa = typeof selectAllSelector === "string" ? document.querySelector(selectAllSelector) : selectAllSelector;
    sa?.addEventListener("change", (e) => {
      if (!getPageItems) return;
      const slice = getPageItems();
      if (e.target.checked) slice.forEach((d) => selectedIds.add(d.id));
      else slice.forEach((d) => selectedIds.delete(d.id));
      onClear?.();
      sync();
    });
    return { sync };
  }

  window.AdminPageKit = {
    escHtml,
    ageSeconds,
    formatAge,
    ageColor,
    numOrNull,
    debounce,
    injectScopedCss,
    renderActionRequired,
    wireKpiClicks,
    setupSavedViews,
    setupAutoRefresh,
    readSnapshot,
    writeSnapshot,
    trendArrow,
    maybeTrend,
    exportCsv,
    setupBulkBar,
  };
})();
