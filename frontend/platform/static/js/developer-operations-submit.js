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
 *   POST  /api/developer/villas/:asset_id/operations/:log_id/documents  (upload + link)
 *   GET   /api/developer/villas/:asset_id/operations/:log_id/documents  (list linked)
 */

let assetId = null;
let year = null;
let month = null;
let logId = null;
let existingLogId = null; // edit mode marker (URL contains /operations/<log_id> instead of /new)
let currentRow = null;
let assetConfig = { reserve_pct_bps: 500, platform_pct: 0, withholding_tax_bps: 0 };
let customExpenseCount = 0;
let maxDaysInMonth = 31;

const MONTHS = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];

document.addEventListener("DOMContentLoaded", () => {
  parseUrl();
  setupHandlers();
  hydrate();
});

function parseUrl() {
  // /developer/villas/<asset_id>/operations/new   (create mode)
  // /developer/villas/<asset_id>/operations/<log_id>  (edit mode)
  const parts = window.location.pathname.split("/").filter(Boolean);
  const villaIdx = parts.indexOf("villas");
  const operationsIdx = parts.indexOf("operations");
  assetId = villaIdx >= 0 ? parts[villaIdx + 1] : null;
  const trailing = operationsIdx >= 0 ? parts[operationsIdx + 1] : null;
  if (trailing && trailing !== "new") {
    existingLogId = trailing;
  }
  const qs = new URLSearchParams(window.location.search);
  year = parseInt(qs.get("year") || "0", 10);
  month = parseInt(qs.get("month") || "0", 10);

  updatePeriodHeader();
}

// Refresh the topbar period badge + breadcrumb + nights clamp from the
// current year/month. Called once on URL parse and again after the edit-mode
// GET returns the persisted period_year / period_month for an existing log.
function updatePeriodHeader() {
  const bc = document.getElementById("dop-breadcrumb");
  if (bc && assetId) {
    bc.textContent = `Asset ${assetId.slice(0, 8)}… · Period ${year}-${String(month).padStart(2, "0")}`;
  }
  const monthName = (month >= 1 && month <= 12) ? MONTHS[month - 1] : String(month || "—");
  const periodEl = document.getElementById("dops-period-text");
  if (periodEl) periodEl.textContent = year > 0 ? `${monthName} ${year}` : "— / —";

  const nameEl = document.getElementById("dops-asset-name");
  if (nameEl && nameEl.textContent === "Loading…" && assetId) {
    nameEl.textContent = `Villa ${assetId.slice(0, 8)}…`;
  }

  if (year > 0 && month > 0) maxDaysInMonth = new Date(year, month, 0).getDate();
}

function setupHandlers() {
  document.getElementById("dop-form").addEventListener("input", recompute);
  document.getElementById("btn-save-draft").addEventListener("click", saveDraft);
  document.getElementById("btn-submit").addEventListener("click", submitForApproval);
  document.getElementById("btn-add-expense").addEventListener("click", addCustomExpense);
  document.getElementById("btn-upload-all")?.addEventListener("click", uploadAllDocuments);
  const btnSaveDraftDocs = document.getElementById("btn-save-draft-docs");
  if (btnSaveDraftDocs) btnSaveDraftDocs.addEventListener("click", saveDraft);
  setupDropzone();
  setupNumberFormatting();
  setupNightsValidation();
}

/* ── Thousands-separator formatting ──────────────────────────────────── */

function stripNum(s) {
  return String(s || "").replace(/[^0-9]/g, "");
}

function fmtNum(raw, currency = true) {
  const n = stripNum(String(raw));
  if (n === "") return "";
  const digits = parseInt(n, 10).toLocaleString("en-US");
  return currency ? "Rp " + digits : digits;
}

function setupNumberFormatting() {
  document.querySelectorAll('#dop-form input[inputmode="numeric"]').forEach(el => {
    attachFormatter(el, !el.hasAttribute("data-nocurrency"));
  });
}

function setupNightsValidation() {
  ["dop-nights-available", "dop-nights-booked"].forEach(id => {
    const el = document.getElementById(id);
    if (!el) return;
    el.placeholder = `0 – ${maxDaysInMonth}`;
    el.addEventListener("input", function () {
      const n = parseInt(stripNum(this.value), 10);
      if (!isNaN(n) && n > maxDaysInMonth) {
        this.value = String(maxDaysInMonth);
        this.setSelectionRange(this.value.length, this.value.length);
      }
    });
  });
}

function attachFormatter(el, currency = true) {
  el.addEventListener("input", function () {
    const start = this.selectionStart;
    const oldLen = this.value.length;
    const formatted = fmtNum(this.value, currency);
    this.value = formatted;
    // Keep cursor roughly in place after comma/prefix shifts
    const diff = formatted.length - oldLen;
    const newPos = Math.max(0, (start || 0) + diff);
    try { this.setSelectionRange(newPos, newPos); } catch (_) {}
  });
}

async function hydrate() {
  try {
    if (!assetId) {
      throw new Error("Missing asset id in URL");
    }

    const cfgResp = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/asset-config`);
    if (cfgResp.ok) {
      assetConfig = await cfgResp.json();
      // V2 topbar: update asset name if config includes it
      const nameEl = document.getElementById("dops-asset-name");
      if (nameEl && (assetConfig.name || assetConfig.asset_name)) {
        nameEl.textContent = assetConfig.name || assetConfig.asset_name;
      }
    }

    if (existingLogId) {
      // Edit mode — fetch the single log row, then derive year/month from it.
      const singleResp = await fetch(
        `/api/developer/villas/${encodeURIComponent(assetId)}/operations/${encodeURIComponent(existingLogId)}`
      );
      if (!singleResp.ok) throw new Error(await responseError(singleResp));
      currentRow = await singleResp.json();
      logId = currentRow.id;
      year = currentRow.period_year;
      month = currentRow.period_month;
      updatePeriodHeader();
      fillFormFromRow(currentRow);
      reflectStatus(currentRow.status);
    } else {
      // Create mode — list by year/month from query string, take first match.
      const listResp = await fetch(`/api/developer/villas/${encodeURIComponent(assetId)}/operations?year=${year}&month=${month}`);
      if (!listResp.ok) throw new Error(await responseError(listResp));
      const rows = await listResp.json();
      if (rows.length > 0) {
        currentRow = rows[0];
        logId = currentRow.id;
        fillFormFromRow(currentRow);
        reflectStatus(currentRow.status);
      }
    }
    recompute();
    reflectDocsSection();
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
    "expense_other_idr_cents", "expense_property_tax_idr_cents", "expense_insurance_idr_cents",
    "expense_accounting_idr_cents", "expense_internet_idr_cents", "expense_capex_idr_cents",
    "ota_fees_idr_cents", "payment_fees_idr_cents",
    "refunds_idr_cents", "mgmt_fee_idr_cents", "mgmt_reported_distributable_idr_cents",
  ];
  for (const name of fields) {
    const el = form.elements[name];
    // Only fill if value is non-null and non-zero — zero means not entered yet
    if (el && row[name] != null && row[name] !== 0) {
      // Use the same fmtNum() the live attachFormatter listener applies on
      // user input, so thousands separators (and the `Rp ` prefix for
      // currency fields) render identically to the post-typing state.
      el.value = fmtNum(row[name], !el.hasAttribute("data-nocurrency"));
    }
  }

  // C-5 hydrate: rebuild the named "other" expense rows from the JSONB
  // breakdown the server persisted. Subtract their total from the catch-all
  // "Other (miscellaneous)" field so the visible amounts add up correctly —
  // the persisted `expense_other_idr_cents` is the sum of (catch-all + custom).
  const notes = Array.isArray(row.expense_other_notes) ? row.expense_other_notes : null;
  if (notes && notes.length) {
    // Clear any pre-existing rows (defensive — hydrate runs once on load).
    const list = document.getElementById("dops-custom-expenses-list");
    if (list) list.replaceChildren();
    let customSum = 0;
    for (const entry of notes) {
      const amount = parseInt(entry && entry.amount_idr_cents, 10);
      if (!isFinite(amount)) continue;
      customSum += amount;
      addCustomExpense({
        name: (entry && entry.name) ? String(entry.name) : "",
        amount,
      });
    }
    const otherEl = form.elements["expense_other_idr_cents"];
    if (otherEl && row.expense_other_idr_cents != null) {
      const residual = Math.max(0, row.expense_other_idr_cents - customSum);
      otherEl.value = residual > 0 ? fmtNum(residual, !otherEl.hasAttribute("data-nocurrency")) : "";
    }
  }
}

function reflectStatus(status) {
  const line = document.getElementById("dop-status");
  line.style.display = "flex";

  const rejected = currentRow?.rejected_reason && status === "draft";
  let variant = "neutral";
  let detail = "";
  if (rejected) {
    variant = "danger";
    detail = `Rejected: ${currentRow.rejected_reason}`;
  } else if (status === "submitted") {
    variant = "info";
    detail = "awaiting admin approval";
  } else if (status === "approved") {
    variant = "info";
    detail = "awaiting publish by admin";
  } else if (status === "published") {
    variant = "success";
    detail = "to edit, request correction with admin";
  }

  line.textContent = "";
  const badge = document.createElement("span");
  badge.className = `ds-badge ds-badge--${variant}`;
  badge.textContent = status || "draft";
  line.appendChild(badge);
  if (detail) {
    const note = document.createElement("span");
    note.className = "ds-text-caption ds-text--muted";
    note.textContent = detail;
    line.appendChild(note);
  }

  const btnDraft = document.getElementById("btn-save-draft");
  const btnSubmit = document.getElementById("btn-submit");
  const editable = status === "draft" || !status;
  btnDraft.disabled = !editable;
  btnSubmit.disabled = !editable;
  for (const el of document.getElementById("dop-form").elements) {
    if (el.tagName === "INPUT" && !el.readOnly) el.disabled = !editable;
  }
}

function gatherCustomExpenses() {
  // C-5: preserve the per-row name + amount alongside the rolled-up total so
  // the server can persist the breakdown into villa_operations_log.expense_other_notes.
  // Rows where both name and amount are blank are skipped to avoid noise.
  const out = [];
  document.querySelectorAll("#dops-custom-expenses-list .dops-custom-expense-row").forEach(row => {
    const nameEl = row.querySelector('[data-role="expense-name"]');
    const amtEl  = row.querySelector('[data-role="expense-amount"]');
    const name   = nameEl ? String(nameEl.value || "").trim() : "";
    const amount = amtEl ? (parseInt(stripNum(amtEl.value), 10) || 0) : 0;
    if (!name && amount === 0) return;
    out.push({ name, amount_idr_cents: amount });
  });
  return out;
}

function gatherCustomExpenseTotal() {
  return gatherCustomExpenses().reduce((s, e) => s + (e.amount_idr_cents || 0), 0);
}

function gatherPayload() {
  const f = document.getElementById("dop-form").elements;
  const num = (n) => parseInt(stripNum(f[n].value), 10) || 0;
  const customExpenses = gatherCustomExpenses();
  const customTotal = customExpenses.reduce((s, e) => s + (e.amount_idr_cents || 0), 0);
  return {
    period_year: year,
    period_month: month,
    currency_code: "IDR",
    gross_rental_idr_cents: num("gross_rental_idr_cents"),
    nights_available: num("nights_available"),
    nights_booked: num("nights_booked"),
    expense_cleaning_idr_cents:      num("expense_cleaning_idr_cents"),
    expense_maintenance_idr_cents:   num("expense_maintenance_idr_cents"),
    expense_utilities_idr_cents:     num("expense_utilities_idr_cents"),
    expense_staff_idr_cents:         num("expense_staff_idr_cents"),
    expense_pool_garden_idr_cents:   num("expense_pool_garden_idr_cents"),
    expense_pest_idr_cents:          num("expense_pest_idr_cents"),
    // expense_other_idr_cents is the authoritative subtotal that feeds
    // recompute()/compute_totals(): catch-all "Other" field + sum(custom rows).
    expense_other_idr_cents:         num("expense_other_idr_cents") + customTotal,
    expense_property_tax_idr_cents:  num("expense_property_tax_idr_cents"),
    expense_insurance_idr_cents:     num("expense_insurance_idr_cents"),
    expense_accounting_idr_cents:    num("expense_accounting_idr_cents"),
    expense_internet_idr_cents:      num("expense_internet_idr_cents"),
    expense_capex_idr_cents:         num("expense_capex_idr_cents"),
    ota_fees_idr_cents: num("ota_fees_idr_cents"),
    payment_fees_idr_cents: num("payment_fees_idr_cents"),
    refunds_idr_cents: num("refunds_idr_cents"),
    mgmt_fee_idr_cents: num("mgmt_fee_idr_cents"),
    mgmt_reported_distributable_idr_cents: (() => {
      const v = document.getElementById("dop-form").elements["mgmt_reported_distributable_idr_cents"].value;
      const n = stripNum(v);
      return n === "" ? null : parseInt(n, 10);
    })(),
    // C-5: send the per-row breakdown so investors/admins can see what the
    // user typed; persisted to JSONB column villa_operations_log.expense_other_notes.
    expense_other_notes: customExpenses.length ? customExpenses : null,
  };
}

function recompute() {
  const p = gatherPayload();
  // CapEx excluded — matches server compute_totals() logic
  const opex =
    p.expense_cleaning_idr_cents + p.expense_maintenance_idr_cents +
    p.expense_utilities_idr_cents + p.expense_staff_idr_cents +
    p.expense_pool_garden_idr_cents + p.expense_pest_idr_cents +
    p.expense_other_idr_cents + p.expense_property_tax_idr_cents +
    p.expense_insurance_idr_cents + p.expense_accounting_idr_cents +
    p.expense_internet_idr_cents + p.ota_fees_idr_cents +
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

  // V2 right panel — live summary
  const setTxt = (id, val) => { const e = document.getElementById(id); if (e) e.textContent = val; };
  const fmt = (n) => n.toLocaleString();
  setTxt("dops-distrib-amount", Math.max(0, distributable).toLocaleString());
  setTxt("dops-mv-gross", fmt(p.gross_rental_idr_cents));
  setTxt("dops-mv-adr", fmt(adr));
  setTxt("dops-mv-opex", fmt(Math.max(0, opex)));
  setTxt("dops-mv-net", fmt(net));
  setTxt("dops-mv-reserve", fmt(reserve));
  setTxt("dops-mv-platform", fmt(platform));
  setTxt("dops-mv-withholding", fmt(withholding));
  // Custom expenses sub-row
  const customTotal = gatherCustomExpenseTotal();
  const customRow = document.getElementById("dops-mv-custom-row");
  if (customRow) customRow.style.display = customTotal > 0 ? "flex" : "none";
  setTxt("dops-mv-custom", fmt(customTotal));
  // Occupancy ring arc
  const circumference = 2 * Math.PI * 18; // ≈ 113.097
  const fraction = p.nights_available > 0 ? Math.min(1, p.nights_booked / p.nights_available) : 0;
  const ringCircle = document.getElementById("dops-occ-ring-circle");
  if (ringCircle) ringCircle.setAttribute("stroke-dashoffset", (circumference * (1 - fraction)).toFixed(2));
  setTxt("dops-occ-pct", `${(occupancy / 100).toFixed(1)}%`);
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
    reflectDocsSection();
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

/* ── Custom expense rows ─────────────────────────────────────────────── */

function addCustomExpense(prefill) {
  customExpenseCount++;
  const row = document.createElement("div");
  row.className = "dops-custom-expense-row";
  row.innerHTML = `
    <div class="ds-form-group" style="flex:1;min-width:0">
      <input class="ds-input" type="text" data-role="expense-name"
        placeholder="Expense name (e.g. Generator rental)" autocomplete="off" />
    </div>
    <div class="ds-form-group" style="flex:0 0 150px">
      <input class="ds-input" type="text" inputmode="numeric" data-role="expense-amount"
        placeholder="0" />
    </div>
    <button type="button" class="dops-custom-expense-remove" title="Remove">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
        stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M18 6L6 18M6 6l12 12"/>
      </svg>
    </button>
  `;
  const nameEl = row.querySelector('[data-role="expense-name"]');
  const amtEl  = row.querySelector('[data-role="expense-amount"]');
  attachFormatter(amtEl, true);
  amtEl.addEventListener("input", recompute);
  row.querySelector(".dops-custom-expense-remove").addEventListener("click", () => {
    row.remove();
    recompute();
  });
  document.getElementById("dops-custom-expenses-list").appendChild(row);

  // Hydrate (edit-mode) path: prefill name + amount with the persisted entry.
  if (prefill && typeof prefill === "object") {
    if (prefill.name) nameEl.value = prefill.name;
    if (typeof prefill.amount === "number" && prefill.amount > 0) {
      amtEl.value = fmtNum(prefill.amount, true);
    }
  } else {
    // Fresh user-initiated row: focus the name input.
    nameEl.focus();
  }
}

/* ── Document upload queue ───────────────────────────────────────────── */

const DOC_TYPES = [
  { value: "receipt",          label: "Receipt" },
  { value: "invoice",          label: "Invoice" },
  { value: "bank_statement",   label: "Bank statement" },
  { value: "payout_statement", label: "Payout statement" },
  { value: "other",            label: "Other" },
];

const ICON_PDF = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><path d="M14 2v6h6"/><path d="M9 15h6M9 11h2"/></svg>`;
const ICON_IMG = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="M21 15l-5-5L5 21"/></svg>`;
const ICON_FILE = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><path d="M14 2v6h6"/></svg>`;

function fileExt(name) { return String(name).split(".").pop().toLowerCase(); }
function fileIcon(name) {
  const ext = fileExt(name);
  if (ext === "pdf") return ICON_PDF;
  if (["png","jpg","jpeg","webp"].includes(ext)) return ICON_IMG;
  return ICON_FILE;
}
function formatSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}
function esc(s) {
  return String(s).replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;").replace(/"/g,"&quot;");
}

function createQueueItem(file) {
  const item = document.createElement("div");
  item.className = "dops-queue-item";
  item._file = file;

  const typeOptions = DOC_TYPES.map((t, i) =>
    `<option value="${t.value}"${i === 0 ? " selected" : ""}>${t.label}</option>`
  ).join("");

  item.innerHTML = `
    <span class="dops-queue-item__icon">${fileIcon(file.name)}</span>
    <span class="dops-queue-item__name" title="${esc(file.name)}">${esc(file.name)}</span>
    <span class="dops-queue-item__size">${formatSize(file.size)}</span>
    <select class="dops-type-select" aria-label="Document type">${typeOptions}</select>
    <button type="button" class="dops-queue-item__remove" title="Remove">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
    </button>
  `;

  item.querySelector(".dops-queue-item__remove").addEventListener("click", () => {
    item.remove();
    updateUploadButton();
  });
  return item;
}

function addFilesToQueue(files) {
  const queue = document.getElementById("dops-upload-queue");
  for (const file of files) queue.appendChild(createQueueItem(file));
  updateUploadButton();
}

function updateUploadButton() {
  const btn = document.getElementById("btn-upload-all");
  if (!btn) return;
  const count = document.querySelectorAll(".dops-queue-item").length;
  btn.style.display = count > 0 ? "inline-flex" : "none";
  btn.textContent = count === 1 ? "Upload document" : `Upload ${count} documents`;
}

function setupDropzone() {
  const dropzone = document.getElementById("dop-dropzone");
  const input = document.getElementById("dop-doc-file");
  if (!dropzone || !input) return;

  input.addEventListener("change", () => {
    addFilesToQueue(input.files);
    input.value = "";
  });
  dropzone.addEventListener("dragover", (e) => {
    e.preventDefault();
    dropzone.classList.add("dops-dropzone--active");
  });
  ["dragleave", "dragend"].forEach(ev =>
    dropzone.addEventListener(ev, () => dropzone.classList.remove("dops-dropzone--active"))
  );
  dropzone.addEventListener("drop", (e) => {
    e.preventDefault();
    dropzone.classList.remove("dops-dropzone--active");
    if (e.dataTransfer.files.length) addFilesToQueue(e.dataTransfer.files);
  });
}

async function uploadAllDocuments() {
  const errEl = document.getElementById("dop-docs-error");
  const btn = document.getElementById("btn-upload-all");
  errEl.textContent = "";
  if (!logId) { errEl.textContent = "Save as draft first."; return; }

  const items = [...document.querySelectorAll(".dops-queue-item")];
  if (!items.length) return;

  btn.disabled = true;
  const origText = btn.textContent;
  btn.textContent = "Uploading…";

  const errors = [];
  for (const item of items) {
    const file = item._file;
    const docType = item.querySelector(".dops-type-select").value;
    item.classList.add("dops-queue-item--uploading");
    try {
      const fd = new FormData();
      fd.append("file", file);
      fd.append("doc_type", docType);
      const resp = await fetch(
        `/api/developer/villas/${encodeURIComponent(assetId)}/operations/${logId}/documents`,
        { method: "POST", headers: csrfHeaders(), body: fd }
      );
      if (!resp.ok) throw new Error(await responseError(resp));
      item.remove();
    } catch (err) {
      item.classList.remove("dops-queue-item--uploading");
      item.classList.add("dops-queue-item--error");
      errors.push(`${file.name}: ${err.message}`);
    }
  }

  await loadDocuments();
  btn.disabled = false;
  updateUploadButton();
  if (errors.length) errEl.textContent = errors.join(" · ");
}

/* ── Period documents (receipts / invoices / statements) ─────────────── */

// The panel needs a log_id to attach against, so it stays hidden until the
// period has been saved as a draft at least once.
function reflectDocsSection() {
  const locked = document.getElementById("dop-docs-locked");
  const uploadForm = document.getElementById("dop-docs-upload-form");
  if (logId) {
    if (locked) locked.style.display = "none";
    if (uploadForm) uploadForm.style.display = "block";
    loadDocuments();
  } else {
    if (locked) locked.style.display = "flex";
    if (uploadForm) uploadForm.style.display = "none";
    // Clear any queued items if lock reinstated
    document.getElementById("dops-upload-queue")?.replaceChildren();
    updateUploadButton();
  }
}

async function loadDocuments() {
  const errEl = document.getElementById("dop-docs-error");
  try {
    const resp = await fetch(
      `/api/developer/villas/${encodeURIComponent(assetId)}/operations/${logId}/documents`
    );
    if (!resp.ok) throw new Error(await responseError(resp));
    renderDocuments(await resp.json());
    errEl.textContent = "";
  } catch (err) {
    errEl.textContent = `Failed to load documents: ${err.message}`;
  }
}

function renderDocuments(docs) {
  const list = document.getElementById("dop-docs-list");
  if (!Array.isArray(docs) || docs.length === 0) {
    list.innerHTML = '<p class="ds-form-hint">No documents attached yet.</p>';
    return;
  }
  list.innerHTML = docs
    .map((d) => {
      const when = new Date(d.created_at).toLocaleDateString();
      const raw = String(d.doc_type || "other").replace(/_/g, " ");
      const label = raw.charAt(0).toUpperCase() + raw.slice(1);
      const href = `/api/documents/${encodeURIComponent(d.document_id)}/download`;
      return `
        <div style="display:flex;justify-content:space-between;align-items:center;gap:12px;
          padding:10px 14px;background:var(--content-bg,#FAFAFA);
          border:1px solid var(--card-border-color,#E5E7EB);border-radius:8px">
          <div>
            <span style="font-size:13px;font-weight:600;color:var(--page-title-color,#101828)">${label}</span>
            <span style="font-size:12px;color:var(--muted-text,#667085);margin-left:8px">linked ${when}</span>
          </div>
          <a href="${href}" target="_blank" rel="noopener"
            class="ds-btn ds-btn--secondary ds-btn--sm">Download</a>
        </div>`;
    })
    .join("");
}

// Single combined upload-and-link call — the developer endpoint takes the
// file + doc_type as multipart and does the asset_documents insert plus the
// villa_period_documents link server-side.
async function uploadDocument() {
  const errEl = document.getElementById("dop-docs-error");
  errEl.textContent = "";
  if (!logId) {
    errEl.textContent = "Save the period as a draft first.";
    return;
  }
  const fileInput = document.getElementById("dop-doc-file");
  const file = fileInput.files[0];
  if (!file) {
    errEl.textContent = "Choose a file to upload.";
    return;
  }
  const docType = document.getElementById("dop-doc-type").value;
  const btn = document.getElementById("btn-upload-doc");
  btn.disabled = true;
  btn.textContent = "Uploading…";
  try {
    const fd = new FormData();
    fd.append("file", file);
    fd.append("doc_type", docType);
    const resp = await fetch(
      `/api/developer/villas/${encodeURIComponent(assetId)}/operations/${logId}/documents`,
      { method: "POST", headers: csrfHeaders(), body: fd }
    );
    if (!resp.ok) throw new Error(await responseError(resp));
    resetFileInput();
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
  try { const b = await resp.json(); return b.error || b.message || `HTTP ${resp.status}`; }
  catch { return `HTTP ${resp.status}`; }
}
