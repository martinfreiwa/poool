/**
 * Developer Operations Submit — Villa-Returns P2.
 * URL: /developer/villas/:asset_id/operations/new?year=YYYY&month=MM
 *
 * Calls:
 *   GET   /api/developer/villas/:asset_id/asset-config
 *   GET   /api/developer/villas/:asset_id/operations?year=&month=
 *   POST  /api/developer/villas/:asset_id/operations
 *   PUT   /api/developer/villas/:asset_id/operations/:log_id
 *   PUT   /api/developer/villas/:asset_id/operations/:log_id/submit
 */

let assetId = null;
let year = null;
let month = null;
let logId = null;
let currentRow = null;
let assetConfig = { reserve_pct_bps: 500, platform_pct: 0, withholding_tax_bps: 0 };

document.addEventListener("DOMContentLoaded", () => {
  parseUrl();
  setupHandlers();
  hydrate();
});

function parseUrl() {
  // /developer/villas/<asset_id>/operations/new
  const parts = window.location.pathname.split("/").filter(Boolean);
  assetId = parts[2];
  const qs = new URLSearchParams(window.location.search);
  year = parseInt(qs.get("year") || "0", 10);
  month = parseInt(qs.get("month") || "0", 10);

  document.getElementById("dop-breadcrumb").textContent =
    `Asset ${assetId.slice(0, 8)}… · Period ${year}-${String(month).padStart(2, "0")}`;
}

function setupHandlers() {
  document.getElementById("dop-form").addEventListener("input", recompute);
  document.getElementById("btn-save-draft").addEventListener("click", saveDraft);
  document.getElementById("btn-submit").addEventListener("click", submitForApproval);
}

async function hydrate() {
  try {
    const cfgResp = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/asset-config`);
    if (cfgResp.ok) assetConfig = await cfgResp.json();

    const listResp = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/operations?year=${year}&month=${month}`);
    if (!listResp.ok) throw new Error(await responseError(listResp));
    const rows = await listResp.json();
    if (rows.length > 0) {
      currentRow = rows[0];
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
  const form = document.getElementById("dop-form");
  const fields = [
    "gross_rental_idr_cents", "nights_available", "nights_booked",
    "expense_cleaning_idr_cents", "expense_maintenance_idr_cents", "expense_utilities_idr_cents",
    "expense_staff_idr_cents", "expense_pool_garden_idr_cents", "expense_pest_idr_cents",
    "expense_other_idr_cents", "ota_fees_idr_cents", "payment_fees_idr_cents",
    "refunds_idr_cents", "mgmt_fee_idr_cents",
  ];
  for (const name of fields) {
    const el = form.elements[name];
    if (el && row[name] != null) el.value = row[name];
  }
}

function reflectStatus(status) {
  const line = document.getElementById("dop-status");
  line.style.display = "block";
  let txt = `Status: ${status}`;
  if (currentRow?.rejected_reason && status === "draft") {
    txt += ` — Rejected: ${currentRow.rejected_reason}`;
    line.classList.add("dop-rejected");
  } else if (status === "submitted") {
    txt += " — awaiting admin approval";
  } else if (status === "approved") {
    txt += " — awaiting publish by admin";
  } else if (status === "published") {
    txt += " — to edit, request correction with admin";
  }
  line.textContent = txt;

  const btnDraft = document.getElementById("btn-save-draft");
  const btnSubmit = document.getElementById("btn-submit");
  const editable = status === "draft" || !status;
  btnDraft.disabled = !editable;
  btnSubmit.disabled = !editable;
  for (const el of document.getElementById("dop-form").elements) {
    if (el.tagName === "INPUT" && !el.readOnly) el.disabled = !editable;
  }
}

function gatherPayload() {
  const f = document.getElementById("dop-form").elements;
  const num = (n) => parseInt(f[n].value || "0", 10);
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
  };
}

function recompute() {
  const p = gatherPayload();
  const opex =
    p.expense_cleaning_idr_cents + p.expense_maintenance_idr_cents +
    p.expense_utilities_idr_cents + p.expense_staff_idr_cents +
    p.expense_pool_garden_idr_cents + p.expense_pest_idr_cents +
    p.expense_other_idr_cents + p.ota_fees_idr_cents +
    p.payment_fees_idr_cents + p.mgmt_fee_idr_cents - p.refunds_idr_cents;
  const net = p.gross_rental_idr_cents - opex;
  const occupancy = p.nights_available > 0 ? Math.floor((p.nights_booked * 10000) / p.nights_available) : 0;
  const adr = p.nights_booked > 0 ? Math.floor(p.gross_rental_idr_cents / p.nights_booked) : 0;
  const reserve = Math.floor((Math.max(0, net) * assetConfig.reserve_pct_bps) / 10000);
  const platform = Math.floor((Math.max(0, net) * (assetConfig.platform_pct || 0)) / 100);
  const after = Math.max(0, net - reserve - platform);
  const withholding = Math.floor((after * assetConfig.withholding_tax_bps) / 10000);
  const distributable = after - withholding;

  document.getElementById("dop-pv-occupancy").value = `${(occupancy / 100).toFixed(2)} %`;
  document.getElementById("dop-pv-adr").value = adr.toLocaleString();
  document.getElementById("dop-pv-opex").value = opex.toLocaleString();
  document.getElementById("dop-pv-net").value = net.toLocaleString();
  document.getElementById("dop-pv-reserve").value = reserve.toLocaleString();
  document.getElementById("dop-pv-platform").value = platform.toLocaleString();
  document.getElementById("dop-pv-withholding").value = withholding.toLocaleString();
  document.getElementById("dop-pv-distributable").value = distributable.toLocaleString();
}

async function saveDraft() {
  showError("");
  try {
    const payload = gatherPayload();
    let resp;
    if (logId) {
      resp = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/operations/${logId}`,
        { method: "PUT", headers: csrfHeaders({ "Content-Type": "application/json" }), body: JSON.stringify(payload) });
    } else {
      resp = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/operations`,
        { method: "POST", headers: csrfHeaders({ "Content-Type": "application/json" }), body: JSON.stringify(payload) });
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
  try {
    const resp = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/operations/${logId}/submit`,
      { method: "PUT", headers: csrfHeaders() });
    if (!resp.ok) throw new Error(await responseError(resp));
    currentRow = await resp.json();
    reflectStatus(currentRow.status);
  } catch (err) {
    showError(`Submit failed: ${err.message}`);
  }
}

function showError(msg) { document.getElementById("dop-error").textContent = msg; }

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
