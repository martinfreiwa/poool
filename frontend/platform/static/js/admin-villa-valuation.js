/**
 * Admin Villa Valuation Entry — Villa-Returns P2.5.
 * URLs:
 *   /admin/villas/:asset_id/valuations/new
 *   /admin/villas/:asset_id/valuations/:val_id/edit
 *
 * Calls:
 *   POST  /api/admin/villas/:asset_id/valuations
 *   PUT   /api/admin/villas/:asset_id/valuations/:val_id
 *   PUT   /api/admin/villas/:asset_id/valuations/:val_id/{submit,approve,publish,reject}
 *   GET   /api/admin/villas/:asset_id/valuations/nav-preview?valuation_idr_cents=...
 */

let assetId = null;
let valId = null;
let currentRow = null;
let navTimer = null;

document.addEventListener("DOMContentLoaded", () => {
  parseUrl();
  setupHandlers();
  hydrate();
});

function parseUrl() {
  // /admin/villas/<asset>/valuations/new OR /admin/villas/<asset>/valuations/<id>/edit
  const parts = window.location.pathname.split("/").filter(Boolean);
  assetId = parts[2];
  if (parts[4] === "new") {
    valId = null;
    document.getElementById("vv-mode-badge").textContent = "new";
  } else {
    valId = parts[4];
    document.getElementById("vv-mode-badge").textContent = "edit";
  }
  document.getElementById("vv-breadcrumb").textContent = `Asset ${assetId.slice(0, 8)}…`;
}

function setupHandlers() {
  const form = document.getElementById("vv-form");
  form.addEventListener("input", (e) => {
    if (e.target?.name === "valuation_idr_cents") {
      schedulePreview();
    }
  });
  document.getElementById("btn-save-draft").addEventListener("click", saveDraft);
  document.getElementById("btn-submit").addEventListener("click", submitForApproval);
  document.getElementById("btn-approve").addEventListener("click", approve);
  document.getElementById("btn-publish").addEventListener("click", publish);
  document.getElementById("btn-reject").addEventListener("click", reject);
  document.getElementById("vv-comp-add").addEventListener("click", () => addComparableRow());
}

async function hydrate() {
  if (valId) {
    try {
      const resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/valuations`);
      if (!resp.ok) throw new Error(await responseError(resp));
      const all = await resp.json();
      currentRow = all.find((r) => String(r.id) === String(valId)) || null;
      if (currentRow) {
        fillForm(currentRow);
        reflectStatus(currentRow.status);
      }
    } catch (err) {
      showError(`Failed to load: ${err.message}`);
    }
  }
  await previewNav();
}

function fillForm(row) {
  const f = document.getElementById("vv-form").elements;
  if (row.valuation_date) f["valuation_date"].value = row.valuation_date;
  if (row.valuation_idr_cents != null) f["valuation_idr_cents"].value = row.valuation_idr_cents;
  if (row.valuation_method) f["valuation_method"].value = row.valuation_method;
  if (row.appraiser_name) f["appraiser_name"].value = row.appraiser_name;
  if (row.appraiser_user_id) f["appraiser_user_id"].value = row.appraiser_user_id;
  if (row.evidence_doc_id) f["evidence_doc_id"].value = row.evidence_doc_id;
  if (row.notes) f["notes"].value = row.notes;
  if (row.correction_reason) f["correction_reason"].value = row.correction_reason;
  // Render comparables.
  const list = document.getElementById("vv-comparables-list");
  list.innerHTML = "";
  if (Array.isArray(row.comparables)) {
    for (const c of row.comparables) addComparableRow(c);
  }
}

function reflectStatus(status) {
  const banner = document.getElementById("vv-status");
  banner.style.display = "block";
  banner.classList.toggle("rejected", status === "rejected");
  let txt = `Status: ${status}`;
  if (status === "rejected" && currentRow?.rejected_reason) txt += ` — ${currentRow.rejected_reason}`;
  banner.textContent = txt;

  document.getElementById("btn-approve").disabled = status !== "submitted";
  document.getElementById("btn-publish").disabled = status !== "approved";
  document.getElementById("btn-reject").disabled = status !== "submitted";

  if (currentRow?.supersedes_id) {
    document.getElementById("vv-correction-section").style.display = "";
  }
}

function gatherPayload() {
  const f = document.getElementById("vv-form").elements;
  return {
    valuation_date: f["valuation_date"].value || null,
    valuation_idr_cents: parseInt(f["valuation_idr_cents"].value || "0", 10),
    valuation_method: f["valuation_method"].value,
    appraiser_name: f["appraiser_name"].value || null,
    appraiser_user_id: f["appraiser_user_id"].value || null,
    evidence_doc_id: f["evidence_doc_id"].value || null,
    comparables: collectComparables(),
    notes: f["notes"].value || null,
    correction_reason: f["correction_reason"]?.value || null,
  };
}

function collectComparables() {
  const rows = document.querySelectorAll(".vv-comparable-row");
  const out = [];
  for (const r of rows) {
    const addr = r.querySelector("[data-k=address]").value.trim();
    const price = r.querySelector("[data-k=price]").value.trim();
    const date = r.querySelector("[data-k=date]").value.trim();
    if (!addr && !price && !date) continue;
    out.push({
      address: addr,
      sale_price_idr_cents: price ? parseInt(price, 10) : null,
      sale_date: date || null,
    });
  }
  return out.length ? out : null;
}

function addComparableRow(c = {}) {
  const list = document.getElementById("vv-comparables-list");
  const row = document.createElement("div");
  row.className = "vv-comparable-row";
  row.innerHTML = `
    <input type="text" data-k="address" placeholder="Address" value="${escapeAttr(c.address)}" />
    <input type="number" data-k="price"   placeholder="Price (IDR cents)" value="${escapeAttr(c.sale_price_idr_cents)}" />
    <input type="date"   data-k="date"    value="${escapeAttr(c.sale_date)}" />
    <button type="button">×</button>
  `;
  row.querySelector("button").addEventListener("click", () => row.remove());
  list.appendChild(row);
}

function schedulePreview() {
  clearTimeout(navTimer);
  navTimer = setTimeout(previewNav, 250);
}

async function previewNav() {
  const v = parseInt(document.querySelector("[name=valuation_idr_cents]")?.value || "0", 10);
  if (!v) {
    document.getElementById("vv-pv-tokpct").value = "";
    document.getElementById("vv-pv-tokpool").value = "";
    document.getElementById("vv-pv-nav").value = "";
    return;
  }
  try {
    const resp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/valuations/nav-preview?valuation_idr_cents=${v}`
    );
    if (!resp.ok) return;
    const p = await resp.json();
    document.getElementById("vv-pv-tokpct").value =
      p.tokenized_pct_bps ? `${(p.tokenized_pct_bps / 100).toFixed(2)} %` : "(unset)";
    document.getElementById("vv-pv-tokpool").value = Number(p.tokens_in_pool).toLocaleString();
    document.getElementById("vv-pv-nav").value = Number(p.nav_token_idr_cents).toLocaleString();
  } catch {
    // Silent — preview is optional.
  }
}

async function saveDraft() {
  showError("");
  try {
    const payload = gatherPayload();
    let resp;
    if (valId) {
      resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/valuations/${valId}`, {
        method: "PUT",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify(payload),
      });
    } else {
      resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/valuations`, {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify(payload),
      });
    }
    if (!resp.ok) throw new Error(await responseError(resp));
    currentRow = await resp.json();
    valId = currentRow.id;
    reflectStatus(currentRow.status);
    if (window.location.pathname.endsWith("/new")) {
      // Switch URL to /edit so reloads keep state.
      const newUrl = `/admin/villas/${encodeURIComponent(assetId)}/valuations/${valId}/edit`;
      window.history.replaceState(null, "", newUrl);
      document.getElementById("vv-mode-badge").textContent = "edit";
    }
  } catch (err) {
    showError(`Save failed: ${err.message}`);
  }
}

async function submitForApproval() {
  await saveDraft();
  if (!valId) return;
  await transition("submit");
}
async function approve() { await transition("approve"); }
async function publish() { await transition("publish"); }

async function transition(action) {
  showError("");
  try {
    const resp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/valuations/${valId}/${action}`,
      { method: "PUT", headers: csrfHeaders() }
    );
    if (!resp.ok) throw new Error(await responseError(resp));
    currentRow = await resp.json();
    reflectStatus(currentRow.status);
  } catch (err) {
    showError(`${action} failed: ${err.message}`);
  }
}

async function reject() {
  const reason = document.getElementById("vv-reject-reason").value.trim();
  if (!reason) {
    showError("Rejection reason is required.");
    return;
  }
  try {
    const resp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/valuations/${valId}/reject`,
      {
        method: "PUT",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({ reason }),
      }
    );
    if (!resp.ok) throw new Error(await responseError(resp));
    currentRow = await resp.json();
    reflectStatus(currentRow.status);
  } catch (err) {
    showError(`Reject failed: ${err.message}`);
  }
}

// ─── Helpers ─────────────────────────────────────────────────

function showError(msg) { document.getElementById("vv-error").textContent = msg; }

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

function escapeAttr(s) {
  if (s == null) return "";
  return String(s).replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])
  );
}
