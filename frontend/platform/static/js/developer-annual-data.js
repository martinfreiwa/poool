/**
 * Developer Annual Data — Villa-Returns C3.
 * URL: /developer/villas/:asset_id/annual/:year
 *
 * Three sections:
 *   1. Annual rollup (read-only) — GET /api/developer/villas/:id/annual/:year/summary
 *   2. CapEx submit + list — POST/GET /api/developer/villas/:id/capex
 *   3. Forecast suggestion submit + list —
 *      POST /api/developer/villas/:id/forecast/:year/suggest
 *      GET  /api/developer/villas/:id/forecast/:year/suggestions
 */

(function () {
  let assetId = null;
  let year = null;

  document.addEventListener("DOMContentLoaded", () => {
    parseUrl();
    wireHandlers();
    hydrate();
  });

  function parseUrl() {
    // /developer/villas/<asset>/annual/<year>
    const parts = window.location.pathname.split("/").filter(Boolean);
    assetId = parts[2];
    year = parseInt(parts[4], 10);
    document.getElementById("dad-breadcrumb").textContent =
      `Asset ${assetId.slice(0, 8)}… · Year ${year}`;
  }

  function wireHandlers() {
    document.getElementById("btn-capex-submit").addEventListener("click", submitCapex);
    document.getElementById("btn-forecast-submit").addEventListener("click", submitForecast);
  }

  async function hydrate() {
    await Promise.all([loadSummary(), loadCapex(), loadForecasts()]);
  }

  // ─── Annual rollup ───────────────────────────────────────────

  async function loadSummary() {
    const out = document.getElementById("dad-summary");
    try {
      const r = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/annual/${year}/summary`);
      if (!r.ok) throw new Error(await responseError(r));
      const s = await r.json();
      out.innerHTML = `
        <div class="dad-row"><span class="dad-row-key">Year</span><span class="dad-row-val">${s.forecast_year}</span></div>
        <div class="dad-row"><span class="dad-row-key">Months with published data</span><span class="dad-row-val">${s.months_published}</span></div>
        <div class="dad-row"><span class="dad-row-key">Total distributable (IDR)</span><span class="dad-row-val">${Number(s.total_distributable_idr_cents).toLocaleString()}</span></div>
        <div class="dad-row"><span class="dad-row-key">Total distributable (USD)</span><span class="dad-row-val">${formatUsd(s.total_distributable_usd_cents)}</span></div>
        <div class="dad-row"><span class="dad-row-key">Total net rental (IDR)</span><span class="dad-row-val">${Number(s.total_net_rental_idr_cents).toLocaleString()}</span></div>
        <div class="dad-row"><span class="dad-row-key">Approved CapEx events</span><span class="dad-row-val">${s.approved_capex_count} · ${Number(s.approved_capex_idr_cents).toLocaleString()} IDR</span></div>
      `;
    } catch (err) {
      out.innerHTML = `<div class="dad-error">${escapeHtml(err.message)}</div>`;
    }
  }

  // ─── CapEx ───────────────────────────────────────────────────

  async function loadCapex() {
    const list = document.getElementById("dad-capex-list");
    try {
      const r = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/capex?year=${year}`);
      if (!r.ok) throw new Error(await responseError(r));
      const rows = await r.json();
      if (!rows.length) {
        list.innerHTML = `<div class="dad-info" style="padding: 12px 0;">No CapEx submitted for ${year} yet.</div>`;
        return;
      }
      list.innerHTML = "";
      for (const r of rows) {
        const el = document.createElement("div");
        el.className = "dad-list-item";
        el.innerHTML = `
          <div>${escapeHtml(r.event_date)}</div>
          <div>${escapeHtml(r.description)}<div style="color: var(--text-muted, #6b7280);">${escapeHtml((r.category || '').replace(/_/g, ' '))}</div></div>
          <div style="text-align: right; font-variant-numeric: tabular-nums;">${Number(r.amount_idr_cents).toLocaleString()}</div>
          <div><span class="dad-status ${escapeAttr(r.status)}">${escapeHtml(r.status)}</span></div>
        `;
        list.appendChild(el);
      }
    } catch (err) {
      list.innerHTML = `<div class="dad-error">${escapeHtml(err.message)}</div>`;
    }
  }

  async function submitCapex() {
    const f = document.getElementById("dad-capex-form").elements;
    const err = document.getElementById("capex-error");
    err.textContent = "";
    const payload = {
      event_date: f["event_date"].value,
      amount_idr_cents: parseInt(f["amount_idr_cents"].value || "0", 10),
      category: f["category"].value,
      description: f["description"].value.trim(),
      evidence_doc_id: f["evidence_doc_id"].value.trim() || null,
    };
    if (!payload.event_date || payload.amount_idr_cents <= 0 || !payload.description) {
      err.textContent = "Date, amount and description are required.";
      return;
    }
    try {
      const r = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/capex`, {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify(payload),
      });
      if (!r.ok) throw new Error(await responseError(r));
      // Reset form on success.
      f["event_date"].value = "";
      f["amount_idr_cents"].value = "";
      f["description"].value = "";
      f["evidence_doc_id"].value = "";
      await loadCapex();
    } catch (e) {
      err.textContent = e.message;
    }
  }

  // ─── Forecast suggestions ────────────────────────────────────

  async function loadForecasts() {
    const list = document.getElementById("dad-forecast-list");
    try {
      const r = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/forecast/${year}/suggestions`);
      if (!r.ok) throw new Error(await responseError(r));
      const rows = await r.json();
      if (!rows.length) {
        list.innerHTML = `<div class="dad-info" style="padding: 12px 0;">No suggestions submitted for ${year} yet.</div>`;
        return;
      }
      list.innerHTML = "";
      for (const r of rows) {
        const el = document.createElement("div");
        el.className = "dad-list-item";
        const occ = r.projected_occupancy_bps != null ? `${(r.projected_occupancy_bps / 100).toFixed(2)}%` : "—";
        el.innerHTML = `
          <div>${escapeHtml(formatDate(r.submitted_at))}</div>
          <div>${escapeHtml(r.notes || "(no notes)")}<div style="color: var(--text-muted, #6b7280);">Occupancy ${occ}, ADR ${r.projected_adr_idr_cents ? Number(r.projected_adr_idr_cents).toLocaleString() + ' IDR' : '—'}</div></div>
          <div></div>
          <div><span class="dad-status ${escapeAttr(r.status)}">${escapeHtml(r.status)}</span></div>
        `;
        list.appendChild(el);
      }
    } catch (err) {
      list.innerHTML = `<div class="dad-error">${escapeHtml(err.message)}</div>`;
    }
  }

  async function submitForecast() {
    const f = document.getElementById("dad-forecast-form").elements;
    const err = document.getElementById("forecast-error");
    err.textContent = "";
    const opt = (n) => (f[n].value ? parseInt(f[n].value, 10) : null);
    const payload = {
      projected_occupancy_bps: opt("projected_occupancy_bps"),
      projected_adr_idr_cents: opt("projected_adr_idr_cents"),
      projected_rent_growth_bps: opt("projected_rent_growth_bps"),
      projected_expense_inflation_bps: opt("projected_expense_inflation_bps"),
      projected_appreciation_bps: opt("projected_appreciation_bps"),
      projected_exit_yield_bps: opt("projected_exit_yield_bps"),
      notes: f["notes"].value.trim() || null,
    };
    try {
      const r = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/forecast/${year}/suggest`, {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify(payload),
      });
      if (!r.ok) throw new Error(await responseError(r));
      f["notes"].value = "";
      await loadForecasts();
    } catch (e) {
      err.textContent = e.message;
    }
  }

  // ─── Helpers ─────────────────────────────────────────────────

  function csrfHeaders(headers = {}) {
    const value = `; ${document.cookie}`;
    const parts = value.split("; csrf_token=");
    const token = parts.length === 2 ? parts.pop().split(";").shift() : null;
    return token ? { ...headers, "X-CSRF-Token": token } : headers;
  }

  async function responseError(resp) {
    try { const b = await resp.json(); return b.error || b.message || `HTTP ${resp.status}`; }
    catch { return `HTTP ${resp.status}`; }
  }

  function formatUsd(cents) {
    const v = Number(cents) / 100;
    return `USD ${v.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  }

  function formatDate(iso) {
    if (!iso) return "—";
    try { return new Date(iso).toISOString().slice(0, 10); } catch { return iso; }
  }

  function escapeHtml(s) {
    return String(s == null ? "" : s).replace(/[&<>"']/g, (c) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    }[c]));
  }
  function escapeAttr(s) { return escapeHtml(s); }
})();
