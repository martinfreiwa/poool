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
 *   GET    /api/admin/assets/:asset_id/detail              (fetch asset title + doc titles)
 *   PATCH  /api/admin/assets/:asset_id/documents/:doc_id  (rename document)
 *   POST   /api/admin/assets/:asset_id/documents           (upload file)
 *   POST   /api/admin/villas/:asset_id/operations/:log_id/documents  (link to period)
 *   GET    /api/admin/villas/:asset_id/operations/:log_id/documents  (list linked)
 *   GET    /api/admin/villas/:asset_id/config-summary      (villa config)
 *   PUT    /api/admin/villas/:asset_id/config              (save villa config)
 */

let assetId    = null;
let year       = null;
let month      = null;
let mode       = "entry";
let logId      = null;
let currentRow = null;

document.addEventListener("DOMContentLoaded", () => {
  parseUrl();
  setupHandlers();
  hydrate();
});

// ─── URL / init ─────────────────────────────────────────────────────────────

function parseUrl() {
  // /admin/villas/<asset_id>/operations/<year>/<month>
  const parts = window.location.pathname.split("/").filter(Boolean);
  // parts: ["admin","villas","<asset>","operations","<year>","<month>"]
  assetId = parts[2] || null;
  year    = parseInt(parts[4], 10);
  month   = parseInt(parts[5], 10);
  const qs = new URLSearchParams(window.location.search);
  mode  = qs.get("mode") || "entry";
  logId = qs.get("log_id") || null;

  // Period badge
  const periodEl = document.getElementById("vop-period-text");
  if (periodEl) periodEl.textContent = `${year}-${String(month).padStart(2, "0")}`;

  // Fetch real asset name async
  fetchAssetName();
}

async function fetchAssetName() {
  if (!assetId) return;
  try {
    const resp = await fetch(`/api/admin/assets/${encodeURIComponent(assetId)}/detail`);
    if (!resp.ok) return;
    const data = await resp.json();
    const title = data?.name || data?.asset_name || data?.title || null;
    if (title) {
      const el = document.getElementById("vop-asset-name");
      if (el) el.textContent = title;
    }
  } catch {
    // Leave default "Loading…" or silently fail
  }
}

// ─── Event wiring ────────────────────────────────────────────────────────────

function setupHandlers() {
  document.getElementById("vop-form")?.addEventListener("input", recompute);

  document.getElementById("btn-save-draft")?.addEventListener("click", saveDraft);
  document.getElementById("btn-submit")?.addEventListener("click", submitForApproval);
  document.getElementById("btn-approve")?.addEventListener("click", approve);
  document.getElementById("btn-publish")?.addEventListener("click", publish);
  document.getElementById("btn-reject")?.addEventListener("click", reject);
  document.getElementById("btn-upload-doc")?.addEventListener("click", uploadDocument);
  document.getElementById("btn-save-config")?.addEventListener("click", saveVillaConfig);
  document.getElementById("btn-distribute")?.addEventListener("click", distribute);

  setupNumberFormatting();
  setupDropzone();
}

// ─── Number formatting ───────────────────────────────────────────────────────

function stripNum(s) {
  return String(s || "").replace(/[^0-9]/g, "");
}

function fmtNum(raw, currency = true) {
  const n = stripNum(String(raw));
  if (n === "") return "";
  const digits = parseInt(n, 10).toLocaleString("en-US");
  return currency ? "Rp " + digits : digits;
}

function attachFormatter(el, currency = true) {
  el.addEventListener("input", function () {
    const start  = this.selectionStart;
    const oldLen = this.value.length;
    const formatted = fmtNum(this.value, currency);
    this.value = formatted;
    const diff   = formatted.length - oldLen;
    const newPos = Math.max(0, (start || 0) + diff);
    try { this.setSelectionRange(newPos, newPos); } catch (_) {}
  });
}

function setupNumberFormatting() {
  document.querySelectorAll('#vop-form input[inputmode="numeric"]').forEach(el => {
    attachFormatter(el, !el.hasAttribute("data-nocurrency"));
  });
}

// ─── Dropzone (visual feedback only — upload is manual via button) ───────────

function setupDropzone() {
  const dropzone = document.getElementById("vop-dropzone");
  const fileInput = document.getElementById("vop-doc-file");
  if (!dropzone || !fileInput) return;

  ["dragenter", "dragover"].forEach(ev => {
    dropzone.addEventListener(ev, e => {
      e.preventDefault();
      dropzone.classList.add("dops-dropzone--active");
    });
  });
  ["dragleave", "dragend"].forEach(ev => {
    dropzone.addEventListener(ev, () => dropzone.classList.remove("dops-dropzone--active"));
  });
  dropzone.addEventListener("drop", e => {
    e.preventDefault();
    dropzone.classList.remove("dops-dropzone--active");
    const file = e.dataTransfer?.files?.[0];
    if (file) {
      // Assign to the file input so uploadDocument() picks it up
      const dt = new DataTransfer();
      dt.items.add(file);
      fileInput.files = dt.files;
      updateDropzoneLabel(file.name);
    }
  });
  fileInput.addEventListener("change", () => {
    if (fileInput.files[0]) updateDropzoneLabel(fileInput.files[0].name);
  });
}

function updateDropzoneLabel(name) {
  const dropzone = document.getElementById("vop-dropzone");
  if (!dropzone) return;
  const span = dropzone.querySelector("span:not(.dops-dropzone__hint)");
  if (span) span.textContent = name;
}

// ─── Hydration ───────────────────────────────────────────────────────────────

async function hydrate() {
  // Load operations row and villa config in parallel
  await Promise.all([hydrateOperations(), loadVillaConfig()]);
}

async function hydrateOperations() {
  try {
    const url = `/api/admin/villas/${encodeURIComponent(assetId)}/operations?year=${year}&month=${month}`;
    const resp = await fetch(url);
    if (!resp.ok) throw new Error(await responseError(resp));
    const rows = await resp.json();

    if (logId) {
      currentRow = rows.find(r => String(r.id) === String(logId)) || null;
    } else if (rows.length > 0 && rows[0].status !== "superseded") {
      currentRow = rows[0];
    }
    if (currentRow) {
      logId = currentRow.id;
      fillFormFromRow(currentRow);
      reflectStatus(currentRow.status);
    } else {
      reflectStatus(null);
    }
    recompute();
    reflectDocsSection();
  } catch (err) {
    showError(`Failed to load: ${err.message}`);
  }
}

// ─── Form population ─────────────────────────────────────────────────────────

function fillFormFromRow(row) {
  const form = document.getElementById("vop-form");
  const currencyFields = [
    "gross_rental_idr_cents",
    "expense_cleaning_idr_cents",
    "expense_maintenance_idr_cents",
    "expense_utilities_idr_cents",
    "expense_staff_idr_cents",
    "expense_pool_garden_idr_cents",
    "expense_pest_idr_cents",
    "expense_other_idr_cents",
    "expense_property_tax_idr_cents",
    "expense_insurance_idr_cents",
    "expense_accounting_idr_cents",
    "expense_internet_idr_cents",
    "expense_capex_idr_cents",
    "ota_fees_idr_cents",
    "payment_fees_idr_cents",
    "refunds_idr_cents",
    "mgmt_fee_idr_cents",
    "mgmt_reported_distributable_idr_cents",
    "reserve_override_idr_cents",
  ];
  const plainFields = ["nights_available", "nights_booked"];
  const textFields  = ["correction_reason"];

  for (const name of currencyFields) {
    const el = form.elements[name];
    if (el && row[name] != null && row[name] !== 0) {
      el.value = fmtNum(row[name], true);
    }
  }
  for (const name of plainFields) {
    const el = form.elements[name];
    if (el && row[name] != null) el.value = row[name];
  }
  for (const name of textFields) {
    const el = form.elements[name];
    if (el && row[name]) el.value = row[name];
  }

  // Show correction-reason field if this is a correction
  const corrGroup = document.getElementById("vop-correction-group");
  if (corrGroup) corrGroup.style.display = row.supersedes_id ? "block" : "none";

  // Pre-fill reject-reason textarea if there's a previous rejection
  const rejectEl = document.getElementById("vop-reject-reason");
  if (rejectEl && row.rejected_reason) rejectEl.value = row.rejected_reason;
}

// ─── Status chip & button visibility ─────────────────────────────────────────

function reflectStatus(status) {
  // Chip
  const chip = document.getElementById("vop-status-chip");
  if (chip) {
    chip.textContent = "";
    if (status) {
      const colors = {
        draft:     "background:#FEF3C7;color:#92400E;",
        submitted: "background:#DBEAFE;color:#1D4ED8;",
        approved:  "background:#D1FAE5;color:#065F46;",
        published: "background:#D1FAE5;color:#065F46;",
        rejected:  "background:#FEE2E2;color:#991B1B;",
      };
      chip.style.cssText = `display:inline-flex;align-items:center;padding:2px 10px;border-radius:20px;font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:0.05em;${colors[status] || "background:#F3F4F6;color:#374151;"}`;
      chip.textContent = status;

      // Append rejection note
      if (status === "draft" && currentRow?.rejected_reason) {
        const note = document.createElement("span");
        note.style.cssText = "margin-left:8px;font-size:11px;font-weight:400;font-style:italic;opacity:0.8;text-transform:none;letter-spacing:0;";
        note.textContent = `Rejected: ${currentRow.rejected_reason}`;
        chip.appendChild(note);
      }
    }
  }

  // Buttons
  const btnSave      = document.getElementById("btn-save-draft");
  const btnSubmit    = document.getElementById("btn-submit");
  const btnReject    = document.getElementById("btn-reject");
  const btnApprove   = document.getElementById("btn-approve");
  const btnPublish   = document.getElementById("btn-publish");
  const btnDistribute = document.getElementById("btn-distribute");

  // draft or no row: can save + submit, cannot approve/reject/publish
  // submitted: can approve/reject, cannot save-edit, cannot publish
  // approved: can publish, nothing else
  // published: read-only

  const isDraft     = !status || status === "draft";
  const isSubmitted = status === "submitted";
  const isApproved  = status === "approved";
  const isPublished = status === "published";

  if (btnSubmit)    btnSubmit.style.display    = isDraft    ? "" : "none";
  if (btnPublish)   btnPublish.style.display   = isApproved ? "" : "none";
  if (btnDistribute) btnDistribute.style.display = isPublished ? "" : "none";

  if (btnSave)    btnSave.disabled    = isPublished;
  if (btnReject)  btnReject.disabled  = !isSubmitted;
  if (btnApprove) btnApprove.disabled = !isSubmitted;
  if (btnPublish) btnPublish.disabled = !isApproved;

  // Lock named period-data inputs only (config inputs have no name — stay editable always)
  const editable = isDraft;
  const form = document.getElementById("vop-form");
  if (form) {
    for (const el of form.elements) {
      if (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT") {
        if (!el.readOnly && el.name) el.disabled = !editable;
      }
    }
  }
}

// ─── Payload ─────────────────────────────────────────────────────────────────

function gatherPayload() {
  const f   = document.getElementById("vop-form").elements;
  const num = n => parseInt(stripNum(f[n]?.value || ""), 10) || 0;
  const opt = n => {
    const s = stripNum(f[n]?.value || "");
    return s === "" ? null : parseInt(s, 10);
  };
  return {
    period_year:                        year,
    period_month:                       month,
    currency_code:                      "IDR",
    gross_rental_idr_cents:             num("gross_rental_idr_cents"),
    nights_available:                   num("nights_available"),
    nights_booked:                      num("nights_booked"),
    expense_cleaning_idr_cents:         num("expense_cleaning_idr_cents"),
    expense_maintenance_idr_cents:      num("expense_maintenance_idr_cents"),
    expense_utilities_idr_cents:        num("expense_utilities_idr_cents"),
    expense_staff_idr_cents:            num("expense_staff_idr_cents"),
    expense_pool_garden_idr_cents:      num("expense_pool_garden_idr_cents"),
    expense_pest_idr_cents:             num("expense_pest_idr_cents"),
    expense_other_idr_cents:            num("expense_other_idr_cents"),
    expense_property_tax_idr_cents:     num("expense_property_tax_idr_cents"),
    expense_insurance_idr_cents:        num("expense_insurance_idr_cents"),
    expense_accounting_idr_cents:       num("expense_accounting_idr_cents"),
    expense_internet_idr_cents:         num("expense_internet_idr_cents"),
    expense_capex_idr_cents:            num("expense_capex_idr_cents"),
    ota_fees_idr_cents:                 num("ota_fees_idr_cents"),
    payment_fees_idr_cents:             num("payment_fees_idr_cents"),
    refunds_idr_cents:                  num("refunds_idr_cents"),
    mgmt_fee_idr_cents:                 num("mgmt_fee_idr_cents"),
    mgmt_reported_distributable_idr_cents: opt("mgmt_reported_distributable_idr_cents"),
    reserve_override_idr_cents:         opt("reserve_override_idr_cents"),
    correction_reason:                  f["correction_reason"]?.value || null,
  };
}

// ─── Live summary recompute ───────────────────────────────────────────────────

function recompute() {
  const p = gatherPayload();

  // CapEx excluded — matches server compute_totals() logic
  const opex =
    p.expense_cleaning_idr_cents +
    p.expense_maintenance_idr_cents +
    p.expense_utilities_idr_cents +
    p.expense_staff_idr_cents +
    p.expense_pool_garden_idr_cents +
    p.expense_pest_idr_cents +
    p.expense_other_idr_cents +
    p.expense_property_tax_idr_cents +
    p.expense_insurance_idr_cents +
    p.expense_accounting_idr_cents +
    p.expense_internet_idr_cents +
    p.ota_fees_idr_cents +
    p.payment_fees_idr_cents +
    p.mgmt_fee_idr_cents -
    p.refunds_idr_cents;

  const net = p.gross_rental_idr_cents - opex;
  const occupancy = p.nights_available > 0
    ? Math.floor((p.nights_booked * 10000) / p.nights_available)
    : 0;
  const adr = p.nights_booked > 0
    ? Math.floor(p.gross_rental_idr_cents / p.nights_booked)
    : 0;

  const setTxt = (id, val) => { const e = document.getElementById(id); if (e) e.textContent = val; };
  const fmt    = n => n.toLocaleString("en-US");

  // Hero amount (net; reserve unknown client-side)
  setTxt("dops-distrib-amount", Math.max(0, net).toLocaleString("en-US"));

  // Metrics
  setTxt("dops-mv-gross", fmt(p.gross_rental_idr_cents));
  setTxt("dops-mv-adr",   fmt(adr));
  setTxt("dops-mv-opex",  fmt(Math.max(0, opex)));
  setTxt("dops-mv-net",   fmt(net));

  // Admin metrics from live fields
  const reserveOverride = p.reserve_override_idr_cents;
  setTxt("dops-mv-reserve-override",
    reserveOverride != null ? fmt(reserveOverride) : "— (policy)");

  const mgmtRep = p.mgmt_reported_distributable_idr_cents;
  const mgmtEl = document.getElementById("dops-mv-mgmt-reported");
  if (mgmtEl) {
    if (mgmtRep != null) {
      const variance = net - mgmtRep;           // calculated vs reported
      const varPct   = mgmtRep !== 0 ? ((variance / mgmtRep) * 100).toFixed(1) : null;
      const absPct   = varPct !== null ? Math.abs(parseFloat(varPct)) : 0;
      const sign     = variance >= 0 ? "+" : "−";
      const color    = absPct > 5  ? "var(--admin-danger,#D92D20)"
                     : absPct > 2  ? "var(--admin-warning,#D97706)"
                     : "var(--admin-success,#059669)";
      mgmtEl.innerHTML =
        `${fmt(mgmtRep)} ` +
        `<span style="font-size:11px;color:${color};font-weight:600;" title="Variance vs POOOL calculated net">` +
        `(${sign}${fmt(Math.abs(variance))}${varPct !== null ? " / " + sign + absPct + "%" : ""})` +
        `</span>`;
    } else {
      mgmtEl.textContent = "—";
    }
  }

  // Occupancy ring
  const circumference = 2 * Math.PI * 18; // ≈ 113.097
  const fraction = p.nights_available > 0
    ? Math.min(1, p.nights_booked / p.nights_available)
    : 0;
  const ringCircle = document.getElementById("dops-occ-ring-circle");
  if (ringCircle) ringCircle.setAttribute("stroke-dashoffset", (circumference * (1 - fraction)).toFixed(2));
  setTxt("dops-occ-pct", `${(occupancy / 100).toFixed(1)}%`);
}

// ─── Save / transitions ───────────────────────────────────────────────────────

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

async function approve()  { await transition("approve"); }
async function publish()  { await transition("publish"); }

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
  const reason = document.getElementById("vop-reject-reason")?.value.trim();
  if (!reason) {
    showError("Rejection reason is required.");
    return;
  }
  showError("");
  try {
    const resp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${logId}/reject`,
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

// ─── Distribute ──────────────────────────────────────────────────────────────

async function distribute() {
  if (!logId) return;
  const period = `${year}-${String(month).padStart(2, "0")}`;
  if (!confirm(
    `Distribute & process payouts for ${period}?\n\n` +
    `Investor cash wallets will be credited and wallet_transactions rows created.\n` +
    `This operation is idempotent — safe to run again if already partially processed.`
  )) return;

  const btn = document.getElementById("btn-distribute");
  if (btn) { btn.disabled = true; btn.textContent = "Distributing…"; }
  showError("");

  try {
    // Step 1: create dividend_payouts rows (scheduled)
    const distResp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${logId}/distribute`,
      { method: "POST", headers: csrfHeaders() }
    );
    if (!distResp.ok) throw new Error("distribute: " + await responseError(distResp));
    const distResult = await distResp.json();

    // Step 2: credit investor wallets
    const procResp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/operations/${logId}/process-payouts`,
      { method: "POST", headers: csrfHeaders() }
    );
    if (!procResp.ok) throw new Error("process-payouts: " + await responseError(procResp));
    const procResult = await procResp.json();

    alert(
      `✓ Distribute & process complete for ${period}.\n\n` +
      `Payouts scheduled: ${distResult.created}  (duplicates skipped: ${distResult.skipped})\n` +
      `Wallets credited:  ${procResult.paid_count}  (already paid: ${procResult.skipped_already_paid})\n` +
      `Total paid out:    ${Number(procResult.paid_total_cents).toLocaleString()} ${distResult.currency || ""} cents`
    );
  } catch (err) {
    showError(`Distribute failed: ${err.message}`);
  } finally {
    if (btn) { btn.disabled = false; btn.textContent = "Distribute"; }
  }
}

// ─── Error bar ───────────────────────────────────────────────────────────────

function showError(msg) {
  const el = document.getElementById("vop-error-bar");
  if (el) el.textContent = msg;
}

// ─── Villa config ────────────────────────────────────────────────────────────

async function loadVillaConfig() {
  try {
    const resp = await fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/config-summary`);
    if (!resp.ok) return;
    const cfg = await resp.json();
    // Convert bps → % for display (reserve_pct_bps / 100, withholding_tax_bps / 100)
    const reserveEl     = document.getElementById("vop-cfg-reserve");
    const platformEl    = document.getElementById("vop-cfg-platform");
    const withholdingEl = document.getElementById("vop-cfg-withholding");
    if (reserveEl)     reserveEl.value     = (cfg.reserve_pct_bps / 100).toFixed(2);
    if (platformEl)    platformEl.value    = (cfg.poool_split_pct).toFixed(2);
    if (withholdingEl) withholdingEl.value = (cfg.withholding_tax_bps / 100).toFixed(2);
  } catch {
    // Non-fatal — config panel stays blank
  }
}

async function saveVillaConfig() {
  const statusEl = document.getElementById("vop-cfg-status");
  const btn      = document.getElementById("btn-save-config");
  if (statusEl) statusEl.textContent = "";

  const reserveVal     = parseFloat(document.getElementById("vop-cfg-reserve")?.value || "0");
  const platformVal    = parseFloat(document.getElementById("vop-cfg-platform")?.value || "0");
  const withholdingVal = parseFloat(document.getElementById("vop-cfg-withholding")?.value || "0");

  // Convert % → bps. poool_split_pct is read-only (not in VillaConfigInput).
  const payload = {
    reserve_pct_bps:     Math.round(reserveVal * 100),
    withholding_tax_bps: Math.round(withholdingVal * 100),
  };

  if (btn) { btn.disabled = true; btn.textContent = "Saving…"; }
  try {
    const resp = await fetch(
      `/api/admin/villas/${encodeURIComponent(assetId)}/config`,
      {
        method: "PUT",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        body: JSON.stringify(payload),
      }
    );
    if (!resp.ok) throw new Error(await responseError(resp));
    const updated = await resp.json();
    // Refresh display from server response
    const reserveEl     = document.getElementById("vop-cfg-reserve");
    const platformEl    = document.getElementById("vop-cfg-platform");
    const withholdingEl = document.getElementById("vop-cfg-withholding");
    if (reserveEl)     reserveEl.value     = (updated.reserve_pct_bps / 100).toFixed(2);
    if (platformEl)    platformEl.value    = (updated.poool_split_pct).toFixed(2);
    if (withholdingEl) withholdingEl.value = (updated.withholding_tax_bps / 100).toFixed(2);
    if (statusEl) {
      statusEl.style.color = "var(--admin-success, #059669)";
      statusEl.textContent = "Saved ✓";
      setTimeout(() => { if (statusEl) statusEl.textContent = ""; }, 3000);
    }
  } catch (err) {
    if (statusEl) {
      statusEl.style.color = "var(--admin-danger, #D92D20)";
      statusEl.textContent = err.message;
    }
  } finally {
    if (btn) { btn.disabled = false; btn.textContent = "Save config"; }
  }
}

// ─── Documents ───────────────────────────────────────────────────────────────

function reflectDocsSection() {
  const section = document.getElementById("vop-docs-section");
  if (!section) return;
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
    // Fetch period documents + asset documents (for titles) in parallel
    const [periodResp, detailResp] = await Promise.all([
      fetch(`/api/admin/villas/${encodeURIComponent(assetId)}/operations/${logId}/documents`),
      fetch(`/api/admin/assets/${encodeURIComponent(assetId)}/detail`),
    ]);
    if (!periodResp.ok) throw new Error(await responseError(periodResp));
    const periodDocs = await periodResp.json();

    // Build document_id → title map from asset detail
    const titleMap = {};
    if (detailResp.ok) {
      const detail = await detailResp.json();
      for (const d of (detail.documents || [])) {
        titleMap[d.id] = d.title;
      }
    }

    renderDocuments(periodDocs, titleMap);
    if (errEl) errEl.textContent = "";
  } catch (err) {
    if (errEl) errEl.textContent = `Failed to load documents: ${err.message}`;
  }
}

function renderDocuments(docs, titleMap = {}) {
  const list = document.getElementById("vop-docs-list");
  if (!list) return;
  if (!Array.isArray(docs) || docs.length === 0) {
    list.innerHTML = `<p style="font-size:13px;color:var(--admin-text-muted,#6b7280);margin:0;">No documents linked to this period yet.</p>`;
    return;
  }
  list.innerHTML = "";
  for (const d of docs) {
    const title     = titleMap[d.document_id] || String(d.doc_type || "other").replace(/_/g, " ");
    const typeLabel = String(d.doc_type || "other").replace(/_/g, " ");
    const href      = `/api/documents/${encodeURIComponent(d.document_id)}/download`;

    const item = document.createElement("div");
    item.className = "dops-queue-item";
    item.style.gap = "8px";

    // Title
    const nameEl = document.createElement("span");
    nameEl.className = "dops-queue-item__name";
    nameEl.textContent = title;

    // Type badge
    const typeEl = document.createElement("span");
    typeEl.className = "dops-queue-item__size";
    typeEl.style.textTransform = "capitalize";
    typeEl.textContent = typeLabel;

    // Download
    const dlLink = document.createElement("a");
    dlLink.href = href;
    dlLink.target = "_blank";
    dlLink.rel = "noopener";
    dlLink.className = "admin-btn admin-btn--secondary admin-btn--sm";
    dlLink.style.cssText = "margin-left:auto; text-decoration:none; flex-shrink:0;";
    dlLink.textContent = "Download";

    // Rename
    const renameBtn = document.createElement("button");
    renameBtn.type = "button";
    renameBtn.className = "admin-btn admin-btn--secondary admin-btn--sm";
    renameBtn.style.flexShrink = "0";
    renameBtn.textContent = "Rename";
    renameBtn.addEventListener("click", () => startRename(item, d.document_id, title, titleMap));

    item.appendChild(nameEl);
    item.appendChild(typeEl);
    item.appendChild(dlLink);
    item.appendChild(renameBtn);
    list.appendChild(item);
  }
}

function startRename(item, documentId, currentTitle, titleMap) {
  // Swap item contents with an inline edit row
  item.innerHTML = "";
  item.style.flexWrap = "wrap";

  const input = document.createElement("input");
  input.type = "text";
  input.className = "ds-input";
  input.value = currentTitle;
  input.style.cssText = "flex: 1; min-width: 140px; font-size: 13px; height: 32px; padding: 4px 10px;";

  const saveBtn = document.createElement("button");
  saveBtn.type = "button";
  saveBtn.className = "admin-btn admin-btn--primary admin-btn--sm";
  saveBtn.style.flexShrink = "0";
  saveBtn.textContent = "Save";

  const cancelBtn = document.createElement("button");
  cancelBtn.type = "button";
  cancelBtn.className = "admin-btn admin-btn--secondary admin-btn--sm";
  cancelBtn.style.flexShrink = "0";
  cancelBtn.textContent = "Cancel";

  const errSpan = document.createElement("span");
  errSpan.style.cssText = "font-size:12px; color:var(--admin-danger,#D92D20); width:100%;";

  cancelBtn.addEventListener("click", () => {
    // Re-render fresh
    const list = document.getElementById("vop-docs-list");
    if (list) loadDocuments();
  });

  saveBtn.addEventListener("click", async () => {
    const newTitle = input.value.trim();
    if (!newTitle) { input.focus(); return; }
    saveBtn.disabled = true;
    saveBtn.textContent = "Saving…";
    errSpan.textContent = "";
    try {
      const resp = await fetch(
        `/api/admin/assets/${encodeURIComponent(assetId)}/documents/${encodeURIComponent(documentId)}`,
        {
          method: "PATCH",
          headers: csrfHeaders({ "Content-Type": "application/json" }),
          body: JSON.stringify({ title: newTitle }),
        }
      );
      if (!resp.ok) throw new Error(await responseError(resp));
      await loadDocuments(); // full reload to reflect new title
    } catch (err) {
      errSpan.textContent = `Rename failed: ${err.message}`;
      saveBtn.disabled = false;
      saveBtn.textContent = "Save";
    }
  });

  // Submit on Enter
  input.addEventListener("keydown", e => {
    if (e.key === "Enter") saveBtn.click();
    if (e.key === "Escape") cancelBtn.click();
  });

  item.appendChild(input);
  item.appendChild(saveBtn);
  item.appendChild(cancelBtn);
  item.appendChild(errSpan);
  input.focus();
  input.select();
}

// Two-step upload: upload file to asset_documents, then link to this period.
async function uploadDocument() {
  const errEl = document.getElementById("vop-docs-error");
  if (errEl) errEl.textContent = "";
  if (!logId) {
    if (errEl) errEl.textContent = "Save the period as a draft first.";
    return;
  }
  const fileInput = document.getElementById("vop-doc-file");
  const file = fileInput?.files?.[0];
  if (!file) {
    if (errEl) errEl.textContent = "Choose a file to upload.";
    return;
  }
  const docType = document.getElementById("vop-doc-type")?.value || "other";
  const btn = document.getElementById("btn-upload-doc");
  if (btn) { btn.disabled = true; btn.textContent = "Uploading…"; }
  try {
    const fd = new FormData();
    fd.append("file", file);
    fd.append("document_type", "financial");
    fd.append("title", `${docType} ${year}-${String(month).padStart(2, "0")} — ${file.name}`);

    const upResp = await fetch(
      `/api/admin/assets/${encodeURIComponent(assetId)}/documents`,
      { method: "POST", headers: csrfHeaders(), body: fd }
    );
    if (!upResp.ok) throw new Error(await responseError(upResp));
    const uploaded   = await upResp.json();
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
    if (fileInput) fileInput.value = "";
    updateDropzoneLabel("Drop file · click to browse");
    await loadDocuments();
  } catch (err) {
    if (errEl) errEl.textContent = `Upload failed: ${err.message}`;
  } finally {
    if (btn) { btn.disabled = false; btn.textContent = "Upload"; }
  }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function csrfHeaders(headers = {}) {
  const value = `; ${document.cookie}`;
  const parts = value.split("; csrf_token=");
  const token = parts.length === 2 ? parts.pop().split(";").shift() : null;
  return token ? { ...headers, "X-CSRF-Token": token } : headers;
}

async function responseError(resp) {
  try {
    const b = await resp.json();
    return b.error || b.message || `HTTP ${resp.status}`;
  } catch {
    return `HTTP ${resp.status}`;
  }
}

function escapeHtml(s) {
  return String(s == null ? "" : s).replace(/[&<>"']/g, c => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  }[c]));
}
