/**
 * Developer Submissions Page — Professional Management UI
 * Fetches drafts from API, renders stat cards + searchable/sortable/paginated table.
 */

let allItems = [];
let currentFilter = "all";
let selectedIds = new Set();

// ─── Sorting State ────────────────────────────────────────
let currentSortField = "updated"; // default sort field
let currentSortDir = "desc";       // "asc" or "desc"

// ─── Pagination State ─────────────────────────────────────
const ITEMS_PER_PAGE = 20;
let currentPage = 1;
let totalPages = 1;

function getCsrfToken() {
  if (typeof window.getCsrfToken === "function") return window.getCsrfToken();
  const value = `; ${document.cookie}`;
  const parts = value.split(`; csrf_token=`);
  if (parts.length === 2) return parts.pop().split(";").shift();
  return "";
}

function isDraftDeletable(status) {
  return status === "draft";
}

async function confirmAction(options) {
  if (typeof window.pooolConfirm === "function") return window.pooolConfirm(options);
  return window.confirm(options.message || options.title || "Continue?");
}

async function readApiErrorMessage(resp, fallback) {
  let message = fallback || "Request failed. Please try again.";
  try {
    const raw = await resp.text();
    try {
      const parsed = JSON.parse(raw);
      message = parsed.error || parsed.message || message;
    } catch {
      const stripped = raw.replace(/<[^>]+>/g, "").trim();
      if (stripped && stripped.length < 300) message = stripped;
    }
  } catch {
    // Keep fallback.
  }
  if (resp.status === 401) return "You are not logged in. Please log in and try again.";
  return message;
}

document.addEventListener("DOMContentLoaded", async function () {
  const tbody = document.getElementById("submissions-tbody");
  const loadingEl = document.getElementById("submissions-loading");
  const emptyEl = document.getElementById("submissions-empty-state");
  const tableContainer = document.getElementById("submissions-table-container");
  const statsRow = document.getElementById("sub-stats-row");

  if (!tbody) return;

  initStatCardFilters();

  // Close sort dropdown when clicking outside
  document.addEventListener("click", (e) => {
    const menu = document.getElementById("sub-sort-menu");
    const trigger = document.getElementById("sub-sort-trigger");
    if (menu && menu.style.display !== "none" && !menu.contains(e.target) && !trigger.contains(e.target)) {
      menu.style.display = "none";
    }
  });

  try {
    const res = await fetch("/api/developer/drafts");
    if (!res.ok) throw new Error("Failed to load submissions");
    const data = await res.json();

    loadingEl.style.display = "none";

    if (!data.items || data.items.length === 0) {
      tableContainer.style.display = "none";
      emptyEl.style.display = "block";
      if (statsRow) statsRow.style.display = "none";
      return;
    }

    tableContainer.style.display = "block";
    if (statsRow) statsRow.style.display = "grid";

    allItems = data.items;
    updateStats(allItems);
    applyFiltersAndSort();
  } catch (err) {
    console.error("Error loading submissions:", err);
    loadingEl.innerHTML =
      '<span style="color:#dc2626;">Failed to load submissions. Please try again.</span>';
  }
});

// ─── Constants ────────────────────────────────────────────

const STEP_LABELS = {
  1: "Asset Type",
  2: "Property Info",
  3: "Documents",
  4: "Content & Review",
  5: "Submitted",
};

const STATUS_LABELS = {
  draft: "Draft",
  submitted: "Submitted",
  in_review: "In Review",
  approved: "Approved",
  rejected: "Rejected",
  live: "Live",
  revision_requested: "Revision Requested",
};

const STATUS_SORT_ORDER = {
  draft: 0,
  revision_requested: 1,
  submitted: 2,
  in_review: 3,
  approved: 4,
  rejected: 5,
  live: 6,
};

const STEP_URLS = {
  1: "/developer/add-asset",
  2: "/developer/application-form",
  3: "/developer/document-upload-step3",
  4: "/developer/property-content",
  5: "/developer/assets",
};

// ─── Stats ────────────────────────────────────────────────

function updateStats(items) {
  const counts = { all: items.length, draft: 0, submitted: 0, in_review: 0, approved: 0, rejected: 0, revision_requested: 0 };
  items.forEach((it) => {
    const s = it.project_status || "draft";
    if (counts[s] !== undefined) counts[s]++;
  });

  Object.keys(counts).forEach((key) => {
    const el = document.getElementById("stat-" + key);
    if (el) animateCount(el, counts[key]);
  });
}

function animateCount(el, target) {
  const dur = 400;
  const start = performance.now();
  const from = parseInt(el.textContent) || 0;
  function tick(now) {
    const t = Math.min((now - start) / dur, 1);
    el.textContent = Math.round(from + (target - from) * t);
    if (t < 1) requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);
}

// ─── Result Count ─────────────────────────────────────────

function updateResultCount(shown, total) {
  const el = document.getElementById("sub-result-count");
  if (!el) return;
  if (shown === total) {
    el.textContent = `${total} total`;
  } else {
    el.textContent = `${shown} of ${total}`;
  }
}

// ─── Sort Dropdown (legacy, still functional) ─────────────

function toggleSortDropdown() {
  const menu = document.getElementById("sub-sort-menu");
  if (menu) menu.style.display = menu.style.display === "none" ? "block" : "none";
}

function setSortOrder(sort, btn) {
  // Map legacy sort values to field+dir
  switch (sort) {
    case "newest":  currentSortField = "updated"; currentSortDir = "desc"; break;
    case "oldest":  currentSortField = "updated"; currentSortDir = "asc";  break;
    case "name-az": currentSortField = "title";   currentSortDir = "asc";  break;
    case "name-za": currentSortField = "title";   currentSortDir = "desc"; break;
  }
  // Update active state
  document.querySelectorAll(".sub-sort-option").forEach((o) => o.classList.remove("active"));
  btn.classList.add("active");
  // Update label
  const label = document.getElementById("sub-sort-label");
  if (label) label.textContent = btn.textContent.trim();
  // Close menu
  const menu = document.getElementById("sub-sort-menu");
  if (menu) menu.style.display = "none";
  // Re-render
  currentPage = 1;
  applyFiltersAndSort();
}

// ─── Column Sorting ───────────────────────────────────────

function sortByColumn(field) {
  if (currentSortField === field) {
    // Toggle direction
    currentSortDir = currentSortDir === "asc" ? "desc" : "asc";
  } else {
    currentSortField = field;
    currentSortDir = field === "title" ? "asc" : "desc"; // dates default desc, text default asc
  }
  currentPage = 1;
  updateSortIndicators();
  applyFiltersAndSort();
}

function updateSortIndicators() {
  document.querySelectorAll(".submissions-table th.sortable").forEach((th) => {
    const field = th.dataset.sortField;
    const svg = th.querySelector(".sort-indicator");
    th.classList.remove("sort-active", "sort-asc", "sort-desc");
    if (field === currentSortField) {
      th.classList.add("sort-active", currentSortDir === "asc" ? "sort-asc" : "sort-desc");
    }
  });
}

// ─── Render Table ─────────────────────────────────────────

function renderTable(items) {
  const tbody = document.getElementById("submissions-tbody");
  tbody.innerHTML = "";

  if (items.length === 0) {
    tbody.innerHTML = `
      <tr class="sub-empty-row">
        <td colspan="8">
          <div class="sub-empty-cell">
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/>
            </svg>
            <span>No submissions match your search.</span>
          </div>
        </td>
      </tr>`;
    return;
  }

  items.forEach((item, index) => {
    const itemId = String(item.id || "");
    const safeId = escapeAttr(itemId);
    const jsId = escapeAttr(JSON.stringify(itemId));
    const safeTitle = escapeHtml(item.title || "Untitled asset");
    const jsTitle = escapeAttr(JSON.stringify(item.title || "Untitled asset"));
    const step = item.submission_step || 1;
    const stepLabel = STEP_LABELS[step] || `Step ${step}`;
    const safeStepLabel = escapeHtml(stepLabel);
    const progressPct = Math.min((step / 5) * 100, 100);
    const rawStatus = item.project_status || "draft";
    const status = STATUS_LABELS[rawStatus] ? rawStatus : "draft";
    const statusLabel = STATUS_LABELS[status] || status;
    const safeStatusLabel = escapeHtml(statusLabel);
    const typeLabel = (item.asset_type || "real_estate")
      .replace(/_/g, " ")
      .replace(/\b\w/g, (c) => c.toUpperCase());
    const safeTypeLabel = escapeHtml(typeLabel);
    const updatedDate = item.updated_at
      ? new Date(item.updated_at).toLocaleDateString("en-US", {
          month: "short",
          day: "numeric",
          year: "numeric",
        })
      : "—";
    const createdDate = item.created_at
      ? new Date(item.created_at).toLocaleDateString("en-US", {
          month: "short",
          day: "numeric",
          year: "numeric",
        })
      : "—";
    const safeUpdatedDate = escapeAttr(updatedDate);
    const safeCreatedDate = escapeAttr(createdDate);

    // Relative time
    const relativeTimeUpdated = item.updated_at ? getRelativeTime(item.updated_at) : "";
    const relativeTimeCreated = item.created_at ? getRelativeTime(item.created_at) : "";
    const safeRelativeTimeUpdated = escapeHtml(relativeTimeUpdated || updatedDate);
    const safeRelativeTimeCreated = escapeHtml(relativeTimeCreated || createdDate);

    const resumeUrl = STEP_URLS[step] || "/developer/add-asset";
    const jsResumeUrl = escapeAttr(JSON.stringify(resumeUrl));
    const safeAssetDetailUrl = `/developer/asset-detail?id=${encodeURIComponent(itemId)}`;
    const jsAssetDetailUrl = escapeAttr(JSON.stringify(safeAssetDetailUrl));

    const coverHtml = item.cover_image_url
      ? `<img class="submission-cover-thumb" src="${escapeAttr(safeImageUrl(item.cover_image_url))}" alt="" />`
      : `<div class="submission-cover-placeholder"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#A4A7AE" stroke-width="1.5"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="M21 15l-5-5L5 21"/></svg></div>`;

    const canDelete = isDraftDeletable(status);
    const isSelected = canDelete && selectedIds.has(item.id);
    const checkboxAttrs = canDelete
      ? `data-id="${safeId}" ${isSelected ? 'checked' : ''} onchange="toggleRowSelect(this, ${jsId})"`
      : `data-id="${safeId}" disabled aria-disabled="true" title="Only draft submissions can be deleted"`;

    const tr = document.createElement("tr");
    tr.dataset.status = status;
    tr.dataset.title = (item.title || "").toLowerCase();
    tr.dataset.id = item.id;
    if (isSelected) tr.classList.add("row-selected");
    tr.innerHTML = `
      <td class="col-checkbox">
        <label class="sub-checkbox-label">
                  <input type="checkbox" class="row-checkbox" ${checkboxAttrs}>
          <span class="sub-checkbox-custom"></span>
        </label>
      </td>
      <td>
        <div class="submission-asset-cell">
          ${coverHtml}
          <div class="submission-asset-info">
                    <span class="submission-title">${safeTitle}</span>
                    <span class="submission-asset-meta">
                      <span class="submission-type-badge">${safeTypeLabel}</span>
                      <span class="submission-app-number">#APP-${escapeHtml(itemId.substring(0, 6).toUpperCase())}</span>
            </span>
          </div>
        </div>
      </td>
      <td class="col-progress">
        <div class="submission-progress">
          <span class="submission-progress-label">
                    <span>${safeStepLabel}</span>
            <strong>${step}/5</strong>
          </span>
          <div class="submission-progress-bar">
            <div class="submission-progress-bar__fill${progressPct >= 100 ? ' submission-progress-bar__fill--complete' : ''}" style="width: ${progressPct}%"></div>
          </div>
        </div>
      </td>
      <td>
        <span class="submission-status submission-status--${status}">
          <span class="submission-status-dot"></span>
                  ${safeStatusLabel}
        </span>
      </td>
      <td class="col-created">
                <span class="submission-date" title="${safeCreatedDate}">${safeRelativeTimeCreated}</span>
      </td>
      <td class="col-updated">
                <span class="submission-date" title="${safeUpdatedDate}">${safeRelativeTimeUpdated}</span>
      </td>
      <td class="col-actions">
        <div class="submission-actions">
          ${
            status === "draft"
                      ? `<button class="sub-icon-btn" title="Resume editing" onclick="resumeDraft(${jsId}, ${jsResumeUrl})">
                   <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
                 </button>
                         <button class="sub-icon-btn" title="Duplicate" onclick="duplicateDraft(${jsId})">
                   <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>
                 </button>
                         <button class="sub-icon-btn sub-icon-btn--danger" title="Delete" onclick="confirmDelete(${jsId}, ${jsTitle})">
                   <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
                 </button>`
              : status === "revision_requested"
                      ? `<button class="sub-edit-btn" title="Edit submission" onclick="resumeDraft(${jsId}, '/developer/property-content')">
                   <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
                   Edit
                 </button>
                         <button class="sub-resubmit-btn" title="Resubmit for review" onclick="resubmitDraft(${jsId}, ${jsTitle})">
                   <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 2L11 13"/><path d="M22 2l-7 20-4-9-9-4 20-7z"/></svg>
                   Resubmit
                 </button>`
                      : `<button class="sub-icon-btn" title="View details" onclick="window.location.href=${jsAssetDetailUrl}">
                   <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                 </button>
                         <button class="sub-icon-btn" title="Duplicate" onclick="duplicateDraft(${jsId})">
                   <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>
                 </button>`
          }
        </div>
      </td>
    `;
    tbody.appendChild(tr);

    // If revision_requested, add a notes banner row below
    if (status === "revision_requested" && item.revision_notes) {
      const notesTr = document.createElement("tr");
      notesTr.className = "revision-notes-row";
      notesTr.dataset.status = status;
      notesTr.innerHTML = `
        <td colspan="8">
          <div class="revision-notes-banner">
            <svg class="revision-notes-banner__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            <div class="revision-notes-banner__content">
              <div class="revision-notes-banner__label">Admin Feedback — Changes Required</div>
              <div class="revision-notes-banner__text">${escapeHtml(item.revision_notes)}</div>
            </div>
          </div>
        </td>
      `;
      tbody.appendChild(notesTr);
    }
  });
}

// ─── Relative Time ────────────────────────────────────────

function getRelativeTime(dateStr) {
  const now = new Date();
  const then = new Date(dateStr);
  const diffMs = now - then;
  const diffMin = Math.floor(diffMs / 60000);
  const diffHr = Math.floor(diffMs / 3600000);
  const diffDay = Math.floor(diffMs / 86400000);

  if (diffMin < 1) return "Just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHr < 24) return `${diffHr}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;
  if (diffDay < 30) return `${Math.floor(diffDay / 7)}w ago`;
  return "";  // Fall back to absolute date
}

// ─── Filtering ────────────────────────────────────────────

function initStatCardFilters() {
  document.querySelectorAll(".sub-stat").forEach((card) => {
    card.setAttribute("role", "button");
    card.setAttribute("tabindex", "0");
    card.setAttribute("aria-pressed", card.classList.contains("active") ? "true" : "false");
    card.addEventListener("keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      card.click();
    });
  });
}

function filterByCard(status, el) {
  document.querySelectorAll(".sub-stat").forEach((c) => {
    c.classList.remove("active");
    c.setAttribute("aria-pressed", "false");
  });
  el.classList.add("active");
  el.setAttribute("aria-pressed", "true");
  currentFilter = status;
  currentPage = 1;
  applyFiltersAndSort();
}

function searchSubmissions(query) {
  currentPage = 1;
  applyFiltersAndSort(query.toLowerCase());
}

function applyFiltersAndSort(searchQuery) {
  searchQuery = searchQuery || (document.getElementById("sub-search-input")?.value || "").toLowerCase();

  let filtered = allItems.filter((item) => {
    const statusMatch = currentFilter === "all" || (item.project_status || "draft") === currentFilter;
    const title = (item.title || "").toLowerCase();
    const type = (item.asset_type || "").replace(/_/g, " ").toLowerCase();
    const appNum = (item.id || "").substring(0, 6).toLowerCase();
    const searchMatch = !searchQuery || title.includes(searchQuery) || type.includes(searchQuery) || appNum.includes(searchQuery);
    return statusMatch && searchMatch;
  });

  // Sort by current field/direction
  filtered.sort((a, b) => {
    let cmp = 0;
    switch (currentSortField) {
      case "title":
        cmp = (a.title || "").localeCompare(b.title || "");
        break;
      case "progress":
        cmp = (a.submission_step || 1) - (b.submission_step || 1);
        break;
      case "status":
        cmp = (STATUS_SORT_ORDER[a.project_status || "draft"] || 0) - (STATUS_SORT_ORDER[b.project_status || "draft"] || 0);
        break;
      case "created":
        cmp = new Date(a.created_at || 0) - new Date(b.created_at || 0);
        break;
      case "updated":
      default:
        cmp = new Date(a.updated_at || 0) - new Date(b.updated_at || 0);
        break;
    }
    return currentSortDir === "asc" ? cmp : -cmp;
  });

  // Update result count
  updateResultCount(filtered.length, allItems.length);

  // Update sort header indicators
  updateSortIndicators();

  // Pagination
  totalPages = Math.max(1, Math.ceil(filtered.length / ITEMS_PER_PAGE));
  if (currentPage > totalPages) currentPage = totalPages;
  const startIdx = (currentPage - 1) * ITEMS_PER_PAGE;
  const pageItems = filtered.slice(startIdx, startIdx + ITEMS_PER_PAGE);

  renderTable(pageItems);
  renderPagination(filtered.length);
}

// ─── Pagination ───────────────────────────────────────────

function renderPagination(totalItems) {
  const container = document.getElementById("sub-pagination");
  if (!container) return;

  if (totalItems <= ITEMS_PER_PAGE) {
    container.innerHTML = "";
    return;
  }

  const startItem = (currentPage - 1) * ITEMS_PER_PAGE + 1;
  const endItem = Math.min(currentPage * ITEMS_PER_PAGE, totalItems);

  let pagesHtml = "";

  // Generate page numbers with ellipsis
  const pages = [];
  if (totalPages <= 7) {
    for (let i = 1; i <= totalPages; i++) pages.push(i);
  } else {
    pages.push(1);
    if (currentPage > 3) pages.push("...");
    for (let i = Math.max(2, currentPage - 1); i <= Math.min(totalPages - 1, currentPage + 1); i++) {
      pages.push(i);
    }
    if (currentPage < totalPages - 2) pages.push("...");
    pages.push(totalPages);
  }

  pages.forEach((p) => {
    if (p === "...") {
      pagesHtml += `<span class="sub-page-ellipsis">…</span>`;
    } else {
      pagesHtml += `<button class="sub-page-btn${p === currentPage ? ' sub-page-btn--active' : ''}" onclick="goToPage(${p})">${p}</button>`;
    }
  });

  container.innerHTML = `
    <span class="sub-page-info">Showing ${startItem}–${endItem} of ${totalItems}</span>
    <div class="sub-page-controls">
      <button class="sub-page-nav" ${currentPage <= 1 ? 'disabled' : ''} onclick="goToPage(${currentPage - 1})">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="15 18 9 12 15 6"/></svg>
        Previous
      </button>
      <div class="sub-page-numbers">${pagesHtml}</div>
      <button class="sub-page-nav" ${currentPage >= totalPages ? 'disabled' : ''} onclick="goToPage(${currentPage + 1})">
        Next
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>
      </button>
    </div>
  `;
}

function goToPage(page) {
  if (page < 1 || page > totalPages) return;
  currentPage = page;
  applyFiltersAndSort();
  // Scroll to top of table
  const table = document.getElementById("submissions-table-container");
  if (table) table.scrollIntoView({ behavior: "smooth", block: "start" });
}

// ─── Multi-Select ──────────────────────────────────────────

function toggleRowSelect(checkbox, id) {
  const tr = checkbox.closest("tr");
  if (checkbox.checked) {
    selectedIds.add(id);
    tr.classList.add("row-selected");
  } else {
    selectedIds.delete(id);
    tr.classList.remove("row-selected");
  }
  updateBulkBar();
  syncSelectAllCheckbox();
}

function toggleSelectAll(checked) {
  document.querySelectorAll(".row-checkbox:not(:disabled)").forEach((cb) => {
    const id = cb.dataset.id;
    cb.checked = checked;
    const tr = cb.closest("tr");
    if (checked) {
      selectedIds.add(id);
      tr.classList.add("row-selected");
    } else {
      selectedIds.delete(id);
      tr.classList.remove("row-selected");
    }
  });
  updateBulkBar();
}

function updateBulkBar() {
  const bar = document.getElementById("sub-bulk-bar");
  const countEl = document.getElementById("sub-bulk-count");
  const normalToolbar = document.getElementById("sub-toolbar-normal");
  if (!bar) return;
  if (selectedIds.size > 0) {
    bar.style.display = "flex";
    if (normalToolbar) normalToolbar.style.display = "none";
    countEl.textContent = `${selectedIds.size} selected`;
  } else {
    bar.style.display = "none";
    if (normalToolbar) normalToolbar.style.display = "";
  }
}

function clearSelection() {
  selectedIds.clear();
  document.querySelectorAll(".row-checkbox").forEach((cb) => {
    cb.checked = false;
    cb.closest("tr").classList.remove("row-selected");
  });
  const allCb = document.getElementById("select-all-checkbox");
  if (allCb) allCb.checked = false;
  updateBulkBar();
}

function syncSelectAllCheckbox() {
  const allCb = document.getElementById("select-all-checkbox");
  if (!allCb) return;
  const visibleDraftIds = Array.from(document.querySelectorAll(".row-checkbox:not(:disabled)"))
    .map((cb) => cb.dataset.id);
  const selectedVisible = visibleDraftIds.filter((id) => selectedIds.has(id)).length;
  allCb.checked = visibleDraftIds.length > 0 && selectedVisible === visibleDraftIds.length;
  allCb.indeterminate = selectedVisible > 0 && selectedVisible < visibleDraftIds.length;
}

async function confirmBulkDelete() {
  const count = selectedIds.size;
  if (count === 0) return;
  const ids = Array.from(selectedIds);

  const confirmed = await confirmAction({
    title: `Delete ${count} asset${count > 1 ? "s" : ""}?`,
    message: `This action cannot be undone. The selected draft${count > 1 ? "s" : ""} and all associated data will be permanently removed.`,
    confirmText: `Delete ${count} Draft${count > 1 ? "s" : ""}`,
    type: "danger",
  });
  if (!confirmed) return;

  let failed = 0;
  for (const id of ids) {
    try {
      const res = await fetch(`/api/developer/draft/${id}`, { method: "DELETE", headers: { "X-CSRF-Token": getCsrfToken() } });
      if (!res.ok) failed++;
    } catch {
      failed++;
    }
  }
  if (failed === 0) {
    showToast("success", `${count} draft${count > 1 ? "s" : ""} deleted`);
  } else {
    showToast("error", `${failed} deletion${failed > 1 ? "s" : ""} failed`);
  }
  selectedIds.clear();
  setTimeout(() => window.location.reload(), 800);
}

// ─── Actions ──────────────────────────────────────────────

function resumeDraft(assetId, url) {
  localStorage.setItem("draft_asset_id", assetId);
  window.location.href = url;
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = String(text ?? "");
  return div.innerHTML;
}

function escapeAttr(text) {
  return escapeHtml(text).replace(/'/g, "&#39;");
}

function safeImageUrl(url) {
  try {
    const parsed = new URL(String(url || ""), window.location.origin);
    if (parsed.protocol === "http:" || parsed.protocol === "https:") {
      return parsed.href;
    }
  } catch (_) {
    // Fall through to the inert placeholder.
  }
  return "";
}

async function duplicateDraft(assetId) {
  try {
    const res = await fetch(`/api/developer/draft/${assetId}/duplicate`, {
      method: "POST",
      headers: { "X-CSRF-Token": getCsrfToken() },
    });
    if (!res.ok) {
      throw new Error(await readApiErrorMessage(res, "Duplicate failed"));
    }
    showToast("success", "Asset duplicated");
    setTimeout(() => window.location.reload(), 800);
  } catch (err) {
    console.error("Duplicate error:", err);
    showToast("error", err.message || "Failed to duplicate. Please try again.");
  }
}

async function resubmitDraft(assetId, title) {
  if (!await confirmAction({ title: 'Resubmit for review', message: `Submit "${title}" for admin review? The team will be notified.`, confirmText: 'Resubmit', type: 'success' })) return;
  try {
    const res = await fetch(`/api/developer/draft/${assetId}/submit`, {
      method: "POST",
      headers: { "X-CSRF-Token": getCsrfToken() },
    });
    if (!res.ok) {
      throw new Error(await readApiErrorMessage(res, "Resubmit failed"));
    }
    showToast("success", "Submission sent for review");
    setTimeout(() => window.location.reload(), 800);
  } catch (err) {
    console.error("Resubmit error:", err);
    showToast("error", err.message || "Failed to resubmit. Please try again.");
  }
}

// ─── Delete Confirmation Modal ────────────────────────────

async function confirmDelete(assetId, title) {
  const confirmed = await confirmAction({
    title: `Delete "${title || "Untitled asset"}"?`,
    message: "This action cannot be undone. The draft and all associated data will be permanently removed.",
    confirmText: "Delete Draft",
    type: "danger",
  });
  if (!confirmed) return;

    try {
      const res = await fetch(`/api/developer/draft/${assetId}`, { method: "DELETE", headers: { "X-CSRF-Token": getCsrfToken() } });
      if (!res.ok) throw new Error(await readApiErrorMessage(res, "Delete failed"));
      showToast("success", "Draft deleted");
      setTimeout(() => window.location.reload(), 800);
    } catch (err) {
      console.error("Delete error:", err);
      showToast("error", err.message || "Failed to delete. Please try again.");
    }
}

// ─── Toast ────────────────────────────────────────────────

function showToast(type, msg) {
  if (window.showPooolToast) {
    window.showPooolToast(null, msg, type);
    return;
  }

  let toast = document.getElementById("submissions-toast");
  if (!toast) {
    toast = document.createElement("div");
    toast.id = "submissions-toast";
    toast.style.cssText =
      "position:fixed;top:24px;right:24px;color:#fff;padding:14px 18px;border-radius:8px;z-index:9999;font-size:0.95rem;box-shadow:0 4px 12px rgba(0,0,0,0.15);max-width:420px;";
    document.body.appendChild(toast);
  }
  toast.style.background = type === "success" ? "#079455" : "#d92d20";
  toast.textContent = msg;
  toast.style.display = "block";
  setTimeout(() => {
    toast.style.display = "none";
  }, 5000);
}
