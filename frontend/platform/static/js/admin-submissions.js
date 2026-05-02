/**
 * admin-submissions.js
 * Developer Submissions Queue — reads from /api/admin/developer-projects
 * (canonical source of truth: developer_projects table joined with assets)
 *
 * Reviewer-Assignment ("Mine only" / "Assign to Me") is a localStorage MVP.
 * Pinned IDs live under PIN_STORAGE_KEY scoped to the current admin user id.
 * Backend migration (developer_projects.assigned_admin_id) is the next step
 * to make this multi-device and audit-loggable.
 */

// ─── State ──────────────────────────────────────────────────────────────────
let allSubmissions = [];
let filteredSubs = [];
let currentPage = 1;
const PAGE_SIZE = 15;
let sortField = "age_hours";
let sortOrder = "desc";
let loadError = "";
let lastFocusedElement = null;
let kpiActiveFilter = ""; // "", "pending", "approved", "rejected"
let selectedIds = new Set();
let lastLoadedAt = null;
let autoRefreshTimer = null;
const AUTO_REFRESH_MS = 60_000;

// SLA thresholds (hours)
const SLA_FRESH_MAX_H = 24;
const SLA_AGING_MAX_H = 72;

// Pin storage scoped to current admin user
const PIN_STORAGE_KEY = (() => {
  const uid =
    (window.userData && window.userData.id) ||
    (window.userData && window.userData.user_id) ||
    "anon";
  return `poool.admin.submissions.pinned.${uid}`;
})();

function loadPinnedIds() {
  try {
    const raw = localStorage.getItem(PIN_STORAGE_KEY);
    return new Set(raw ? JSON.parse(raw) : []);
  } catch (e) {
    return new Set();
  }
}
function savePinnedIds(set) {
  try {
    localStorage.setItem(PIN_STORAGE_KEY, JSON.stringify([...set]));
  } catch (e) {}
}
let pinnedIds = loadPinnedIds();

// ─── Boot ───────────────────────────────────────────────────────────────────
document.addEventListener("DOMContentLoaded", () => {
  loadSubmissions();
  setupEventListeners();
  setupSorting();
  hydrateFromUrl();
  startAutoRefresh();
  // Tick the "last updated" indicator every 15s so it stays fresh
  setInterval(updateLastUpdatedIndicator, 15_000);
});

function startAutoRefresh() {
  stopAutoRefresh();
  autoRefreshTimer = setInterval(() => {
    if (document.visibilityState === "visible") loadSubmissions();
  }, AUTO_REFRESH_MS);
}
function stopAutoRefresh() {
  if (autoRefreshTimer) clearInterval(autoRefreshTimer);
  autoRefreshTimer = null;
}

function updateLastUpdatedIndicator() {
  const el = document.getElementById("last-updated-indicator");
  if (!el || !lastLoadedAt) return;
  const secs = Math.round((Date.now() - lastLoadedAt.getTime()) / 1000);
  let txt;
  if (secs < 10) txt = "Updated just now";
  else if (secs < 60) txt = `Updated ${secs}s ago`;
  else if (secs < 3600) txt = `Updated ${Math.round(secs / 60)}m ago`;
  else txt = `Updated ${Math.round(secs / 3600)}h ago`;
  el.textContent = `${txt} • Auto-refresh 60s`;
}

function hydrateFromUrl() {
  const params = new URLSearchParams(location.search);
  const status = params.get("status");
  const kpi = params.get("kpi");
  if (status) {
    const el = document.getElementById("filter-status");
    if (el) el.value = status;
  }
  if (kpi) {
    kpiActiveFilter = kpi;
    highlightActiveKpi();
  }
}

// ─── Sorting ─────────────────────────────────────────────────────────────────
function setupSorting() {
  const table = document.getElementById("submissions-table");
  if (!table) return;
  table.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    th.tabIndex = 0;
    th.setAttribute("role", "button");
    th.setAttribute("aria-sort", "none");
    th.addEventListener("click", () => updateSort(th.dataset.sort));
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
    const isActive = th.dataset.sort === sortField;
    const value = isActive ? (sortOrder === "asc" ? "ascending" : "descending") : "none";
    th.setAttribute("aria-sort", value);
    // Visual sort arrow
    let arrow = th.querySelector(".sort-arrow");
    if (!arrow) {
      arrow = document.createElement("span");
      arrow.className = "sort-arrow";
      arrow.style.cssText = "margin-left:4px;font-size:10px;opacity:0.5;display:inline-block;width:8px;";
      th.appendChild(arrow);
    }
    if (isActive) {
      arrow.textContent = sortOrder === "asc" ? "▲" : "▼";
      arrow.style.opacity = "1";
      arrow.style.color = "var(--admin-primary, #4f46e5)";
    } else {
      arrow.textContent = "▾";
      arrow.style.opacity = "0.3";
      arrow.style.color = "";
    }
  });
}

// ─── Event Listeners ─────────────────────────────────────────────────────────
function setupEventListeners() {
  const onFilter = () => {
    currentPage = 1;
    applyFilters();
  };
  document
    .getElementById("sub-search")
    ?.addEventListener("input", debounce(onFilter, 250));
  document.getElementById("filter-status")?.addEventListener("change", () => {
    kpiActiveFilter = "";
    highlightActiveKpi();
    onFilter();
  });
  document.getElementById("filter-type")?.addEventListener("change", onFilter);
  document.getElementById("filter-age")?.addEventListener("change", onFilter);
  document.getElementById("filter-mine")?.addEventListener("change", onFilter);

  // KPI cards as filters
  document.querySelectorAll("[data-kpi-filter]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const val = btn.getAttribute("data-kpi-filter");
      kpiActiveFilter = kpiActiveFilter === val ? "" : val;
      // Clear status select when KPI filter takes over
      const statusEl = document.getElementById("filter-status");
      if (statusEl) statusEl.value = "";
      highlightActiveKpi();
      onFilter();
    });
  });

  // Pagination
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

  // Export + Refresh
  document.getElementById("btn-export-csv")?.addEventListener("click", () => {
    exportCsv(filteredSubs, "submissions");
  });
  document.getElementById("btn-refresh")?.addEventListener("click", () => {
    loadSubmissions();
  });

  // Bulk
  document
    .getElementById("bulk-select-all")
    ?.addEventListener("change", (e) => {
      const checked = e.target.checked;
      const slice = currentSlice();
      slice.forEach((s) => {
        if (checked) selectedIds.add(s.id);
        else selectedIds.delete(s.id);
      });
      renderTable();
    });
  document
    .getElementById("bulk-mark-review")
    ?.addEventListener("click", () => bulkAction("in_review"));
  document
    .getElementById("bulk-reject")
    ?.addEventListener("click", () => bulkAction("reject"));
  document.getElementById("bulk-pin-mine")?.addEventListener("click", () => {
    selectedIds.forEach((id) => pinnedIds.add(id));
    savePinnedIds(pinnedIds);
    selectedIds.clear();
    applyFilters();
  });
  document.getElementById("bulk-export")?.addEventListener("click", () => {
    const subset = allSubmissions.filter((s) => selectedIds.has(s.id));
    exportCsv(subset, "submissions-selected");
  });
  document.getElementById("bulk-clear")?.addEventListener("click", () => {
    selectedIds.clear();
    renderTable();
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

function highlightActiveKpi() {
  document.querySelectorAll("[data-kpi-filter]").forEach((btn) => {
    const active = btn.getAttribute("data-kpi-filter") === kpiActiveFilter;
    btn.style.boxShadow = active
      ? "0 0 0 2px var(--admin-primary, #4f46e5)"
      : "";
    btn.setAttribute("aria-pressed", active ? "true" : "false");
  });
}

// ─── Data Loading ─────────────────────────────────────────────────────────────
async function loadSubmissions() {
  loadError = "";
  try {
    const resp = await fetch("/api/admin/developer-projects");
    if (resp.ok) {
      const data = await resp.json();
      allSubmissions = (data.projects || []).map(decorate);
      lastLoadedAt = new Date();
    } else {
      allSubmissions = [];
      const errorBody = await resp.json().catch(() => ({}));
      loadError =
        errorBody.error || `Failed to load submissions (HTTP ${resp.status})`;
    }
  } catch (e) {
    allSubmissions = [];
    loadError = e.message || "Network error while loading submissions.";
  }
  applyFilters();
  updateStats();
}

// Decorate each submission with derived fields
function decorate(s) {
  const created = s.created_at ? new Date(s.created_at) : null;
  const ageHours = created
    ? Math.max(0, (Date.now() - created.getTime()) / 3_600_000)
    : null;
  return { ...s, age_hours: ageHours };
}

// ─── KPI Stats ────────────────────────────────────────────────────────────────
function updateStats() {
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

  // 7-day deltas (count change over last 7 days)
  const sevenDays = 7 * 24 * 3_600_000;
  const sinceCutoff = (status) =>
    allSubmissions.filter(
      (s) =>
        (Array.isArray(status) ? status.includes(s.status) : s.status === status) &&
        s.age_hours != null &&
        s.age_hours * 3_600_000 <= sevenDays,
    ).length;

  const pendingNew7d = sinceCutoff(["submitted", "in_review"]);
  const approvedNew7d = sinceCutoff(["approved", "live"]);
  const rejectedNew7d = sinceCutoff("rejected");

  // SLA-overdue count among pending
  const overdue = allSubmissions.filter(
    (s) =>
      (s.status === "submitted" || s.status === "in_review") &&
      s.age_hours != null &&
      s.age_hours > SLA_AGING_MAX_H,
  ).length;

  // Currency mix warning: developer_projects values are stored in USD cents
  // but submissions may originate from non-USD locales. Total is a USD-equivalent
  // sum based on stored cents at submission time.

  const el = (id) => document.getElementById(id);
  const set = (id, v) => {
    if (el(id)) el(id).textContent = v;
  };
  set("stat-pending", pending);
  set("stat-approved", approved);
  set("stat-rejected", rejected);
  set("stat-total-value", formatUSD(totalValue));

  set(
    "stat-pending-sub",
    pending === 0
      ? "Queue clear"
      : overdue > 0
        ? `${overdue} SLA-overdue • +${pendingNew7d} this week`
        : `+${pendingNew7d} this week`,
  );
  set("stat-approved-sub", `+${approvedNew7d} this week`);
  set(
    "stat-rejected-sub",
    rejectedNew7d > 0 ? `+${rejectedNew7d} this week` : "—",
  );
  set("stat-total-value-sub", "Aggregate asset valuation");

  // Subtitle: actionable
  const subtitle = el("page-subtitle");
  if (subtitle) {
    if (loadError) {
      subtitle.textContent = "Failed to load queue.";
    } else {
      const parts = [];
      parts.push(`${pending} awaiting your review`);
      if (overdue > 0) parts.push(`${overdue} SLA-overdue`);
      if (lastLoadedAt) parts.push(`updated ${formatTime(lastLoadedAt)}`);
      subtitle.textContent = parts.join(" • ");
    }
  }

  // SLA-overdue color cue on Pending KPI
  const pendingCard = document.querySelector(
    '[data-kpi-filter="pending"]',
  );
  if (pendingCard) {
    pendingCard.style.borderColor =
      overdue > 0
        ? "var(--admin-danger, #dc2626)"
        : "var(--admin-border)";
  }
}

// ─── Filters ──────────────────────────────────────────────────────────────────
function applyFilters() {
  const search = (
    document.getElementById("sub-search")?.value || ""
  ).toLowerCase();
  const status = document.getElementById("filter-status")?.value || "";
  const type = document.getElementById("filter-type")?.value || "";
  const age = document.getElementById("filter-age")?.value || "";
  const mineOnly = !!document.getElementById("filter-mine")?.checked;

  let result = allSubmissions.filter((s) => {
    // KPI buckets override status select
    if (kpiActiveFilter === "pending") {
      if (s.status !== "submitted" && s.status !== "in_review") return false;
    } else if (kpiActiveFilter === "approved") {
      if (s.status !== "approved" && s.status !== "live") return false;
    } else if (kpiActiveFilter === "rejected") {
      if (s.status !== "rejected") return false;
    } else if (status) {
      if (s.status !== status) return false;
    }

    if (type && s.asset_type !== type) return false;

    if (age && s.age_hours != null) {
      if (age === "fresh" && s.age_hours >= SLA_FRESH_MAX_H) return false;
      if (
        age === "aging" &&
        (s.age_hours < SLA_FRESH_MAX_H || s.age_hours > SLA_AGING_MAX_H)
      )
        return false;
      if (age === "overdue" && s.age_hours <= SLA_AGING_MAX_H) return false;
    } else if (age && s.age_hours == null) {
      return false;
    }

    if (mineOnly && !pinnedIds.has(s.id)) return false;

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
    if (typeof valA === "string" && typeof valB === "string") {
      valA = valA.toLowerCase();
      valB = valB.toLowerCase();
    }
    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredSubs = result;
  currentPage = Math.min(
    currentPage,
    Math.max(1, Math.ceil(filteredSubs.length / PAGE_SIZE)),
  );
  updateSortHeaders();

  const countEl = document.getElementById("sub-count-label");
  if (countEl)
    countEl.textContent = loadError
      ? "Unable to load submissions"
      : `Showing ${filteredSubs.length} of ${allSubmissions.length} submission${allSubmissions.length !== 1 ? "s" : ""}`;
  renderTable();
}

function currentSlice() {
  const start = (currentPage - 1) * PAGE_SIZE;
  return filteredSubs.slice(start, start + PAGE_SIZE);
}

// ─── Table Render ─────────────────────────────────────────────────────────────
function renderTable() {
  const tbody = document.getElementById("submissions-table-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(filteredSubs.length / PAGE_SIZE));
  currentPage = Math.min(currentPage, totalPages);
  const slice = currentSlice();

  // Pagination
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${filteredSubs.length} total)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;

  // Bulk bar visibility + select-all checkbox state
  const bulkBar = document.getElementById("bulk-action-bar");
  const bulkCount = document.getElementById("bulk-count");
  if (bulkBar) bulkBar.style.display = selectedIds.size > 0 ? "flex" : "none";
  if (bulkCount) bulkCount.textContent = `${selectedIds.size} selected`;
  const sel = document.getElementById("bulk-select-all");
  if (sel) {
    const allOnPageSelected =
      slice.length > 0 && slice.every((s) => selectedIds.has(s.id));
    sel.checked = allOnPageSelected;
    sel.indeterminate =
      !allOnPageSelected && slice.some((s) => selectedIds.has(s.id));
  }

  if (loadError) {
    tbody.innerHTML = `
      <tr>
        <td colspan="11" style="text-align:center;padding:40px;color:var(--admin-danger);">
          <div style="font-weight:700;margin-bottom:8px;">Could not load submissions</div>
          <div style="font-size:13px;color:var(--admin-text-secondary);margin-bottom:16px;">${esc(loadError)}</div>
          <button type="button" class="admin-btn admin-btn--secondary admin-btn--sm" id="retry-submissions-load">Retry</button>
        </td>
      </tr>`;
    document
      .getElementById("retry-submissions-load")
      ?.addEventListener("click", loadSubmissions);
    return;
  }

  if (slice.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="11" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No submissions match your filters.</td></tr>';
    return;
  }

  tbody.innerHTML = slice
    .map((s) => {
      const isPinned = pinnedIds.has(s.id);
      const isSelected = selectedIds.has(s.id);
      const showRaisedAndProgress =
        s.status === "approved" ||
        s.status === "live" ||
        (s.total_raised_cents || 0) > 0;
      return `
        <tr data-id="${esc(s.id)}" ${isSelected ? 'style="background:var(--admin-primary-soft, #eef2ff);"' : ""}>
            <td style="padding-right:0;">
                <input type="checkbox" class="row-select" data-id="${esc(s.id)}" ${isSelected ? "checked" : ""} aria-label="Select submission ${esc(s.title || s.project_name || "")}" />
            </td>
            <td>
                <div style="display:flex;align-items:center;gap:6px;">
                  ${isPinned ? '<span title="Pinned to you" style="color:var(--admin-primary, #4f46e5);font-size:14px;line-height:1;">★</span>' : ""}
                  <span style="font-weight:600;color:var(--admin-text-primary);">${esc(s.title || s.project_name)}</span>
                </div>
                <div style="margin-top:2px;">
                    <code style="font-family:monospace;font-size:10px;padding:2px 4px;background:var(--admin-border);border-radius:4px;color:var(--admin-text-secondary);font-weight:500;">#APP-${(s.id || "").substring(0, 6).toUpperCase()}</code>
                </div>
            </td>
            <td>${getTypeBadge(s.asset_type)}</td>
            <td>
                <div class="admin-user-inline">
                    <div>
                        <div class="admin-user-inline-name">
                            <a href="/admin/user-details?id=${esc(s.developer_user_id)}"
                               style="color:var(--admin-primary);text-decoration:none;">${esc(s.developer_name)}</a>
                        </div>
                        <div class="admin-user-inline-email">${esc(s.developer_email)}</div>
                    </div>
                </div>
            </td>
            <td>${getStatusBadge(s.status)}</td>
            <td>${getAgeBadge(s.status, s.age_hours)}</td>
            <td style="font-weight:600;font-variant-numeric:tabular-nums;">${formatUSD(s.total_value_cents)}</td>
            <td style="font-variant-numeric:tabular-nums;">${
              showRaisedAndProgress
                ? `<div>${formatUSD(s.total_raised_cents)}</div><div style="font-size:11px;color:var(--admin-text-muted);">${(s.investors_count || 0).toLocaleString()} investor${(s.investors_count || 0) !== 1 ? "s" : ""}</div>`
                : '<span style="color:var(--admin-text-muted);">—</span>'
            }</td>
            <td>${
              showRaisedAndProgress
                ? `<div style="display:flex;align-items:center;gap:6px;">
                    <div style="flex:1;background:var(--admin-border);border-radius:4px;height:6px;min-width:60px;">
                        <div style="background:var(--admin-primary);border-radius:4px;height:6px;width:${Math.min(100, (s.funding_progress_bps || 0) / 100)}%"></div>
                    </div>
                    <span style="font-size:11px;color:var(--admin-text-muted);white-space:nowrap;">${((s.funding_progress_bps || 0) / 100).toFixed(0)}%</span>
                  </div>`
                : '<span style="color:var(--admin-text-muted);font-size:12px;">—</span>'
            }</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;" title="${formatRelative(s.created_at)} • ${esc(s.created_at || "")}">
              <div>${formatDate(s.created_at)}</div>
              <div style="font-size:10px;opacity:0.8;">${formatTime24(s.created_at)}</div>
            </td>
            <td>
                <div style="display:flex;gap:4px;">
                    <a class="admin-btn admin-btn--primary admin-btn--sm"
                       href="/admin/developer-submission-review?id=${esc(s.id)}"
                       title="Full deep-dive review">
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M1 8s3-6 7-6 7 6 7 6-3 6-7 6-7-6-7-6z"/><circle cx="8" cy="8" r="2.5"/></svg>
                        Review
                    </a>
                    <button class="admin-btn admin-btn--secondary admin-btn--sm"
                            data-action="quick" data-id="${esc(s.id)}" data-name="${esc(s.project_name || s.title || "")}"
                            title="Quick status review"
                            aria-label="Quick status review for ${esc(s.project_name || s.title || "submission")}">
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M3 8l3.5 3.5L13 4"/></svg>
                    </button>
                </div>
            </td>
        </tr>`;
    })
    .join("");

  // Wire row checkboxes + quick actions
  tbody.querySelectorAll(".row-select").forEach((cb) => {
    cb.addEventListener("change", (e) => {
      const id = e.target.getAttribute("data-id");
      if (e.target.checked) selectedIds.add(id);
      else selectedIds.delete(id);
      renderTable();
    });
  });
  tbody.querySelectorAll('[data-action="quick"]').forEach((btn) => {
    btn.addEventListener("click", () => {
      openQuickModal(btn.getAttribute("data-id"), btn.getAttribute("data-name"));
    });
  });
}

// ─── Age Badge ───────────────────────────────────────────────────────────────
function getAgeBadge(status, hours) {
  // Only show SLA color for items still actionable
  const isActionable = status === "submitted" || status === "in_review";
  if (hours == null) return '<span style="color:var(--admin-text-muted);">—</span>';
  const txt = formatAge(hours);
  if (!isActionable) {
    return `<span style="font-size:12px;color:var(--admin-text-muted);font-variant-numeric:tabular-nums;">${txt}</span>`;
  }
  let cls = "admin-badge--success";
  let label = "Fresh";
  if (hours > SLA_AGING_MAX_H) {
    cls = "admin-badge--danger";
    label = "Overdue";
  } else if (hours > SLA_FRESH_MAX_H) {
    cls = "admin-badge--warning";
    label = "Aging";
  }
  return `<span class="admin-badge ${cls}" title="${label} • SLA fresh &lt;${SLA_FRESH_MAX_H}h, aging &lt;${SLA_AGING_MAX_H}h">${txt}</span>`;
}

function formatAge(hours) {
  if (hours < 1) return `${Math.round(hours * 60)}m`;
  if (hours < 48) return `${Math.round(hours)}h`;
  return `${Math.round(hours / 24)}d`;
}

// ─── CSV Export ──────────────────────────────────────────────────────────────
function exportCsv(rows, filenamePrefix) {
  if (!rows || rows.length === 0) {
    alert("Nothing to export.");
    return;
  }
  const headers = [
    "id",
    "project_name",
    "title",
    "asset_type",
    "status",
    "developer_name",
    "developer_email",
    "location_city",
    "location_country",
    "total_value_cents",
    "total_raised_cents",
    "investors_count",
    "funding_progress_bps",
    "kyc_status",
    "age_hours",
    "created_at",
  ];
  const escapeCell = (v) => {
    if (v == null) return "";
    const s = String(v);
    if (/[",\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
    return s;
  };
  const lines = [headers.join(",")];
  rows.forEach((r) => {
    lines.push(headers.map((h) => escapeCell(r[h])).join(","));
  });
  const blob = new Blob([lines.join("\n")], {
    type: "text/csv;charset=utf-8;",
  });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  const stamp = new Date().toISOString().slice(0, 10);
  a.href = url;
  a.download = `${filenamePrefix}-${stamp}.csv`;
  document.body.appendChild(a);
  a.click();
  setTimeout(() => {
    URL.revokeObjectURL(url);
    a.remove();
  }, 0);
}

// ─── Bulk Actions ────────────────────────────────────────────────────────────
async function bulkAction(action) {
  if (selectedIds.size === 0) return;

  let notes = "";
  if (action === "reject") {
    notes = prompt(
      `Reject ${selectedIds.size} submission(s)? Provide a reason (required, will be sent to all developers):`,
      "",
    );
    if (notes == null) return;
    notes = notes.trim();
    if (!notes) {
      alert("Rejection reason is required.");
      return;
    }
  } else {
    if (
      !confirm(
        `Mark ${selectedIds.size} submission(s) as In Review? This logs to audit history.`,
      )
    )
      return;
  }

  const ids = [...selectedIds];
  const results = await Promise.allSettled(
    ids.map((id) =>
      fetch(`/api/admin/developer-projects/${encodeURIComponent(id)}/review`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": getCsrfToken(),
        },
        body: JSON.stringify({ action, notes }),
      }).then((r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return id;
      }),
    ),
  );
  const ok = results.filter((r) => r.status === "fulfilled").length;
  const fail = results.length - ok;
  if (fail > 0) {
    alert(`${ok} succeeded, ${fail} failed. See console for details.`);
    results.forEach((r, i) => {
      if (r.status === "rejected")
        console.error(`Failed: ${ids[i]}`, r.reason);
    });
  }
  selectedIds.clear();
  loadSubmissions();
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
          "X-CSRF-Token": getCsrfToken(),
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

function formatTime(d) {
  return d.toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatTime24(iso) {
  if (!iso) return "";
  return new Date(iso).toLocaleTimeString("en-GB", {
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatRelative(iso) {
  if (!iso) return "";
  const ms = Date.now() - new Date(iso).getTime();
  if (ms < 0) return "in the future";
  const mins = Math.round(ms / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.round(mins / 60);
  if (hours < 48) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  if (days < 30) return `${days}d ago`;
  const months = Math.round(days / 30);
  return `${months}mo ago`;
}

/**
 * Status badge using developer_projects.status enum:
 * draft | submitted | in_review | approved | rejected | live
 */
function getStatusBadge(status) {
  // Distinct visual treatment so green tones for Approved vs Live are
  // distinguishable at a glance:
  //   Approved = green outline (decision made, not yet trading)
  //   Live     = green solid + dot (trading active)
  const map = {
    draft: ["admin-badge--neutral", "Draft"],
    submitted: ["admin-badge--warning", "Submitted"],
    in_review: ["admin-badge--info", "In Review"],
    approved: ["admin-badge--success-outline", "Approved"],
    rejected: ["admin-badge--danger", "Rejected"],
    revision_requested: ["admin-badge--warning", "Revision Requested"],
    live: ["admin-badge--success admin-badge--live", "Live"],
  };
  const [cls, label] = map[status] || [
    "admin-badge--neutral",
    status || "Unknown",
  ];
  const dot = status === "live"
    ? '<span aria-hidden="true" style="display:inline-block;width:6px;height:6px;border-radius:50%;background:currentColor;margin-right:4px;animation:admin-pulse 2s ease-in-out infinite;"></span>'
    : "";
  return `<span class="admin-badge ${cls}">${dot}${label}</span>`;
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

function getCsrfToken() {
  if (typeof window.getCsrfToken === "function" && window.getCsrfToken !== getCsrfToken) {
    return window.getCsrfToken();
  }
  const meta = document.querySelector('meta[name="csrf-token"]');
  if (meta) return meta.getAttribute("content") || "";
  const m = document.cookie.match(/(?:^|;\s*)csrf_token=([^;]+)/);
  return m ? decodeURIComponent(m[1]) : "";
}

function debounce(fn, ms) {
  let t;
  return function (...args) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, args), ms);
  };
}

// Expose for inline handlers / debugging
window.openQuickModal = openQuickModal;
