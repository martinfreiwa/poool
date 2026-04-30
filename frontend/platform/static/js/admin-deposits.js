// State Management
let allDeposits = [];
let filteredDeposits = [];
let currentPage = 1;
const PAGE_SIZE = 15;
let sortField = "created_at";
let sortOrder = "desc";
let confirmModalReturnFocus = null;

document.addEventListener("DOMContentLoaded", () => {
  loadDeposits();

  // Filters
  document
    .getElementById("deposit-search")
    ?.addEventListener("input", debounce(applyFilters, 250));
  ["filter-status", "filter-currency", "filter-provider"].forEach((id) => {
    document.getElementById(id)?.addEventListener("change", applyFilters);
  });

  // Pagination
  document.getElementById("prev-page")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      renderTable();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    const totalPages = Math.ceil(filteredDeposits.length / PAGE_SIZE);
    if (currentPage < totalPages) {
      currentPage++;
      renderTable();
    }
  });

  // Refresh
  document
    .getElementById("btn-refresh")
    ?.addEventListener("click", loadDeposits);

  // Sorting
  setupSorting();

  // Confirm modal
  document
    .getElementById("confirm-cancel")
    ?.addEventListener("click", closeConfirmModal);
  document.getElementById("confirm-modal")?.addEventListener("click", (e) => {
    if (e.target === e.currentTarget) closeConfirmModal();
  });
  document.addEventListener("keydown", (e) => {
    const modal = document.getElementById("confirm-modal");
    if (e.key === "Escape" && modal?.style.display === "flex") {
      closeConfirmModal();
    }
  });

  // Tabs
  document.querySelectorAll(".admin-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      document
        .querySelectorAll(".admin-tab")
        .forEach((t) => t.classList.remove("active"));
      document
        .querySelectorAll(".admin-tab-panel")
        .forEach((p) => (p.style.display = "none"));
      tab.classList.add("active");
      const panelId = `tab-${tab.dataset.tab}`;
      const panel = document.getElementById(panelId);
      if (panel) panel.style.display = "block";
      if (tab.dataset.tab === "disputes") loadDisputes();
    });
  });
});

function setupSorting() {
  const table = document.getElementById("deposits-table");
  if (!table) return;
  table.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    th.setAttribute("role", "button");
    th.setAttribute("tabindex", "0");
    th.setAttribute("aria-sort", getAriaSortValue(th.dataset.sort));
    th.addEventListener("click", () => {
      const field = th.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      applyFilters();
      updateSortIndicators();
    });
    th.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        th.click();
      }
    });
  });
}

function updateSortIndicators() {
  document.querySelectorAll("#deposits-table th[data-sort]").forEach((th) => {
    th.setAttribute("aria-sort", getAriaSortValue(th.dataset.sort));
  });
}

function getAriaSortValue(field) {
  if (field !== sortField) return "none";
  return sortOrder === "asc" ? "ascending" : "descending";
}

async function loadDeposits() {
  const btn = document.getElementById("btn-refresh");
  if (btn) btn.classList.add("admin-btn--loading");

  try {
    const resp = await fetch("/api/admin/deposits");
    if (resp.ok) {
      const data = await resp.json();
      allDeposits = data.deposits || data;
      applyFilters();
      updateStats(data.stats);
      if (btn) btn.classList.remove("admin-btn--loading");
    } else {
      allDeposits = [];
      filteredDeposits = [];
      updateStats();
      renderDepositError("Could not load deposit requests.");
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Could not load deposit requests.", "danger");
      if (btn) btn.classList.remove("admin-btn--loading");
    }
  } catch (e) {
    allDeposits = [];
    filteredDeposits = [];
    updateStats();
    renderDepositError("Network error loading deposit requests.");
    showToast("Network error loading deposit requests.", "danger");
    if (btn) btn.classList.remove("admin-btn--loading");
  }
}
function updateStats(stats) {
  // If the backend didn't return stats, compute from client-side data
  if (!stats) {
    stats = {
      pending_count: allDeposits.filter((d) => d.status === "pending").length,
      confirmed_24h: allDeposits.filter((d) => d.status === "paid").length,
      expired_count: allDeposits.filter((d) => d.status === "expired").length,
      pending_value_cents: allDeposits
        .filter((d) => d.status === "pending")
        .reduce((s, d) => s + (d.amount_cents || 0), 0),
      confirmed_24h_value_cents: allDeposits
        .filter((d) => d.status === "paid")
        .reduce((s, d) => s + (d.amount_cents || 0), 0),
      volume_30d_cents: allDeposits
        .filter((d) => d.status === "paid")
        .reduce((s, d) => s + (d.amount_cents || 0), 0),
      volume_30d_count: allDeposits.filter((d) => d.status === "paid").length,
    };
  }

  // Pending
  const pendingEl = document.getElementById("stat-pending");
  const pendingSubEl = document.getElementById("stat-pending-value");
  if (pendingEl) pendingEl.textContent = stats.pending_count ?? 0;
  if (pendingSubEl)
    pendingSubEl.textContent = stats.pending_value_cents
      ? `${formatUSD(stats.pending_value_cents)} awaiting`
      : "Awaiting confirmation";

  // Confirmed (24h)
  const confirmedEl = document.getElementById("stat-confirmed");
  const confirmedSubEl = document.getElementById("stat-confirmed-value");
  if (confirmedEl) confirmedEl.textContent = stats.confirmed_24h ?? 0;
  if (confirmedSubEl)
    confirmedSubEl.textContent = `${formatUSD(stats.confirmed_24h_value_cents || 0)} total`;

  // Expired
  const expiredEl = document.getElementById("stat-expired");
  if (expiredEl) expiredEl.textContent = stats.expired_count ?? 0;

  // Total Volume (30d)
  const volumeEl = document.getElementById("stat-volume");
  const volumeCountEl = document.getElementById("stat-volume-count");
  if (volumeEl) volumeEl.textContent = formatUSD(stats.volume_30d_cents || 0);
  if (volumeCountEl)
    volumeCountEl.textContent = `${stats.volume_30d_count || 0} deposits`;
}

function applyFilters() {
  const search = (
    document.getElementById("deposit-search")?.value || ""
  ).toLowerCase();
  const status = document.getElementById("filter-status")?.value || "";
  const currency = document.getElementById("filter-currency")?.value || "";
  const provider = document.getElementById("filter-provider")?.value || "";

  let result = allDeposits.filter((d) => {
    if (status && d.status !== status) return false;
    if (currency && d.currency !== currency) return false;
    if (provider && d.provider !== provider) return false;
    if (search) {
      const hay =
        `${d.user_name} ${d.user_email} ${d.external_ref_id || ""}`.toLowerCase();
      if (!hay.includes(search)) return false;
    }
    return true;
  });

  // Sort Result
  result.sort((a, b) => {
    let valA = a[sortField];
    let valB = b[sortField];
    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredDeposits = result;
  currentPage = 1;
  const depositCountEl = document.getElementById("deposit-count-label");
  if (depositCountEl)
    depositCountEl.textContent = `Showing ${filteredDeposits.length} deposits`;
  renderTable();
}

// ─── Render ─────────────────────────────────────────────────────

function renderTable() {
  const tbody = document.getElementById("deposits-table-body");
  if (!tbody) return;

  const totalPages = Math.max(
    1,
    Math.ceil(filteredDeposits.length / PAGE_SIZE),
  );
  currentPage = Math.min(currentPage, totalPages);
  const start = (currentPage - 1) * PAGE_SIZE;
  const slice = filteredDeposits.slice(start, start + PAGE_SIZE);

  if (slice.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="9" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No deposits match your filters.</td></tr>';
    updatePagination(totalPages);
    return;
  }

  updatePagination(totalPages);

  tbody.innerHTML = slice
    .map(
      (d) => `
            <tr data-id="${esc(d.id)}">
                <td>
                    <div class="admin-user-inline">
                        <div>
                            <div class="admin-user-inline-name">${esc(d.user_name)}</div>
                            <div class="admin-user-inline-email">${esc(d.user_email)}</div>
                        </div>
                    </div>
                </td>
                <td style="font-weight:700;font-variant-numeric:tabular-nums;font-size:14px;">${formatAmount(d.amount_cents, d.currency)}</td>
                <td><span class="admin-badge admin-badge--neutral">${esc(d.currency)}</span></td>
                <td>${getProviderBadge(d.provider)}</td>
                <td style="font-family:monospace;font-size:11px;color:var(--admin-text-muted);max-width:120px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(d.external_ref_id || "—")}</td>
                <td>${getStatusBadge(d.status)}</td>
                <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${d.expires_at ? formatDateTime(d.expires_at) : "—"}</td>
                <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDateTime(d.created_at)}</td>
                <td>
                    ${d.status === "pending"
          ? `
                        <div style="display:flex;gap:4px;">
                            <button class="admin-btn admin-btn--primary admin-btn--sm" onclick="openConfirmModal('${esc(d.id)}')" title="Confirm deposit">
                                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 8l3.5 3.5L13 4"/></svg>
                                Confirm
                            </button>
                            <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="extendDeposit('${esc(d.id)}')" title="Extend expiry by 48 hours" style="color:var(--admin-info);">
                                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><circle cx="8" cy="8" r="6.5"/><path d="M8 4.5v4l2.5 1.5"/></svg>
                            </button>
                            <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="cancelDeposit('${esc(d.id)}')" title="Cancel deposit" style="color:var(--admin-danger);">
                                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg>
                            </button>
                        </div>
                    `
          : `<span style="font-size:12px;color:var(--admin-text-muted);">—</span>`
        }
                </td>
            </tr>
        `,
    )
    .join("");
}

function updatePagination(totalPages) {
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${filteredDeposits.length} total)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;
}

function renderDepositError(message) {
  const tbody = document.getElementById("deposits-table-body");
  if (!tbody) return;
  const depositCountEl = document.getElementById("deposit-count-label");
  if (depositCountEl) depositCountEl.textContent = "Showing 0 deposits";
  tbody.innerHTML = `<tr><td colspan="9" role="alert" style="text-align:center;padding:40px;color:var(--admin-danger);">${esc(message)}</td></tr>`;
  updatePagination(1);
}

// ─── Confirm Modal ──────────────────────────────────────────────

let confirmingDepositId = null;

function openConfirmModal(depositId) {
  confirmingDepositId = depositId;
  confirmModalReturnFocus = document.activeElement;
  const dep = allDeposits.find((d) => d.id === depositId);
  if (!dep) return;
  document.getElementById("confirm-modal-text").textContent =
    `Confirm deposit of ${formatAmount(dep.amount_cents, dep.currency)} from ${dep.user_name} (${dep.user_email})? This will credit their wallet.`;
  document.getElementById("confirm-notes").value = "";
  const modal = document.getElementById("confirm-modal");
  modal.style.display = "flex";
  document.getElementById("confirm-notes")?.focus();

  document.getElementById("confirm-submit").onclick = () => {
    confirmDeposit(confirmingDepositId);
    closeConfirmModal();
  };
}

function closeConfirmModal() {
  document.getElementById("confirm-modal").style.display = "none";
  confirmingDepositId = null;
  if (confirmModalReturnFocus && typeof confirmModalReturnFocus.focus === "function") {
    confirmModalReturnFocus.focus();
  }
  confirmModalReturnFocus = null;
}

async function confirmDeposit(depositId) {
  const notes = document.getElementById("confirm-notes")?.value || "";
  try {
    const resp = await fetch(`/api/admin/deposits/${depositId}/confirm`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ notes }),
    });
    if (resp.ok) {
      loadDeposits();
      showToast("Deposit successfully confirmed!", "success");
      return;
    } else {
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Failed to confirm deposit.", "danger");
    }
  } catch (e) {
    showToast("Network error confirming deposit.", "danger");
  }
}

async function cancelDeposit(depositId) {
  const dep = allDeposits.find((d) => d.id === depositId);
  const label = dep
    ? `${dep.user_name} (${formatAmount(dep.amount_cents, dep.currency)})`
    : depositId;
  const confirmed = await confirmAction({
    title: "Cancel deposit",
    message: `Cancel the pending deposit from ${label}? This does not credit the wallet.`,
    confirmText: "Cancel deposit",
    type: "danger",
  });
  if (!confirmed) return;

  try {
    const resp = await fetch(`/api/admin/deposits/${depositId}/cancel`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ reason: "Admin cancelled from deposits board" }),
    });

    if (resp.ok) {
      loadDeposits();
      showToast("Deposit request cancelled.", "success");
    } else {
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Failed to cancel deposit.", "danger");
    }
  } catch (e) {
    showToast("Network error canceling deposit.", "danger");
  }
}

async function extendDeposit(depositId) {
  const dep = allDeposits.find((d) => d.id === depositId);
  const label = dep
    ? `${dep.user_name} (${formatAmount(dep.amount_cents, dep.currency)})`
    : depositId;
  if (!await confirmAction({ title: 'Extend deposit expiry', message: `Extend expiry by 48 hours for deposit from ${label}?`, confirmText: 'Extend', type: 'warning' })) return;
  try {
    const resp = await fetch(`/api/admin/deposits/${depositId}/extend`, {
      method: "POST",
    });
    if (resp.ok) {
      loadDeposits();
      showToast("Deposit expiry extended by 48 hours.", "success");
    } else {
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Failed to extend deposit.", "danger");
    }
  } catch (e) {
    showToast("Network error extending deposit.", "danger");
  }
}

async function loadDisputes() {
  const tbody = document.getElementById("disputes-table-body");
  if (!tbody) return;
  tbody.innerHTML =
    '<tr><td colspan="7" style="text-align:center;padding:40px;color:var(--admin-text-muted);">Loading disputes…</td></tr>';
  try {
    const resp = await fetch("/api/admin/disputes");
    if (resp.ok) {
      const data = await resp.json();
      const disputes = Array.isArray(data) ? data : data.disputes || [];
      const countEl = document.getElementById("dispute-count");
      if (countEl) countEl.textContent = `${disputes.length} active`;
      if (disputes.length === 0) {
        tbody.innerHTML =
          '<tr><td colspan="7" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No active disputes found.</td></tr>';
      } else {
        tbody.innerHTML = disputes
          .map(
            (d) => `
                    <tr>
	                        <td>
	                            <div class="admin-user-inline">
	                                <div>
	                                    <div class="admin-user-inline-name">${esc(d.user_email)}</div>
	                                </div>
	                            </div>
	                        </td>
	                        <td>${getProviderBadge(d.provider)}</td>
	                        <td style="font-weight:700;">${formatAmount(d.amount_cents, d.currency)}</td>
	                        <td>${getStatusBadge(d.status)}</td>
	                        <td style="font-size:12px;color:var(--admin-text-muted);">${formatDateTime(d.created_at)}</td>
	                        <td>
	                            ${renderEvidenceBundleControl(d)}
	                        </td>
	                        <td style="display:flex;gap:6px;align-items:center;">
	                            ${renderDisputeStatusSelect(d)}
	                            <button class="admin-btn admin-btn--primary admin-btn--sm" onclick="resolveDispute('${esc(d.id)}')">Save</button>
	                        </td>
	                    </tr>
	                `,
          )
          .join("");
      }
    } else {
        const err = await resp.json().catch(() => ({}));
        tbody.innerHTML = `<tr><td colspan="7" role="alert" style="text-align:center;padding:40px;color:var(--admin-danger);">${esc(err.error || "Could not load disputes.")}</td></tr>`;
        showToast(err.error || "Could not load disputes.", "danger");
    }
  } catch (e) {
    console.error("Error loading disputes", e);
    if (typeof Sentry !== 'undefined') Sentry.captureException(e);
    tbody.innerHTML =
      '<tr><td colspan="7" role="alert" style="text-align:center;padding:40px;color:var(--admin-danger);">Network error loading disputes.</td></tr>';
    showToast("Network error loading disputes.", "danger");
  }
}

function renderDisputeStatusSelect(dispute) {
  const id = esc(dispute.id);
  const current = dispute.status || "under_review";
  const statuses = ["under_review", "resolved", "escalated", "won", "lost"];
  return `<select id="dispute-status-${id}" class="admin-select" aria-label="Update dispute status">${statuses
    .map((status) => `<option value="${esc(status)}"${status === current ? " selected" : ""}>${esc(status.replace("_", " "))}</option>`)
    .join("")}</select>`;
}

function renderEvidenceBundleControl(dispute) {
  const evidenceUrl = dispute.evidence_url || "";
  if (isSafeEvidenceUrl(evidenceUrl)) {
    return `<a href="${esc(evidenceUrl)}" target="_blank" rel="noopener" class="admin-btn admin-btn--secondary admin-btn--sm">View Bundle</a>`;
  }
  return `<button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="buildEvidenceBundle('${esc(dispute.id)}')">Build Bundle</button>`;
}

function isSafeEvidenceUrl(url) {
  return url.startsWith("/api/admin/disputes/") || url.startsWith("https://") || url.startsWith("http://");
}

async function buildEvidenceBundle(disputeId) {
  try {
    const resp = await fetch(`/api/admin/disputes/${disputeId}/evidence`, {
      method: "POST",
      headers: { "X-CSRF-Token": getCsrfToken() },
    });
    if (resp.ok) {
      showToast("Evidence bundle generated.", "success");
      loadDisputes();
    } else {
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Failed to generate evidence bundle.", "danger");
    }
  } catch (e) {
    showToast("Network error generating evidence bundle.", "danger");
  }
}

async function resolveDispute(disputeId) {
  const newStatus = document.getElementById(`dispute-status-${disputeId}`)?.value;
  if (!newStatus) return;
  try {
    const resp = await fetch(`/api/admin/disputes/${disputeId}/status`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status: newStatus }),
    });
    if (resp.ok) {
      showToast("Dispute status updated.", "success");
      loadDisputes();
    } else {
      const err = await resp.json().catch(() => ({}));
      showToast(err.error || "Failed to update dispute status.", "danger");
    }
  } catch (e) {
    showToast("Update failed.", "danger");
  }
}

function showToast(msg, type = "success") {
  if(window.showPooolToast) {
    window.showPooolToast(null, msg, type);
  }
}

async function confirmAction(options) {
  if (typeof window.pooolConfirm === "function") return window.pooolConfirm(options);
  return window.confirm(options.message || options.title || "Continue?");
}

// ─── Helpers ────────────────────────────────────────────────────

function esc(str) {
  if (typeof str !== "string") return str || "";
  const d = document.createElement("div");
  d.textContent = str;
  return d.innerHTML;
}

function formatUSD(cents) {
  if (typeof cents !== "number") return "$0.00";
  return (
    "$" +
    (Math.abs(cents) / 100).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
}

function formatAmount(cents, currency) {
  const abs = Math.abs(cents || 0);
  if (currency === "IDR") {
    return "Rp " + Math.round(abs / 100).toLocaleString("id-ID");
  }
  return (
    "$" +
    (abs / 100).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
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

function getStatusBadge(status) {
  const map = {
    pending: ["admin-badge--warning", "Pending"],
    processing: ["admin-badge--info", "Processing"],
    completed: ["admin-badge--success", "Confirmed"],
    paid: ["admin-badge--success", "Confirmed"],
    failed: ["admin-badge--danger", "Failed"],
    cancelled: ["admin-badge--danger", "Cancelled"],
    expired: ["admin-badge--danger", "Expired"],
  };
  const [cls, label] = map[status] || ["admin-badge--neutral", status];
  return `<span class="admin-badge ${cls}"><span class="admin-badge-dot"></span>${esc(label)}</span>`;
}

function getProviderBadge(provider) {
  const map = {
    stripe: ["admin-badge--info", "Stripe"],
    ocbc: ["admin-badge--warning", "OCBC"],
    midtrans: ["admin-badge--warning", "Midtrans"],
    mangopay: ["admin-badge--info", "Mangopay"],
    manual: ["admin-badge--neutral", "Manual"],
  };
  const [cls, label] = map[provider] || ["admin-badge--neutral", provider];
  return `<span class="admin-badge ${cls}"><span class="admin-badge-dot"></span>${esc(label)}</span>`;
}

function debounce(fn, ms) {
  let t;
  return function (...args) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, args), ms);
  };
}
