/**
 * Admin Villa Operations History — Villa-Returns B4.
 * URL: /admin/villas/:asset_id/history
 *
 * Read-only forensic time-travel viewer. No new backend endpoint — reuses
 * GET /api/admin/villas/:asset_id/operations[?as_of=&year=&month=] which
 * already returns rows sorted by (period_year DESC, period_month DESC,
 * recorded_at DESC, id DESC).
 *
 * Behaviour:
 *   - Top controls: "Jump to N months ago" chips + as-of date picker.
 *   - Monthly grid: one cell per (year, month) the villa has any data for,
 *     showing the latest non-superseded row's status (clipped to as-of).
 *   - Click a cell: expand the supersession chain with field-level diff
 *     between consecutive rows (oldest → newest).
 */

(function () {
  // Fields that are interesting to diff. Excluding _usd derived (frozen at publish)
  // and timestamps. Adjust as needed.
  const DIFF_FIELDS = [
    "gross_rental_idr_cents",
    "nights_available",
    "nights_booked",
    "expense_cleaning_idr_cents",
    "expense_maintenance_idr_cents",
    "expense_utilities_idr_cents",
    "expense_staff_idr_cents",
    "expense_pool_garden_idr_cents",
    "expense_pest_idr_cents",
    "expense_other_idr_cents",
    "ota_fees_idr_cents",
    "payment_fees_idr_cents",
    "refunds_idr_cents",
    "mgmt_fee_idr_cents",
    "total_opex_idr_cents",
    "net_rental_income_idr_cents",
    "reserve_applied_idr_cents",
    "platform_fee_idr_cents",
    "withholding_idr_cents",
    "distributable_idr_cents",
    "status",
    "correction_reason",
  ];

  let assetId = null;
  let asOfIso = null; // YYYY-MM-DD or null
  let allRows = []; // raw rows from API
  let selectedPeriod = null; // { year, month }

  document.addEventListener("DOMContentLoaded", () => {
    parseUrl();
    wireControls();
    refresh();
  });

  function parseUrl() {
    const parts = window.location.pathname.split("/").filter(Boolean);
    // /admin/villas/<asset>/history
    assetId = parts[2];
    document.getElementById("vh-breadcrumb").textContent =
      `Asset ${assetId.slice(0, 8)}…`;
  }

  function wireControls() {
    document.querySelectorAll(".vh-jump-btn").forEach((b) => {
      b.addEventListener("click", () => {
        document.querySelectorAll(".vh-jump-btn").forEach((o) => o.classList.remove("active"));
        b.classList.add("active");
        const j = b.dataset.jump;
        if (j === "now") {
          asOfIso = null;
        } else {
          const d = new Date();
          d.setUTCMonth(d.getUTCMonth() - parseInt(j, 10));
          asOfIso = d.toISOString().slice(0, 10);
        }
        const asofEl = document.getElementById("vh-asof");
        if (asofEl) asofEl.value = asOfIso || "";
        refresh();
      });
    });
    document.getElementById("vh-asof").addEventListener("change", (e) => {
      asOfIso = e.target.value || null;
      document.querySelectorAll(".vh-jump-btn").forEach((o) => o.classList.remove("active"));
      if (!asOfIso) document.querySelector('.vh-jump-btn[data-jump="now"]')?.classList.add("active");
      refresh();
    });
  }

  async function refresh() {
    const banner = document.getElementById("vh-asof-banner");
    const display = document.getElementById("vh-asof-display");
    if (banner) banner.style.display = asOfIso ? "block" : "none";
    if (display && asOfIso) display.textContent = asOfIso;

    try {
      let url = `/api/admin/villas/${encodeURIComponent(assetId)}/operations`;
      if (asOfIso) url += `?as_of=${encodeURIComponent(asOfIso + "T23:59:59Z")}`;
      const resp = await fetch(url);
      if (!resp.ok) throw new Error(await responseError(resp));
      allRows = await resp.json();
      renderGrid();
      if (selectedPeriod) renderChain();
    } catch (err) {
      document.getElementById("vh-grid").innerHTML =
        `<div class="vh-empty" style="color: var(--admin-danger, #dc2626);">Failed to load: ${escapeHtml(err.message)}</div>`;
    }
  }

  function renderGrid() {
    const grid = document.getElementById("vh-grid");
    grid.innerHTML = "";

    // Build a map of (year, month) → array of rows (rows are already DESC by recorded_at, id).
    const byPeriod = new Map();
    for (const r of allRows) {
      const k = `${r.period_year}-${String(r.period_month).padStart(2, "0")}`;
      if (!byPeriod.has(k)) byPeriod.set(k, []);
      byPeriod.get(k).push(r);
    }

    if (byPeriod.size === 0) {
      grid.innerHTML = `<div class="vh-empty">No operations data within current as-of window.</div>`;
      return;
    }

    // Sort periods desc.
    const periods = Array.from(byPeriod.keys()).sort().reverse();
    for (const p of periods) {
      const rows = byPeriod.get(p);
      const latest = rows[0]; // already sorted DESC
      const cell = document.createElement("div");
      cell.className = "vh-cell";
      if (selectedPeriod && `${selectedPeriod.year}-${String(selectedPeriod.month).padStart(2, "0")}` === p) {
        cell.classList.add("active");
      }
      cell.innerHTML = `
        <div class="vh-cell-period">${escapeHtml(p)}</div>
        <div class="vh-cell-meta">${rows.length} row${rows.length === 1 ? "" : "s"} · ${Number(latest.distributable_idr_cents).toLocaleString()} IDR</div>
        <span class="vh-badge ${escapeAttr(latest.status)}">${escapeHtml(latest.status)}</span>
      `;
      cell.addEventListener("click", () => {
        selectedPeriod = { year: latest.period_year, month: latest.period_month };
        renderGrid();
        renderChain();
      });
      grid.appendChild(cell);
    }
  }

  function renderChain() {
    const section = document.getElementById("vh-chain-section");
    const title = document.getElementById("vh-chain-title");
    const chain = document.getElementById("vh-chain");
    if (!selectedPeriod) {
      section.style.display = "none";
      return;
    }
    section.style.display = "block";
    title.textContent = `${selectedPeriod.year}-${String(selectedPeriod.month).padStart(2, "0")}`;

    const rows = allRows
      .filter((r) => r.period_year === selectedPeriod.year && r.period_month === selectedPeriod.month)
      .slice()
      .sort((a, b) => new Date(a.recorded_at) - new Date(b.recorded_at) || a.id - b.id); // oldest → newest

    if (rows.length === 0) {
      chain.innerHTML = `<div class="vh-empty">No rows for this period in current as-of window.</div>`;
      return;
    }

    chain.innerHTML = "";
    let prev = null;
    for (const r of rows) {
      const block = document.createElement("div");
      block.className = "vh-chain-row";

      const tdWhen = document.createElement("div");
      tdWhen.className = "vh-chain-when";
      tdWhen.innerHTML = `<div>${escapeHtml(formatDateTime(r.recorded_at))}</div><div style="font-size: 10px;">log_id ${r.id}</div>`;
      block.appendChild(tdWhen);

      const tdStatus = document.createElement("div");
      tdStatus.className = "vh-chain-status";
      tdStatus.innerHTML = `<span class="vh-badge ${escapeAttr(r.status)}">${escapeHtml(r.status)}</span>` +
        (r.supersedes_id ? `<div style="margin-top: 6px; font-size: 10px; color: var(--admin-text-muted, #6b7280);">supersedes #${r.supersedes_id}</div>` : "");
      block.appendChild(tdStatus);

      const tdDiff = document.createElement("div");
      tdDiff.className = "vh-chain-diff";
      tdDiff.innerHTML = renderDiff(prev, r);
      block.appendChild(tdDiff);

      chain.appendChild(block);
      prev = r;
    }
  }

  function renderDiff(prev, curr) {
    if (!prev) {
      // First row — show key initial values.
      const lines = [];
      for (const k of ["status", "gross_rental_idr_cents", "distributable_idr_cents", "submitted_by"]) {
        if (curr[k] == null) continue;
        lines.push(`<div class="vh-diff-row"><span class="vh-diff-key">${escapeHtml(k)}</span><span class="vh-diff-new">${escapeHtml(String(curr[k]))}</span></div>`);
      }
      lines.unshift(`<div style="font-size: 11px; color: var(--admin-text-muted, #6b7280); margin-bottom: 4px;">Initial row — no prior to diff against</div>`);
      return lines.join("");
    }

    const lines = [];
    for (const k of DIFF_FIELDS) {
      const a = prev[k];
      const b = curr[k];
      if (a === b) continue;
      // Normalise null/undefined for comparison.
      if ((a == null && b == null)) continue;
      lines.push(
        `<div class="vh-diff-row">
          <span class="vh-diff-key">${escapeHtml(k)}</span>
          <span class="vh-diff-old">${escapeHtml(String(a == null ? "∅" : a))}</span>
          <span class="vh-diff-arrow">→</span>
          <span class="vh-diff-new">${escapeHtml(String(b == null ? "∅" : b))}</span>
        </div>`
      );
    }
    if (!lines.length) {
      return `<span style="color: var(--admin-text-muted, #6b7280);">(no field changes vs prior)</span>`;
    }
    return lines.join("");
  }

  function formatDateTime(iso) {
    if (!iso) return "—";
    try {
      const d = new Date(iso);
      return `${d.toISOString().slice(0, 10)} ${d.toISOString().slice(11, 19)}`;
    } catch {
      return iso;
    }
  }

  async function responseError(resp) {
    try { const b = await resp.json(); return b.error || b.message || `HTTP ${resp.status}`; }
    catch { return `HTTP ${resp.status}`; }
  }

  function escapeHtml(s) {
    return String(s == null ? "" : s).replace(/[&<>"']/g, (c) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    }[c]));
  }
  function escapeAttr(s) { return escapeHtml(s); }
})();
