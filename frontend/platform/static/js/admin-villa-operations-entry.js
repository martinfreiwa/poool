/**
 * Admin Villa Operations Entry — Villa-Returns P2.
 * URL: /admin/villas/:asset_id/operations/:year/:month
 * Optional query: ?log_id=<id>&mode=entry|review|override
 *
 * Calls:
 *   POST   /api/admin/villas/:asset_id/operations
 *   PUT    /api/admin/villas/:asset_id/operations/:log_id
 *   PUT    /api/admin/villas/:asset_id/operations/:log_id/{submit,approve,publish,reject}
 *   GET    /api/admin/villas/:asset_id/operations?year=&month=
 *   POST   /api/admin/assets/:asset_id/documents              (upload file)
 *   POST   /api/admin/villas/:asset_id/operations/:log_id/documents  (link to period)
 *   GET    /api/admin/villas/:asset_id/operations/:log_id/documents  (list linked)
 */

let assetId = null;
let year = null;
let month = null;
let mode = "entry";
let logId = null;
let currentRow = null;

document.addEventListener("DOMContentLoaded", () => {
  parseUrl();
  setupHandlers();
  hydrate();
});

function parseUrl() {
  // /admin/villas/<asset_id>/operations/<year>/<month>
  const parts = window.location.pathname.split("/").filter(Boolean);
  // parts = ["admin", "villas", "<asset>", "operations", "<year>", "<month>"]
  assetId = parts[2];
  year = parseInt(parts[4], 10);
  month = parseInt(parts[5], 10);
  const qs = new URLSearchParams(window.location.search);
  mode = qs.get("mode") || "entry";
  logId = qs.get("log_id") || null;

  document.getElementById("vop-mode-badge").textContent = mode;
  document.getElementById("vop-breadcrumb").textContent =
    `Asset ${assetId.slice(0, 8)}… · Period ${year}-${String(month).padStart(2, "0")}`;
}

function setupHandlers() {
  // Live preview as user types.
  document
    .getElementById("vop-form")
    .addEventListener("input", () => recompute());

  document
    .getElementById("btn-save-draft")
    .addEventListener("click", saveDraft);
  document.getElementById("btn-submit").addEventListener("click", submitForApproval);
  document.getElementById("btn-approve").addEventListener("click", approve);
  document.getElementById("btn-publish").addEventListener("click", publish);
  document.getElementById("btn-reject").addEventListener("click", reject);
  document.getElementById("btn-upload-doc").addEventListener("click", uploadDocument);
}

async function hydrate() {
  try {
    const url = `/api/admin/villas/${encodeURIComponent(assetId)}/operations?year=${year}&month=${month}`;
    const resp = await fetch(url);
    if (!resp.ok) throw new Error(await responseError(resp));
    const rows = await resp.json();

    if (logId) {
      currentRow = rows.find((r) => String(r.id) === String(logId)) || null;
    } else if (rows.length > 0 && rows[0].status !== "superseded") {
      currentRow = rows[0]; // latest non-superseded
    }
    if (currentRow) {
      logId = currentRow.id;
      fillFormFromRow(currentRow);
      reflectStatus(currentRow.status);
    }
    recompute();
    reflectDocsSection();
  } catch (err) {
    showError(`Failed to load: ${err.message}`);
  }
}

function fillFormFromRow(row) {
  const form = document.getElementById("vop-form");
  const fields = [
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
    "reserve_override_idr_cents",
    "correction_reason",
  ];
  for (const name of fields) {
    const el = form.elements[name];
    if (el && row[name] != null) el.value = row[name];
  }
}

function reflectStatus(status) {
  const line = document.getElementById("vop-status-line");
  line.style.display = "block";
  line.textContent = `Status: ${status}`;
  line.classList.toggle("vop-rejected", status === "draft" && currentRow?.rejected_reason);
  if (status === "draft" && currentRow?.rejected_reason) {
    line.textContent += ` — Rejected: ${currentRow.rejected_reason}`;
  }

  const btnApprove = document.getElementById("btn-approve");
  const btnPublish = document.getElementById("btn-publish");
  const btnReject = document.getElementById("btn-reject");

  btnApprove.disabled = status !== "submitted";
  btnPublish.disabled = status !== "approved";
  btnReject.disabled = status !== "submitted";
}

function gatherPayload() {
  const f = document.getElementById("vop-form").elements;
  const num = (n) => parseInt(f[n].value || "0", 10);
  const opt = (n) => (f[n].value ? parseInt(f[n].value, 10) : null);
  return {
    period_year: year,
    period_month: month,
    currency_code: "IDR",
    gross_rental_idr_cents: num("gross_rental_idr_cents"),
    nights_available: num("nights_available"),
    nights_booked: num("nights_booked"),
    expense_cleaning_idr_cents: num("expense_cleaning_idr_cents"),
    expense_maintenance_idr_cents: num("expense_maintenance_idr_cents"),
    expense_utilities_idr_cents: num("expense_utilities_idr_cents"),
    expense_staff_idr_cents: num("expense_staff_idr_cents"),
    expense_pool_garden_idr_cents: num("expense_pool_garden_idr_cents"),
    expense_pest_idr_cents: num("expense_pest_idr_cents"),
    expense_other_idr_cents: num("expense_other_idr_cents"),
    ota_fees_idr_cents: num("ota_fees_idr_cents"),
    payment_fees_idr_cents: num("payment_fees_idr_cents"),
    refunds_idr_cents: num("refunds_idr_cents"),
    mgmt_fee_idr_cents: num("mgmt_fee_idr_cents"),
    reserve_override_idr_cents: opt("reserve_override_idr_cents"),
    correction_reason: f["correction_reason"].value || null,
  };
}

function recompute() {
  const p = gatherPayload();
  const opex =
    p.expense_cleaning_idr_cents +
    p.expense_maintenance_idr_cents +
    p.expense_utilities_idr_cents +
    p.expense_staff_idr_cents +
    p.expense_pool_garden_idr_cents +
    p.expense_pest_idr_cents +
    p.expense_other_idr_cents +
    p.ota_fees_idr_cents +
    p.payment_fees_idr_cents +
    p.mgmt_fee_idr_cents -
    p.refunds_idr_cents;
  const net = p.gross_rental_idr_cents - opex;
  const occupancy =
    p.nights_available > 0
      ? Math.floor((p.nights_booked * 10000) / p.nights_available)
      : 0;
  const adr = p.nights_booked > 0 ? Math.floor(p.gross_rental_idr_cents / p.nights_booked) : 0;
  document.getElementById("vop-pv-occupancy").value = `${(occupancy / 100).toFixed(2)} %`;
  document.getElementById("vop-pv-adr").value = adr.toLocaleString();
  document.getElementById("vop-pv-opex").value = opex.toLocaleString();
  document.getElementById("vop-pv-net").value = net.toLocaleString();
  // Reserve / platform / withholding need server config — server is authority on publish.
}

async function saveDraft() {
  showError("");
  try {
    const payload = gatherPayload();
    let resp;
    if (logId) {
      resp = await fetch(
        `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${logId}`,
        { method: "PUT", headers: csrfHeaders({ "Content-Type": "application/json" }), body: JSON.stringify(payload) }
      );
    } else {
      resp = await fetch(
        `/api/admin/villas/${encodeURIComponent(assetId)}/operations`,
        { method: "POST", headers: csrfHeaders({ "Content-Type": "application/json" }), body: JSON.stringify(payload) }
      );
    }
    if (!resp.ok) throw new Error(await responseError(resp));
    currentRow = await resp.json();
    logId = currentRow.id;
    reflectStatus(currentRow.status);
    reflectDocsSection();
  } catch (err) {
    showError(`Save failed: ${err.message}`);
  }
}

async function submitForApproval() {
  await saveDraft();
  if (!logId) return;
  await transition("submit");
}
async function approve() { await transition("approve"); }
async function publish() { await transition("publish"); }

async function transition(action) {
  showError("");
  try {
    const resp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${logId}/${action}`,
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
  const reason = document.getElementById("vop-reject-reason").value.trim();
  if (!reason) {
    showError("Rejection reason is required.");
    return;
  }
  try {
    const resp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${logId}/reject`,
      { method: "PUT", headers: csrfHeaders({ "Content-Type": "application/json" }), body: JSON.stringify({ reason }) }
    );
    if (!resp.ok) throw new Error(await responseError(resp));
    currentRow = await resp.json();
    reflectStatus(currentRow.status);
  } catch (err) {
    showError(`Reject failed: ${err.message}`);
  }
}

function showError(msg) {
  document.getElementById("vop-error").textContent = msg;
}

/* ── Period documents (receipts / invoices / statements) ─────────────── */

// Show the documents panel only once a log row exists (it needs a log_id
// to link against). Reloads the linked-docs list each time it becomes
// visible.
function reflectDocsSection() {
  const section = document.getElementById("vop-docs-section");
  if (logId) {
    section.style.display = "block";
    loadDocuments();
  } else {
    section.style.display = "none";
  }
}

async function loadDocuments() {
  const errEl = document.getElementById("vop-docs-error");
  try {
    const resp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${logId}/documents`
    );
    if (!resp.ok) throw new Error(await responseError(resp));
    renderDocuments(await resp.json());
    errEl.textContent = "";
  } catch (err) {
    errEl.textContent = `Failed to load documents: ${err.message}`;
  }
}

function renderDocuments(docs) {
  const list = document.getElementById("vop-docs-list");
  if (!Array.isArray(docs) || docs.length === 0) {
    list.innerHTML =
      '<p style="font-size: 13px; color: var(--text-muted, #6b7280); margin: 0;">No documents linked to this period yet.</p>';
    return;
  }
  list.innerHTML = docs
    .map((d) => {
      const when = new Date(d.created_at).toLocaleDateString();
      const label = String(d.doc_type || "other").replace(/_/g, " ");
      const href = `/api/documents/${encodeURIComponent(d.document_id)}/download`;
      return `<div style="display: flex; justify-content: space-between; align-items: center; padding: 8px 0; border-bottom: 1px solid var(--border, #e5e7eb);">
        <span style="font-size: 13px;"><strong style="text-transform: capitalize;">${label}</strong> &middot; linked ${when}</span>
        <a href="${href}" target="_blank" rel="noopener" class="vop-btn" style="padding: 4px 12px; font-size: 12px; text-decoration: none;">Download</a>
      </div>`;
    })
    .join("");
}

// Two-step: upload the file into asset_documents under the generic
// 'financial' type (the only operational type the CHECK constraint
// allows), then link the returned document_id to this monthly period
// with the real operational subtype in villa_period_documents.doc_type.
async function uploadDocument() {
  const errEl = document.getElementById("vop-docs-error");
  errEl.textContent = "";
  if (!logId) {
    errEl.textContent = "Save the period as a draft first.";
    return;
  }
  const fileInput = document.getElementById("vop-doc-file");
  const file = fileInput.files[0];
  if (!file) {
    errEl.textContent = "Choose a file to upload.";
    return;
  }
  const docType = document.getElementById("vop-doc-type").value;
  const btn = document.getElementById("btn-upload-doc");
  btn.disabled = true;
  btn.textContent = "Uploading…";
  try {
    const fd = new FormData();
    fd.append("file", file);
    fd.append("document_type", "financial");
    fd.append(
      "title",
      `${docType} ${year}-${String(month).padStart(2, "0")} — ${file.name}`
    );
    const upResp = await fetch(
      `/api/admin/assets/${encodeURIComponent(assetId)}/documents`,
      { method: "POST", headers: csrfHeaders(), body: fd }
    );
    if (!upResp.ok) throw new Error(await responseError(upResp));
    const uploaded = await upResp.json();
    const documentId = uploaded.document_id || uploaded.id;
    if (!documentId) throw new Error("upload did not return a document id");

    const linkResp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${logId}/documents`,
      {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify({ document_id: documentId, doc_type: docType }),
      }
    );
    if (!linkResp.ok) throw new Error(await responseError(linkResp));
    fileInput.value = "";
    await loadDocuments();
  } catch (err) {
    errEl.textContent = `Upload failed: ${err.message}`;
  } finally {
    btn.disabled = false;
    btn.textContent = "Upload & link";
  }
}

function csrfHeaders(headers = {}) {
  const value = `; ${document.cookie}`;
  const parts = value.split("; csrf_token=");
  const token = parts.length === 2 ? parts.pop().split(";").shift() : null;
  return token ? { ...headers, "X-CSRF-Token": token } : headers;
}

async function responseError(resp) {
  try {
    const body = await resp.json();
    return body.error || body.message || `HTTP ${resp.status}`;
  } catch {
    return `HTTP ${resp.status}`;
  }
}
