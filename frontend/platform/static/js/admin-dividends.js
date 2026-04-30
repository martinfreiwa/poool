/**
 * Admin Dividend Tool — Logic
 * 
 * Supports two modes:
 * 1. Legacy: /api/admin/dividends/calculate + /api/admin/dividends/process
 * 2. Phase 9: /api/admin/dividends/distributions (full lifecycle with anti-sniping)
 */

let currentSplits = [];
let selectedAssetId = "";
let currentTotalAmountCents = 0;
let currentDistributionId = null;
let assetsListData = []; // Store globally for yield projection

function clearNode(node) {
  if (!node) return;
  while (node.firstChild) node.removeChild(node.firstChild);
}

function appendText(parent, text, tagName = "span", className = "") {
  const el = document.createElement(tagName);
  if (className) el.className = className;
  el.textContent = text ?? "";
  parent.appendChild(el);
  return el;
}

function formatUsd(cents) {
  const value = Number.isFinite(Number(cents)) ? Math.trunc(Number(cents)) : 0;
  const sign = value < 0 ? "-" : "";
  const abs = Math.abs(value);
  return `${sign}$${Math.floor(abs / 100).toLocaleString()}.${String(abs % 100).padStart(2, "0")}`;
}

function setPreviewMessage(message, isLoading = false) {
  const placeholder = document.getElementById("preview-placeholder");
  if (!placeholder) return;
  clearNode(placeholder);
  if (isLoading) {
    const spinner = document.createElement("div");
    spinner.className = "loading-spinner";
    placeholder.appendChild(spinner);
  }
  appendText(placeholder, message, "p");
  placeholder.style.display = "block";
}

function showPreviewPanel() {
  document.getElementById("preview-placeholder").style.display = "none";
  document.getElementById("preview-data").style.display = "block";
  document.getElementById("step-1-label").className = "step completed";
  document.getElementById("step-2-label").className = "step active";
}

async function parseError(response, fallback) {
  const body = await response.json().catch(() => ({}));
  return body.error || body.message || fallback;
}

document.addEventListener("DOMContentLoaded", () => {
  loadAssets();
  loadDistributionHistory();

  // Set default period to current month
  const now = new Date();
  const firstDay = new Date(now.getFullYear(), now.getMonth(), 1);
  const lastDay = new Date(now.getFullYear(), now.getMonth() + 1, 0);
  const startEl = document.getElementById("period-start");
  const endEl = document.getElementById("period-end");
  if (startEl) startEl.value = firstDay.toISOString().split("T")[0];
  if (endEl) endEl.value = lastDay.toISOString().split("T")[0];

  document.getElementById("btn-preview")?.addEventListener("click", calculatePreview);
  document.getElementById("btn-process")?.addEventListener("click", processBatch);
  document.getElementById("btn-cancel")?.addEventListener("click", () => window.location.reload());
  document.getElementById("btn-refresh-distributions")?.addEventListener("click", loadDistributionHistory);
  document.getElementById("btn-export-csv")?.addEventListener("click", exportCsv);
  document.getElementById("distributions-history-body")?.addEventListener("click", handleHistoryAction);
  document.getElementById("btn-start-new-distribution")?.addEventListener("click", () => window.location.reload());

  // Auto-reset preview on config changes to prevent stale data
  document.querySelectorAll('#asset-select, #total-amount, #period-start, #period-end, #min-holding-days').forEach(input => {
    input.addEventListener('input', () => {
      document.getElementById("preview-placeholder").style.display = "block";
      document.getElementById("preview-data").style.display = "none";
      document.getElementById("step-1-label").className = "step active";
      document.getElementById("step-2-label").className = "step";
    });
  });

  // Attach Yield Calculator
  document.getElementById("total-amount")?.addEventListener('input', updateYieldProjection);
  document.getElementById("asset-select")?.addEventListener('change', updateYieldProjection);
});

function updateYieldProjection() {
  const amountStr = document.getElementById("total-amount")?.value;
  const assetId = document.getElementById("asset-select")?.value;
  const yieldEl = document.getElementById("projected-yield");
  
  if (!yieldEl) return;
  if (!amountStr || !assetId || isNaN(parseFloat(amountStr))) {
    yieldEl.style.display = "none";
    return;
  }
  
  const amountCents = parseFloat(amountStr) * 100;
  const asset = assetsListData.find(a => a.id === assetId);
  
  if (asset && asset.total_value_cents > 0) {
    const projectedYield = (amountCents / asset.total_value_cents) * 100;
    yieldEl.textContent = `Projected Yield: ${projectedYield.toFixed(2)}% APY (relative to Total Asset Value)`;
    yieldEl.style.display = "block";
  } else {
    yieldEl.style.display = "none";
  }
}

function exportCsv() {
  if (!currentSplits || currentSplits.length === 0) return;
  
  let csv = "User_Email,Tokens_Held,Share_Percentage,Holding_Days,Eligible,Payout_Amount_USD\n";
  currentSplits.forEach(p => {
    const share = p.percentage_bps ? (p.percentage_bps / 100).toFixed(2) : ((p.tokens / currentTotalAmountCents) * 100).toFixed(2);
    const amountStr = p.payout_cents ? (p.payout_cents / 100).toFixed(2) : (p.amount_cents / 100).toFixed(2);
    csv += [p.user_email || p.email, p.tokens_held || p.tokens, `${share}%`, p.holding_days || 0, p.eligible !== false ? 'Yes' : 'No', `$${amountStr}`]
      .map(csvCell)
      .join(",") + "\n";
  });
  
  const a = document.createElement("a");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
  a.href = URL.createObjectURL(blob);
  a.download = `dividend_preview_${selectedAssetId}_${new Date().toISOString().split('T')[0]}.csv`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  if (window.mpToast) mpToast('CSV Exported!', 'success');
}

function csvCell(value) {
  const text = String(value ?? "");
  const safe = /^[=+\-@]/.test(text) ? `'${text}` : text;
  return `"${safe.replace(/"/g, '""')}"`;
}

async function loadAssets() {
  try {
    const response = await fetch("/api/admin/assets");
    if (response.ok) {
      const data = await response.json();
      assetsListData = data.assets;
      const select = document.getElementById("asset-select");

      // Unwrap if already a PooolDropdown from poool-dropdown-init.js
      const wrapper = select.closest(".poool-dropdown");
      if (wrapper) {
        wrapper.parentNode.insertBefore(select, wrapper);
        wrapper.remove();
        select.style.display = "";
      }

      clearNode(select);
      const emptyOption = document.createElement("option");
      emptyOption.value = "";
      emptyOption.textContent = "-- Select Published Asset --";
      select.appendChild(emptyOption);
      data.assets.filter(a => a.published === true || a.status === 'live' || a.funding_status === 'funded').forEach((a) => {
        const option = document.createElement("option");
        option.value = a.id;
        option.textContent = `${a.title || "Untitled asset"} (${formatUsd(a.total_value_cents || 0)})`;
        select.appendChild(option);
      });

      // Auto-select from URL
      const urlParams = new URLSearchParams(window.location.search);
      const preselectId = urlParams.get('asset') || urlParams.get('asset_id');
      if (preselectId && data.assets.some(a => a.id === preselectId)) {
        select.value = preselectId;
      }

      // Reinitialize the custom dropdown
      if (window.PooolDropdown) {
        window.pooolInstance = PooolDropdown.fromSelect(select, {
          noLabel: true,
          className: select.classList.contains("admin-select")
            ? "poool-dropdown--sm poool-dropdown--inline"
            : "",
        });
      }
      
      const selectNode = document.getElementById("asset-select");
      if (selectNode && preselectId) {
         selectNode.dispatchEvent(new Event('change'));
      }
    }
  } catch (err) {
    console.error('Failed to load assets:', err);
    if (window.Sentry) Sentry.captureException(err);
  }
}

async function calculatePreview() {
  selectedAssetId = document.getElementById("asset-select").value;
  const amountVal = document.getElementById("total-amount").value;
  const periodStart = document.getElementById("period-start")?.value;
  const periodEnd = document.getElementById("period-end")?.value;
  const minHoldingDays = parseInt(document.getElementById("min-holding-days")?.value || "7", 10);

  if (!selectedAssetId) {
    alert("Please select an asset.");
    return;
  }
  if (!amountVal || amountVal <= 0) {
    alert("Please enter a valid amount.");
    return;
  }
  
  if (periodStart && periodEnd) {
    const startDate = new Date(periodStart);
    const endDate = new Date(periodEnd);
    if (startDate >= endDate) {
      alert("Invalid Dates: Period End must be after Period Start.");
      return;
    }
  }

  currentTotalAmountCents = Math.round(parseFloat(amountVal) * 100);

  setPreviewMessage("Calculating splits with anti-sniping filter...", true);

  // Try Phase 9 API first (with anti-sniping)
  if (periodStart && periodEnd) {
    try {
      const response = await fetch("/api/admin/dividends/distributions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          asset_id: selectedAssetId,
          total_amount_cents: currentTotalAmountCents,
          period_start: periodStart,
          period_end: periodEnd,
          min_holding_days: minHoldingDays,
        }),
      });

      if (!response.ok) {
        const message = await parseError(response, "Failed to calculate dividend distribution.");
        alert(message);
        setPreviewMessage(message);
        return;
      }

      const data = await response.json();
      if (data.result) {
        currentDistributionId = data.result.distribution_id;
        renderPhase9Preview(data.result);
        return;
      }
    } catch (err) {
      alert("Network error during dividend calculation.");
      setPreviewMessage("Network error during dividend calculation.");
      return;
    }
  }

  // Fallback: Legacy API
  try {
    const response = await fetch("/api/admin/dividends/calculate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        asset_id: selectedAssetId,
        total_amount_cents: currentTotalAmountCents,
      }),
    });

    if (response.ok) {
      const data = await response.json();
      renderPreview(data);
    } else {
      alert("Failed to calculate splits. Ensure asset has active investors.");
      document.getElementById("preview-placeholder").style.display = "block";
      document.getElementById("preview-data").style.display = "none";
    }
  } catch (err) {
    alert("Network error during calculation.");
  }
}

function renderPhase9Preview(result) {
  showPreviewPanel();

  document.getElementById("tokens-total-label").textContent =
    `Total Tokens: ${result.total_tokens.toLocaleString()}`;

  const eligibleEl = document.getElementById("eligible-label");
  if (eligibleEl) {
    eligibleEl.textContent = `Eligible: ${result.eligible_holders} / ${result.eligible_holders + result.ineligible_holders}`;
    if (result.ineligible_holders > 0) {
      eligibleEl.style.color = 'var(--admin-warning)';
    }
  }

  const tbody = document.getElementById("splits-body");
  if (!result.payouts || result.payouts.length === 0) {
    renderEmptyRow(tbody, 6, "No investors found for this asset.");
    document.getElementById("btn-process").disabled = true;
    return;
  }

  clearNode(tbody);
  result.payouts.forEach((p) => {
    const share = (p.percentage_bps / 100).toFixed(2);
    const row = document.createElement("tr");
    if (!p.eligible) row.style.opacity = "0.5";
    appendText(row.insertCell(), p.user_email, "strong");
    appendText(row.insertCell(), Number(p.tokens_held || 0).toLocaleString());
    appendText(row.insertCell(), `${share}%`);
    appendText(row.insertCell(), `${p.holding_days}d`);
    row.insertCell().appendChild(statusBadge(p.eligible ? "success" : "danger", p.eligible ? "Yes" : "No"));
    const amountCell = row.insertCell();
    amountCell.style.textAlign = "right";
    amountCell.style.fontWeight = "700";
    if (p.eligible && p.payout_cents > 0) {
      amountCell.textContent = formatUsd(p.payout_cents);
    } else {
      appendText(amountCell, "-", "span").style.color = "var(--admin-text-muted)";
    }
    tbody.appendChild(row);
  });

  document.getElementById("btn-process").disabled = false;
  document.getElementById("btn-process").textContent = "Submit for Approval";
  currentSplits = result.payouts;
}

function renderPreview(data) {
  showPreviewPanel();

  document.getElementById("tokens-total-label").textContent =
    `Total Tokens: ${data.total_tokens.toLocaleString()}`;

  const tbody = document.getElementById("splits-body");
  if (!data.splits || data.splits.length === 0) {
    renderEmptyRow(tbody, 6, "No investors found for this asset.");
    document.getElementById("btn-process").disabled = true;
    return;
  }

  clearNode(tbody);
  data.splits.forEach((s) => {
    const share = ((s.tokens / data.total_tokens) * 100).toFixed(2);
    const row = document.createElement("tr");
    appendText(row.insertCell(), s.email, "strong");
    appendText(row.insertCell(), Number(s.tokens || 0).toLocaleString());
    appendText(row.insertCell(), `${share}%`);
    appendText(row.insertCell(), "-");
    row.insertCell().appendChild(statusBadge("success", "Yes"));
    const amountCell = row.insertCell();
    amountCell.style.textAlign = "right";
    amountCell.style.fontWeight = "700";
    amountCell.textContent = formatUsd(s.amount_cents);
    tbody.appendChild(row);
  });

  document.getElementById("btn-process").disabled = false;
  document.getElementById("btn-process").textContent = "Queue for Approval";
  currentSplits = data.splits;
}

function renderEmptyRow(tbody, colspan, message) {
  clearNode(tbody);
  const row = document.createElement("tr");
  const cell = row.insertCell();
  cell.colSpan = colspan;
  cell.style.textAlign = "center";
  cell.style.padding = "40px";
  cell.textContent = message;
  tbody.appendChild(row);
}

function statusBadge(variant, label) {
  const badge = document.createElement("span");
  badge.className = `admin-badge admin-badge--${variant}`;
  badge.textContent = label;
  return badge;
}

async function processBatch() {
  if (
    !await pooolConfirm({
      title: 'Process dividend batch',
      message: `${currentDistributionId ? 'Submit this calculated distribution for a separate admin approval?' : 'Queue this dividend request for Four-Eyes approval?'}`,
      confirmText: 'Process',
      type: 'success',
    })
  )
    return;

  document.getElementById("preview-data").style.display = "none";
  document.getElementById("processing-overlay").style.display = "block";
  document.getElementById("step-2-label").className = "step completed";
  document.getElementById("step-3-label").className = "step active";

  if (currentDistributionId) {
    showPhase9Queued();
    return;
  }

  // Legacy flow
  try {
    const response = await fetch("/api/admin/dividends/process", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        asset_id: selectedAssetId,
        total_amount_cents: currentTotalAmountCents,
      }),
    });

    if (response.ok) {
      const result = await response.json();
      showSuccess(result);
    } else {
      const err = await response.json();
      alert("Processing failed: " + (err.error || "Unknown error"));
      document.getElementById("processing-overlay").style.display = "none";
      document.getElementById("preview-data").style.display = "block";
    }
  } catch (err) {
    alert("Critical error during processing.");
  }
}

function showPhase9Queued() {
  document.getElementById("processing-overlay").style.display = "none";
  document.getElementById("success-message").style.display = "block";
  document.getElementById("step-3-label").className = "step completed";

  const summaryEl = document.getElementById("final-summary-text");
  clearNode(summaryEl);
  appendText(summaryEl, "Distribution calculated and ready for separate admin approval.", "p");
  const detail = appendText(summaryEl, `Distribution ID: ${currentDistributionId}`, "p");
  detail.style.marginTop = "12px";
  detail.style.fontSize = "12px";

  loadDistributionHistory();
}

function showPhase9Success(result) {
  document.getElementById("processing-overlay").style.display = "none";
  document.getElementById("success-message").style.display = "block";
  document.getElementById("step-3-label").className = "step completed";

  const summary = result.summary || {};
  const summaryEl = document.getElementById("final-summary-text");
  clearNode(summaryEl);
  appendText(summaryEl, `Successfully distributed ${formatUsd(summary.total_credited_cents || 0)} to ${summary.holders_credited || 0} eligible investors.`, "p");
  if (summary.holders_skipped > 0) {
    const skipped = appendText(summaryEl, `${summary.holders_skipped} holders were skipped (no wallet found).`, "p");
    skipped.style.marginTop = "8px";
    skipped.style.color = "var(--admin-warning)";
  }
  const id = appendText(summaryEl, `Distribution ID: ${summary.distribution_id || currentDistributionId}`, "p");
  id.style.marginTop = "12px";
  id.style.fontSize = "12px";

  loadDistributionHistory();
}

function showSuccess(result) {
  document.getElementById("processing-overlay").style.display = "none";
  document.getElementById("success-message").style.display = "block";
  document.getElementById("step-3-label").className = "step completed";

  const summaryEl = document.getElementById("final-summary-text");
  clearNode(summaryEl);
  appendText(summaryEl, `Requested distribution of ${formatUsd(currentTotalAmountCents)} across ${currentSplits.length} investors.`, "p");
  const approval = appendText(summaryEl, "Approval Required: This action has been queued for Four-Eyes approval. Another administrator must approve this request before funds are released.", "p");
  approval.style.marginTop = "12px";
  approval.style.color = "var(--admin-warning)";
  const request = appendText(summaryEl, `Request ID: ${result.payout_id || result.approval_id || result.id || "Pending"}`, "p");
  request.style.marginTop = "12px";
  request.style.fontSize = "12px";
}

// ═══════════════════════════════════════════════════════════════
// ── DISTRIBUTION HISTORY (Phase 9) ────────────────────────────
// ═══════════════════════════════════════════════════════════════

async function loadDistributionHistory() {
  const tbody = document.getElementById("distributions-history-body");
  if (!tbody) return;

  try {
    const response = await fetch("/api/admin/dividends/distributions");
    if (!response.ok) {
      renderEmptyRow(tbody, 7, await parseError(response, "Failed to load distributions."));
      return;
    }

    const data = await response.json();

    const distributions = data.distributions || [];
    if (distributions.length === 0) {
      renderEmptyRow(tbody, 7, "No distributions created yet. Use the form above to create one.");
      return;
    }

    clearNode(tbody);
    distributions.forEach((d) => {
      const date = d.created_at ? new Date(d.created_at).toLocaleDateString() : "—";
      const row = document.createElement("tr");
      appendText(row.insertCell(), d.asset_name || "Unknown", "strong");
      const periodCell = row.insertCell();
      periodCell.style.fontSize = "12px";
      periodCell.textContent = d.period || "-";
      const amountCell = row.insertCell();
      amountCell.style.fontWeight = "700";
      amountCell.textContent = d.total_amount_display || formatUsd(d.total_amount_cents);
      appendText(row.insertCell(), d.eligible_holders ?? 0);
      row.insertCell().appendChild(getStatusBadge(d.status));
      const dateCell = row.insertCell();
      dateCell.style.fontSize = "12px";
      dateCell.textContent = date;
      row.insertCell().appendChild(getDistributionActions(d));
      tbody.appendChild(row);
    });
  } catch (err) {
    console.error("Failed to load distribution history:", err);
    renderEmptyRow(tbody, 7, "Network error while loading distributions.");
  }
}

function getStatusBadge(status) {
  switch (status) {
    case 'draft':
      return statusBadge("neutral", "Draft");
    case 'calculated':
      return statusBadge("info", "Calculated");
    case 'approved':
      return statusBadge("warning", "Approved");
    case 'distributed':
      return statusBadge("success", "Distributed");
    case 'cancelled':
      return statusBadge("danger", "Cancelled");
    default:
      return statusBadge("neutral", status || "Unknown");
  }
}

function getDistributionActions(d) {
  const container = document.createElement("div");
  if (d.status === 'calculated') {
    container.appendChild(actionButton("approve", d.id, "Approve", "admin-btn--primary"));
    container.appendChild(actionButton("cancel", d.id, "Cancel", "admin-btn--danger"));
    return container;
  }
  if (d.status === 'approved') {
    container.appendChild(actionButton("execute", d.id, "Execute", "admin-btn--primary"));
    container.appendChild(actionButton("cancel", d.id, "Cancel", "admin-btn--danger"));
    return container;
  }
  if (d.status === 'distributed') {
    const done = appendText(container, "Done", "span");
    done.style.fontSize = "12px";
    done.style.color = "var(--admin-success)";
    return container;
  }
  appendText(container, "-");
  return container;
}

function actionButton(action, id, label, variant) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = `admin-btn ${variant} admin-btn--sm`;
  button.dataset.action = action;
  button.dataset.id = id;
  button.textContent = label;
  if (action !== "approve") button.style.marginLeft = "4px";
  return button;
}

function handleHistoryAction(event) {
  const button = event.target.closest("button[data-action][data-id]");
  if (!button) return;
  const { action, id } = button.dataset;
  if (action === "approve") approveDistribution(id);
  if (action === "execute") executeDistribution(id);
  if (action === "cancel") cancelDistribution(id);
}

async function approveDistribution(distId) {
  if (!await pooolConfirm({ title: 'Approve Distribution', message: 'Approve this dividend distribution?', confirmText: 'Approve', type: 'success' })) return;

  try {
    const resp = await fetch(`/api/admin/dividends/distributions/${distId}/approve`, { method: "POST", headers: { "Content-Type": "application/json" } });
    if (resp.ok) {
      window.pooolToast?.('Distribution approved!', 'success');
      loadDistributionHistory();
    } else {
      const err = await resp.json().catch(() => ({}));
      alert("Approval failed: " + (err.error || "Unknown error"));
    }
  } catch (err) {
    alert("Error: " + err.message);
  }
}

async function executeDistribution(distId) {
  if (!await pooolConfirm({
    title: 'Execute Distribution',
    message: '🔴 This will credit real money to investor wallets. Are you absolutely sure?',
    confirmText: 'Execute Now',
    type: 'danger'
  })) return;

  try {
    const resp = await fetch(`/api/admin/dividends/distributions/${distId}/execute`, { method: "POST", headers: { "Content-Type": "application/json" } });
    if (resp.ok) {
      const result = await resp.json();
      window.pooolToast?.(`Distribution executed! ${result.summary?.holders_credited || 0} wallets credited.`, 'success');
      loadDistributionHistory();
    } else {
      const err = await resp.json().catch(() => ({}));
      alert("Execution failed: " + (err.error || "Unknown error"));
    }
  } catch (err) {
    alert("Critical error: " + err.message);
  }
}

async function cancelDistribution(distId) {
  if (!await pooolConfirm({ title: 'Cancel Distribution', message: 'Cancel this distribution? This cannot be undone.', confirmText: 'Cancel It', type: 'danger' })) return;

  try {
    const resp = await fetch(`/api/admin/dividends/distributions/${distId}/cancel`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ reason: "Cancelled by admin" }),
    });
    if (resp.ok) {
      window.pooolToast?.('Distribution cancelled.', 'info');
      loadDistributionHistory();
    } else {
      alert("Cancel failed.");
    }
  } catch (err) {
    alert("Error: " + err.message);
  }
}
