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
let loadError = "";
let lastFocusedElement = null;

document.addEventListener("DOMContentLoaded", () => {
  loadSubmissions();
  setupEventListeners();
  setupSorting();
});

// ─── Sorting ─────────────────────────────────────────────────────────────────
function setupSorting() {
  const table = document.getElementById("submissions-table");
  if (!table) return;
  table.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    th.tabIndex = 0;
    th.setAttribute("role", "button");
    th.setAttribute("aria-sort", "none");
    th.addEventListener("click", () => {
      updateSort(th.dataset.sort);
    });
    th.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        updateSort(th.dataset.sort);
      }
    });
  });
  updateSortHeaders();
}

function updateSort(field) {
  if (sortField === field) {
    sortOrder = sortOrder === "asc" ? "desc" : "asc";
  } else {
    sortField = field;
    sortOrder = "asc";
  }
  applyFilters();
}

function updateSortHeaders() {
  const table = document.getElementById("submissions-table");
  if (!table) return;
  table.querySelectorAll("th[data-sort]").forEach((th) => {
    const value = th.dataset.sort === sortField
      ? (sortOrder === "asc" ? "ascending" : "descending")
      : "none";
    th.setAttribute("aria-sort", value);
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
  document.addEventListener("keydown", (e) => {
    const modal = document.getElementById("review-modal");
    if (!modal || modal.style.display !== "flex") return;
    if (e.key === "Escape") {
      closeModal();
      return;
    }
    if (e.key === "Tab") trapModalFocus(e);
  });
}

// ─── Data Loading ─────────────────────────────────────────────────────────────
async function loadSubmissions() {
  loadError = "";
  try {
    const resp = await fetch("/api/admin/developer-projects");
    if (resp.ok) {
      const data = await resp.json();
      allSubmissions = data.projects || [];
    } else {
      allSubmissions = [];
      const errorBody = await resp.json().catch(() => ({}));
      loadError = errorBody.error || `Failed to load submissions (HTTP ${resp.status})`;
    }
  } catch (e) {
    allSubmissions = [];
    loadError = e.message || "Network error while loading submissions.";
  }
  applyFilters();
  updateStats();
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
  updateSortHeaders();

  const countEl = document.getElementById("sub-count-label");
  if (countEl)
    countEl.textContent = loadError
      ? "Unable to load submissions"
      : `Showing ${filteredSubs.length} submission${filteredSubs.length !== 1 ? "s" : ""}`;
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

  if (loadError) {
    tbody.innerHTML = `
      <tr>
        <td colspan="10" style="text-align:center;padding:40px;color:var(--admin-danger);">
          <div style="font-weight:700;margin-bottom:8px;">Could not load submissions</div>
          <div style="font-size:13px;color:var(--admin-text-secondary);margin-bottom:16px;">${esc(loadError)}</div>
          <button type="button" class="admin-btn admin-btn--secondary admin-btn--sm" id="retry-submissions-load">Retry</button>
        </td>
      </tr>`;
    document.getElementById("retry-submissions-load")?.addEventListener("click", loadSubmissions);
    return;
  }

  if (slice.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="10" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No submissions match your filters.</td></tr>';
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
                            title="Quick status review"
                            aria-label="Quick status review for ${esc(s.project_name || s.title || "submission")}">
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
  lastFocusedElement = document.activeElement;
  reviewingId = id;
  reviewingName = name;

  document.getElementById("review-modal-title").textContent =
    `Quick Review: ${name}`;
  document.getElementById("review-modal-details").innerHTML = `
        <div style="font-size:13px;color:var(--admin-text-secondary);margin-bottom:16px;">
            Publishing requires full document, checklist, and KYC review. Use the
            <a href="/admin/developer-submission-review?id=${esc(id)}" style="color:var(--admin-primary);">Full Review Page</a>.
        </div>
        <p style="font-size:13px;color:var(--admin-text-muted);">
            Quick actions can mark a submission in review or reject it with a required reason.
        </p>
    `;
  document.getElementById("review-notes").value = "";
  document.getElementById("review-modal-reject").onclick = () =>
    handleQuickAction("reject");
  const inReviewBtn = document.getElementById("review-modal-in-review");
  if (inReviewBtn) {
    inReviewBtn.onclick = () => handleQuickAction("in_review");
  }
  document.getElementById("review-modal").style.display = "flex";
  setTimeout(() => document.getElementById("review-notes")?.focus(), 50);
}

function closeModal() {
  document.getElementById("review-modal").style.display = "none";
  reviewingId = null;
  reviewingName = null;
  if (lastFocusedElement && typeof lastFocusedElement.focus === "function") {
    lastFocusedElement.focus();
  }
  lastFocusedElement = null;
}

function trapModalFocus(event) {
  const modal = document.getElementById("review-modal");
  const focusable = modal?.querySelectorAll(
    'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
  );
  if (!focusable || focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

async function handleQuickAction(action) {
  const notes = document.getElementById("review-notes")?.value?.trim() || "";

  if (action === "reject" && !notes) {
    document.getElementById("review-notes").style.borderColor =
      "var(--admin-danger)";
    document.getElementById("review-notes").focus();
    return;
  }

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
      loadSubmissions();
    } else {
      const err = await resp.json();
      alert("Error: " + (err.error || "Failed to process action"));
    }
  } catch (e) {
    alert("Network error. Please try again.");
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
    revision_requested: ["admin-badge--warning", "Revision Requested"],
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
