/**
 * Admin Villa Deduction Policy — Villa-Returns B3.
 * URL: /admin/villas/:asset_id/deduction-policy
 *
 * Calls:
 *   GET  /api/villa-expense-categories
 *   GET  /api/admin/villas/:asset_id/deduction-policies
 *   POST /api/admin/villas/:asset_id/deduction-policies
 */

(function () {
  let assetId = null;

  document.addEventListener("DOMContentLoaded", () => {
    parseUrl();
    document.getElementById("vdp-save").addEventListener("click", savePolicy);
    hydrate();
  });

  function parseUrl() {
    const parts = window.location.pathname.split("/").filter(Boolean);
    // /admin/villas/<asset>/deduction-policy
    assetId = parts[2];
    document.getElementById("vdp-breadcrumb").textContent =
      `Asset ${assetId.slice(0, 8)}…`;
  }

  async function hydrate() {
    await Promise.all([loadCategories(), loadHistory()]);
  }

  async function loadCategories() {
    const grid = document.getElementById("vdp-cat-grid");
    try {
      const r = await fetch("/api/villa-expense-categories");
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const cats = await r.json();
      grid.innerHTML = "";
      for (const c of cats) {
        const cell = document.createElement("div");
        cell.className = "vdp-cat-cell";
        cell.dataset.code = c.code;
        cell.innerHTML = `
          <input type="checkbox" id="vdp-cat-${escapeAttr(c.code)}" data-k="enabled" ${c.is_default ? "checked" : ""} />
          <label for="vdp-cat-${escapeAttr(c.code)}" style="flex: 1;">
            ${escapeHtml(c.label)}
            <span class="vdp-cat-code">${escapeHtml(c.code)}</span>
          </label>
          <input type="number" min="0" max="10000" step="1" placeholder="cap bps" data-k="cap" title="Optional cap as bps of gross rental (10000 = 100%)" />
        `;
        grid.appendChild(cell);
      }
    } catch (err) {
      grid.innerHTML = `<div style="grid-column: 1 / -1; color: var(--admin-danger, #dc2626);">Failed: ${escapeHtml(err.message)}</div>`;
    }
  }

  async function loadHistory() {
    const list = document.getElementById("vdp-history");
    try {
      const r = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/deduction-policies`);
      if (!r.ok) throw new Error(await responseError(r));
      const rows = await r.json();
      if (!rows.length) {
        list.innerHTML = `<div style="padding: 12px; text-align: center; color: var(--admin-text-muted, #6b7280);">No policy set yet — append the first below.</div>`;
        return;
      }
      list.innerHTML = "";
      for (const r of rows) {
        const el = document.createElement("div");
        el.className = "vdp-history-row";
        const codes = (r.allowed_codes || []).join(", ");
        const caps = r.per_category_cap_bps
          ? Object.entries(r.per_category_cap_bps).map(([k, v]) => `${k}=${v}bps`).join(", ")
          : null;
        el.innerHTML = `
          <div class="vdp-history-date">Effective from ${escapeHtml(r.effective_from)}</div>
          <div class="vdp-history-codes">Allowed: ${escapeHtml(codes)}</div>
          ${caps ? `<div class="vdp-history-codes">Caps: ${escapeHtml(caps)}</div>` : ""}
          ${r.notes ? `<div class="vdp-history-notes">${escapeHtml(r.notes)}</div>` : ""}
        `;
        list.appendChild(el);
      }
    } catch (err) {
      list.innerHTML = `<div style="padding: 12px; color: var(--admin-danger, #dc2626);">Failed: ${escapeHtml(err.message)}</div>`;
    }
  }

  async function savePolicy() {
    const errEl = document.getElementById("vdp-error");
    errEl.textContent = "";
    const effective = document.getElementById("vdp-effective-from").value;
    if (!effective) {
      errEl.textContent = "Effective from date required.";
      return;
    }
    const allowed = [];
    const caps = {};
    document.querySelectorAll(".vdp-cat-cell").forEach((cell) => {
      const code = cell.dataset.code;
      const enabled = cell.querySelector('[data-k="enabled"]').checked;
      const cap = cell.querySelector('[data-k="cap"]').value;
      if (enabled) {
        allowed.push(code);
        if (cap && parseInt(cap, 10) > 0) caps[code] = parseInt(cap, 10);
      }
    });
    if (!allowed.length) {
      errEl.textContent = "At least one allowed category required.";
      return;
    }
    const payload = {
      effective_from: effective,
      allowed_codes: allowed,
      per_category_cap_bps: Object.keys(caps).length ? caps : null,
      notes: document.getElementById("vdp-notes").value.trim() || null,
    };
    try {
      const r = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/deduction-policies`, {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify(payload),
      });
      if (!r.ok) throw new Error(await responseError(r));
      document.getElementById("vdp-effective-from").value = "";
      document.getElementById("vdp-notes").value = "";
      await loadHistory();
    } catch (err) {
      errEl.textContent = err.message;
    }
  }

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

  function escapeHtml(s) {
    return String(s == null ? "" : s).replace(/[&<>"']/g, (c) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    }[c]));
  }
  function escapeAttr(s) { return escapeHtml(s); }
})();
