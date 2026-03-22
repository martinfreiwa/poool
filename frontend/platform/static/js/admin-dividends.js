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
    csv += `${p.user_email || p.email},${p.tokens_held || p.tokens},${share}%,${p.holding_days || 0},${p.eligible !== false ? 'Yes' : 'No'},$${amountStr}\n`;
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

      select.innerHTML = '<option value="">-- Select Published Asset --</option>';
      data.assets.filter(a => a.published === true || a.status === 'live' || a.funding_status === 'funded').forEach((a) => {
        select.innerHTML += `<option value="${a.id}">${a.title} ($${(a.total_value_cents / 100).toLocaleString()})</option>`;
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

  // Show loading
  document.getElementById("preview-placeholder").innerHTML =
    '<div class="loading-spinner"></div><p>Calculating splits with anti-sniping filter...</p>';

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

      if (response.ok) {
        const data = await response.json();
        if (data.result) {
          currentDistributionId = data.result.distribution_id;
          renderPhase9Preview(data.result);
          return;
        }
      }
    } catch (err) {
      console.warn("Phase 9 API failed, falling back to legacy:", err);
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
  document.getElementById("preview-placeholder").style.display = "none";
  document.getElementById("preview-data").style.display = "block";
  document.getElementById("step-1-label").className = "step completed";
  document.getElementById("step-2-label").className = "step active";

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
    tbody.innerHTML =
      '<tr><td colspan="6" style="text-align:center;padding:40px;">No investors found for this asset.</td></tr>';
    document.getElementById("btn-process").disabled = true;
    return;
  }

  tbody.innerHTML = result.payouts
    .map((p) => {
      const share = (p.percentage_bps / 100).toFixed(2);
      const eligibleBadge = p.eligible
        ? '<span class="admin-badge admin-badge--success">Yes</span>'
        : '<span class="admin-badge admin-badge--danger">No</span>';
      const amountDisplay = p.eligible && p.payout_cents > 0
        ? `$${(p.payout_cents / 100).toLocaleString(undefined, { minimumFractionDigits: 2 })}`
        : '<span style="color: var(--admin-text-muted);">—</span>';

      return `
        <tr style="${!p.eligible ? 'opacity: 0.5;' : ''}">
          <td><strong>${p.user_email}</strong></td>
          <td>${p.tokens_held.toLocaleString()}</td>
          <td>${share}%</td>
          <td>${p.holding_days}d</td>
          <td>${eligibleBadge}</td>
          <td style="text-align:right;font-weight:700;">${amountDisplay}</td>
        </tr>
      `;
    })
    .join("");

  document.getElementById("btn-process").disabled = false;
  currentSplits = result.payouts;
}

function renderPreview(data) {
  document.getElementById("preview-placeholder").style.display = "none";
  document.getElementById("preview-data").style.display = "block";
  document.getElementById("step-1-label").className = "step completed";
  document.getElementById("step-2-label").className = "step active";

  document.getElementById("tokens-total-label").textContent =
    `Total Tokens: ${data.total_tokens.toLocaleString()}`;

  const tbody = document.getElementById("splits-body");
  if (!data.splits || data.splits.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="6" style="text-align:center;padding:40px;">No investors found for this asset.</td></tr>';
    document.getElementById("btn-process").disabled = true;
    return;
  }

  tbody.innerHTML = data.splits
    .map((s) => {
      const share = ((s.tokens / data.total_tokens) * 100).toFixed(2);
      return `
        <tr>
          <td><strong>${s.email}</strong></td>
          <td>${s.tokens.toLocaleString()}</td>
          <td>${share}%</td>
          <td>—</td>
          <td><span class="admin-badge admin-badge--success">Yes</span></td>
          <td style="text-align:right;font-weight:700;">$${(s.amount_cents / 100).toLocaleString(undefined, { minimumFractionDigits: 2 })}</td>
        </tr>
      `;
    })
    .join("");

  document.getElementById("btn-process").disabled = false;
  currentSplits = data.splits;
}

async function processBatch() {
  if (
    !await pooolConfirm({
      title: 'Process dividend batch',
      message: `Distribute $${(currentTotalAmountCents / 100).toLocaleString()} to ${currentSplits.length} investors? ${currentDistributionId ? 'This will use the Phase 9 approval workflow.' : 'This will be queued for Four-Eyes approval.'}`,
      confirmText: 'Process',
      type: 'success',
    })
  )
    return;

  document.getElementById("preview-data").style.display = "none";
  document.getElementById("processing-overlay").style.display = "block";
  document.getElementById("step-2-label").className = "step completed";
  document.getElementById("step-3-label").className = "step active";

  // If Phase 9 distribution exists, approve + execute it
  if (currentDistributionId) {
    try {
      // Step 1: Approve
      const approveResp = await fetch(`/api/admin/dividends/distributions/${currentDistributionId}/approve`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
      });

      if (!approveResp.ok) {
        const err = await approveResp.json().catch(() => ({}));
        alert("Approval failed: " + (err.error || "Unknown error"));
        document.getElementById("processing-overlay").style.display = "none";
        document.getElementById("preview-data").style.display = "block";
        return;
      }

      // Step 2: Execute
      const execResp = await fetch(`/api/admin/dividends/distributions/${currentDistributionId}/execute`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
      });

      if (execResp.ok) {
        const result = await execResp.json();
        showPhase9Success(result);
      } else {
        const err = await execResp.json().catch(() => ({}));
        alert("Execution failed: " + (err.error || "Unknown error"));
        document.getElementById("processing-overlay").style.display = "none";
        document.getElementById("preview-data").style.display = "block";
      }
    } catch (err) {
      alert("Critical error during processing: " + err.message);
      document.getElementById("processing-overlay").style.display = "none";
      document.getElementById("preview-data").style.display = "block";
    }
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

function showPhase9Success(result) {
  document.getElementById("processing-overlay").style.display = "none";
  document.getElementById("success-message").style.display = "block";
  document.getElementById("step-3-label").className = "step completed";

  const summary = result.summary || {};
  document.getElementById("final-summary-text").innerHTML = `
    <p>Successfully distributed $${((summary.total_credited_cents || 0) / 100).toLocaleString(undefined, { minimumFractionDigits: 2 })} to ${summary.holders_credited || 0} eligible investors.</p>
    ${summary.holders_skipped > 0 ? `<p style="margin-top: 8px; color: var(--admin-warning);">${summary.holders_skipped} holders were skipped (no wallet found).</p>` : ''}
    <p style="margin-top: 12px; font-size: 12px;">Distribution ID: ${summary.distribution_id || currentDistributionId}</p>
  `;

  loadDistributionHistory();
}

function showSuccess(result) {
  document.getElementById("processing-overlay").style.display = "none";
  document.getElementById("success-message").style.display = "block";
  document.getElementById("step-3-label").className = "step completed";

  document.getElementById("final-summary-text").innerHTML = `
    <p>Requested distribution of $${(currentTotalAmountCents / 100).toLocaleString()} across ${currentSplits.length} investors.</p>
    <p style="margin-top: 12px; color: var(--admin-warning);"><strong>Approval Required:</strong> This action has been queued for Four-Eyes approval. Another administrator must approve this request before funds are released.</p>
    <p style="margin-top: 12px; font-size: 12px;">Request ID: ${result.payout_id || result.approval_id || result.id || "Pending"}</p>
    </div>
  `;
}

// ═══════════════════════════════════════════════════════════════
// ── DISTRIBUTION HISTORY (Phase 9) ────────────────────────────
// ═══════════════════════════════════════════════════════════════

async function loadDistributionHistory() {
  try {
    const response = await fetch("/api/admin/dividends/distributions");
    if (!response.ok) return;

    const data = await response.json();
    const tbody = document.getElementById("distributions-history-body");
    if (!tbody) return;

    const distributions = data.distributions || [];
    if (distributions.length === 0) {
      tbody.innerHTML =
        '<tr><td colspan="7" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No distributions created yet. Use the form above to create one.</td></tr>';
      return;
    }

    tbody.innerHTML = distributions.map((d) => {
      const statusBadge = getStatusBadge(d.status);
      const actions = getDistributionActions(d);
      const date = d.created_at ? new Date(d.created_at).toLocaleDateString() : "—";

      return `
        <tr>
          <td><strong>${d.asset_name || 'Unknown'}</strong></td>
          <td style="font-size: 12px;">${d.period}</td>
          <td style="font-weight: 700;">${d.total_amount_display}</td>
          <td>${d.eligible_holders}</td>
          <td>${statusBadge}</td>
          <td style="font-size: 12px;">${date}</td>
          <td>${actions}</td>
        </tr>
      `;
    }).join("");
  } catch (err) {
    console.error("Failed to load distribution history:", err);
  }
}

function getStatusBadge(status) {
  switch (status) {
    case 'draft':
      return '<span class="admin-badge admin-badge--neutral">Draft</span>';
    case 'calculated':
      return '<span class="admin-badge admin-badge--info">Calculated</span>';
    case 'approved':
      return '<span class="admin-badge admin-badge--warning">Approved</span>';
    case 'distributed':
      return '<span class="admin-badge admin-badge--success">Distributed</span>';
    case 'cancelled':
      return '<span class="admin-badge admin-badge--danger">Cancelled</span>';
    default:
      return `<span class="admin-badge">${status}</span>`;
  }
}

function getDistributionActions(d) {
  if (d.status === 'calculated') {
    return `
      <button class="admin-btn admin-btn--primary admin-btn--sm" onclick="approveDistribution('${d.id}')">Approve</button>
      <button class="admin-btn admin-btn--danger admin-btn--sm" onclick="cancelDistribution('${d.id}')" style="margin-left: 4px;">Cancel</button>
    `;
  }
  if (d.status === 'approved') {
    return `
      <button class="admin-btn admin-btn--primary admin-btn--sm" onclick="executeDistribution('${d.id}')">Execute</button>
      <button class="admin-btn admin-btn--danger admin-btn--sm" onclick="cancelDistribution('${d.id}')" style="margin-left: 4px;">Cancel</button>
    `;
  }
  if (d.status === 'distributed') {
    return '<span style="font-size: 12px; color: var(--admin-success);">✓ Done</span>';
  }
  return '—';
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

