// ─── State Management ───────────────────────────────────────────
const PAGE_SIZE = 20;
let currentTab = "queue";
let sortField = "created_at";
let sortOrder = "desc";

// Separate pagination state for queue vs filtered views
let queuePage = 1;
let queueTotalPages = 1;
let queueTotalCount = 0;

let filteredPage = 1;
let filteredTotalPages = 1;
let filteredTotalCount = 0;

// Tab label map for the filter banner
const TAB_LABELS = {
  approved: "Approved Records",
  rejected: "Rejected Records",
  pep: "PEP Flagged Records",
  expiring: "Expiring Soon (within 30 days)",
  all: "All Records",
};

document.addEventListener("DOMContentLoaded", () => {
  loadCurrentView();
  setupTabs();
  setupFilters();
  setupModal();
  setupSorting();
  setupPagination();
  setupDelegatedActions();
});

// ─── API Call — Server-Side Pagination ──────────────────────────

async function fetchKYCRecords(tab, page) {
  const search = (document.getElementById("kyc-search")?.value || "").trim();
  const statusDropdown = document.getElementById("kyc-filter-status");
  const statusFilter = statusDropdown ? statusDropdown.value : "";

  const params = new URLSearchParams({
    tab: tab,
    page: String(page),
    page_size: String(PAGE_SIZE),
    sort: sortField,
    order: sortOrder,
  });

  if (search) params.set("search", search);
  if (statusFilter && !["approved", "rejected", "queue"].includes(tab)) {
    params.set("status", statusFilter);
  }

  const resp = await fetch(`/api/admin/kyc?${params.toString()}`);
  if (!resp.ok) {
    const errText = await resp.text().catch(() => "Unknown error");
    throw new Error(`API ${resp.status}: ${errText}`);
  }
  return resp.json();
}

// ─── Load Current View ──────────────────────────────────────────

async function loadCurrentView() {
  if (currentTab === "queue") {
    await loadQueue();
  } else {
    await loadFiltered();
  }
}

async function loadQueue() {
  const tbody = document.getElementById("kyc-queue-body");
  if (!tbody) return;

  // Show loading state
  tbody.innerHTML = `<tr><td colspan="7" class="kyc-empty-cell">
    <div class="kyc-spinner"></div>
    Loading KYC queue…
  </td></tr>`;

  try {
    const data = await fetchKYCRecords("queue", queuePage);

    // Update stats (always returned)
    if (data.stats) updateStats(data.stats);

    // Update pagination state
    const pg = data.pagination || {};
    queuePage = pg.page || 1;
    queueTotalPages = pg.total_pages || 1;
    queueTotalCount = pg.total_count || 0;

    // Update count label
    const queueCountEl = document.getElementById("queue-count");
    if (queueCountEl) queueCountEl.textContent = `${queueTotalCount} pending`;

    // Render
    renderQueue(data.records || []);
    updateQueuePagination();
  } catch (e) {
    console.error("Error loading queue:", e);
    if (typeof Sentry !== "undefined") Sentry.captureException(e);
    showLoadError("kyc-queue-body", 7, e.message);
  }
}

async function loadFiltered() {
  const tbody = document.getElementById("kyc-all-body");
  if (!tbody) return;

  // Show loading state
  tbody.innerHTML = `<tr><td colspan="9" class="kyc-empty-cell">
    <div class="kyc-spinner"></div>
    Loading records…
  </td></tr>`;

  try {
    const data = await fetchKYCRecords(currentTab, filteredPage);

    // Update stats (always returned)
    if (data.stats) updateStats(data.stats);

    // Update pagination state
    const pg = data.pagination || {};
    filteredPage = pg.page || 1;
    filteredTotalPages = pg.total_pages || 1;
    filteredTotalCount = pg.total_count || 0;

    // Update count label
    const countLabel = document.getElementById("kyc-count-label");
    if (countLabel) countLabel.textContent = `Showing ${filteredTotalCount} records`;

    // Render
    renderAllRecords(data.records || []);
    updateFilteredPagination();
  } catch (e) {
    console.error("Error loading filtered:", e);
    if (typeof Sentry !== "undefined") Sentry.captureException(e);
    showLoadError("kyc-all-body", 9, e.message);
  }
}

function showLoadError(tbodyId, colSpan, statusInfo) {
  const tbody = document.getElementById(tbodyId);
  if (!tbody) return;
  tbody.innerHTML = `<tr><td colspan="${colSpan}" class="kyc-error-cell">
    <strong>Failed to load KYC records</strong><br>
    <span class="kyc-text-muted">
      ${esc(String(statusInfo))}. <button data-action="retry-load" class="kyc-retry-btn">Retry</button>
    </span>
  </td></tr>`;
  tbody.querySelector("[data-action='retry-load']")?.addEventListener("click", loadCurrentView);
}

// ─── Event Delegation (replaces inline onclick handlers — XSS-safe) ──

function setupDelegatedActions() {
  document.addEventListener("click", (e) => {
    const reviewBtn = e.target.closest("[data-action='review']");
    if (reviewBtn) {
      openReviewModal(reviewBtn.dataset.id);
      return;
    }
    const docsBtn = e.target.closest("[data-action='view-docs']");
    if (docsBtn) {
      viewKYCDocuments(docsBtn.dataset.id);
      return;
    }
  });
}

// ─── Setup Functions ────────────────────────────────────────────

function setupSorting() {
  document.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    th.addEventListener("click", () => {
      const field = th.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      // Reset to page 1 and reload
      if (currentTab === "queue") {
        queuePage = 1;
        loadQueue();
      } else {
        filteredPage = 1;
        loadFiltered();
      }
    });
  });
}

function setupPagination() {
  // Queue pagination
  document.getElementById("queue-prev-page")?.addEventListener("click", () => {
    if (queuePage > 1) {
      queuePage--;
      loadQueue();
    }
  });
  document.getElementById("queue-next-page")?.addEventListener("click", () => {
    if (queuePage < queueTotalPages) {
      queuePage++;
      loadQueue();
    }
  });

  // Filtered/All pagination
  document.getElementById("all-prev-page")?.addEventListener("click", () => {
    if (filteredPage > 1) {
      filteredPage--;
      loadFiltered();
    }
  });
  document.getElementById("all-next-page")?.addEventListener("click", () => {
    if (filteredPage < filteredTotalPages) {
      filteredPage++;
      loadFiltered();
    }
  });
}

function setupTabs() {
  document.querySelectorAll(".admin-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      // Update active tab styling
      document.querySelectorAll(".admin-tab").forEach((t) => {
        t.classList.remove("active");
        t.setAttribute("aria-selected", "false");
      });
      tab.classList.add("active");
      tab.setAttribute("aria-selected", "true");

      // Hide all panels
      document.querySelectorAll(".admin-tab-panel").forEach((p) => (p.style.display = "none"));

      const tabName = tab.dataset.tab;
      currentTab = tabName;

      if (tabName === "queue") {
        document.getElementById("tab-queue").style.display = "";
        queuePage = 1;
        loadQueue();
      } else {
        document.getElementById("tab-filtered").style.display = "";

        // Show or hide the filter banner
        const banner = document.getElementById("kyc-filter-banner");
        const bannerLabel = document.getElementById("kyc-filter-banner-label");
        if (tabName === "all") {
          banner.style.display = "none";
        } else {
          banner.style.display = "";
          bannerLabel.textContent = TAB_LABELS[tabName] || tabName;
        }

        // Show/hide the status dropdown for tabs that already filter by status
        const statusDropdown = document.getElementById("kyc-filter-status");
        if (statusDropdown) {
          statusDropdown.style.display = ["approved", "rejected"].includes(tabName) ? "none" : "";
        }

        filteredPage = 1;
        loadFiltered();
      }
    });
  });
}

function setupFilters() {
  const searchInput = document.getElementById("kyc-search");
  const statusDropdown = document.getElementById("kyc-filter-status");

  if (searchInput) {
    searchInput.addEventListener("input", debounce(() => {
      if (currentTab === "queue") {
        queuePage = 1;
        loadQueue();
      } else {
        filteredPage = 1;
        loadFiltered();
      }
    }, 300));
  }

  if (statusDropdown) {
    statusDropdown.addEventListener("change", () => {
      filteredPage = 1;
      loadFiltered();
    });
  }
}

function setupModal() {
  document.getElementById("kyc-modal-cancel")?.addEventListener("click", closeModal);
  document.getElementById("kyc-modal")?.addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeModal();
  });
}

// ─── Pagination UI Updates ──────────────────────────────────────

function updateQueuePagination() {
  const info = document.getElementById("queue-pagination-info");
  if (info)
    info.textContent = `Page ${queuePage} of ${queueTotalPages} (${queueTotalCount} total)`;
  const prevBtn = document.getElementById("queue-prev-page");
  const nextBtn = document.getElementById("queue-next-page");
  if (prevBtn) prevBtn.disabled = queuePage <= 1;
  if (nextBtn) nextBtn.disabled = queuePage >= queueTotalPages;
}

function updateFilteredPagination() {
  const info = document.getElementById("all-pagination-info");
  if (info)
    info.textContent = `Page ${filteredPage} of ${filteredTotalPages} (${filteredTotalCount} total)`;
  const prevBtn = document.getElementById("all-prev-page");
  const nextBtn = document.getElementById("all-next-page");
  if (prevBtn) prevBtn.disabled = filteredPage <= 1;
  if (nextBtn) nextBtn.disabled = filteredPage >= filteredTotalPages;
}

// ─── Stats ──────────────────────────────────────────────────────

function updateStats(stats) {
  if (!stats) return;
  document.getElementById("kyc-pending").textContent = stats.pending || 0;
  document.getElementById("kyc-approved").textContent = stats.approved || 0;
  document.getElementById("kyc-rejected").textContent = stats.rejected || 0;
  document.getElementById("kyc-pep").textContent = stats.pep_flags || 0;
  document.getElementById("kyc-expiring").textContent = stats.expiring_soon || 0;
}

// ─── Render: Review Queue ───────────────────────────────────────

function renderQueue(records) {
  const tbody = document.getElementById("kyc-queue-body");
  if (!tbody) return;

  if (records.length === 0) {
    tbody.innerHTML = `<tr><td colspan="7">
      <div class="admin-empty-state">
        <div class="admin-empty-state-icon"><svg width="40" height="40" viewBox="0 0 40 40" fill="none" stroke="var(--admin-success)" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><circle cx="20" cy="20" r="16"/><path d="M14 20l4 4 8-8"/></svg></div>
        <div class="admin-empty-state-title">Queue Empty</div>
        <div class="admin-empty-state-text">All KYC submissions have been reviewed.</div>
      </div>
    </td></tr>`;
    return;
  }

  tbody.innerHTML = records
    .map(
      (r) => `
      <tr data-id="${esc(r.id)}">
        <td>
          <div class="admin-user-inline">
            <div>
              <div class="admin-user-inline-name"><a href="/admin/user-details?id=${encodeURIComponent(r.user_id)}" class="admin-link">${esc(r.user_name)}</a></div>
              <div class="admin-user-inline-email">${esc(r.user_email)}</div>
            </div>
          </div>
        </td>
        <td><span class="admin-badge admin-badge--neutral">${esc(r.provider)}</span></td>
        <td>${getDocTypeBadge(r.document_type)}</td>
        <td>${getPEPBadge(r.pep_check_passed)}</td>
        <td>${getSanctionsBadge(r.sanctions_check)}</td>
        <td class="kyc-date">${formatDateTime(r.created_at)}</td>
        <td>
          <div class="kyc-action-group">
            <button class="admin-btn admin-btn--primary admin-btn--sm" data-action="review" data-id="${esc(r.id)}">
              <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M1 8s3-6 7-6 7 6 7 6-3 6-7 6-7-6-7-6z"/><circle cx="8" cy="8" r="2.5"/></svg>
              Review
            </button>
            ${r.provider === "sumsub"
        ? `<a href="https://cockpit.sumsub.com/checkus#/applicant/${encodeURIComponent(r.provider_ref_id)}" target="_blank" rel="noopener noreferrer" class="admin-btn admin-btn--secondary admin-btn--sm" title="View in SumSub">↗ SumSub</a>`
        : r.provider === "didit"
          ? `<a href="https://business.didit.me/sessions/${encodeURIComponent(r.provider_ref_id)}" target="_blank" rel="noopener noreferrer" class="admin-btn admin-btn--secondary admin-btn--sm" title="View in Didit">↗ Didit</a>`
          : r.provider_ref_id
            ? `<span class="admin-badge admin-badge--neutral" title="Ref ID: ${esc(r.provider_ref_id)}">Ref: ${esc(r.provider_ref_id.slice(0, 8))}…</span>`
            : ""
      }
          </div>
          ${r.has_documents
        ? `<div class="kyc-action-full"><button class="admin-btn admin-btn--secondary admin-btn--sm" style="width:100%;" data-action="view-docs" data-id="${esc(r.id)}">📁 View Documents</button></div>`
        : ""
      }
        </td>
      </tr>`,
    )
    .join("");
}

// ─── Render: Filtered/All Records ───────────────────────────────

function renderAllRecords(records) {
  const tbody = document.getElementById("kyc-all-body");
  if (!tbody) return;

  if (records.length === 0) {
    tbody.innerHTML = `<tr><td colspan="9" class="kyc-empty-cell">No records match your filters.</td></tr>`;
    return;
  }

  tbody.innerHTML = records
    .map(
      (r) => `
      <tr>
        <td>
          <div class="admin-user-inline">
            <div>
              <div class="admin-user-inline-name"><a href="/admin/user-details?id=${encodeURIComponent(r.user_id)}" class="admin-link">${esc(r.user_name)}</a></div>
              <div class="admin-user-inline-email">${esc(r.user_email)}</div>
            </div>
          </div>
        </td>
        <td>${getKYCStatusBadge(r.status)}</td>
        <td><span class="admin-badge admin-badge--neutral">${esc(r.provider)}</span></td>
        <td>${getDocTypeBadge(r.document_type)}</td>
        <td>${getPEPBadge(r.pep_check_passed)}</td>
        <td>${getSanctionsBadge(r.sanctions_check)}</td>
        <td class="kyc-date">${r.verified_at ? formatDate(r.verified_at) : "—"}</td>
        <td class="${isExpiringSoon(r.expires_at) ? "kyc-date-danger" : "kyc-date"}">${r.expires_at ? formatDate(r.expires_at) : "—"}</td>
        <td>
          ${r.status === "pending" || r.status === "in_review" ? `<button class="admin-btn admin-btn--primary admin-btn--sm" data-action="review" data-id="${esc(r.id)}">Review</button>` : "—"}
        </td>
      </tr>`,
    )
    .join("");
}

// ─── Review Modal ───────────────────────────────────────────────

let reviewingKYCId = null;
let reviewingRecord = null;
let _modalEscHandler = null;

function openReviewModal(kycId) {
  reviewingKYCId = kycId;

  // We need to find the record from the rendered table row
  // Since data is now server-side, store the KYC ID and fetch details if needed
  // For now, we'll read from the already-loaded page data via the API
  // But since the modal only needs basic info, we'll look at what's in the DOM

  document.getElementById("kyc-modal-title").textContent = `Review KYC — ${kycId.slice(0, 8)}…`;
  document.getElementById("kyc-modal-text").innerHTML = `Loading details…`;
  document.getElementById("kyc-rejection-reason").value = "";
  document.getElementById("kyc-rejection-reason").style.borderColor = "";

  document.getElementById("kyc-modal-approve").onclick = () => handleKYCAction("approve");
  document.getElementById("kyc-modal-reject").onclick = () => handleKYCAction("reject");

  const modal = document.getElementById("kyc-modal");
  modal.style.display = "flex";

  // Focus the textarea for accessibility
  setTimeout(() => {
    document.getElementById("kyc-rejection-reason").focus();
  }, 100);

  // Escape key handler
  _modalEscHandler = (e) => {
    if (e.key === "Escape") closeModal();
  };
  document.addEventListener("keydown", _modalEscHandler);

  // Fetch fresh record details from row data
  const row = document.querySelector(`tr[data-id="${CSS.escape(kycId)}"]`);
  if (row) {
    const nameEl = row.querySelector(".admin-user-inline-name");
    const emailEl = row.querySelector(".admin-user-inline-email");
    const userName = nameEl ? nameEl.textContent.trim() : "Unknown";
    const userEmail = emailEl ? emailEl.textContent.trim() : "";

    document.getElementById("kyc-modal-title").textContent = `Review KYC — ${userName}`;
    document.getElementById("kyc-modal-text").innerHTML = `
      <strong>User:</strong> ${esc(userName)}<br>
      <strong>Email:</strong> ${esc(userEmail)}<br>
      <strong>KYC ID:</strong> <code class="kyc-id-badge">${esc(kycId)}</code>
    `;
  }
}

function closeModal() {
  document.getElementById("kyc-modal").style.display = "none";
  reviewingKYCId = null;
  if (_modalEscHandler) {
    document.removeEventListener("keydown", _modalEscHandler);
    _modalEscHandler = null;
  }
}

async function handleKYCAction(action) {
  const reason = document.getElementById("kyc-rejection-reason")?.value || "";

  if (action === "reject" && !reason.trim()) {
    document.getElementById("kyc-rejection-reason").style.borderColor = "var(--admin-danger)";
    document.getElementById("kyc-rejection-reason").focus();
    return;
  }

  // Confirmation step for approve
  if (action === "approve") {
    const confirmed =
      typeof window.pooolConfirm === "function"
        ? await window.pooolConfirm(
            "Approve KYC?",
            "This will mark the user as identity-verified and grant full platform access. This action cannot be easily undone.",
          )
        : confirm(
            "Are you sure you want to approve this KYC record? This grants the user full platform access.",
          );
    if (!confirmed) return;
  }

  // Disable buttons during request
  const approveBtn = document.getElementById("kyc-modal-approve");
  const rejectBtn = document.getElementById("kyc-modal-reject");
  approveBtn.disabled = true;
  rejectBtn.disabled = true;

  try {
    const resp = await fetch(`/api/admin/kyc/${reviewingKYCId}/${action}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ rejection_reason: reason }),
    });
    if (resp.ok) {
      closeModal();
      showToast(
        `KYC record ${action === "approve" ? "approved" : "rejected"} successfully.`,
        action === "approve" ? "success" : "danger",
      );
      // Reload current view to reflect the change
      loadCurrentView();
      return;
    } else {
      const errText = await resp.text().catch(() => "Unknown error");
      console.error(`Failed to ${action} KYC record: ${resp.status} — ${errText}`);
      showToast(`Failed to ${action} KYC record. Please try again.`, "danger");
    }
  } catch (e) {
    console.error(`Error handling KYC action ${action}`, e);
    if (typeof Sentry !== "undefined") Sentry.captureException(e);
    showToast(`Network error while trying to ${action}. Please try again.`, "danger");
  } finally {
    approveBtn.disabled = false;
    rejectBtn.disabled = false;
  }
}

// ─── Toast Notifications ────────────────────────────────────────

function showToast(message, type = "success") {
  const toast = document.getElementById("kyc-toast");
  if (!toast) return;

  toast.textContent = message;
  toast.style.background =
    type === "danger"
      ? "var(--admin-danger, #dc2626)"
      : type === "warning"
        ? "var(--admin-warning, #f59e0b)"
        : "var(--admin-success, #16a34a)";
  toast.style.display = "block";

  clearTimeout(toast._timeout);
  toast._timeout = setTimeout(() => {
    toast.style.display = "none";
  }, 4000);
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
    const resp = await fetch(`/api/admin/kyc/${encodeURIComponent(kycId)}/documents`);
    if (!resp.ok) throw new Error("Failed to fetch documents");

    const docs = await resp.json();
    if (!docs || docs.length === 0) {
      showToast("No documents found for this record.", "warning");
      return;
    }

    let viewer = document.getElementById("kyc-doc-viewer");
    if (!viewer) {
      viewer = document.createElement("div");
      viewer.id = "kyc-doc-viewer";
      viewer.setAttribute("role", "dialog");
      viewer.setAttribute("aria-modal", "true");
      viewer.setAttribute("aria-label", "KYC Document Viewer");
      viewer.className = "kyc-doc-viewer-overlay";
      viewer.innerHTML = `
        <button class="kyc-doc-viewer-close" aria-label="Close document viewer" id="kyc-doc-viewer-close">×</button>
        <div id="kyc-doc-content" class="kyc-doc-content-box"></div>
        <div id="kyc-doc-footer" class="kyc-doc-footer"></div>
      `;
      document.body.appendChild(viewer);
      document.getElementById("kyc-doc-viewer-close").addEventListener("click", () => {
        viewer.style.display = "none";
      });
      viewer.addEventListener("keydown", (e) => {
        if (e.key === "Escape") viewer.style.display = "none";
      });
      viewer.addEventListener("click", (e) => {
        if (e.target === viewer) viewer.style.display = "none";
      });
    }

    const content = document.getElementById("kyc-doc-content");
    const footer = document.getElementById("kyc-doc-footer");

    content.innerHTML = docs
      .map((d) => {
        const safeUrl = esc(d.url);
        if (d.url.toLowerCase().includes(".pdf") || d.document_type === "pdf") {
          return `<embed src="${safeUrl}" type="application/pdf" width="800px" height="600px" />`;
        }
        return `<img src="${safeUrl}" class="kyc-doc-img" alt="KYC Document" />`;
      })
      .join("<hr class='kyc-doc-sep'>");

    footer.textContent = `Viewing ${docs.length} document(s) for KYC ${kycId}`;
    viewer.style.display = "flex";
  } catch (err) {
    console.error("Error viewing documents:", err);
    if (typeof Sentry !== "undefined") Sentry.captureException(err);
    showToast("Could not load documents. Please try again.", "danger");
  }
}

window.viewKYCDocuments = viewKYCDocuments;
