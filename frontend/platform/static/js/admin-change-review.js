/**
 * Admin — Asset Change Request Review (Diff + Approve/Reject)
 * Fetches from GET /api/admin/change-requests/:id
 *
 * Production hardened:
 * - Regex-based HTML escaping (no DOM allocation in loops)
 * - AbortController timeout on all fetches
 * - Toast notifications instead of alert()
 * - Focus management after approve/reject
 * - Keyboard shortcuts (Cmd+Enter = approve, Cmd+Backspace = reject)
 * - Graceful 403 handling for RBAC
 */

let requestData = null;
let requestId = null;

const FETCH_TIMEOUT_MS = 15000; // 15 seconds

const ESC_MAP = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };
function esc(str) {
  if (!str) return "";
  return String(str).replace(/[&<>"']/g, (m) => ESC_MAP[m]);
}

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

/**
 * Fetch with an AbortController timeout.
 * If the request exceeds FETCH_TIMEOUT_MS, it is aborted.
 */
async function fetchWithTimeout(url, options = {}) {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS);
  try {
    const resp = await fetch(url, { ...options, signal: controller.signal });
    clearTimeout(timeoutId);
    return resp;
  } catch (err) {
    clearTimeout(timeoutId);
    if (err.name === "AbortError") {
      throw new Error("Request timed out. Please check your connection and try again.");
    }
    throw err;
  }
}

document.addEventListener("DOMContentLoaded", () => {
  const params = new URLSearchParams(window.location.search);
  requestId = params.get("id");

  if (!requestId) {
    showError("No change request ID provided.");
    return;
  }

  // Validate UUID format to prevent injection
  const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  if (!uuidPattern.test(requestId)) {
    showError("Invalid change request ID format.");
    return;
  }

  loadRequest();
  setupKeyboardShortcuts();
  setupButtonHandlers();
});

/**
 * Attach click handlers to buttons (replaces inline onclick)
 */
function setupButtonHandlers() {
  const approveBtn = document.getElementById("btn-approve");
  const rejectBtn = document.getElementById("btn-reject");
  if (approveBtn) approveBtn.addEventListener("click", approveRequest);
  if (rejectBtn) rejectBtn.addEventListener("click", rejectRequest);
}

/**
 * Keyboard shortcuts for power users.
 * Cmd/Ctrl + Enter = Approve
 * Cmd/Ctrl + Backspace = Reject
 */
function setupKeyboardShortcuts() {
  document.addEventListener("keydown", (e) => {
    // Only act if request is pending and buttons are visible
    if (!requestData || requestData.status !== "pending") return;
    // Don't intercept if a modal/dialog is open
    if (document.querySelector(".pc-overlay")) return;

    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      approveRequest();
    }
    if ((e.metaKey || e.ctrlKey) && e.key === "Backspace") {
      e.preventDefault();
      rejectRequest();
    }
  });
}

async function loadRequest() {
  try {
    const resp = await fetchWithTimeout(`/api/admin/change-requests/${requestId}`);

    // Graceful RBAC handling
    if (resp.status === 403) {
      showError("You do not have permission to review asset change requests. Contact your administrator.");
      return;
    }
    if (resp.status === 404) {
      showError("Change request not found. It may have been deleted.");
      return;
    }
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);

    requestData = await resp.json();

    document.getElementById("loading-state").style.display = "none";
    document.getElementById("review-content").style.display = "block";

    renderInfo(requestData);
    renderDiff(requestData);

    // Hide actions if already reviewed
    if (requestData.status !== "pending") {
      const actionsEl = document.getElementById("decision-actions");
      const hintEl = document.getElementById("kbd-hint");
      if (actionsEl) {
        actionsEl.innerHTML = `
          <div style="text-align:center; padding: 12px; color: var(--admin-text-muted, #888); font-size: 14px;" role="status">
            This request has been <strong>${esc(requestData.status)}</strong>
            ${requestData.admin_notes ? `<br><br><em>"${esc(requestData.admin_notes)}"</em>` : ""}
          </div>
        `;
      }
      if (hintEl) hintEl.style.display = "none";
    }
  } catch (err) {
    showError(`Failed to load: ${err.message}`);
  }
}

function showError(msg) {
  document.getElementById("loading-state").style.display = "none";
  const errorState = document.getElementById("error-state");
  const errorMessage = document.getElementById("error-message");
  if (errorState && errorMessage) {
    errorMessage.textContent = msg;
    errorState.style.display = "block";
    errorState.focus();
  }
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
      <div class="request-info-value">${esc(dateStr)}</div>
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

  // BPS fields → percentage (always escape after formatting)
  if (key.endsWith("_bps")) {
    const num = Number(val);
    if (isNaN(num)) return esc(String(val));
    return esc(`${(num / 100).toFixed(2)}%`);
  }

  // Size fields (always escape after formatting)
  if (key === "land_size_sqm" || key === "building_size_sqm") {
    const num = Number(val);
    if (isNaN(num)) return esc(String(val));
    return esc(`${num.toLocaleString()} sqm`);
  }

  // Truncate long text
  const str = String(val);
  if (str.length > 200) {
    return esc(str.substring(0, 200)) + "…";
  }

  return esc(str);
}

async function approveRequest() {
  if (!await pooolConfirm({ title: 'Approve changes', message: 'Apply these proposed changes to the live asset?', confirmText: 'Approve & Apply', type: 'success' })) return;

  const notes = document.getElementById("admin-notes")?.value || "";
  const btn = document.getElementById("btn-approve");
  const rejectBtn = document.getElementById("btn-reject");
  btn.disabled = true;
  if (rejectBtn) rejectBtn.disabled = true;
  btn.textContent = "Applying...";

  try {
    const resp = await fetchWithTimeout(`/api/admin/change-requests/${requestId}/approve`, {
      method: "POST",
      headers: { 
        "Content-Type": "application/json",
        "X-CSRF-Token": getCsrfToken(),
      },
      body: JSON.stringify({ notes: notes || null }),
    });

    const data = await resp.json();
    if (!resp.ok) throw new Error(data.error || "Failed to approve");

    // Show inline success
    const actionsEl = document.getElementById("decision-actions");
    if (actionsEl) {
      actionsEl.innerHTML = `
        <div style="text-align:center; padding: 16px; background: rgba(16,185,129,0.1); border-radius: 8px; color: #059669; font-weight: 600;" role="status" tabindex="-1" id="decision-result">
          ✓ Changes approved and applied successfully
        </div>
      `;
      // Move focus to the result for keyboard/screen reader users
      document.getElementById("decision-result")?.focus();
    }
    const hintEl = document.getElementById("kbd-hint");
    if (hintEl) hintEl.style.display = "none";

    // Toast notification
    if (typeof showPooolToast === "function") {
      showPooolToast("Approved", "Changes have been applied to the live asset.", "success");
    }

    // Reload to reflect status
    setTimeout(() => loadRequest(), 1500);
  } catch (err) {
    if (typeof showPooolToast === "function") {
      showPooolToast("Error", err.message, "error");
    }
    btn.disabled = false;
    if (rejectBtn) rejectBtn.disabled = false;
    btn.textContent = "✓ Approve & Apply";
  }
}

async function rejectRequest() {
  const notes = document.getElementById("admin-notes")?.value || "";
  if (!notes.trim()) {
    if (typeof showPooolToast === "function") {
      showPooolToast("Notes Required", "Please provide a reason for rejection.", "warning");
    }
    document.getElementById("admin-notes")?.focus();
    return;
  }

  if (!await pooolConfirm({ title: 'Reject changes', message: 'The developer will be notified with the reason you provided.', confirmText: 'Reject', type: 'danger' })) return;

  const btn = document.getElementById("btn-reject");
  const approveBtn = document.getElementById("btn-approve");
  btn.disabled = true;
  if (approveBtn) approveBtn.disabled = true;
  btn.textContent = "Rejecting...";

  try {
    const resp = await fetchWithTimeout(`/api/admin/change-requests/${requestId}/reject`, {
      method: "POST",
      headers: { 
        "Content-Type": "application/json",
        "X-CSRF-Token": getCsrfToken(),
      },
      body: JSON.stringify({ notes }),
    });

    const data = await resp.json();
    if (!resp.ok) throw new Error(data.error || "Failed to reject");

    const actionsEl = document.getElementById("decision-actions");
    if (actionsEl) {
      actionsEl.innerHTML = `
        <div style="text-align:center; padding: 16px; background: rgba(239,68,68,0.1); border-radius: 8px; color: #dc2626; font-weight: 600;" role="status" tabindex="-1" id="decision-result">
          ✕ Change request rejected
        </div>
      `;
      document.getElementById("decision-result")?.focus();
    }
    const hintEl = document.getElementById("kbd-hint");
    if (hintEl) hintEl.style.display = "none";

    if (typeof showPooolToast === "function") {
      showPooolToast("Rejected", "The developer will be notified.", "error");
    }

    setTimeout(() => loadRequest(), 1500);
  } catch (err) {
    if (typeof showPooolToast === "function") {
      showPooolToast("Error", err.message, "error");
    }
    btn.disabled = false;
    if (approveBtn) approveBtn.disabled = false;
    btn.textContent = "✕ Reject";
  }
}
