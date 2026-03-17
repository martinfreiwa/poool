/**
 * Admin — Asset Change Request Review (Diff + Approve/Reject)
 * Fetches from GET /api/admin/change-requests/:id
 */

let requestData = null;
let requestId = null;

// Human-readable field labels
const FIELD_LABELS = {
  title: "Title",
  description: "Description",
  short_description: "Short Description",
  annual_yield_bps: "Annual Yield (bps)",
  capital_appreciation_bps: "Capital Appreciation (bps)",
  occupancy_rate_bps: "Occupancy Rate (bps)",
  video_url: "Video URL",
  location_city: "City",
  location_country: "Country",
  location_address: "Address",
  location_description: "Location Description",
  google_maps_url: "Google Maps URL",
  property_type: "Property Type",
  area: "Area",
  lease_type: "Lease Type",
  lease_term_years: "Lease Term (years)",
  land_size_sqm: "Land Size (sqm)",
  building_size_sqm: "Building Size (sqm)",
  bedrooms: "Bedrooms",
  bathrooms: "Bathrooms",
  construction_status: "Construction Status",
  year_built: "Year Built",
};

document.addEventListener("DOMContentLoaded", () => {
  const params = new URLSearchParams(window.location.search);
  requestId = params.get("id");

  if (!requestId) {
    showError("No change request ID provided.");
    return;
  }

  loadRequest();
});

async function loadRequest() {
  try {
    const resp = await fetch(`/api/admin/change-requests/${requestId}`);
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    requestData = await resp.json();

    document.getElementById("loading-state").style.display = "none";
    document.getElementById("review-content").style.display = "block";

    renderInfo(requestData);
    renderDiff(requestData);

    // Hide actions if already reviewed
    if (requestData.status !== "pending") {
      document.getElementById("decision-actions").innerHTML = `
        <div style="text-align:center; padding: 12px; color: var(--admin-text-muted, #888); font-size: 14px;">
          This request has been <strong>${esc(requestData.status)}</strong>
          ${requestData.admin_notes ? `<br><br><em>"${esc(requestData.admin_notes)}"</em>` : ""}
        </div>
      `;
    }
  } catch (err) {
    showError(`Failed to load: ${err.message}`);
  }
}

function showError(msg) {
  document.getElementById("loading-state").innerHTML = `
    <div style="color: var(--admin-danger, #ef4444); font-weight: 600; margin-bottom: 12px;">${esc(msg)}</div>
    <a href="/admin/asset-change-requests.html" style="color: var(--admin-primary, #3b82f6);">← Back to list</a>
  `;
}

function renderInfo(data) {
  const date = new Date(data.created_at);
  const dateStr = date.toLocaleDateString("en-US", {
    month: "long", day: "numeric", year: "numeric", hour: "2-digit", minute: "2-digit",
  });

  const statusClass =
    data.status === "pending" ? "admin-badge--warning" :
    data.status === "approved" ? "admin-badge--success" : "admin-badge--danger";

  const statusLabel =
    data.status === "pending" ? "Pending Review" :
    data.status === "approved" ? "Approved" : "Rejected";

  document.getElementById("page-title").textContent = `Review: ${data.asset_title}`;

  const proposed = data.proposed_values || {};
  const fieldCount = Object.keys(proposed).length;

  document.getElementById("request-info").innerHTML = `
    <div class="request-info-item">
      <div class="request-info-label">Asset</div>
      <div class="request-info-value">${esc(data.asset_title)}</div>
    </div>
    <div class="request-info-item">
      <div class="request-info-label">Developer</div>
      <div class="request-info-value">${esc(data.developer_name)}<br><span style="font-weight:400; font-size:12px; color: var(--admin-text-muted, #888);">${esc(data.developer_email)}</span></div>
    </div>
    <div class="request-info-item">
      <div class="request-info-label">Fields Changed</div>
      <div class="request-info-value">${fieldCount} field${fieldCount !== 1 ? 's' : ''}</div>
    </div>
    <div class="request-info-item">
      <div class="request-info-label">Status</div>
      <div class="request-info-value"><span class="admin-badge ${statusClass}">${statusLabel}</span></div>
    </div>
    <div class="request-info-item">
      <div class="request-info-label">Submitted</div>
      <div class="request-info-value">${dateStr}</div>
    </div>
  `;
}

function renderDiff(data) {
  const original = data.original_values || {};
  const proposed = data.proposed_values || {};
  const tbody = document.getElementById("diff-body");

  const allKeys = new Set([...Object.keys(original), ...Object.keys(proposed)]);
  let html = "";

  for (const key of allKeys) {
    const label = FIELD_LABELS[key] || key;
    const oldVal = formatValue(key, original[key]);
    const newVal = formatValue(key, proposed[key]);
    const isChanged = JSON.stringify(original[key]) !== JSON.stringify(proposed[key]);

    html += `
      <tr class="${isChanged ? 'diff-row-changed' : ''}">
        <td class="diff-field-label">${esc(label)}</td>
        <td class="${isChanged ? 'diff-old' : ''}">${oldVal}</td>
        <td class="${isChanged ? 'diff-new' : ''}">${newVal}</td>
      </tr>
    `;
  }

  tbody.innerHTML = html || `<tr><td colspan="3" style="text-align:center; padding:40px; color: var(--admin-text-muted, #888);">No changes found</td></tr>`;
}

function formatValue(key, val) {
  if (val === null || val === undefined) return '<span style="color: var(--admin-text-muted, #888);">—</span>';

  // BPS fields → percentage
  if (key.endsWith("_bps")) {
    return `${(val / 100).toFixed(2)}%`;
  }

  // Size fields
  if (key === "land_size_sqm" || key === "building_size_sqm") {
    return `${Number(val).toLocaleString()} sqm`;
  }

  // Truncate long text
  const str = String(val);
  if (str.length > 200) {
    return esc(str.substring(0, 200)) + "…";
  }

  return esc(str);
}

async function approveRequest() {
  if (!confirm("Are you sure you want to approve and apply these changes?")) return;

  const notes = document.getElementById("admin-notes")?.value || "";
  const btn = document.getElementById("btn-approve");
  btn.disabled = true;
  btn.textContent = "Applying...";

  try {
    const resp = await fetch(`/api/admin/change-requests/${requestId}/approve`, {
      method: "POST",
      headers: { 
        "Content-Type": "application/json",
        "X-CSRF-Token": typeof csrfToken === "function" ? csrfToken() : "",
      },
      body: JSON.stringify({ notes: notes || null }),
    });

    const data = await resp.json();
    if (!resp.ok) throw new Error(data.error || "Failed");

    // Show success
    document.getElementById("decision-actions").innerHTML = `
      <div style="text-align:center; padding: 16px; background: rgba(16,185,129,0.1); border-radius: 8px; color: #059669; font-weight: 600;">
        ✓ Changes approved and applied successfully
      </div>
    `;

    // Reload to reflect status
    setTimeout(() => loadRequest(), 1000);
  } catch (err) {
    alert("Error: " + err.message);
    btn.disabled = false;
    btn.textContent = "✓ Approve & Apply";
  }
}

async function rejectRequest() {
  const notes = document.getElementById("admin-notes")?.value || "";
  if (!notes.trim()) {
    alert("Please provide a reason for rejection.");
    document.getElementById("admin-notes")?.focus();
    return;
  }

  if (!confirm("Are you sure you want to reject these changes?")) return;

  const btn = document.getElementById("btn-reject");
  btn.disabled = true;
  btn.textContent = "Rejecting...";

  try {
    const resp = await fetch(`/api/admin/change-requests/${requestId}/reject`, {
      method: "POST",
      headers: { 
        "Content-Type": "application/json",
        "X-CSRF-Token": typeof csrfToken === "function" ? csrfToken() : "",
      },
      body: JSON.stringify({ notes }),
    });

    const data = await resp.json();
    if (!resp.ok) throw new Error(data.error || "Failed");

    document.getElementById("decision-actions").innerHTML = `
      <div style="text-align:center; padding: 16px; background: rgba(239,68,68,0.1); border-radius: 8px; color: #dc2626; font-weight: 600;">
        ✕ Change request rejected
      </div>
    `;

    setTimeout(() => loadRequest(), 1000);
  } catch (err) {
    alert("Error: " + err.message);
    btn.disabled = false;
    btn.textContent = "✕ Reject";
  }
}

function esc(str) {
  if (!str) return "";
  const d = document.createElement("div");
  d.textContent = String(str);
  return d.innerHTML;
}
