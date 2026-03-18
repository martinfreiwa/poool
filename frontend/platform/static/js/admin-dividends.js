/**
 * Admin Dividend Tool — Logic
 */

let currentSplits = [];
let selectedAssetId = "";
let currentTotalAmountCents = 0;

document.addEventListener("DOMContentLoaded", () => {
  loadAssets();
  loadTracking();

  document
    .getElementById("btn-preview")
    ?.addEventListener("click", calculatePreview);
  document
    .getElementById("btn-process")
    ?.addEventListener("click", processBatch);
  document
    .getElementById("btn-cancel")
    ?.addEventListener("click", () => window.location.reload());
});

async function loadAssets() {
  try {
    const response = await fetch("/api/admin/assets");
    if (response.ok) {
      const data = await response.json();
      const select = document.getElementById("asset-select");

      // Unwrap if already a PooolDropdown from poool-dropdown-init.js
      const wrapper = select.closest(".poool-dropdown");
      if (wrapper) {
        wrapper.parentNode.insertBefore(select, wrapper);
        wrapper.remove();
        select.style.display = "";
      }

      select.innerHTML =
        '<option value="">-- Select Published Asset --</option>';
      data.assets.filter(a => a.published === true || a.status === 'live' || a.funding_status === 'funded').forEach((a) => {
        select.innerHTML += `<option value="${a.id}">${a.title} ($${(a.total_value_cents / 100).toLocaleString()})</option>`;
      });

      // Reinitialize the custom dropdown
      if (window.PooolDropdown) {
        PooolDropdown.fromSelect(select, {
          noLabel: true,
          className: select.classList.contains("admin-select")
            ? "poool-dropdown--sm poool-dropdown--inline"
            : "",
        });
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

  if (!selectedAssetId) {
    alert("Please select an asset.");
    return;
  }
  if (!amountVal || amountVal <= 0) {
    alert("Please enter a valid amount.");
    return;
  }

  currentTotalAmountCents = Math.round(parseFloat(amountVal) * 100);

  // Show loading
  document.getElementById("preview-placeholder").innerHTML =
    '<div class="loading-spinner"></div><p>Calculating splits...</p>';

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
      '<tr><td colspan="4" style="text-align:center;padding:40px;">No investors found for this asset.</td></tr>';
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
      message: `Distribute $${(currentTotalAmountCents / 100).toLocaleString()} to ${currentSplits.length} investors? This will be queued for Four-Eyes approval.`,
      confirmText: 'Process',
      type: 'success',
    })
  )
    return;

  document.getElementById("preview-data").style.display = "none";
  document.getElementById("processing-overlay").style.display = "block";
  document.getElementById("step-2-label").className = "step completed";
  document.getElementById("step-3-label").className = "step active";

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

function showSuccess(result) {
  document.getElementById("processing-overlay").style.display = "none";
  document.getElementById("success-message").style.display = "block";
  document.getElementById("step-3-label").className = "step completed";

  document.getElementById("final-summary-text").innerHTML = `
        <p>Requested distribution of $${(currentTotalAmountCents / 100).toLocaleString()} across ${currentSplits.length} investors.</p>
        <p style="margin-top: 12px; color: var(--admin-warning);"><strong>Approval Required:</strong> This action has been queued for Four-Eyes approval. Another administrator must approve this request before funds are released.</p>
        <p style="margin-top: 12px; font-size: 12px;">Request ID: ${result.payout_id || result.approval_id || result.id || "Pending"}</p>
        <div style="margin-top: 24px;">
            <a href="/admin/approvals" class="admin-btn admin-btn--primary" style="text-decoration: none; display: inline-block;">View Pending Approvals</a>
        </div>
    `;
  loadTracking(); // Reload the tracking table to show the new request
}

async function loadTracking() {
  try {
    const response = await fetch("/api/admin/approvals");
    if (response.ok) {
      const data = await response.json();
      const tbody = document.getElementById("distribution-tracking-body");

      const dividendRequests = data.approvals.filter(
        (a) => a.action_type === "dividend.process",
      );

      if (dividendRequests.length === 0) {
        tbody.innerHTML =
          '<tr><td colspan="5" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No distribution requests found.</td></tr>';
        return;
      }

      tbody.innerHTML = dividendRequests
        .map((r) => {
          const dateRaw = r.created_at || "";
          const date = dateRaw ? new Date(dateRaw).toLocaleString() : "N/A";

          let amount = "Unknown";
          if (r.payload && typeof r.payload.total_amount_cents === "number") {
            amount =
              "$" + (r.payload.total_amount_cents / 100).toLocaleString();
          } else if (r.payload && typeof r.payload.amount_cents === "number") {
            amount = "$" + (r.payload.amount_cents / 100).toLocaleString();
          }

          // Fetching asset title is complicated without a lookup map, so we'll just show the ID or fetch it if available in payload
          const assetTitle =
            r.payload && r.payload.asset_title
              ? r.payload.asset_title
              : r.entity_id || "Unknown Asset";

          // Status Badge
          let statusBadge = `<span class="admin-badge admin-badge--warning">Pending</span>`;
          if (r.status === "approved")
            statusBadge = `<span class="admin-badge admin-badge--success">Approved</span>`;
          else if (r.status === "rejected")
            statusBadge = `<span class="admin-badge admin-badge--danger">Rejected</span>`;

          return `
                    <tr>
                        <td>${date}</td>
                        <td style="font-family: monospace; font-size: 12px;">${(r.id || "").substring(0, 8)}...</td>
                        <td>${assetTitle}</td>
                        <td style="font-weight: 700;">${amount}</td>
                        <td>${statusBadge}</td>
                    </tr>
                `;
        })
        .join("");
    }
  } catch (err) {
    console.error('Failed to load dividend tracking:', err);
    if (window.Sentry) Sentry.captureException(err);
  }
}
