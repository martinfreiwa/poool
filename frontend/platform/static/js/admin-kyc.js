// State Management
let allKYCRecords = [];
let queueRecords = [];
let allRecords = [];
let queuePage = 1;
let allPage = 1;
const PAGE_SIZE = 10;
let sortField = "created_at";
let sortOrder = "desc";
let currentTab = "queue";

let kycAutoRefresh = null;

document.addEventListener("DOMContentLoaded", () => {
  if (window.AdminPageKit) AdminPageKit.injectScopedCss();
  loadKYCRecords();
  setupTabs();
  setupFilters();
  setupModal();
  setupSorting();
  setupPagination();
  setupKitFeatures();
  setupDashboardControls();
});

function setupKitFeatures() {
  if (!window.AdminPageKit) return;
  // KPI click-to-tab
  AdminPageKit.wireKpiClicks((card) => {
    const tabName = card.dataset.tab;
    if (!tabName) return;
    const tabBtn = document.querySelector(`.admin-tab[data-tab="${tabName}"]`);
    tabBtn?.click();
  });
  // Manual refresh
  document.getElementById("kyc-refresh-btn")?.addEventListener("click", () => loadKYCRecords());
  // CSV export
  document.getElementById("kyc-export-csv")?.addEventListener("click", () => {
    const rows = currentTab === "queue" ? queueRecords : allRecords;
    AdminPageKit.exportCsv(rows, [
      [(r) => r.user_name || r.user_email, "User"],
      ["user_email", "Email"],
      ["status", "Status"],
      ["provider", "Provider"],
      ["doc_type", "Doc Type"],
      ["pep_check", "PEP"],
      ["sanctions_check", "Sanctions"],
      ["submitted_at", "Submitted"],
      ["expires_at", "Expires"],
      [(r) => AdminPageKit.ageSeconds(r.submitted_at || r.created_at), "Age (sec)"],
      ["id", "Record ID"],
      ["user_id", "User ID"],
    ], `kyc-${currentTab}.csv`);
  });
  // Auto-refresh
  kycAutoRefresh = AdminPageKit.setupAutoRefresh({
    refreshFn: () => loadKYCRecords(),
    toggleSelector: "#auto-refresh",
    lastUpdatedSelector: "#last-updated-label",
    isBusy: () => document.getElementById("kyc-modal")?.style.display === "flex",
  });
}

function setupDashboardControls() {
  document.querySelectorAll(".admin-kpi-card[data-tab]").forEach((card) => {
    card.setAttribute("draggable", "true");
    card.setAttribute("role", "button");
    card.setAttribute("aria-pressed", card.dataset.tab === currentTab ? "true" : "false");
  });

  document.getElementById("queue-search")?.addEventListener("input", debounce(applyFilters, 200));
  document.getElementById("queue-filter-provider")?.addEventListener("change", applyFilters);
  document.getElementById("queue-filter-risk")?.addEventListener("change", applyFilters);
  document.getElementById("kyc-review-next")?.addEventListener("click", reviewNextRecord);
  document.getElementById("kyc-save-view")?.addEventListener("click", saveCurrentView);
  document.getElementById("kyc-shortcuts-btn")?.addEventListener("click", () => toggleShortcuts(true));
  document.getElementById("kyc-shortcuts-close")?.addEventListener("click", () => toggleShortcuts(false));
  document.getElementById("kyc-shortcuts-dialog")?.addEventListener("click", (e) => {
    if (e.target === e.currentTarget) toggleShortcuts(false);
  });
  document.getElementById("kyc-modal-assign-btn")?.addEventListener("click", () => {
    const audit = document.getElementById("kyc-modal-audit-list");
    if (audit) audit.textContent = "Assignment saved locally for this review session.";
  });

  document.addEventListener("keydown", (e) => {
    const activeTag = document.activeElement?.tagName || "";
    const isTyping = ["INPUT", "TEXTAREA", "SELECT"].includes(activeTag);
    if (e.key === "/" && !isTyping) {
      e.preventDefault();
      document.getElementById("queue-search")?.focus();
    }
    if (e.key.toLowerCase() === "r" && !isTyping) {
      e.preventDefault();
      reviewNextRecord();
    }
  });

  renderSavedViewChips();
  loadProviderHealth();
  startCountdown();
}

async function loadProviderHealth() {
  try {
    const resp = await fetch("/api/admin/kyc/providers/health");
    if (!resp.ok) return;
    const data = await resp.json();
    setText("kyc-provider-sumsub", data.providers?.sumsub?.status || "unknown");
    setText("kyc-sanctions-health", data.sanctions?.status || "unknown");
    setText("kyc-oldest-pending", formatAgeSeconds(data.freshness?.oldest_pending_seconds || 0));
  } catch (e) {
    console.error("Error loading KYC provider health", e);
  }
}

function startCountdown() {
  const el = document.getElementById("auto-refresh-countdown");
  const toggle = document.getElementById("auto-refresh");
  if (!el) return;
  let remaining = 30;
  setInterval(() => {
    if (toggle && !toggle.checked) {
      el.textContent = "off";
      return;
    }
    remaining = remaining <= 1 ? 30 : remaining - 1;
    el.textContent = `${remaining}s`;
  }, 1000);
}

function saveCurrentView() {
  const views = readSavedViews();
  views.push({
    name: `View ${views.length + 1}`,
    state: {
      tab: currentTab,
      search: document.getElementById("queue-search")?.value || "",
      provider: document.getElementById("queue-filter-provider")?.value || "",
      risk: document.getElementById("queue-filter-risk")?.value || "",
    },
  });
  localStorage.setItem("kyc.savedViews.v1", JSON.stringify(views));
  renderSavedViewChips();
}

function renderSavedViewChips() {
  const list = document.getElementById("kyc-views-list");
  if (!list) return;
  const views = readSavedViews();
  list.innerHTML = views
    .map((view, idx) => `<button type="button" class="admin-saved-view-chip" data-idx="${idx}">${esc(view.name)}</button>`)
    .join("");
  list.querySelectorAll(".admin-saved-view-chip").forEach((chip) => {
    chip.addEventListener("click", () => {
      const view = views[Number(chip.dataset.idx)];
      if (!view) return;
      const state = view.state || {};
      if (state.tab) document.querySelector(`.admin-tab[data-tab="${state.tab}"]`)?.click();
      const queueSearch = document.getElementById("queue-search");
      const queueProvider = document.getElementById("queue-filter-provider");
      const queueRisk = document.getElementById("queue-filter-risk");
      if (queueSearch) queueSearch.value = state.search || "";
      if (queueProvider) queueProvider.value = state.provider || "";
      if (queueRisk) queueRisk.value = state.risk || "";
      applyFilters();
    });
  });
}

function readSavedViews() {
  try {
    const parsed = JSON.parse(localStorage.getItem("kyc.savedViews.v1") || "[]");
    if (Array.isArray(parsed)) return parsed;
    return Object.keys(parsed).map((name) => ({ name, state: parsed[name] }));
  } catch {
    return [];
  }
}

function toggleShortcuts(open) {
  const dialog = document.getElementById("kyc-shortcuts-dialog");
  if (dialog) dialog.style.display = open ? "flex" : "none";
}

function reviewNextRecord() {
  const next = queueRecords[0] || allKYCRecords.find((r) => r.status === "pending" || r.status === "in_review");
  if (next) openReviewModal(next.id);
}

function setText(id, value) {
  const el = document.getElementById(id);
  if (el) el.textContent = value;
}

function formatAgeSeconds(seconds) {
  const secs = Number(seconds) || 0;
  if (secs <= 0) return "none";
  const days = Math.floor(secs / 86400);
  const hours = Math.floor((secs % 86400) / 3600);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h`;
  return `${Math.max(1, Math.floor(secs / 60))}m`;
}

function renderKycActionRequired() {
  if (!window.AdminPageKit) return;
  const SLA_HOURS = 48;
  const slaSec = SLA_HOURS * 3600;
  const overSla = allKYCRecords.filter((r) =>
    (r.status === "pending" || r.status === "in_review") &&
    AdminPageKit.ageSeconds(r.submitted_at || r.created_at) > slaSec,
  );
  const pepFlagged = allKYCRecords.filter((r) =>
    (r.status === "pending" || r.status === "in_review") && r.pep_check === false,
  );
  const sanctionsFlagged = allKYCRecords.filter((r) =>
    (r.status === "pending" || r.status === "in_review") && r.sanctions_check === false,
  );
  const expiringSoon = allKYCRecords.filter((r) =>
    r.status === "approved" && isExpiringSoon(r.expires_at),
  );

  const items = [];
  const goToTab = (tab) => () => document.querySelector(`.admin-tab[data-tab="${tab}"]`)?.click();
  if (overSla.length) items.push({ label: `Pending >${SLA_HOURS}h (SLA breach)`, count: overSla.length, color: "var(--admin-danger, #C2410C)", onClick: goToTab("queue") });
  if (pepFlagged.length) items.push({ label: "PEP flagged in queue", count: pepFlagged.length, color: "var(--admin-danger, #C2410C)", onClick: goToTab("pep") });
  if (sanctionsFlagged.length) items.push({ label: "Sanctions hit in queue", count: sanctionsFlagged.length, color: "var(--admin-danger, #C2410C)", onClick: goToTab("queue") });
  if (expiringSoon.length) items.push({ label: "Approved expiring <30d", count: expiringSoon.length, color: "var(--admin-warning)", onClick: goToTab("expiring") });

  AdminPageKit.renderActionRequired(items, "#action-required-banner");
}

function setupSorting() {
  document.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    th.setAttribute("tabindex", "0");
    th.setAttribute("role", "columnheader");
    const handler = () => {
      const field = th.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      th.setAttribute("aria-sort", sortOrder === "asc" ? "ascending" : "descending");
      applyFilters();
    };
    th.addEventListener("click", handler);
    th.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); handler(); }
    });
  });
}

function setupPagination() {
  document.getElementById("queue-prev-page")?.addEventListener("click", () => {
    if (queuePage > 1) {
      queuePage--;
      renderQueue();
    }
  });
  document.getElementById("queue-next-page")?.addEventListener("click", () => {
    const total = Math.ceil(queueRecords.length / PAGE_SIZE);
    if (queuePage < total) {
      queuePage++;
      renderQueue();
    }
  });
  document.getElementById("all-prev-page")?.addEventListener("click", () => {
    if (allPage > 1) {
      allPage--;
      renderAllRecords();
    }
  });
  document.getElementById("all-next-page")?.addEventListener("click", () => {
    const total = Math.ceil(allRecords.length / PAGE_SIZE);
    if (allPage < total) {
      allPage++;
      renderAllRecords();
    }
  });
}

function setupTabs() {
  document.querySelectorAll(".admin-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      document
        .querySelectorAll(".admin-tab")
        .forEach((t) => t.classList.remove("active"));
      document
        .querySelectorAll(".admin-tab-panel")
        .forEach((p) => (p.style.display = "none"));
      tab.classList.add("active");

      const tabName = tab.dataset.tab;
      currentTab = tabName;
      document.querySelectorAll(".admin-kpi-card[data-tab]").forEach((card) => {
        card.setAttribute("aria-pressed", card.dataset.tab === currentTab ? "true" : "false");
      });

      if (tabName === "queue") {
        document.getElementById("tab-queue").style.display = "";
      } else {
        document.getElementById("tab-all").style.display = "";
      }
      applyFilters();
    });
  });
}

function setupFilters() {
  document
    .getElementById("kyc-search")
    ?.addEventListener("input", debounce(applyFilters, 200));
  document
    .getElementById("kyc-filter-status")
    ?.addEventListener("change", applyFilters);
}

function setupModal() {
  document
    .getElementById("kyc-modal-cancel")
    ?.addEventListener("click", closeModal);
  document.getElementById("kyc-modal")?.addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeModal();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && document.getElementById("kyc-modal")?.style.display === "flex") {
      closeModal();
    }
  });
}

async function loadKYCRecords() {
  try {
    const resp = await fetch("/api/admin/kyc");
    if (resp.ok) {
      const data = await resp.json();
      allKYCRecords = data.records || data;
      applyFilters();
      if (data.stats) {
        data.stats.expiring_soon = allKYCRecords.filter(
          (r) => r.status === "approved" && isExpiringSoon(r.expires_at),
        ).length;
        updateStats(data.stats);
      }
      renderKycActionRequired();
      if (kycAutoRefresh) kycAutoRefresh.markFetched();
      loadProviderHealth();
    } else {
      const msg = resp.status === 403 ? "Access denied." : `Failed to load KYC records (${resp.status}).`;
      const tbody = document.getElementById("kyc-queue-body");
      if (tbody) tbody.innerHTML = `<tr><td colspan="7" style="text-align:center;color:#c00;">${msg}</td></tr>`;
      const tbody2 = document.getElementById("kyc-all-body");
      if (tbody2) tbody2.innerHTML = `<tr><td colspan="7" style="text-align:center;color:#c00;">${msg}</td></tr>`;
    }
  } catch (e) {
    console.error("Error loading kyc records", e);
    if (typeof Sentry !== 'undefined') Sentry.captureException(e);
    const tbody = document.getElementById("kyc-queue-body");
    if (tbody) tbody.innerHTML = `<tr><td colspan="7" style="text-align:center;color:#c00;">Network error loading KYC records.</td></tr>`;
  }
}

function applyFilters() {
  // 1. Queue Filter
  queueRecords = allKYCRecords.filter(
    (r) => r.status === "pending" || r.status === "in_review",
  );
  const queueSearch = (document.getElementById("queue-search")?.value || "").toLowerCase();
  const queueProvider = document.getElementById("queue-filter-provider")?.value || "";
  const queueRisk = document.getElementById("queue-filter-risk")?.value || "";
  if (queueSearch) {
    queueRecords = queueRecords.filter((r) =>
      `${r.user_name} ${r.user_email}`.toLowerCase().includes(queueSearch),
    );
  }
  if (queueProvider) {
    queueRecords = queueRecords.filter((r) => r.provider === queueProvider);
  }
  if (queueRisk === "pep") {
    queueRecords = queueRecords.filter((r) => r.pep_check_passed === false);
  } else if (queueRisk === "sanctions") {
    queueRecords = queueRecords.filter((r) => r.sanctions_check === false);
  }

  // 2. All Filter
  const search = (
    document.getElementById("kyc-search")?.value || ""
  ).toLowerCase();
  const statusDropdown = document.getElementById("kyc-filter-status");
  // Use the PooolDropdown wrapper if present, otherwise the native select
  const statusContainer = statusDropdown?.closest("[data-dropdown]") || statusDropdown;
  const statusFilter = statusDropdown ? statusDropdown.value : "";

  let filtered = [...allKYCRecords];

  // Tab filtering
  if (currentTab === "approved") {
    filtered = filtered.filter((r) => r.status === "approved");
    if (statusContainer) statusContainer.style.display = "none";
  } else if (currentTab === "rejected") {
    filtered = filtered.filter((r) => r.status === "rejected");
    if (statusContainer) statusContainer.style.display = "none";
  } else if (currentTab === "pep") {
    filtered = filtered.filter((r) => r.pep_check_passed === false);
    if (statusContainer) statusContainer.style.display = "";
  } else if (currentTab === "expiring") {
    filtered = filtered.filter((r) => isExpiringSoon(r.expires_at));
    if (statusContainer) statusContainer.style.display = "";
  } else if (currentTab === "all" || currentTab === "queue") {
    if (statusContainer) statusContainer.style.display = "";
  }

  if (
    statusDropdown &&
    statusContainer?.style.display !== "none" &&
    statusFilter
  ) {
    filtered = filtered.filter((r) => r.status === statusFilter);
  }

  if (search)
    filtered = filtered.filter((r) =>
      `${r.user_name} ${r.user_email}`.toLowerCase().includes(search),
    );

  // 3. Sorting (Shared logic)
  const sorter = (a, b) => {
    let valA = a[sortField],
      valB = b[sortField];
    if (sortField === "user_name") {
      valA = (a.user_name || "").toLowerCase();
      valB = (b.user_name || "").toLowerCase();
    }
    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  };

  queueRecords.sort(sorter);
  allRecords = filtered;
  allRecords.sort(sorter);

  queuePage = 1;
  allPage = 1;

  renderQueue();
  renderAllRecords();
}

function updateStats(stats) {
  if (!stats) return;
  document.getElementById("kyc-pending").textContent = stats.pending || 0;
  document.getElementById("kyc-approved").textContent = stats.approved || 0;
  document.getElementById("kyc-rejected").textContent = stats.rejected || 0;
  document.getElementById("kyc-pep").textContent = stats.pep_flags || 0;
  document.getElementById("kyc-expiring").textContent =
    stats.expiring_soon || 0;
}

// ─── Render: Review Queue (pending only) ────────────────────────

// ─── Render: Review Queue (pending only) ────────────────────────

function renderQueue() {
  const tbody = document.getElementById("kyc-queue-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(queueRecords.length / PAGE_SIZE));
  queuePage = Math.min(queuePage, totalPages);
  const start = (queuePage - 1) * PAGE_SIZE;
  const slice = queueRecords.slice(start, start + PAGE_SIZE);

  const queueCountEl = document.getElementById("queue-count");
  if (queueCountEl) queueCountEl.textContent = `${queueRecords.length} pending`;

  if (queueRecords.length === 0) {
    tbody.innerHTML = `<tr><td colspan="7">
            <div class="admin-empty-state">
                <div class="admin-empty-state-icon"><svg width="40" height="40" viewBox="0 0 40 40" fill="none" stroke="var(--admin-success)" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><circle cx="20" cy="20" r="16"/><path d="M14 20l4 4 8-8"/></svg></div>
                <div class="admin-empty-state-title">Queue Empty</div>
                <div class="admin-empty-state-text">All KYC submissions have been reviewed.</div>
            </div>
        </td></tr>`;
    return;
  }

  // Update Pagination UI
  const info = document.getElementById("queue-pagination-info");
  if (info)
    info.textContent = `Page ${queuePage} of ${totalPages} (${queueRecords.length} total)`;
  const queuePrev = document.getElementById("queue-prev-page");
  const queueNext = document.getElementById("queue-next-page");
  if (queuePrev) queuePrev.disabled = queuePage <= 1;
  if (queueNext) queueNext.disabled = queuePage >= totalPages;

  tbody.innerHTML = slice
    .map(
      (r) => `
        <tr data-id="${esc(r.id)}">
            <td>
                <div class="admin-user-inline">
                    <div>
                        <div class="admin-user-inline-name"><a href="/admin/user-details?id=${esc(r.user_id)}" class="admin-link">${esc(r.user_name)}</a></div>
                        <div class="admin-user-inline-email">${esc(r.user_email)}</div>
                    </div>
                </div>
            </td>
            <td><span class="admin-badge admin-badge--neutral">${esc(r.provider)}</span></td>
            <td>${getDocTypeBadge(r.document_type)}</td>
            <td>${getPEPBadge(r.pep_check_passed)}</td>
            <td>${getSanctionsBadge(r.sanctions_check)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDateTime(r.created_at)}</td>
            <td>
                <div style="display:flex;gap:4px;">
                    <button class="admin-btn admin-btn--primary admin-btn--sm" onclick="openReviewModal('${esc(r.id)}')">
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M1 8s3-6 7-6 7 6 7 6-3 6-7 6-7-6-7-6z"/><circle cx="8" cy="8" r="2.5"/></svg>
                        Review
                    </button>
                    ${r.provider === "sumsub"
          ? `<a href="https://cockpit.sumsub.com/checkus#/applicant/${esc(r.provider_ref_id)}" target="_blank" rel="noopener" class="admin-btn admin-btn--secondary admin-btn--sm" title="View in SumSub">↗ SumSub</a>`
          : r.provider === "didit"
            ? `<a href="https://business.didit.me/sessions/${esc(r.provider_ref_id)}" target="_blank" rel="noopener" class="admin-btn admin-btn--secondary admin-btn--sm" title="View in Didit">↗ Didit</a>`
            : r.provider_ref_id
              ? `<span class="admin-badge admin-badge--neutral" title="Ref ID: ${esc(r.provider_ref_id)}">Ref: ${esc(r.provider_ref_id.slice(0, 8))}…</span>`
              : ""
        }
                </div>
                ${r.has_documents
          ? `<div style="margin-top:4px;"><button class="admin-btn admin-btn--secondary admin-btn--sm" style="width:100%;" onclick="viewKYCDocuments('${esc(r.id)}')">📁 View Documents</button></div>`
          : ""
        }
            </td>
        </tr>
    `,
    )
    .join("");
}

// ─── Render: All Records ────────────────────────────────────────

function renderAllRecords() {
  const tbody = document.getElementById("kyc-all-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(allRecords.length / PAGE_SIZE));
  allPage = Math.min(allPage, totalPages);
  const start = (allPage - 1) * PAGE_SIZE;
  const slice = allRecords.slice(start, start + PAGE_SIZE);

  const countLabel = document.getElementById("kyc-count-label");
  if (countLabel)
    countLabel.textContent = `Showing ${allRecords.length} records`;

  if (allRecords.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="9" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No records match your filters.</td></tr>';
    return;
  }

  // Update Pagination UI
  const info = document.getElementById("all-pagination-info");
  if (info)
    info.textContent = `Page ${allPage} of ${totalPages} (${allRecords.length} total)`;
  const allPrev = document.getElementById("all-prev-page");
  const allNext = document.getElementById("all-next-page");
  if (allPrev) allPrev.disabled = allPage <= 1;
  if (allNext) allNext.disabled = allPage >= totalPages;

  tbody.innerHTML = slice
    .map(
      (r) => `
        <tr>
            <td>
                <div class="admin-user-inline">
                    <div>
                        <div class="admin-user-inline-name"><a href="/admin/user-details?id=${esc(r.user_id)}" class="admin-link">${esc(r.user_name)}</a></div>
                        <div class="admin-user-inline-email">${esc(r.user_email)}</div>
                    </div>
                </div>
            </td>
            <td>${getKYCStatusBadge(r.status)}</td>
            <td><span class="admin-badge admin-badge--neutral">${esc(r.provider)}</span></td>
            <td>${getDocTypeBadge(r.document_type)}</td>
            <td>${getPEPBadge(r.pep_check_passed)}</td>
            <td>${getSanctionsBadge(r.sanctions_check)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${r.verified_at ? formatDate(r.verified_at) : "—"}</td>
            <td style="font-size:12px;color:${isExpiringSoon(r.expires_at) ? "var(--admin-danger)" : "var(--admin-text-muted)"};">${r.expires_at ? formatDate(r.expires_at) : "—"}</td>
            <td>
                ${r.status === "pending" || r.status === "in_review" ? `<button class="admin-btn admin-btn--primary admin-btn--sm" onclick="openReviewModal('${esc(r.id)}')">Review</button>` : "—"}
            </td>
        </tr>
    `,
    )
    .join("");
}

// ─── Review Modal ───────────────────────────────────────────────

let reviewingKYCId = null;

function openReviewModal(kycId) {
  reviewingKYCId = kycId;
  const record = allKYCRecords.find((r) => r.id === kycId);
  if (!record) return;

  document.getElementById("kyc-modal-title").textContent =
    `Review KYC — ${record.user_name}`;

  let providerLink = "";
  if (record.provider === "sumsub" && record.provider_ref_id) {
    providerLink = ` <a href="https://cockpit.sumsub.com/checkus#/applicant/${esc(record.provider_ref_id)}" target="_blank" rel="noopener" style="color:var(--admin-accent);font-size:12px;">↗ SumSub</a>`;
  } else if (record.provider === "didit" && record.provider_ref_id) {
    providerLink = ` <a href="https://business.didit.me/sessions/${esc(record.provider_ref_id)}" target="_blank" rel="noopener" style="color:var(--admin-accent);font-size:12px;">↗ Didit</a>`;
  }

  document.getElementById("kyc-modal-text").innerHTML = `
        <strong>Email:</strong> ${esc(record.user_email)}<br>
        <strong>Provider:</strong> ${esc(record.provider)}${providerLink}<br>
        <strong>Document:</strong> ${esc(record.document_type || "Not specified")}<br>
        <strong>PEP Check:</strong> ${record.pep_check_passed === true ? "Passed" : record.pep_check_passed === false ? "Flagged" : "Pending"}<br>
        <strong>Sanctions:</strong> ${record.sanctions_check === true ? "Clear" : record.sanctions_check === false ? "Hit" : "Pending"}
    `;

  const viewDocsBtn = document.getElementById("kyc-modal-view-docs");
  if (viewDocsBtn) {
    if (record.has_documents) {
      viewDocsBtn.style.display = "";
      viewDocsBtn.onclick = () => { closeModal(); viewKYCDocuments(kycId); };
    } else {
      viewDocsBtn.style.display = "none";
    }
  }

  document.getElementById("kyc-rejection-reason").value = "";
  const audit = document.getElementById("kyc-modal-audit-list");
  if (audit) audit.textContent = "No audit entries for this browser session.";

  document.getElementById("kyc-modal-approve").onclick = () =>
    handleKYCAction("approve");
  document.getElementById("kyc-modal-reject").onclick = () =>
    handleKYCAction("reject");

  const modal = document.getElementById("kyc-modal");
  modal.style.display = "flex";
  modal.setAttribute("aria-modal", "true");
  modal.setAttribute("role", "dialog");
  setTimeout(() => document.getElementById("kyc-modal-cancel")?.focus(), 50);
}

function closeModal() {
  const modal = document.getElementById("kyc-modal");
  modal.style.display = "none";
  modal.removeAttribute("aria-modal");
  reviewingKYCId = null;
}

async function handleKYCAction(action) {
  const reason = document.getElementById("kyc-rejection-reason")?.value || "";

  if (action === "reject" && !reason.trim()) {
    document.getElementById("kyc-rejection-reason").style.borderColor =
      "var(--admin-danger)";
    document.getElementById("kyc-rejection-reason").focus();
    return;
  }

  try {
    const resp = await fetch(`/api/admin/kyc/${reviewingKYCId}/${action}`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Token": typeof window.getCsrfToken === "function" ? window.getCsrfToken() : "",
      },
      body: JSON.stringify({ rejection_reason: reason }),
    });
    if (resp.ok) {
      closeModal();
      loadKYCRecords();
      return;
    } else {
      console.error(`Failed to ${action} KYC record`);
    }
  } catch (e) {
    console.error(`Error handling KYC action ${action}`, e);
    if (typeof Sentry !== 'undefined') Sentry.captureException(e);
  }
}

// ─── Helpers ────────────────────────────────────────────────────

function esc(str) {
  if (typeof str !== "string") return str || "";
  const d = document.createElement("div");
  d.textContent = str;
  return d.innerHTML;
}

function formatDate(iso) {
  if (!iso) return "—";
  return new Date(iso).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function formatDateTime(iso) {
  if (!iso) return "—";
  const d = new Date(iso);
  return (
    d.toLocaleDateString("en-US", { month: "short", day: "numeric" }) +
    " " +
    d.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    })
  );
}

function isExpiringSoon(expiresAt) {
  if (!expiresAt) return false;
  const expiresDate = new Date(expiresAt).getTime();
  const now = Date.now();
  // Consider it expiring soon if it is less than 30 days away and hasn't already expired
  return expiresDate > now && expiresDate - now < 30 * 86400000;
}

function getKYCStatusBadge(status) {
  const map = {
    pending: ["admin-badge--warning", "Pending"],
    in_review: ["admin-badge--info", "In Review"],
    approved: ["admin-badge--success", "Approved"],
    rejected: ["admin-badge--danger", "Rejected"],
    expired: ["admin-badge--danger", "Expired"],
  };
  const [cls, label] = map[status] || ["admin-badge--neutral", status];
  return `<span class="admin-badge ${cls}"><span class="admin-badge-dot"></span>${label}</span>`;
}

function getDocTypeBadge(type) {
  const map = {
    passport: "Passport",
    id_card: "ID Card",
    drivers_license: "Driver's License",
  };
  return `<span class="admin-badge admin-badge--neutral">${map[type] || type || "—"}</span>`;
}

function getPEPBadge(passed) {
  if (passed === true)
    return '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Clear</span>';
  if (passed === false)
    return '<span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>Flagged</span>';
  return '<span class="admin-badge admin-badge--neutral">Pending</span>';
}

function getSanctionsBadge(passed) {
  if (passed === true)
    return '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Clear</span>';
  if (passed === false)
    return '<span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>Hit</span>';
  return '<span class="admin-badge admin-badge--neutral">Pending</span>';
}

function debounce(fn, ms) {
  let t;
  return function (...args) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, args), ms);
  };
}

async function viewKYCDocuments(kycId) {
  try {
    const resp = await fetch(`/api/admin/kyc/${kycId}/documents`);
    if (!resp.ok) throw new Error("Failed to fetch documents");

    const docs = await resp.json();
    if (!docs || docs.length === 0) {
      alert("No documents found for this record.");
      return;
    }

    // Create a simple document viewer overlay if it doesn't exist
    let viewer = document.getElementById("kyc-doc-viewer");
    if (!viewer) {
      viewer = document.createElement("div");
      viewer.id = "kyc-doc-viewer";
      viewer.style = "position:fixed;top:0;left:0;width:100%;height:100%;background:rgba(0,0,0,0.85);z-index:10000;display:flex;flex-direction:column;align-items:center;justify-content:center;padding:40px;";
      viewer.innerHTML = `
        <div style="position:absolute;top:20px;right:20px;color:white;cursor:pointer;font-size:30px;" onclick="this.parentElement.style.display='none'">×</div>
        <div id="kyc-doc-content" style="max-width:90%;max-height:80%;overflow:auto;background:white;padding:10px;border-radius:8px;"></div>
        <div id="kyc-doc-footer" style="margin-top:20px;color:white;text-align:center;"></div>
      `;
      document.body.appendChild(viewer);
    }

    const content = document.getElementById("kyc-doc-content");
    const footer = document.getElementById("kyc-doc-footer");

    content.textContent = "";
    docs.forEach((d, i) => {
      if (i > 0) {
        const hr = document.createElement("hr");
        hr.style.margin = "20px 0";
        content.appendChild(hr);
      }
      if (!d.url) {
        const msg = document.createElement("p");
        msg.style.cssText = "color:#888;text-align:center;padding:20px;";
        msg.textContent = `Document file not available (${d.document_type || "unknown type"})`;
        content.appendChild(msg);
      } else {
        const isPdf = d.url.toLowerCase().includes(".pdf") || d.document_type === "pdf";
        if (isPdf) {
          const embed = document.createElement("embed");
          embed.src = d.url;
          embed.type = "application/pdf";
          embed.width = "800px";
          embed.height = "600px";
          content.appendChild(embed);
        } else {
          const img = document.createElement("img");
          img.src = d.url;
          img.style.cssText = "max-width:100%;display:block;margin:0 auto;";
          img.alt = d.document_type || "KYC document";
          content.appendChild(img);
        }
      }
    });

    footer.textContent = `Viewing ${docs.length} document(s) for KYC ${kycId}`;
    viewer.style.display = "flex";

  } catch (err) {
    console.error("Error viewing documents:", err);
    alert("Could not load documents.");
  }
}

window.viewKYCDocuments = viewKYCDocuments;
