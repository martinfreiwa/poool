/**
 * Developer Annual Data — Villa-Returns C3.
 * URL: /developer/villas/:asset_id/annual/:year
 *
 * Four sections:
 *   1. Annual rollup (read-only) — GET /api/developer/villas/:id/annual/:year/summary
 *   2. CapEx submit + list — POST/GET /api/developer/villas/:id/capex
 *   3. Forecast suggestion submit + list —
 *      POST /api/developer/villas/:id/forecast/:year/suggest
 *      GET  /api/developer/villas/:id/forecast/:year/suggestions
 *   4. Annual documents (tax statement / report) upload + list —
 *      POST /api/developer/villas/:id/annual/:year/documents  (multipart file + doc_type)
 *      GET  /api/developer/villas/:id/annual/:year/documents
 */

(function () {
  let assetId = null;
  let year = null;

  // capex / forecast-suggestion / annual-document status → ds-badge variant.
  const STATUS_VARIANT = {
    submitted: "info",
    draft: "neutral",
    pending: "warning",
    approved: "success",
    accepted: "success",
    rejected: "danger",
    discarded: "neutral",
  };

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
    const back = document.getElementById("dad-back");
    if (back) back.href = `/developer/asset-detail?id=${encodeURIComponent(assetId)}`;
  }

  function wireHandlers() {
    document.getElementById("btn-capex-submit").addEventListener("click", submitCapex);
    document.getElementById("btn-forecast-submit").addEventListener("click", submitForecast);
    document.getElementById("btn-doc-upload").addEventListener("click", uploadDoc);
  }

  async function hydrate() {
    await Promise.all([loadSummary(), loadCapex(), loadForecasts(), loadDocs()]);
  }

  // ─── Annual rollup ───────────────────────────────────────────

  async function loadSummary() {
    const out = document.getElementById("dad-summary");
    try {
      const r = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/annual/${year}/summary`);
      if (!r.ok) throw new Error(await responseError(r));
      const s = await r.json();
      const kv = (k, v) =>
        `<div class="ds-flex ds-justify-between ds-gap-12 ds-mb-8"><span class="ds-text-body ds-text--muted">${k}</span><span class="ds-text-body--semibold">${v}</span></div>`;
      out.innerHTML =
        kv("Year", s.forecast_year) +
        kv("Months with published data", s.months_published) +
        kv("Total distributable (IDR)", Number(s.total_distributable_idr_cents).toLocaleString()) +
        kv("Total distributable (USD)", formatUsd(s.total_distributable_usd_cents)) +
        kv("Total net rental (IDR)", Number(s.total_net_rental_idr_cents).toLocaleString()) +
        kv("Approved CapEx events", `${s.approved_capex_count} · ${Number(s.approved_capex_idr_cents).toLocaleString()} IDR`);
    } catch (err) {
      out.innerHTML = `<p class="ds-form-error">${escapeHtml(err.message)}</p>`;
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
        list.innerHTML = `<p class="ds-text-caption ds-text--muted">No CapEx submitted for ${year} yet.</p>`;
        return;
      }
      list.innerHTML = "";
      for (const r of rows) {
        const el = document.createElement("div");
        el.className = "ds-flex ds-justify-between ds-items-center ds-gap-12 ds-mb-8";
        el.innerHTML = `
          <div>
            <div class="ds-text-body">${escapeHtml(r.description)}</div>
            <div class="ds-text-caption ds-text--muted">${escapeHtml(r.event_date)} · ${escapeHtml((r.category || '').replace(/_/g, ' '))}</div>
          </div>
          <div class="ds-flex ds-items-center ds-gap-8">
            <span class="ds-text-body--semibold">${Number(r.amount_idr_cents).toLocaleString()}</span>
            <span class="ds-badge ds-badge--${STATUS_VARIANT[r.status] || 'neutral'}">${escapeHtml(r.status)}</span>
          </div>
        `;
        list.appendChild(el);
      }
    } catch (err) {
      list.innerHTML = `<p class="ds-form-error">${escapeHtml(err.message)}</p>`;
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
        list.innerHTML = `<p class="ds-text-caption ds-text--muted">No suggestions submitted for ${year} yet.</p>`;
        return;
      }
      list.innerHTML = "";
      for (const r of rows) {
        const el = document.createElement("div");
        el.className = "ds-flex ds-justify-between ds-items-center ds-gap-12 ds-mb-8";
        const occ = r.projected_occupancy_bps != null ? `${(r.projected_occupancy_bps / 100).toFixed(2)}%` : "—";
        const adr = r.projected_adr_idr_cents ? Number(r.projected_adr_idr_cents).toLocaleString() + ' IDR' : '—';
        el.innerHTML = `
          <div>
            <div class="ds-text-body">${escapeHtml(r.notes || "(no notes)")}</div>
            <div class="ds-text-caption ds-text--muted">${escapeHtml(formatDate(r.submitted_at))} · Occupancy ${occ}, ADR ${adr}</div>
          </div>
          <span class="ds-badge ds-badge--${STATUS_VARIANT[r.status] || 'neutral'}">${escapeHtml(r.status)}</span>
        `;
        list.appendChild(el);
      }
    } catch (err) {
      list.innerHTML = `<p class="ds-form-error">${escapeHtml(err.message)}</p>`;
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

  // ─── Annual documents (tax statement / report) ───────────────

  async function loadDocs() {
    const list = document.getElementById("dad-doc-list");
    try {
      const r = await fetch(
        `/api/developer/villas/${encodeURIComponent(assetId)}/annual/${year}/documents`
      );
      if (!r.ok) throw new Error(await responseError(r));
      const rows = await r.json();
      if (!rows.length) {
        list.innerHTML = `<p class="ds-text-caption ds-text--muted">No documents uploaded for ${year} yet.</p>`;
        return;
      }
      list.innerHTML = "";
      for (const d of rows) {
        const el = document.createElement("div");
        el.className = "ds-flex ds-justify-between ds-items-center ds-gap-12 ds-mb-8";
        const raw = String(d.doc_type || "other").replace(/_/g, " ");
        const label = raw.charAt(0).toUpperCase() + raw.slice(1);
        const href = `/api/documents/${encodeURIComponent(d.document_id)}/download`;
        el.innerHTML = `
          <div>
            <div class="ds-text-body">${escapeHtml(label)}</div>
            <div class="ds-text-caption ds-text--muted">${escapeHtml(formatDate(d.created_at))}</div>
          </div>
          <a href="${escapeAttr(href)}" target="_blank" rel="noopener" class="ds-btn ds-btn--secondary ds-btn--sm">Download</a>
        `;
        list.appendChild(el);
      }
    } catch (err) {
      list.innerHTML = `<p class="ds-form-error">${escapeHtml(err.message)}</p>`;
    }
  }

  // Single combined upload-and-link call — the server uploads to GCS,
  // inserts asset_documents under the generic 'financial' type, and links
  // into villa_annual_documents with the chosen subtype.
  async function uploadDoc() {
    const f = document.getElementById("dad-doc-form").elements;
    const err = document.getElementById("doc-error");
    err.textContent = "";
    const file = f["file"].files[0];
    if (!file) {
      err.textContent = "Choose a file to upload.";
      return;
    }
    const btn = document.getElementById("btn-doc-upload");
    btn.disabled = true;
    btn.textContent = "Uploading…";
    try {
      const fd = new FormData();
      fd.append("file", file);
      fd.append("doc_type", f["doc_type"].value);
      const r = await fetch(
        `/api/developer/villas/${encodeURIComponent(assetId)}/annual/${year}/documents`,
        { method: "POST", headers: csrfHeaders(), body: fd }
      );
      if (!r.ok) throw new Error(await responseError(r));
      f["file"].value = "";
      await loadDocs();
    } catch (e) {
      err.textContent = `Upload failed: ${e.message}`;
    } finally {
      btn.disabled = false;
      btn.textContent = "Upload & link";
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
