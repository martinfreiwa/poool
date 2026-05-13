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
