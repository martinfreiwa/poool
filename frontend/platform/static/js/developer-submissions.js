/**
 * Developer Submissions Page — Professional Management UI
 * Fetches drafts from API, renders stat cards + searchable/sortable table.
 */

let allItems = [];
let currentFilter = "all";
let currentSort = "newest";
let selectedIds = new Set();

function getCsrfToken() {
  const value = `; ${document.cookie}`;
  const parts = value.split(`; csrf_token=`);
  if (parts.length === 2) return parts.pop().split(';').shift();
  return "";
}

document.addEventListener("DOMContentLoaded", async function () {
  const tbody = document.getElementById("submissions-tbody");
  const loadingEl = document.getElementById("submissions-loading");
  const emptyEl = document.getElementById("submissions-empty-state");
  const tableContainer = document.getElementById("submissions-table-container");
  const statsRow = document.getElementById("sub-stats-row");

  if (!tbody) return;

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
    renderTable(allItems);
    updateResultCount(allItems.length, allItems.length);
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

// ─── Sort Dropdown ────────────────────────────────────────

function toggleSortDropdown() {
  const menu = document.getElementById("sub-sort-menu");
  if (menu) menu.style.display = menu.style.display === "none" ? "block" : "none";
}

function setSortOrder(sort, btn) {
  currentSort = sort;
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
  applyFiltersAndSort();
}

// ─── Render Table ─────────────────────────────────────────

function renderTable(items) {
  const tbody = document.getElementById("submissions-tbody");
  tbody.innerHTML = "";

  if (items.length === 0) {
    tbody.innerHTML = `
      <tr class="sub-empty-row">
        <td colspan="6">
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
    const step = item.submission_step || 1;
    const stepLabel = STEP_LABELS[step] || `Step ${step}`;
    const progressPct = Math.min((step / 5) * 100, 100);
    const status = item.project_status || "draft";
    const statusLabel = STATUS_LABELS[status] || status;
    const typeLabel = (item.asset_type || "real_estate")
      .replace(/_/g, " ")
      .replace(/\b\w/g, (c) => c.toUpperCase());
    const updatedDate = item.updated_at
      ? new Date(item.updated_at).toLocaleDateString("en-US", {
          month: "short",
          day: "numeric",
          year: "numeric",
        })
      : "—";

    // Relative time
    const relativeTime = item.updated_at ? getRelativeTime(item.updated_at) : "";

    const resumeUrl = STEP_URLS[step] || "/developer/add-asset";

    const coverHtml = item.cover_image_url
      ? `<img class="submission-cover-thumb" src="${item.cover_image_url}" alt="" />`
      : `<div class="submission-cover-placeholder"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#A4A7AE" stroke-width="1.5"><rect x="3" y="3" width="18" height="18" rx="2"/><circle cx="8.5" cy="8.5" r="1.5"/><path d="M21 15l-5-5L5 21"/></svg></div>`;

    const isSelected = selectedIds.has(item.id);

    const tr = document.createElement("tr");
    tr.dataset.status = status;
    tr.dataset.title = (item.title || "").toLowerCase();
    tr.dataset.id = item.id;
    if (isSelected) tr.classList.add("row-selected");
    tr.innerHTML = `
      <td class="col-checkbox">
        <label class="sub-checkbox-label">
          <input type="checkbox" class="row-checkbox" data-id="${item.id}" ${isSelected ? 'checked' : ''} onchange="toggleRowSelect(this, '${item.id}')">
          <span class="sub-checkbox-custom"></span>
        </label>
      </td>
      <td>
        <div class="submission-asset-cell">
          ${coverHtml}
          <div class="submission-asset-info">
            <span class="submission-title">${escapeHtml(item.title)}</span>
            <span class="submission-asset-meta">
              <span class="submission-type-badge">${typeLabel}</span>
              <span class="submission-app-number">#APP-${(item.id || '').substring(0, 6).toUpperCase()}</span>
            </span>
          </div>
        </div>
      </td>
      <td class="col-progress">
        <div class="submission-progress">
          <span class="submission-progress-label">
            <span>${stepLabel}</span>
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
          ${statusLabel}
        </span>
      </td>
      <td class="col-updated">
        <span class="submission-date" title="${updatedDate}">${relativeTime || updatedDate}</span>
      </td>
      <td class="col-actions">
        <div class="submission-actions">
          ${
            status === "draft"
              ? `<button class="sub-icon-btn" title="Resume editing" onclick="resumeDraft('${item.id}', '${resumeUrl}')"> 
                   <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
                 </button>
                 <button class="sub-icon-btn" title="Duplicate" onclick="duplicateDraft('${item.id}')">
                   <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>
                 </button>
                 <button class="sub-icon-btn sub-icon-btn--danger" title="Delete" onclick="confirmDelete('${item.id}', '${escapeHtml(item.title)}')">
                   <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
                 </button>`
              : status === "revision_requested"
              ? `<button class="sub-edit-btn" title="Edit submission" onclick="resumeDraft('${item.id}', '/developer/property-content')">
                   <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
                   Edit
                 </button>
                 <button class="sub-resubmit-btn" title="Resubmit for review" onclick="resubmitDraft('${item.id}', '${escapeHtml(item.title)}')">
                   <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 2L11 13"/><path d="M22 2l-7 20-4-9-9-4 20-7z"/></svg>
                   Resubmit
                 </button>`
              : `<button class="sub-icon-btn" title="View details" onclick="window.location.href='/developer/asset-detail?id=${item.id}'">
                   <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                 </button>
                 <button class="sub-icon-btn" title="Duplicate" onclick="duplicateDraft('${item.id}')">
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
        <td colspan="6">
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

function filterByCard(status, el) {
  document.querySelectorAll(".sub-stat").forEach((c) => c.classList.remove("active"));
  el.classList.add("active");
  currentFilter = status;
  applyFiltersAndSort();
}

function searchSubmissions(query) {
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

  // Sort
  filtered.sort((a, b) => {
    switch (currentSort) {
      case "oldest":
        return new Date(a.updated_at || 0) - new Date(b.updated_at || 0);
      case "name-az":
        return (a.title || "").localeCompare(b.title || "");
      case "name-za":
        return (b.title || "").localeCompare(a.title || "");
      default: // newest
        return new Date(b.updated_at || 0) - new Date(a.updated_at || 0);
    }
  });

  updateResultCount(filtered.length, allItems.length);
  renderTable(filtered);
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
  document.querySelectorAll(".row-checkbox").forEach((cb) => {
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
  const total = document.querySelectorAll(".row-checkbox").length;
  allCb.checked = total > 0 && selectedIds.size === total;
  allCb.indeterminate = selectedIds.size > 0 && selectedIds.size < total;
}

function confirmBulkDelete() {
  const count = selectedIds.size;
  if (count === 0) return;
  const ids = Array.from(selectedIds);

  const overlay = document.createElement("div");
  overlay.className = "sub-modal-overlay";
  overlay.innerHTML = `
    <div class="sub-modal">
      <div class="sub-modal__icon">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
          <line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/>
        </svg>
      </div>
      <h3 class="sub-modal__title">Delete ${count} asset${count > 1 ? 's' : ''}?</h3>
      <p class="sub-modal__text">This action cannot be undone. The selected draft${count > 1 ? 's' : ''} and all associated data will be permanently removed.</p>
      <div class="sub-modal__actions">
        <button class="sub-modal__btn sub-modal__btn--cancel" onclick="this.closest('.sub-modal-overlay').remove()">Cancel</button>
        <button class="sub-modal__btn sub-modal__btn--danger" id="confirm-bulk-delete-btn">Delete ${count} Draft${count > 1 ? 's' : ''}</button>
      </div>
    </div>
  `;
  document.body.appendChild(overlay);

  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) overlay.remove();
  });

  document.getElementById("confirm-bulk-delete-btn").onclick = async () => {
    const btn = document.getElementById("confirm-bulk-delete-btn");
    btn.textContent = "Deleting...";
    btn.disabled = true;
    let failed = 0;
    for (const id of ids) {
      try {
        const res = await fetch(`/api/developer/draft/${id}`, { method: "DELETE", headers: { "X-CSRF-Token": getCsrfToken() } });
        if (!res.ok) failed++;
      } catch { failed++; }
    }
    overlay.remove();
    if (failed === 0) {
      showToast("success", `${count} draft${count > 1 ? 's' : ''} deleted`);
    } else {
      showToast("error", `${failed} deletion${failed > 1 ? 's' : ''} failed`);
    }
    selectedIds.clear();
    setTimeout(() => window.location.reload(), 800);
  };
}

// ─── Actions ──────────────────────────────────────────────

function resumeDraft(assetId, url) {
  localStorage.setItem("draft_asset_id", assetId);
  window.location.href = url;
}

function escapeHtml(text) {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

async function duplicateDraft(assetId) {
  try {
    const res = await fetch(`/api/developer/draft/${assetId}/duplicate`, {
      method: "POST",
      headers: { "X-CSRF-Token": getCsrfToken() },
    });
    if (!res.ok) throw new Error("Duplicate failed");
    showToast("success", "Asset duplicated");
    setTimeout(() => window.location.reload(), 800);
  } catch (err) {
    console.error("Duplicate error:", err);
    showToast("error", "Failed to duplicate. Please try again.");
  }
}

async function resubmitDraft(assetId, title) {
  if (!confirm(`Resubmit "${title}" for review?`)) return;
  try {
    const res = await fetch(`/api/developer/draft/${assetId}/submit`, {
      method: "POST",
      headers: { "X-CSRF-Token": getCsrfToken() },
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.message || "Resubmit failed");
    }
    showToast("success", "Submission sent for review");
    setTimeout(() => window.location.reload(), 800);
  } catch (err) {
    console.error("Resubmit error:", err);
    showToast("error", err.message || "Failed to resubmit. Please try again.");
  }
}

// ─── Delete Confirmation Modal ────────────────────────────

function confirmDelete(assetId, title) {
  const overlay = document.createElement("div");
  overlay.className = "sub-modal-overlay";
  overlay.innerHTML = `
    <div class="sub-modal">
      <div class="sub-modal__icon">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
          <line x1="10" y1="11" x2="10" y2="17"/><line x1="14" y1="11" x2="14" y2="17"/>
        </svg>
      </div>
      <h3 class="sub-modal__title">Delete "${title}"?</h3>
      <p class="sub-modal__text">This action cannot be undone. The draft and all associated data will be permanently removed.</p>
      <div class="sub-modal__actions">
        <button class="sub-modal__btn sub-modal__btn--cancel" onclick="this.closest('.sub-modal-overlay').remove()">Cancel</button>
        <button class="sub-modal__btn sub-modal__btn--danger" id="confirm-delete-btn">Delete Draft</button>
      </div>
    </div>
  `;
  document.body.appendChild(overlay);

  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) overlay.remove();
  });

  document.getElementById("confirm-delete-btn").onclick = async () => {
    const btn = document.getElementById("confirm-delete-btn");
    btn.textContent = "Deleting...";
    btn.disabled = true;
    try {
      const res = await fetch(`/api/developer/draft/${assetId}`, { method: "DELETE", headers: { "X-CSRF-Token": getCsrfToken() } });
      if (!res.ok) throw new Error("Delete failed");
      overlay.remove();
      showToast("success", "Draft deleted");
      setTimeout(() => window.location.reload(), 800);
    } catch (err) {
      console.error("Delete error:", err);
      overlay.remove();
      showToast("error", "Failed to delete. Please try again.");
    }
  };
}

// ─── Toast ────────────────────────────────────────────────

function showToast(type, msg) {
  const t = document.createElement("div");
  t.className = `sub-toast sub-toast--${type}`;
  t.textContent = msg;
  document.body.appendChild(t);
  setTimeout(() => t.remove(), 3000);
}
