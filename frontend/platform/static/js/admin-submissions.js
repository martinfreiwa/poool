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
let expandedRowId = null;
const detailCache = new Map(); // id → { fetchedAt, data }
const DETAIL_TTL_MS = 30_000;

// SLA thresholds (hours)
const SLA_FRESH_MAX_H = 24;
const SLA_AGING_MAX_H = 72;

// Reviewer assignment is now backend-driven (migration 102):
//   developer_projects.assigned_admin_id + assigned_at
// "Mine only" filter compares to the current admin's user id.
function currentAdminId() {
  return (
    (window.userData && (window.userData.id || window.userData.user_id)) || ""
  );
}

let includeTest = false; // when true, fetch with ?include_test=1

// Preset storage scoped to current admin
function presetsKey() {
  return `poool.admin.submissions.presets.${currentAdminId() || "anon"}`;
}
function loadPresets() {
  try {
    return JSON.parse(localStorage.getItem(presetsKey()) || "{}");
  } catch (e) {
    return {};
  }
}
function savePresets(obj) {
  try {
    localStorage.setItem(presetsKey(), JSON.stringify(obj));
  } catch (e) {}
}

// ─── Boot ───────────────────────────────────────────────────────────────────
document.addEventListener("DOMContentLoaded", () => {
  if (window.AdminPageKit) AdminPageKit.injectScopedCss();
  renderSkeletonRows();
  loadSubmissions();
  setupEventListeners();
  setupSorting();
  hydrateFromUrl();
  startAutoRefresh();
  setupCmdk();
  setupHelpModal();
  setupAnomalyDismiss();
  // Tick the "last updated" indicator every 15s so it stays fresh
  setInterval(updateLastUpdatedIndicator, 15_000);
});

function setupHelpModal() {
  const modal = document.getElementById("submissions-help-modal");
  if (!modal) return;
  document.querySelectorAll("[data-help-close]").forEach((el) =>
    el.addEventListener("click", () => { modal.hidden = true; }));
  document.addEventListener("keydown", (e) => {
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    const tag = (e.target.tagName || "").toLowerCase();
    const typing = tag === "input" || tag === "textarea" || e.target.isContentEditable;
    if (e.key === "Escape" && !modal.hidden) { modal.hidden = true; return; }
    if (typing) return;
    if (e.key === "?") { e.preventDefault(); modal.hidden = false; }
  });
}

function setupAnomalyDismiss() {
  const btn = document.getElementById("anomaly-banner-dismiss");
  const banner = document.getElementById("anomaly-banner");
  if (!btn || !banner) return;
  btn.addEventListener("click", () => {
    window.__submissionsAnomalyDismissed = true;
    banner.hidden = true;
  });
}

function renderSkeletonRows() {
  const tbody = document.getElementById("submissions-table-body");
  if (!tbody) return;
  const skel = (w, h = 12) =>
    `<span class="admin-skeleton" style="width:${w};height:${h}px;"></span>`;
  const row = `<tr aria-hidden="true">
    <td><span class="admin-skeleton" style="width:14px;height:14px;border-radius:3px;display:inline-block;"></span></td>
    <td><div>${skel("70%", 13)}</div><div style="margin-top:4px;">${skel("40%", 10)}</div></td>
    <td>${skel("60px", 18)}</td>
    <td><div>${skel("80%", 12)}</div><div style="margin-top:3px;">${skel("60%", 10)}</div></td>
    <td>${skel("50px", 18)}</td>
    <td>${skel("40px", 18)}</td>
    <td>${skel("70px")}</td>
    <td>${skel("80px")}</td>
    <td>${skel("90%")}</td>
    <td>${skel("70%")}</td>
    <td>${skel("100%", 28)}</td>
  </tr>`;
  tbody.innerHTML = row.repeat(6);
}

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
  document.getElementById("filter-risk")?.addEventListener("change", onFilter);
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
  document
    .getElementById("bulk-pin-mine")
    ?.addEventListener("click", () => bulkAssignToMe());
  document
    .getElementById("filter-include-test")
    ?.addEventListener("change", (e) => {
      includeTest = !!e.target.checked;
      loadSubmissions();
    });

  // Advanced filters
  document
    .getElementById("btn-toggle-advanced")
    ?.addEventListener("click", toggleAdvancedPanel);
  ["filter-developer", "filter-value-min", "filter-value-max", "filter-investors-min", "filter-progress-min", "filter-date-from", "filter-date-to"].forEach((id) => {
    const el = document.getElementById(id);
    if (!el) return;
    const handler = el.tagName === "SELECT" ? "change" : "input";
    el.addEventListener(handler, debounce(() => {
      currentPage = 1;
      applyFilters();
      updateAdvancedFilterCount();
    }, 200));
  });
  document
    .getElementById("filter-date-range")
    ?.addEventListener("change", (e) => {
      const isCustom = e.target.value === "custom";
      const w1 = document.getElementById("filter-date-custom-wrap");
      const w2 = document.getElementById("filter-date-custom-wrap-2");
      if (w1) w1.style.display = isCustom ? "" : "none";
      if (w2) w2.style.display = isCustom ? "" : "none";
      currentPage = 1;
      applyFilters();
      updateAdvancedFilterCount();
    });

  // Presets
  document
    .getElementById("filter-preset-select")
    ?.addEventListener("change", (e) => {
      applyPreset(e.target.value);
      const delBtn = document.getElementById("btn-preset-delete");
      if (delBtn) delBtn.disabled = !e.target.value;
    });
  document
    .getElementById("btn-preset-save")
    ?.addEventListener("click", () => {
      const name = prompt("Save current filters as preset. Name:");
      if (!name) return;
      const trimmed = name.trim();
      if (!trimmed) return;
      const presets = loadPresets();
      presets[trimmed] = collectFiltersForPreset();
      savePresets(presets);
      renderPresetOptions(trimmed);
    });
  document
    .getElementById("btn-preset-delete")
    ?.addEventListener("click", () => {
      const sel = document.getElementById("filter-preset-select");
      const name = sel?.value;
      if (!name) return;
      if (!confirm(`Delete preset "${name}"?`)) return;
      const presets = loadPresets();
      delete presets[name];
      savePresets(presets);
      renderPresetOptions("");
    });
  document
    .getElementById("btn-clear-advanced")
    ?.addEventListener("click", () => {
      clearAdvancedFilters();
      currentPage = 1;
      applyFilters();
      updateAdvancedFilterCount();
    });

  renderPresetOptions("");
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
    const url =
      "/api/admin/developer-projects" + (includeTest ? "?include_test=1" : "");
    const resp = await fetch(url);
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
  // Compute fraud heuristics across the full set
  allSubmissions = computeFraudSignalsForAll(allSubmissions);
  populateDeveloperDropdown();
  applyFilters();
  updateStats();
  updateAdvancedFilterCount();
  renderSubmissionsActionRequired();
}

function renderSubmissionsActionRequired() {
  if (!window.AdminPageKit) return;
  const SLA_HOURS = 72;
  const slaSec = SLA_HOURS * 3600;
  const overSla = allSubmissions.filter((s) =>
    s.status === "submitted" && AdminPageKit.ageSeconds(s.created_at) > slaSec,
  );
  const fraudFlagged = allSubmissions.filter((s) =>
    s.status === "submitted" && Array.isArray(s.fraud_signals) && s.fraud_signals.length > 0,
  );
  const inReview = allSubmissions.filter((s) => s.status === "in_review");
  const items = [];
  const goToFilter = (status) => () => {
    const sel = document.getElementById("filter-status");
    if (sel) { sel.value = status; applyFilters(); }
  };
  if (overSla.length) items.push({ label: `Submitted >${SLA_HOURS}h (SLA breach)`, count: overSla.length, color: "var(--admin-danger, #C2410C)", onClick: goToFilter("submitted") });
  if (fraudFlagged.length) items.push({ label: "Fraud signals in queue", count: fraudFlagged.length, color: "var(--admin-danger, #C2410C)", onClick: goToFilter("submitted") });
  if (inReview.length) items.push({ label: "Currently in review", count: inReview.length, color: "var(--admin-info)", onClick: goToFilter("in_review") });
  AdminPageKit.renderActionRequired(items, "#action-required-banner");
}

function populateDeveloperDropdown() {
  const sel = document.getElementById("filter-developer");
  if (!sel) return;
  const prev = sel.value;
  const seen = new Map();
  allSubmissions.forEach((s) => {
    if (s.developer_user_id && !seen.has(s.developer_user_id)) {
      seen.set(s.developer_user_id, s.developer_name || s.developer_email || "—");
    }
  });
  const sorted = [...seen.entries()].sort((a, b) =>
    (a[1] || "").toLowerCase().localeCompare((b[1] || "").toLowerCase()),
  );
  sel.innerHTML =
    '<option value="">All developers</option>' +
    sorted
      .map(([id, name]) => `<option value="${esc(id)}">${esc(name)}</option>`)
      .join("");
  if (prev && seen.has(prev)) sel.value = prev;
}

// ─── Advanced Filter UI helpers ──────────────────────────────────────────────
function toggleAdvancedPanel() {
  const panel = document.getElementById("advanced-filters");
  const btn = document.getElementById("btn-toggle-advanced");
  if (!panel) return;
  const open = panel.style.display !== "none" && panel.style.display !== "";
  // First-call: display is "" (CSS default = none in inline style). Treat as closed.
  const reallyOpen = panel.style.display === "block" || panel.style.display === "grid" || (open && panel.offsetHeight > 0);
  panel.style.display = reallyOpen ? "none" : "block";
  if (btn) btn.setAttribute("aria-expanded", reallyOpen ? "false" : "true");
}

function updateAdvancedFilterCount() {
  const ids = [
    "filter-developer",
    "filter-date-range",
    "filter-value-min",
    "filter-value-max",
    "filter-investors-min",
    "filter-progress-min",
  ];
  let count = 0;
  ids.forEach((id) => {
    const v = document.getElementById(id)?.value;
    if (v != null && v !== "") count++;
  });
  const badge = document.getElementById("advanced-filter-count");
  if (badge) {
    badge.textContent = count;
    badge.style.display = count > 0 ? "inline-block" : "none";
  }
}

function clearAdvancedFilters() {
  ["filter-developer", "filter-date-range", "filter-date-from", "filter-date-to", "filter-value-min", "filter-value-max", "filter-investors-min", "filter-progress-min"].forEach((id) => {
    const el = document.getElementById(id);
    if (el) el.value = "";
  });
  const w1 = document.getElementById("filter-date-custom-wrap");
  const w2 = document.getElementById("filter-date-custom-wrap-2");
  if (w1) w1.style.display = "none";
  if (w2) w2.style.display = "none";
}

function collectFiltersForPreset() {
  const get = (id) => document.getElementById(id)?.value ?? "";
  return {
    search: get("sub-search"),
    status: get("filter-status"),
    type: get("filter-type"),
    age: get("filter-age"),
    mine: !!document.getElementById("filter-mine")?.checked,
    includeTest: !!document.getElementById("filter-include-test")?.checked,
    developer: get("filter-developer"),
    dateRange: get("filter-date-range"),
    dateFrom: get("filter-date-from"),
    dateTo: get("filter-date-to"),
    valueMin: get("filter-value-min"),
    valueMax: get("filter-value-max"),
    investorsMin: get("filter-investors-min"),
    progressMin: get("filter-progress-min"),
    kpi: kpiActiveFilter,
  };
}

function applyPreset(name) {
  if (!name) return;
  const presets = loadPresets();
  const p = presets[name];
  if (!p) return;
  const set = (id, v) => {
    const el = document.getElementById(id);
    if (!el) return;
    if (el.type === "checkbox") el.checked = !!v;
    else el.value = v ?? "";
  };
  set("sub-search", p.search);
  set("filter-status", p.status);
  set("filter-type", p.type);
  set("filter-age", p.age);
  set("filter-mine", p.mine);
  set("filter-include-test", p.includeTest);
  set("filter-developer", p.developer);
  set("filter-date-range", p.dateRange);
  set("filter-date-from", p.dateFrom);
  set("filter-date-to", p.dateTo);
  set("filter-value-min", p.valueMin);
  set("filter-value-max", p.valueMax);
  set("filter-investors-min", p.investorsMin);
  set("filter-progress-min", p.progressMin);

  // Surface custom-range fields if needed
  const isCustom = p.dateRange === "custom";
  const w1 = document.getElementById("filter-date-custom-wrap");
  const w2 = document.getElementById("filter-date-custom-wrap-2");
  if (w1) w1.style.display = isCustom ? "" : "none";
  if (w2) w2.style.display = isCustom ? "" : "none";

  kpiActiveFilter = p.kpi || "";
  highlightActiveKpi();

  // Reload if includeTest toggled
  const wantInclude = !!p.includeTest;
  if (wantInclude !== includeTest) {
    includeTest = wantInclude;
    loadSubmissions();
    return;
  }
  currentPage = 1;
  applyFilters();
  updateAdvancedFilterCount();
}

function renderPresetOptions(selectName) {
  const sel = document.getElementById("filter-preset-select");
  if (!sel) return;
  const presets = loadPresets();
  const names = Object.keys(presets).sort();
  sel.innerHTML =
    '<option value="">Select preset…</option>' +
    names.map((n) => `<option value="${esc(n)}">${esc(n)}</option>`).join("");
  if (selectName && names.includes(selectName)) sel.value = selectName;
  const delBtn = document.getElementById("btn-preset-delete");
  if (delBtn) delBtn.disabled = !sel.value;
}

// Decorate each submission with derived fields
function decorate(s) {
  const created = s.created_at ? new Date(s.created_at) : null;
  const ageHours = created
    ? Math.max(0, (Date.now() - created.getTime()) / 3_600_000)
    : null;
  return { ...s, age_hours: ageHours };
}

// ─── Fraud Heuristics (#22) ──────────────────────────────────────────────────
// Pure client-side. Computed across the loaded submission set.
// Score is 0–100, additive across triggered signals. Visible at ≥30.
function computeFraudSignalsForAll(submissions) {
  if (!submissions || submissions.length === 0) return submissions;

  // Pre-compute aggregates
  const byDev = new Map();           // dev_id → array of subs
  const byProjectName = new Map();   // lowercased project_name → array of subs
  const valuesCents = [];            // for percentile

  submissions.forEach((s) => {
    if (s.developer_user_id) {
      if (!byDev.has(s.developer_user_id)) byDev.set(s.developer_user_id, []);
      byDev.get(s.developer_user_id).push(s);
    }
    const key = ((s.project_name || s.title || "") + "").trim().toLowerCase();
    if (key) {
      if (!byProjectName.has(key)) byProjectName.set(key, []);
      byProjectName.get(key).push(s);
    }
    if (typeof s.total_value_cents === "number" && s.total_value_cents > 0) {
      valuesCents.push(s.total_value_cents);
    }
  });

  valuesCents.sort((a, b) => a - b);
  const p95 = valuesCents.length
    ? valuesCents[Math.min(valuesCents.length - 1, Math.floor(valuesCents.length * 0.95))]
    : 0;
  const sevenDaysMs = 7 * 24 * 3_600_000;

  return submissions.map((s) => {
    const signals = [];
    let score = 0;

    // Signal 1: high submission rate from same developer in last 7d
    if (s.developer_user_id) {
      const peers = byDev.get(s.developer_user_id) || [];
      const recent = peers.filter((p) => {
        if (!p.created_at) return false;
        return Date.now() - new Date(p.created_at).getTime() <= sevenDaysMs;
      });
      if (recent.length >= 3) {
        score += 25;
        signals.push(`Rate: ${recent.length} submissions in last 7d from this developer`);
      }
    }

    // Signal 2: KYC missing or not approved (only flag for actionable items)
    if (s.status === "submitted" || s.status === "in_review") {
      if (!s.kyc_status) {
        score += 25;
        signals.push("No KYC record on file");
      } else if (s.kyc_status !== "approved") {
        score += 20;
        signals.push(`KYC status: ${s.kyc_status}`);
      }
    }

    // Signal 3: duplicate project name
    const nameKey = ((s.project_name || s.title || "") + "").trim().toLowerCase();
    if (nameKey) {
      const dupes = (byProjectName.get(nameKey) || []).filter(
        (p) => p.id !== s.id && p.status !== "rejected",
      );
      if (dupes.length > 0) {
        score += 30;
        signals.push(`Duplicate project name (${dupes.length} other${dupes.length > 1 ? "s" : ""})`);
      }
    }

    // Signal 4: value outlier
    if (s.total_value_cents != null) {
      if (s.total_value_cents > p95 * 3 && p95 > 0) {
        score += 15;
        signals.push("Valuation > 3× 95th percentile");
      } else if (s.total_value_cents > 0 && s.total_value_cents < 100_00) {
        // <$100 valuation is implausible for asset tokenization
        score += 10;
        signals.push("Implausibly low valuation");
      }
    }

    // Signal 5: brand-new developer (this is their first project)
    if ((s.other_projects_count || 0) <= 1 && (s.status === "submitted" || s.status === "in_review")) {
      score += 5;
      signals.push("First-time developer");
    }

    // Signal 6: no developer email (data quality / suspicious)
    if (!s.developer_email) {
      score += 10;
      signals.push("Missing developer email");
    }

    return { ...s, fraud_score: Math.min(100, score), fraud_signals: signals };
  });
}

function fraudBadge(score, signals) {
  if (!score || score < 30) return "";
  let cls = "admin-badge--warning";
  let label = "⚠";
  if (score >= 60) {
    cls = "admin-badge--danger";
    label = "⛔";
  }
  const tip = (signals || []).map((s) => "• " + s).join("\n");
  return `<span class="admin-badge ${cls}" title="Fraud risk: ${score}/100\n${tip}" style="font-size:10px;padding:1px 6px;cursor:help;" aria-label="Fraud risk score ${score} of 100">${label} ${score}</span>`;
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

  // Subtitle: actionable (preserve any existing .cr-related span)
  const subtitle = el("page-subtitle");
  if (subtitle) {
    const related = subtitle.querySelector(".cr-related");
    if (loadError) {
      subtitle.textContent = "Failed to load queue.";
    } else {
      const parts = [];
      parts.push(`${pending} awaiting your review`);
      if (overdue > 0) parts.push(`${overdue} SLA-overdue`);
      if (lastLoadedAt) parts.push(`updated ${formatTime(lastLoadedAt)}`);
      subtitle.textContent = parts.join(" • ");
    }
    if (related) subtitle.appendChild(related);
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

  // SLA pill on Pending KPI label (oldest pending)
  const slaPill = el("stat-pending-sla");
  if (slaPill) {
    const oldestH = allSubmissions
      .filter((s) => (s.status === "submitted" || s.status === "in_review") && s.age_hours != null)
      .reduce((max, s) => Math.max(max, s.age_hours), 0);
    slaPill.classList.remove("admin-sla-pill--ok", "admin-sla-pill--warn", "admin-sla-pill--crit");
    if (pending === 0 || oldestH === 0) {
      slaPill.hidden = true;
    } else {
      slaPill.hidden = false;
      const fmt = oldestH < 1
        ? `${Math.round(oldestH * 60)}m`
        : oldestH < 48
          ? `${Math.round(oldestH)}h`
          : `${Math.round(oldestH / 24)}d`;
      let tier = "ok";
      if (oldestH > SLA_AGING_MAX_H) tier = "crit";
      else if (oldestH >= SLA_FRESH_MAX_H) tier = "warn";
      slaPill.classList.add(`admin-sla-pill--${tier}`);
      slaPill.textContent = `Oldest ${fmt}`;
    }
  }

  detectSubmissionAnomalies();
}

function detectSubmissionAnomalies() {
  const banner = document.getElementById("anomaly-banner");
  if (!banner || window.__submissionsAnomalyDismissed) return;
  const ANOMALY_WINDOW_H = 24;
  const ANOMALY_THRESHOLD = 5;
  const counts = new Map();
  allSubmissions.forEach((s) => {
    if (s.age_hours == null || s.age_hours > ANOMALY_WINDOW_H) return;
    const dev = s.developer_name || s.developer_email || "Unknown";
    counts.set(dev, (counts.get(dev) || 0) + 1);
  });
  const offenders = [...counts.entries()].filter(([, n]) => n >= ANOMALY_THRESHOLD);
  if (offenders.length === 0) {
    banner.hidden = true;
    return;
  }
  banner.hidden = false;
  const txt = document.getElementById("anomaly-banner-text");
  if (txt) txt.textContent =
    `Unusual activity (24h): ${offenders.map(([d, n]) => `${d} (${n} submissions)`).join(", ")}`;
}

// ─── Filters ──────────────────────────────────────────────────────────────────
function applyFilters() {
  const search = (
    document.getElementById("sub-search")?.value || ""
  ).toLowerCase();
  const status = document.getElementById("filter-status")?.value || "";
  const type = document.getElementById("filter-type")?.value || "";
  const age = document.getElementById("filter-age")?.value || "";
  const risk = document.getElementById("filter-risk")?.value || "";
  const mineOnly = !!document.getElementById("filter-mine")?.checked;

  // Advanced filters
  const developer = document.getElementById("filter-developer")?.value || "";
  const dateRange = document.getElementById("filter-date-range")?.value || "";
  const dateFromStr = document.getElementById("filter-date-from")?.value || "";
  const dateToStr = document.getElementById("filter-date-to")?.value || "";
  const valueMin = parseFloat(document.getElementById("filter-value-min")?.value || "") || null;
  const valueMax = parseFloat(document.getElementById("filter-value-max")?.value || "") || null;
  const investorsMin = parseInt(document.getElementById("filter-investors-min")?.value || "", 10);
  const progressMin = parseFloat(document.getElementById("filter-progress-min")?.value || "") || null;

  // Compute date cutoffs
  let cutoffFrom = null, cutoffTo = null;
  if (dateRange === "custom") {
    if (dateFromStr) cutoffFrom = new Date(dateFromStr + "T00:00:00").getTime();
    if (dateToStr)   cutoffTo   = new Date(dateToStr   + "T23:59:59").getTime();
  } else if (dateRange) {
    const hours = { "24h": 24, "7d": 168, "30d": 720, "90d": 2160 }[dateRange];
    if (hours) cutoffFrom = Date.now() - hours * 3_600_000;
  }

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

    if (mineOnly && s.assigned_admin_id !== currentAdminId()) return false;

    if (risk) {
      const fs = s.fraud_score || 0;
      if (risk === "high" && fs < 60) return false;
      if (risk === "any" && fs < 30) return false;
      if (risk === "clean" && fs >= 30) return false;
    }

    if (developer && s.developer_user_id !== developer) return false;

    if (cutoffFrom != null || cutoffTo != null) {
      const ts = s.created_at ? new Date(s.created_at).getTime() : null;
      if (ts == null) return false;
      if (cutoffFrom != null && ts < cutoffFrom) return false;
      if (cutoffTo != null && ts > cutoffTo) return false;
    }

    const valCents = s.total_value_cents || 0;
    if (valueMin != null && valCents < valueMin * 100) return false;
    if (valueMax != null && valCents > valueMax * 100) return false;

    if (!isNaN(investorsMin) && (s.investors_count || 0) < investorsMin) return false;

    if (progressMin != null && (s.funding_progress_bps || 0) / 100 < progressMin) return false;

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
    const hasFilters =
      !!document.getElementById("sub-search")?.value ||
      !!document.getElementById("filter-status")?.value ||
      !!document.getElementById("filter-type")?.value ||
      !!document.getElementById("filter-age")?.value ||
      !!document.getElementById("filter-mine")?.checked ||
      kpiActiveFilter !== "";
    const totalCount = allSubmissions.length;
    const empty = hasFilters
      ? {
          icon: "🔍",
          title: "No submissions match your filters",
          msg: `${totalCount} submission${totalCount !== 1 ? "s" : ""} exist in the queue. Adjust filters or clear them.`,
          action: '<button type="button" class="admin-btn admin-btn--secondary admin-btn--sm" id="empty-clear-filters">Clear filters</button>',
        }
      : totalCount === 0
        ? {
            icon: "📭",
            title: "Queue is empty",
            msg: "No developer submissions yet. They'll appear here when developers submit projects for review.",
            action: "",
          }
        : {
            icon: "✓",
            title: "Nothing to review",
            msg: "All caught up.",
            action: "",
          };
    tbody.innerHTML = `<tr><td colspan="11" style="padding:60px 20px;text-align:center;">
      <div style="font-size:36px;line-height:1;margin-bottom:12px;opacity:0.6;">${empty.icon}</div>
      <div style="font-size:15px;font-weight:600;color:var(--admin-text-primary);margin-bottom:4px;">${esc(empty.title)}</div>
      <div style="font-size:13px;color:var(--admin-text-muted);max-width:420px;margin:0 auto 16px;">${esc(empty.msg)}</div>
      ${empty.action}
    </td></tr>`;
    document
      .getElementById("empty-clear-filters")
      ?.addEventListener("click", () => {
        const ids = ["sub-search", "filter-status", "filter-type", "filter-age"];
        ids.forEach((id) => {
          const el = document.getElementById(id);
          if (!el) return;
          if (el.type === "checkbox") el.checked = false;
          else el.value = "";
        });
        const mine = document.getElementById("filter-mine");
        if (mine) mine.checked = false;
        kpiActiveFilter = "";
        highlightActiveKpi();
        applyFilters();
      });
    return;
  }

  const myId = currentAdminId();
  tbody.innerHTML = slice
    .map((s) => {
      const isMine = !!(s.assigned_admin_id && s.assigned_admin_id === myId);
      const isAssignedOther =
        !!(s.assigned_admin_id && s.assigned_admin_id !== myId);
      const isSelected = selectedIds.has(s.id);
      const isTest = !!s.is_test;
      const showRaisedAndProgress =
        s.status === "approved" ||
        s.status === "live" ||
        (s.total_raised_cents || 0) > 0;
      const assigneeBadge = isMine
        ? '<span title="Assigned to you" style="color:var(--admin-primary, #4f46e5);font-size:14px;line-height:1;">★</span>'
        : isAssignedOther
          ? `<span title="Assigned to ${esc(s.assigned_admin_name || "another admin")}" style="color:var(--admin-text-muted);font-size:11px;font-weight:600;background:var(--admin-border);padding:1px 5px;border-radius:8px;">@${esc((s.assigned_admin_name || "?").split(" ")[0])}</span>`
          : "";
      const testTag = isTest
        ? '<span title="Marked as test/dummy submission" style="font-size:10px;font-weight:600;color:var(--admin-warning);background:var(--admin-warning-bg);padding:1px 6px;border-radius:8px;text-transform:uppercase;letter-spacing:0.5px;">test</span>'
        : "";
      return `
        <tr data-id="${esc(s.id)}" ${isSelected ? 'style="background:var(--admin-primary-soft, #eef2ff);"' : ""}>
            <td style="padding-right:0;">
                <input type="checkbox" class="row-select" data-id="${esc(s.id)}" ${isSelected ? "checked" : ""} aria-label="Select submission ${esc(s.title || s.project_name || "")}" />
            </td>
            <td>
                <div style="display:flex;align-items:center;gap:6px;flex-wrap:wrap;">
                  ${assigneeBadge}
                  <span style="font-weight:600;color:var(--admin-text-primary);">${esc(s.title || s.project_name)}</span>
                  ${testTag}
                  ${fraudBadge(s.fraud_score, s.fraud_signals)}
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
                            data-action="expand" data-id="${esc(s.id)}"
                            title="Expand inline detail (assign, mark test, recent notes, quick actions)"
                            aria-expanded="${expandedRowId === s.id ? "true" : "false"}"
                            aria-label="Toggle detail for ${esc(s.project_name || s.title || "submission")}">
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" style="transform:${expandedRowId === s.id ? "rotate(180deg)" : "none"};transition:transform 0.15s;"><path d="M3 6l5 5 5-5"/></svg>
                    </button>
                </div>
            </td>
        </tr>`;
    })
    .join("");

  // Wire row checkboxes + expand
  tbody.querySelectorAll(".row-select").forEach((cb) => {
    cb.addEventListener("change", (e) => {
      const id = e.target.getAttribute("data-id");
      if (e.target.checked) selectedIds.add(id);
      else selectedIds.delete(id);
      renderTable();
    });
  });
  tbody.querySelectorAll('[data-action="expand"]').forEach((btn) => {
    btn.addEventListener("click", () => toggleExpand(btn.getAttribute("data-id")));
  });

  // Re-inject detail row if one was open
  if (expandedRowId) injectDetailRow(expandedRowId);
}

// ─── Inline Detail Drawer (#7) ───────────────────────────────────────────────
function toggleExpand(id) {
  if (expandedRowId === id) {
    expandedRowId = null;
    renderTable();
    return;
  }
  expandedRowId = id;
  renderTable();
  injectDetailRow(id);
}

function injectDetailRow(id) {
  const tbody = document.getElementById("submissions-table-body");
  if (!tbody) return;
  const targetRow = tbody.querySelector(`tr[data-id="${cssEsc(id)}"]`);
  if (!targetRow) return;

  // Remove any existing detail row first
  tbody.querySelectorAll("tr.detail-row").forEach((tr) => tr.remove());

  const sub = allSubmissions.find((s) => s.id === id);
  if (!sub) return;

  const detailTr = document.createElement("tr");
  detailTr.className = "detail-row";
  detailTr.innerHTML = `<td colspan="11" style="padding:0;background:var(--admin-bg-card-hover);">
    <div id="detail-content-${esc(id)}" style="padding:16px 20px;border-top:2px solid var(--admin-primary, #4f46e5);">
      ${renderDetailSkeleton()}
    </div>
  </td>`;
  targetRow.after(detailTr);

  fetchDetail(id).then((detail) => {
    const c = document.getElementById(`detail-content-${cssEsc(id)}`);
    if (!c) return;
    c.innerHTML = renderDetailBody(sub, detail);
    wireDetailActions(c, sub, detail);
  });
}

async function fetchDetail(id) {
  const cached = detailCache.get(id);
  if (cached && Date.now() - cached.fetchedAt < DETAIL_TTL_MS) {
    return cached.data;
  }
  try {
    const [detailResp, notesResp, historyResp] = await Promise.all([
      fetch(`/api/admin/developer-projects/${encodeURIComponent(id)}`),
      fetch(`/api/admin/developer-projects/${encodeURIComponent(id)}/notes`),
      fetch(`/api/admin/developer-projects/${encodeURIComponent(id)}/history`),
    ]);
    const data = {
      detail: detailResp.ok ? await detailResp.json() : null,
      notes: notesResp.ok ? (await notesResp.json()).notes || [] : [],
      history: historyResp.ok ? (await historyResp.json()).history || [] : [],
    };
    detailCache.set(id, { fetchedAt: Date.now(), data });
    return data;
  } catch (e) {
    return { detail: null, notes: [], history: [], error: e.message };
  }
}

function renderDetailSkeleton() {
  const bar = (w) =>
    `<div style="height:10px;width:${w};background:var(--admin-border);border-radius:4px;margin:6px 0;animation:admin-skeleton-pulse 1.4s ease-in-out infinite;"></div>`;
  return `
    <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:24px;">
      <div>${bar("60%")}${bar("90%")}${bar("70%")}</div>
      <div>${bar("80%")}${bar("50%")}${bar("75%")}</div>
      <div>${bar("65%")}${bar("85%")}${bar("55%")}</div>
    </div>`;
}

function renderDetailBody(sub, data) {
  const detail = data.detail || {};
  const docs = (detail.documents || []).length;
  const images = (detail.images || []).length;
  const milestones = (detail.milestones || []).length;
  const notes = data.notes || [];
  const myId = currentAdminId();
  const isMine = sub.assigned_admin_id && sub.assigned_admin_id === myId;
  const assignee = sub.assigned_admin_id
    ? `Assigned to <strong>${esc(sub.assigned_admin_name || "another admin")}</strong>${isMine ? " (you)" : ""}`
    : "<em>Unassigned</em>";
  const kyc = sub.kyc_status || "—";
  const kycCls =
    kyc === "approved" ? "admin-badge--success" :
    kyc === "rejected" ? "admin-badge--danger" :
    kyc === "pending"  ? "admin-badge--warning" : "admin-badge--neutral";

  const recentNotes = notes
    .slice(0, 3)
    .map(
      (n) =>
        `<div style="padding:8px 10px;background:var(--admin-bg-card);border-radius:6px;margin-top:6px;">
          <div style="font-size:11px;color:var(--admin-text-muted);margin-bottom:2px;">${esc(n.author_name || n.author_email || "—")} • ${formatRelative(n.created_at)}</div>
          <div style="font-size:13px;color:var(--admin-text-primary);white-space:pre-wrap;">${esc((n.content || "").slice(0, 300))}${(n.content || "").length > 300 ? "…" : ""}</div>
        </div>`,
    )
    .join("");

  return `
    <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:24px;">
      <!-- Column 1: Workflow -->
      <div>
        <div style="font-size:11px;font-weight:600;text-transform:uppercase;color:var(--admin-text-muted);letter-spacing:0.5px;margin-bottom:6px;">Workflow</div>
        <div style="font-size:13px;color:var(--admin-text-secondary);margin-bottom:8px;">${assignee}</div>
        <div style="display:flex;gap:6px;flex-wrap:wrap;margin-bottom:10px;">
          <button class="admin-btn admin-btn--secondary admin-btn--sm" data-detail-action="${isMine ? "unassign" : "assign-self"}" data-id="${esc(sub.id)}">
            ${isMine ? "Unassign" : "Assign to Me"}
          </button>
          <button class="admin-btn admin-btn--secondary admin-btn--sm" data-detail-action="toggle-test" data-id="${esc(sub.id)}" data-current="${sub.is_test ? "1" : "0"}" title="${sub.is_test ? "Remove test flag" : "Mark as test/dummy submission"}">
            ${sub.is_test ? "Unmark Test" : "Mark as Test"}
          </button>
        </div>
        <div style="font-size:11px;color:var(--admin-text-muted);margin-bottom:6px;">KYC: <span class="admin-badge ${kycCls}" style="font-size:10px;">${esc(kyc)}</span></div>
        ${
          (sub.fraud_score || 0) > 0
            ? `<div style="font-size:11px;font-weight:600;text-transform:uppercase;color:var(--admin-text-muted);letter-spacing:0.5px;margin-top:10px;margin-bottom:4px;">Risk Signals (${sub.fraud_score}/100)</div>
               <ul style="margin:0;padding-left:14px;font-size:11px;color:var(--admin-text-secondary);line-height:1.6;">
                 ${(sub.fraud_signals || []).map((sig) => `<li>${esc(sig)}</li>`).join("")}
               </ul>`
            : '<div style="font-size:11px;color:var(--admin-text-muted);">No risk signals.</div>'
        }
      </div>

      <!-- Column 2: Content audit -->
      <div>
        <div style="font-size:11px;font-weight:600;text-transform:uppercase;color:var(--admin-text-muted);letter-spacing:0.5px;margin-bottom:6px;">Content</div>
        <div style="font-size:13px;color:var(--admin-text-secondary);line-height:1.7;">
          <div>📄 <strong>${docs}</strong> document${docs !== 1 ? "s" : ""}</div>
          <div>🖼️ <strong>${images}</strong> image${images !== 1 ? "s" : ""}</div>
          <div>🎯 <strong>${milestones}</strong> milestone${milestones !== 1 ? "s" : ""}</div>
          <div style="margin-top:6px;">📍 ${esc(sub.location_city || "—")}${sub.location_country ? ", " + esc(sub.location_country) : ""}</div>
        </div>
      </div>

      <!-- Column 3: Recent notes + actions -->
      <div>
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:6px;">
          <span style="font-size:11px;font-weight:600;text-transform:uppercase;color:var(--admin-text-muted);letter-spacing:0.5px;">Recent Notes (${notes.length})</span>
          <a href="/admin/developer-submission-review?id=${esc(sub.id)}" style="font-size:11px;color:var(--admin-primary);">Full review →</a>
        </div>
        ${notes.length === 0 ? '<div style="font-size:12px;color:var(--admin-text-muted);font-style:italic;">No notes yet.</div>' : recentNotes}
        <div style="display:flex;gap:6px;margin-top:10px;flex-wrap:wrap;">
          ${
            sub.status === "submitted"
              ? `<button class="admin-btn admin-btn--secondary admin-btn--sm" data-detail-action="mark-review" data-id="${esc(sub.id)}">Mark In Review</button>`
              : ""
          }
          <button class="admin-btn admin-btn--danger admin-btn--sm" data-detail-action="quick-reject" data-id="${esc(sub.id)}" data-name="${esc(sub.project_name || sub.title || "")}">Reject…</button>
        </div>
      </div>
    </div>

    <!-- History timeline + diff (#15) -->
    <div style="margin-top:16px;border-top:1px solid var(--admin-border);padding-top:12px;">
      <button type="button" class="admin-btn admin-btn--ghost admin-btn--sm" data-detail-action="toggle-history" data-id="${esc(sub.id)}" aria-expanded="false" style="font-size:11px;">
        <svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" class="history-chevron" style="transition:transform 0.15s;"><path d="M3 6l5 5 5-5"/></svg>
        Show history (${(data.history || []).length} entries)
      </button>
      <div class="history-panel" style="display:none;margin-top:10px;"></div>
    </div>`;
}

function wireDetailActions(container, sub, data) {
  container.querySelectorAll("[data-detail-action]").forEach((btn) => {
    const action = btn.getAttribute("data-detail-action");
    const id = btn.getAttribute("data-id");
    btn.addEventListener("click", async () => {
      // Toggle-history is a pure UI toggle; doesn't bust cache
      if (action === "toggle-history") {
        const panel = container.querySelector(".history-panel");
        const chev = btn.querySelector(".history-chevron");
        const open =
          panel && panel.style.display !== "none" && panel.style.display !== "";
        if (panel) {
          if (open) {
            panel.style.display = "none";
            btn.setAttribute("aria-expanded", "false");
            if (chev) chev.style.transform = "none";
          } else {
            panel.innerHTML = renderHistoryTimeline(data.history || []);
            panel.style.display = "block";
            btn.setAttribute("aria-expanded", "true");
            if (chev) chev.style.transform = "rotate(180deg)";
          }
        }
        return;
      }

      btn.disabled = true;
      detailCache.delete(id); // bust cache so next expand reflects changes
      switch (action) {
        case "assign-self":
          await toggleAssignSelf(id, false);
          break;
        case "unassign":
          await toggleAssignSelf(id, true);
          break;
        case "toggle-test":
          await toggleTestFlag(id, btn.getAttribute("data-current") === "1");
          break;
        case "mark-review":
          await singleAction(id, "in_review", "");
          break;
        case "quick-reject":
          openQuickModal(id, btn.getAttribute("data-name") || "");
          break;
      }
      btn.disabled = false;
    });
  });
}

async function singleAction(id, action, notes) {
  try {
    const resp = await fetch(
      `/api/admin/developer-projects/${encodeURIComponent(id)}/review`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": getCsrfToken(),
        },
        body: JSON.stringify({ action, notes }),
      },
    );
    if (!resp.ok) {
      const err = await resp.json().catch(() => ({}));
      alert("Action failed: " + (err.error || `HTTP ${resp.status}`));
      return;
    }
    loadSubmissions();
  } catch (e) {
    alert("Network error.");
  }
}

function cssEsc(s) {
  if (typeof CSS !== "undefined" && CSS.escape) return CSS.escape(s);
  return String(s).replace(/[^a-zA-Z0-9_-]/g, "\\$&");
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
async function bulkAssignToMe() {
  if (selectedIds.size === 0) return;
  const ids = [...selectedIds];
  const results = await Promise.allSettled(
    ids.map((id) =>
      fetch(`/api/admin/developer-projects/${encodeURIComponent(id)}/assign`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": getCsrfToken(),
        },
        body: JSON.stringify({}),
      }).then((r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
      }),
    ),
  );
  const fail = results.filter((r) => r.status === "rejected").length;
  if (fail > 0) {
    alert(
      `${results.length - fail} assigned, ${fail} failed. See console for details.`,
    );
    results.forEach((r, i) => {
      if (r.status === "rejected") console.error(`Assign failed: ${ids[i]}`, r.reason);
    });
  }
  selectedIds.clear();
  loadSubmissions();
}

async function toggleAssignSelf(id, currentlyMine) {
  try {
    const resp = await fetch(
      `/api/admin/developer-projects/${encodeURIComponent(id)}/assign`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": getCsrfToken(),
        },
        body: JSON.stringify(currentlyMine ? { admin_id: null } : {}),
      },
    );
    if (!resp.ok) {
      const err = await resp.json().catch(() => ({}));
      alert("Assignment failed: " + (err.error || `HTTP ${resp.status}`));
      return;
    }
    loadSubmissions();
  } catch (e) {
    alert("Network error during assignment.");
  }
}

async function toggleTestFlag(id, currentlyTest) {
  if (
    !confirm(
      currentlyTest
        ? "Remove the TEST flag from this submission? It will reappear in the default queue."
        : "Mark this submission as TEST? It will be hidden from the default queue (visible only when 'Include test' is checked).",
    )
  ) {
    return;
  }
  try {
    const resp = await fetch(
      `/api/admin/developer-projects/${encodeURIComponent(id)}/test-flag`,
      {
        method: "PATCH",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": getCsrfToken(),
        },
        body: JSON.stringify({ is_test: !currentlyTest }),
      },
    );
    if (!resp.ok) {
      const err = await resp.json().catch(() => ({}));
      alert("Test flag failed: " + (err.error || `HTTP ${resp.status}`));
      return;
    }
    loadSubmissions();
  } catch (e) {
    alert("Network error.");
  }
}

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

// ─── Command Palette (#29) ───────────────────────────────────────────────────
let cmdkOpen = false;
let cmdkSelectedIdx = 0;
let cmdkResults = [];

function openCmdk() {
  const modal = document.getElementById("cmdk");
  if (!modal) return;
  cmdkOpen = true;
  modal.style.display = "flex";
  cmdkSelectedIdx = 0;
  const input = document.getElementById("cmdk-input");
  if (input) {
    input.value = "";
    setTimeout(() => input.focus(), 0);
  }
  renderCmdkResults("");
}
function closeCmdk() {
  const modal = document.getElementById("cmdk");
  if (!modal) return;
  cmdkOpen = false;
  modal.style.display = "none";
}

// Static actions independent of search
function cmdkActions() {
  const selCount = selectedIds.size;
  return [
    {
      kind: "filter",
      label: "Show pending submissions",
      hint: "KPI: pending",
      run: () => {
        kpiActiveFilter = "pending";
        document.getElementById("filter-status").value = "";
        highlightActiveKpi();
        applyFilters();
      },
    },
    {
      kind: "filter",
      label: "Show SLA-overdue items",
      hint: "Age > 3d",
      run: () => {
        document.getElementById("filter-age").value = "overdue";
        applyFilters();
      },
    },
    {
      kind: "filter",
      label: "Show high-risk submissions",
      hint: "Fraud score ≥ 60",
      run: () => {
        document.getElementById("filter-risk").value = "high";
        applyFilters();
      },
    },
    {
      kind: "filter",
      label: "Show only mine",
      hint: "Toggle Mine only",
      run: () => {
        const el = document.getElementById("filter-mine");
        if (el) {
          el.checked = !el.checked;
          applyFilters();
        }
      },
    },
    {
      kind: "filter",
      label: "Clear all filters",
      run: () => {
        ["sub-search", "filter-status", "filter-type", "filter-age", "filter-risk"].forEach((id) => {
          const el = document.getElementById(id);
          if (el) el.value = "";
        });
        const mine = document.getElementById("filter-mine");
        if (mine) mine.checked = false;
        clearAdvancedFilters();
        kpiActiveFilter = "";
        highlightActiveKpi();
        applyFilters();
        updateAdvancedFilterCount();
      },
    },
    {
      kind: "action",
      label: "Refresh now",
      hint: "Re-fetch /api/admin/developer-projects",
      run: () => loadSubmissions(),
    },
    {
      kind: "action",
      label: "Toggle advanced filters panel",
      run: () => toggleAdvancedPanel(),
    },
    {
      kind: "action",
      label: "Export filtered list as CSV",
      run: () => exportCsv(filteredSubs, "submissions"),
    },
    {
      kind: "action",
      label: `Assign ${selCount} selected to me`,
      hint: selCount === 0 ? "(no selection)" : `${selCount} item${selCount !== 1 ? "s" : ""}`,
      disabled: selCount === 0,
      run: () => bulkAssignToMe(),
    },
    {
      kind: "action",
      label: `Mark ${selCount} selected In Review`,
      hint: selCount === 0 ? "(no selection)" : `${selCount} item${selCount !== 1 ? "s" : ""}`,
      disabled: selCount === 0,
      run: () => bulkAction("in_review"),
    },
    {
      kind: "action",
      label: `Reject ${selCount} selected…`,
      hint: selCount === 0 ? "(no selection)" : `${selCount} item${selCount !== 1 ? "s" : ""}`,
      disabled: selCount === 0,
      run: () => bulkAction("reject"),
    },
  ];
}

function fuzzyScore(query, text) {
  // Simple subsequence-with-bonus scorer. Higher = better. -1 = no match.
  if (!query) return 0;
  const q = query.toLowerCase();
  const t = (text || "").toLowerCase();
  if (!t) return -1;
  if (t.includes(q)) {
    return 100 - (t.indexOf(q) / Math.max(1, t.length)) * 30;
  }
  let qi = 0, score = 0, lastIdx = -1;
  for (let i = 0; i < t.length && qi < q.length; i++) {
    if (t[i] === q[qi]) {
      score += lastIdx >= 0 && i === lastIdx + 1 ? 4 : 2;
      lastIdx = i;
      qi++;
    }
  }
  return qi === q.length ? score : -1;
}

function renderCmdkResults(query) {
  const container = document.getElementById("cmdk-results");
  if (!container) return;

  const q = (query || "").trim();
  const items = [];

  // Submissions (top 8 by fuzzy score)
  if (q) {
    const subs = allSubmissions
      .map((s) => {
        const haystack = [s.title, s.project_name, s.developer_name, s.developer_email, s.id]
          .filter(Boolean)
          .join(" ");
        return { s, score: fuzzyScore(q, haystack) };
      })
      .filter((x) => x.score >= 0)
      .sort((a, b) => b.score - a.score)
      .slice(0, 8);
    subs.forEach(({ s }) => {
      items.push({
        kind: "submission",
        label: s.title || s.project_name || s.id,
        hint: `${s.developer_name || s.developer_email || "—"} • ${s.status}`,
        right: `#APP-${(s.id || "").substring(0, 6).toUpperCase()}`,
        run: () => {
          location.href = `/admin/developer-submission-review?id=${encodeURIComponent(s.id)}`;
        },
        secondary: () => {
          // Shift+Enter or click "details" → expand inline instead of navigating
          closeCmdk();
          expandedRowId = s.id;
          renderTable();
          injectDetailRow(s.id);
          document.querySelector(`tr[data-id="${cssEsc(s.id)}"]`)?.scrollIntoView({ behavior: "smooth", block: "center" });
        },
      });
    });
  }

  // Filter all actions by fuzzy
  const acts = cmdkActions()
    .map((a) => ({ a, score: q ? fuzzyScore(q, a.label + " " + (a.hint || "")) : 50 }))
    .filter((x) => x.score >= 0)
    .sort((a, b) => b.score - a.score);
  acts.forEach(({ a }) => items.push(a));

  cmdkResults = items;
  if (cmdkSelectedIdx >= items.length) cmdkSelectedIdx = 0;

  if (items.length === 0) {
    container.innerHTML = `<div style="padding:24px;text-align:center;color:var(--admin-text-muted);font-size:13px;">No matches.</div>`;
    return;
  }

  container.innerHTML = items
    .map((it, i) => {
      const active = i === cmdkSelectedIdx;
      const kindIcon = {
        submission: "📄",
        filter: "🔎",
        action: "⚡",
      }[it.kind] || "•";
      return `<button type="button" class="cmdk-row" data-cmdk-idx="${i}" ${it.disabled ? "disabled" : ""} role="option" aria-selected="${active}" style="
          width:100%;text-align:left;display:flex;align-items:center;gap:10px;padding:10px 16px;background:${active ? "var(--admin-bg-card-hover)" : "transparent"};border:none;cursor:${it.disabled ? "not-allowed" : "pointer"};color:var(--admin-text-primary);opacity:${it.disabled ? 0.5 : 1};">
          <span style="font-size:14px;width:18px;text-align:center;">${kindIcon}</span>
          <span style="flex:1;min-width:0;">
            <div style="font-size:13px;font-weight:500;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${esc(it.label)}</div>
            ${it.hint ? `<div style="font-size:11px;color:var(--admin-text-muted);">${esc(it.hint)}</div>` : ""}
          </span>
          ${it.right ? `<code style="font-size:10px;color:var(--admin-text-muted);background:var(--admin-border);padding:2px 5px;border-radius:3px;">${esc(it.right)}</code>` : ""}
        </button>`;
    })
    .join("");

  container.querySelectorAll(".cmdk-row").forEach((el) => {
    el.addEventListener("mouseenter", () => {
      cmdkSelectedIdx = parseInt(el.getAttribute("data-cmdk-idx"), 10);
      // Repaint active state without full re-render
      container.querySelectorAll(".cmdk-row").forEach((r, i) => {
        r.style.background = i === cmdkSelectedIdx ? "var(--admin-bg-card-hover)" : "transparent";
        r.setAttribute("aria-selected", i === cmdkSelectedIdx ? "true" : "false");
      });
    });
    el.addEventListener("click", (ev) => {
      const idx = parseInt(el.getAttribute("data-cmdk-idx"), 10);
      runCmdkItem(idx, ev.shiftKey);
    });
  });
}

function runCmdkItem(idx, useSecondary) {
  const it = cmdkResults[idx];
  if (!it || it.disabled) return;
  closeCmdk();
  if (useSecondary && typeof it.secondary === "function") it.secondary();
  else if (typeof it.run === "function") it.run();
}

function setupCmdk() {
  const input = document.getElementById("cmdk-input");
  const modal = document.getElementById("cmdk");
  if (!input || !modal) return;

  // Global hotkey: ⌘K / Ctrl+K
  document.addEventListener("keydown", (e) => {
    const isCmdK = (e.metaKey || e.ctrlKey) && (e.key === "k" || e.key === "K");
    if (isCmdK) {
      e.preventDefault();
      cmdkOpen ? closeCmdk() : openCmdk();
      return;
    }
    if (!cmdkOpen) return;
    if (e.key === "Escape") {
      e.preventDefault();
      closeCmdk();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      cmdkSelectedIdx = Math.min(cmdkResults.length - 1, cmdkSelectedIdx + 1);
      renderCmdkResults(input.value);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      cmdkSelectedIdx = Math.max(0, cmdkSelectedIdx - 1);
      renderCmdkResults(input.value);
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      runCmdkItem(cmdkSelectedIdx, e.shiftKey);
      return;
    }
  });

  input.addEventListener("input", () => {
    cmdkSelectedIdx = 0;
    renderCmdkResults(input.value);
  });

  modal.addEventListener("click", (e) => {
    if (e.target === modal) closeCmdk();
  });
}

// ─── History Timeline + JSON Diff (#15) ──────────────────────────────────────
function renderHistoryTimeline(entries) {
  if (!entries || entries.length === 0) {
    return '<div style="font-size:12px;color:var(--admin-text-muted);font-style:italic;padding:8px;">No history yet. Audit logs are written when status, assignment, notes, or test flag change.</div>';
  }
  const actionLabel = (a) => {
    const map = {
      "developer_project.status_changed": "Status changed",
      "developer_project.note_created": "Note added",
      "developer_project.assign": "Assignment changed",
      "developer_project.test_flag": "Test flag toggled",
      "asset.published": "Asset published",
      "asset.created": "Asset created",
    };
    return map[a] || a;
  };
  return (
    '<ol style="list-style:none;padding:0;margin:0 0 0 6px;border-left:2px solid var(--admin-border);">' +
    entries
      .map((e) => {
        const diff = renderJsonDiff(e.previous_state, e.new_state);
        const actor = e.actor_name || e.actor_email || "system";
        return `<li style="position:relative;padding:8px 0 12px 16px;">
          <span style="position:absolute;left:-7px;top:13px;width:12px;height:12px;border-radius:50%;background:var(--admin-bg-card);border:2px solid var(--admin-primary, #4f46e5);"></span>
          <div style="display:flex;justify-content:space-between;align-items:baseline;gap:8px;">
            <strong style="font-size:12px;color:var(--admin-text-primary);">${esc(actionLabel(e.action))}</strong>
            <span style="font-size:11px;color:var(--admin-text-muted);">${formatRelative(e.created_at)} • ${esc(actor)}</span>
          </div>
          ${diff}
        </li>`;
      })
      .join("") +
    "</ol>"
  );
}

function renderJsonDiff(prev, next) {
  prev = prev && typeof prev === "object" ? prev : {};
  next = next && typeof next === "object" ? next : {};
  const keys = new Set([...Object.keys(prev), ...Object.keys(next)]);
  if (keys.size === 0) return "";
  const lines = [];
  keys.forEach((k) => {
    const a = prev[k];
    const b = next[k];
    if (JSON.stringify(a) === JSON.stringify(b)) return;
    const fmt = (v) =>
      v === undefined
        ? "—"
        : v === null
          ? "null"
          : typeof v === "string"
            ? v
            : JSON.stringify(v);
    if (a === undefined) {
      lines.push(`<div style="color:var(--admin-success);font-family:monospace;font-size:11px;">+ ${esc(k)}: ${esc(fmt(b))}</div>`);
    } else if (b === undefined) {
      lines.push(`<div style="color:var(--admin-danger);font-family:monospace;font-size:11px;">- ${esc(k)}: ${esc(fmt(a))}</div>`);
    } else {
      lines.push(`<div style="color:var(--admin-danger);font-family:monospace;font-size:11px;">- ${esc(k)}: ${esc(fmt(a))}</div>`);
      lines.push(`<div style="color:var(--admin-success);font-family:monospace;font-size:11px;">+ ${esc(k)}: ${esc(fmt(b))}</div>`);
    }
  });
  if (lines.length === 0) return "";
  return `<div style="margin-top:4px;padding:6px 10px;background:var(--admin-bg-card);border:1px solid var(--admin-border);border-radius:4px;">${lines.join("")}</div>`;
}

// Expose for inline handlers / debugging
window.openQuickModal = openQuickModal;
window.adminSubmissionsOpenCmdk = openCmdk;
