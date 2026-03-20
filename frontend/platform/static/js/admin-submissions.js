/**
 * admin-submissions.js
 * Developer Submissions Queue — reads from /api/admin/developer-projects
 * (canonical source of truth: developer_projects table joined with assets)
 */

// ─── State ──────────────────────────────────────────────────────────────────
let allSubmissions = [];
let filteredSubs = [];
let currentPage = 1;
const PAGE_SIZE = 15;
let sortField = "created_at";
let sortOrder = "desc";

document.addEventListener("DOMContentLoaded", () => {
  loadSubmissions();
  setupEventListeners();
  setupSorting();
  setupKeyboardHandlers();
});

// ─── Sorting ─────────────────────────────────────────────────────────────────
function setupSorting() {
  const table = document.getElementById("submissions-table");
  if (!table) return;
  table.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    th.addEventListener("click", () => {
      const field = th.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      applyFilters();
    });
  });
}

// ─── Event Listeners ─────────────────────────────────────────────────────────
function setupEventListeners() {
  document
    .getElementById("sub-search")
    ?.addEventListener("input", debounce(applyFilters, 250));
  document
    .getElementById("filter-status")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("filter-type")
    ?.addEventListener("change", applyFilters);
  document.getElementById("prev-page")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      renderTable();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    const totalPages = Math.ceil(filteredSubs.length / PAGE_SIZE);
    if (currentPage < totalPages) {
      currentPage++;
      renderTable();
    }
  });

  // Quick review modal
  document
    .getElementById("review-modal-cancel")
    ?.addEventListener("click", closeModal);
  document.getElementById("review-modal")?.addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeModal();
  });

  // Export CSV
  document.getElementById("btn-export-csv")?.addEventListener("click", exportSubmissionsCsv);
}

// ─── Keyboard Handlers ──────────────────────────────────────────────────────
function setupKeyboardHandlers() {
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      const modal = document.getElementById("review-modal");
      if (modal && modal.style.display !== "none") {
        closeModal();
      }
    }
  });
}

// ─── Data Loading ─────────────────────────────────────────────────────────────
async function loadSubmissions() {
  try {
    const resp = await fetch("/api/admin/developer-projects");
    if (resp.ok) {
      const data = await resp.json();
      allSubmissions = data.projects || [];
    } else {
      allSubmissions = [];
      showLoadError();
      return;
    }
  } catch (e) {
    allSubmissions = [];
    showLoadError();
    return;
  }
  applyFilters();
  updateStats();
}

function showLoadError() {
  const tbody = document.getElementById("submissions-table-body");
  if (!tbody) return;
  tbody.innerHTML = `
    <tr>
      <td colspan="10" style="text-align:center;padding:40px;">
        <div style="color:var(--admin-danger);margin-bottom:12px;">Failed to load submissions.</div>
        <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="loadSubmissions()">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/></svg>
          Retry
        </button>
      </td>
    </tr>`;
}

// ─── KPI Stats ────────────────────────────────────────────────────────────────
function updateStats() {
  // Use developer_projects.status (not assets.published boolean hack)
  const pending = allSubmissions.filter(
    (s) => s.status === "submitted" || s.status === "in_review",
  ).length;
  const approved = allSubmissions.filter(
    (s) => s.status === "approved" || s.status === "live",
  ).length;
  const rejected = allSubmissions.filter((s) => s.status === "rejected").length;
  const totalValue = allSubmissions.reduce(
    (sum, s) => sum + (s.total_value_cents || 0),
    0,
  );

  const el = (id) => document.getElementById(id);
  if (el("stat-pending")) el("stat-pending").textContent = pending;
  if (el("stat-approved")) el("stat-approved").textContent = approved;
  if (el("stat-rejected")) el("stat-rejected").textContent = rejected;
  if (el("stat-total-value"))
    el("stat-total-value").textContent = formatUSD(totalValue);
}

// ─── Filters ──────────────────────────────────────────────────────────────────
function applyFilters() {
  const search = (
    document.getElementById("sub-search")?.value || ""
  ).toLowerCase();
  const status = document.getElementById("filter-status")?.value || "";
  const type = document.getElementById("filter-type")?.value || "";

  let result = allSubmissions.filter((s) => {
    // Status filter using developer_projects.status
    if (status && s.status !== status) return false;
    // Asset type filter
    if (type && s.asset_type !== type) return false;
    // Text search: project name, asset title, developer name/email
    if (search) {
      const hay =
        `${s.project_name} ${s.title} ${s.developer_name} ${s.developer_email} ${s.location_city}`.toLowerCase();
      if (!hay.includes(search)) return false;
    }
    return true;
  });

  // Sort
  result.sort((a, b) => {
    let valA = a[sortField] ?? "";
    let valB = b[sortField] ?? "";
    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredSubs = result;
  currentPage = 1;

  const countEl = document.getElementById("sub-count-label");
  if (countEl)
    countEl.textContent = `Showing ${filteredSubs.length} submission${filteredSubs.length !== 1 ? "s" : ""}`;
  renderTable();
}

// ─── Table Render ─────────────────────────────────────────────────────────────
function renderTable() {
  const tbody = document.getElementById("submissions-table-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(filteredSubs.length / PAGE_SIZE));
  currentPage = Math.min(currentPage, totalPages);
  const start = (currentPage - 1) * PAGE_SIZE;
  const slice = filteredSubs.slice(start, start + PAGE_SIZE);

  // Update pagination
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${filteredSubs.length} total)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;

  if (slice.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="9" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No submissions match your filters.</td></tr>';
    return;
  }

  tbody.innerHTML = slice
    .map(
      (s) => `
        <tr data-id="${esc(s.id)}">
            <td>
                <div style="font-weight:600;color:var(--admin-text-primary);margin-bottom:2px;">${esc(s.title || s.project_name)}</div>
                <div style="display:flex;gap:6px;align-items:center;">
                    <code style="font-family:monospace;font-size:10px;padding:2px 4px;background:var(--admin-border);border-radius:4px;color:var(--admin-text-secondary);font-weight:500;">#APP-${(s.id || '').substring(0, 6).toUpperCase()}</code>
                    <div style="font-size:11px;color:var(--admin-text-muted);">${esc(s.title || "—")}</div>
                </div>
            </td>
            <td>${getTypeBadge(s.asset_type)}</td>
            <td>
                <div class="admin-user-inline">
                    <div>
                        <div class="admin-user-inline-name">
                            <a href="/admin/user-details?id=${esc(s.developer_user_id)}"
                               style="color:var(--admin-primary);text-decoration:none;">
                               ${esc(s.developer_name)}
                            </a>
                        </div>
                        <div class="admin-user-inline-email">${esc(s.developer_email)}</div>
                    </div>
                </div>
            </td>
            <td>
                ${s.asset_id
          ? `<a href="/admin/asset-details?id=${esc(s.asset_id)}"
                          style="font-size:12px;color:var(--admin-primary);">${esc(s.title || s.asset_id)}</a>`
          : '<span style="color:var(--admin-text-muted);font-size:12px;">No asset</span>'
        }
            </td>
            <td>${getStatusBadge(s.status)}</td>
            <td style="font-weight:700;font-variant-numeric:tabular-nums;">${formatUSD(s.total_raised_cents)}</td>
            <td style="font-variant-numeric:tabular-nums;text-align:center;">${(s.investors_count || 0).toLocaleString()}</td>
            <td>
                <div style="display:flex;align-items:center;gap:6px;">
                    <div style="flex:1;background:var(--admin-border);border-radius:4px;height:6px;min-width:60px;">
                        <div style="background:var(--admin-primary);border-radius:4px;height:6px;width:${Math.min(100, (s.funding_progress_bps || 0) / 100)}%"></div>
                    </div>
                    <span style="font-size:11px;color:var(--admin-text-muted);white-space:nowrap;">${((s.funding_progress_bps || 0) / 100).toFixed(0)}%</span>
                </div>
            </td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDate(s.created_at)}</td>
            <td>
                <div style="display:flex;gap:4px;">
                    <a class="admin-btn admin-btn--primary admin-btn--sm"
                       href="/admin/developer-submission-review?id=${esc(s.id)}"
                       title="Full deep-dive review">
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M1 8s3-6 7-6 7 6 7 6-3 6-7 6-7-6-7-6z"/><circle cx="8" cy="8" r="2.5"/></svg>
                        Review
                    </a>
                    <button class="admin-btn admin-btn--secondary admin-btn--sm"
                            onclick="openQuickModal('${esc(s.id)}', '${esc(s.project_name || s.title)}')"
                            title="Quick approve/reject">
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M3 8l3.5 3.5L13 4"/></svg>
                    </button>
                </div>
            </td>
        </tr>
    `,
    )
    .join("");
}

// ─── Quick Review Modal ───────────────────────────────────────────────────────
let reviewingId = null;
let reviewingName = null;

function openQuickModal(id, name) {
  reviewingId = id;
  reviewingName = name;

  // Safe DOM assignment — prevents XSS via project name
  const titleEl = document.getElementById("review-modal-title");
  titleEl.textContent = "";
  titleEl.appendChild(document.createTextNode("Quick Review: " + name));

  document.getElementById("review-modal-details").innerHTML = `
        <div style="font-size:13px;color:var(--admin-text-secondary);margin-bottom:16px;">
            For full document data room, financials, and KYC status, please use the 
            <a href="/admin/developer-submission-review?id=${esc(id)}" style="color:var(--admin-primary);">Full Review Page</a>.
        </div>
        <p style="font-size:13px;color:var(--admin-text-muted);">
            Quick actions will immediately update the project status and notify the developer.
        </p>
    `;
  document.getElementById("review-notes").value = "";
  document.getElementById("review-modal-approve").onclick = () =>
    handleQuickAction("approve");
  document.getElementById("review-modal-reject").onclick = () =>
    handleQuickAction("reject");
  const inReviewBtn = document.getElementById("review-modal-in-review");
  if (inReviewBtn) {
    inReviewBtn.onclick = () => handleQuickAction("in_review");
  }
  document.getElementById("review-modal").style.display = "flex";
  // Focus the modal for keyboard accessibility
  document.getElementById("review-notes").focus();
}

function closeModal() {
  document.getElementById("review-modal").style.display = "none";
  reviewingId = null;
  reviewingName = null;
}

async function handleQuickAction(action) {
  const notes = document.getElementById("review-notes")?.value?.trim() || "";

  if (action === "reject" && !notes) {
    document.getElementById("review-notes").style.borderColor =
      "var(--admin-danger)";
    document.getElementById("review-notes").focus();
    showAdminToast("warning", "Please provide a rejection reason.");
    return;
  }

  // Disable buttons during request
  const approveBtn = document.getElementById("review-modal-approve");
  const rejectBtn = document.getElementById("review-modal-reject");
  const inReviewBtn = document.getElementById("review-modal-in-review");
  [approveBtn, rejectBtn, inReviewBtn].forEach(b => { if (b) b.disabled = true; });

  try {
    const resp = await fetch(
      `/api/admin/developer-projects/${reviewingId}/review`,
      {
        method: "POST",
        headers: { 
          "Content-Type": "application/json",
          "X-CSRF-Token": getCsrfToken()
        },
        body: JSON.stringify({ action, notes }),
      },
    );
    if (resp.ok) {
      closeModal();
      showAdminToast("success", `Submission ${action === 'approve' ? 'approved' : action === 'reject' ? 'rejected' : 'marked in review'} successfully.`);
      // Optimistic UI: update the item in allSubmissions, re-render without full reload
      const newStatus = action === "approve" ? "approved" : action === "reject" ? "rejected" : "in_review";
      const idx = allSubmissions.findIndex(s => s.id === reviewingId);
      if (idx !== -1) {
        allSubmissions[idx].status = newStatus;
      }
      applyFilters();
      updateStats();
    } else {
      const err = await resp.json().catch(() => ({}));
      showAdminToast("error", err.error || "Failed to process action.");
    }
  } catch (e) {
    showAdminToast("error", "Network error. Please try again.");
  } finally {
    [approveBtn, rejectBtn, inReviewBtn].forEach(b => { if (b) b.disabled = false; });
  }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────
function esc(str) {
  if (typeof str !== "string") return str || "";
  const d = document.createElement("div");
  d.textContent = str;
  return d.innerHTML;
}

function formatUSD(cents) {
  if (typeof cents !== "number") return "$0";
  return (
    "$" +
    (Math.abs(cents) / 100).toLocaleString("en-US", {
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    })
  );
}

function formatDate(iso) {
  if (!iso) return "—";
  return new Date(iso).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

/**
 * Status badge using developer_projects.status enum:
 * draft | submitted | in_review | approved | rejected | live
 */
function getStatusBadge(status) {
  const map = {
    draft: ["admin-badge--neutral", "Draft"],
    submitted: ["admin-badge--warning", "Submitted"],
    in_review: ["admin-badge--info", "In Review"],
    approved: ["admin-badge--success", "Approved"],
    rejected: ["admin-badge--danger", "Rejected"],
    live: ["admin-badge--success", "Live"],
  };
  const [cls, label] = map[status] || [
    "admin-badge--neutral",
    status || "Unknown",
  ];
  return `<span class="admin-badge ${cls}">${label}</span>`;
}

function getTypeBadge(type) {
  const labels = {
    real_estate: "Real Estate",
    commercial_property: "Commercial",
    commodity: "Commodity",
    business: "Business",
    startup: "Startup",
    land_plot: "Land",
  };
  return `<span class="admin-badge admin-badge--neutral">${labels[type] || type || "—"}</span>`;
}

function debounce(fn, ms) {
  let t;
  return function (...args) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, args), ms);
  };
}

// ─── Toast Notification ──────────────────────────────────────────────────────
function showAdminToast(type, message) {
  if (window.showPooolToast) {
    window.showPooolToast(null, message, type);
    return;
  }
  // Fallback: create inline toast
  let container = document.getElementById("admin-toast-container");
  if (!container) {
    container = document.createElement("div");
    container.id = "admin-toast-container";
    container.className = "admin-toast-container";
    container.setAttribute("aria-live", "polite");
    document.body.appendChild(container);
  }
  const colors = { success: "#059669", error: "#dc2626", warning: "#d97706", info: "#2563eb" };
  const toast = document.createElement("div");
  toast.style.cssText = `padding:12px 20px;border-radius:8px;background:${colors[type] || colors.info};color:#fff;font-size:13px;font-weight:600;margin-bottom:8px;box-shadow:0 4px 12px rgba(0,0,0,0.15);animation:admin-fadeIn 0.2s ease;`;
  toast.textContent = message;
  container.appendChild(toast);
  setTimeout(() => { toast.style.opacity = "0"; toast.style.transition = "opacity 0.3s"; }, 3500);
  setTimeout(() => toast.remove(), 4000);
}

// ─── CSV Export ──────────────────────────────────────────────────────────────
function exportSubmissionsCsv() {
  if (filteredSubs.length === 0) {
    showAdminToast("warning", "No submissions to export.");
    return;
  }
  const headers = ["ID", "Project Name", "Title", "Type", "Developer", "Email", "Status", "Total Raised (USD)", "Investors", "Progress %", "Created"];
  const rows = filteredSubs.map(s => [
    s.id || "",
    (s.project_name || "").replace(/"/g, '""'),
    (s.title || "").replace(/"/g, '""'),
    s.asset_type || "",
    (s.developer_name || "").replace(/"/g, '""'),
    s.developer_email || "",
    s.status || "",
    ((s.total_raised_cents || 0) / 100).toFixed(2),
    s.investors_count || 0,
    ((s.funding_progress_bps || 0) / 100).toFixed(1),
    s.created_at || ""
  ]);
  const csvContent = [headers, ...rows].map(r => r.map(c => `"${c}"`).join(",")).join("\n");
  const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `developer-submissions-${new Date().toISOString().split("T")[0]}.csv`;
  a.click();
  URL.revokeObjectURL(url);
  showAdminToast("success", `Exported ${filteredSubs.length} submissions.`);
}
