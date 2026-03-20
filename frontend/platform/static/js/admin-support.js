// State Management
let allTickets = [];
let totalCount = 0;
let currentPage = 1;
const PAGE_SIZE = 15;
let sortField = "created_at";
let sortOrder = "desc";
let selectedTicketIds = new Set();
let lastCheckedIndex = null; // For Shift+Click batch selection

document.addEventListener("DOMContentLoaded", () => {
  loadTickets();

  // Filters
  document.getElementById("ticket-search")?.addEventListener(
    "input",
    debounce(() => {
      currentPage = 1;
      loadTickets();
    }, 400),
  );
  document.getElementById("filter-status")?.addEventListener("change", () => {
    currentPage = 1;
    loadTickets();
  });
  document.getElementById("filter-priority")?.addEventListener("change", () => {
    currentPage = 1;
    loadTickets();
  });
  document.getElementById("filter-date")?.addEventListener("change", () => {
    currentPage = 1;
    loadTickets();
  });

  // Pagination
  document
    .getElementById("support-prev-page")
    ?.addEventListener("click", () => {
      if (currentPage > 1) {
        currentPage--;
        loadTickets();
      }
    });
  document
    .getElementById("support-next-page")
    ?.addEventListener("click", () => {
      const totalPages = Math.ceil(totalCount / PAGE_SIZE);
      if (currentPage < totalPages) {
        currentPage++;
        loadTickets();
      }
    });

  setupSorting();
  setupBulkActions();
  setupExport();
  setupKeyboard();
  setupModal();
});

function setupSorting() {
  const table = document.querySelector(".admin-table");
  if (!table) return;
  table.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    // Handle keyboard activation for a11y
    th.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); th.click(); }
    });
    th.addEventListener("click", () => {
      const field = th.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      // Update UI sort indicators
      table
        .querySelectorAll("th[data-sort]")
        .forEach((el) => {
          el.textContent = el.textContent.replace(/ ↑| ↓/g, "");
          el.setAttribute("aria-sort", "none");
        });
      th.textContent += sortOrder === "asc" ? " ↑" : " ↓";
      th.setAttribute("aria-sort", sortOrder === "asc" ? "ascending" : "descending");
      loadTickets();
    });
  });
}

function setupExport() {
  document.getElementById("btn-export-csv")?.addEventListener("click", exportTicketsCsv);
}

function setupKeyboard() {
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && selectedTicketIds.size > 0) {
      selectedTicketIds.clear();
      updateBulkUI();
      renderTable();
    }
  });
}

function setupBulkActions() {
  const selectAllCheckbox = document.getElementById("select-all-tickets");
  if (selectAllCheckbox) {
    selectAllCheckbox.addEventListener("change", (e) => {
      const isChecked = e.target.checked;
      const checkboxes = document.querySelectorAll(".ticket-checkbox");
      checkboxes.forEach((cb) => {
        cb.checked = isChecked;
        if (isChecked) selectedTicketIds.add(cb.value);
        else selectedTicketIds.delete(cb.value);
      });
      updateBulkUI();
    });
  }

  const btnApplyBulk = document.getElementById("btn-apply-bulk");
  if (btnApplyBulk) {
    btnApplyBulk.addEventListener("click", applyBulkActions);
  }
}

function setupModal() {
  const modal = document.getElementById("ticket-modal");
  const closeBtn = document.getElementById("ticket-modal-close");

  if (closeBtn) {
    closeBtn.addEventListener("click", closeModal);
  }

  // Close on backdrop click
  if (modal) {
    modal.addEventListener("click", (e) => {
      if (e.target === modal) closeModal();
    });

    // Close on Escape key + focus trap
    modal.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        closeModal();
        return;
      }

      // Focus trap
      if (e.key === "Tab") {
        const focusable = modal.querySelectorAll(
          'button, select, input, [tabindex]:not([tabindex="-1"])',
        );
        if (focusable.length === 0) return;

        const first = focusable[0];
        const last = focusable[focusable.length - 1];

        if (e.shiftKey) {
          if (document.activeElement === first) {
            e.preventDefault();
            last.focus();
          }
        } else {
          if (document.activeElement === last) {
            e.preventDefault();
            first.focus();
          }
        }
      }
    });
  }
}

let _modalTrigger = null;

function openModal() {
  const modal = document.getElementById("ticket-modal");
  if (!modal) return;
  _modalTrigger = document.activeElement;
  modal.style.display = "flex";
  // Focus the first interactive element
  const firstFocusable = modal.querySelector("select, button, input");
  if (firstFocusable) {
    setTimeout(() => firstFocusable.focus(), 50);
  }
}

function closeModal() {
  const modal = document.getElementById("ticket-modal");
  if (!modal) return;
  modal.style.display = "none";
  // Return focus to trigger element
  if (_modalTrigger && _modalTrigger.focus) {
    _modalTrigger.focus();
  }
  _modalTrigger = null;
}

function updateBulkUI() {
  const container = document.getElementById("bulk-actions-container");
  const label = document.getElementById("bulk-count-label");
  const selectAllCheckbox = document.getElementById("select-all-tickets");

  if (!container || !label) return;

  if (selectedTicketIds.size > 0) {
    container.style.display = "block";
    label.textContent = `${selectedTicketIds.size} selected`;
  } else {
    container.style.display = "none";
    if (selectAllCheckbox) selectAllCheckbox.checked = false;
  }
}

async function loadTickets() {
  showLoading();

  const search = document.getElementById("ticket-search")?.value || "";
  const status = document.getElementById("filter-status")?.value || "";
  const priority = document.getElementById("filter-priority")?.value || "";
  const dateFilter = document.getElementById("filter-date")?.value || "";

  const params = new URLSearchParams({
    page: currentPage,
    limit: PAGE_SIZE,
    sort_field: sortField,
    sort_order: sortOrder,
  });

  if (search) params.append("search", search);
  if (status) params.append("status", status);
  if (priority) params.append("priority", priority);
  if (dateFilter) params.append("date_filter", dateFilter);

  try {
    const resp = await fetch(`/api/admin/support?${params.toString()}`);
    if (resp.ok) {
      const d = await resp.json();
      allTickets = d.tickets || [];
      totalCount = d.total_count || 0;
      updateStats(d.stats);
      renderTable();
    } else {
      showError();
    }
  } catch (e) {
    showError();
  }
}

function showLoading() {
  const tbody = document.getElementById("tickets-table-body");
  if (!tbody) return;
  tbody.innerHTML = `
        <tr>
            <td colspan="8" class="support-loading-cell">
                <div class="support-skeleton-group">
                    <div class="admin-skeleton admin-skeleton--text"></div>
                    <div class="admin-skeleton admin-skeleton--text" style="width: 80%;"></div>
                    <div class="admin-skeleton admin-skeleton--text" style="width: 90%;"></div>
                </div>
            </td>
        </tr>`;
}

function showError() {
  const tbody = document.getElementById("tickets-table-body");
  if (!tbody) return;
  tbody.innerHTML = `
    <tr>
      <td colspan="8" style="text-align:center;padding:40px;">
        <div style="color:var(--admin-danger);margin-bottom:12px;">Failed to load tickets.</div>
        <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="loadTickets()">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/></svg>
          Retry
        </button>
      </td>
    </tr>`;
}

function updateStats(stats) {
  if (!stats) return;
  const elOpen = document.getElementById("stat-open");
  const elProgress = document.getElementById("stat-progress");
  const elResolved = document.getElementById("stat-resolved");
  const elUrgent = document.getElementById("stat-urgent");
  if (elOpen) elOpen.textContent = stats.open || 0;
  if (elProgress) elProgress.textContent = stats.in_progress || 0;
  if (elResolved) elResolved.textContent = stats.resolved || 0;
  if (elUrgent) elUrgent.textContent = stats.urgent || 0;
}

function renderTable() {
  const tbody = document.getElementById("tickets-table-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(totalCount / PAGE_SIZE));

  // Update Pagination UI
  const info = document.getElementById("support-pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${totalCount} total)`;
  const ticketCountEl = document.getElementById("ticket-count-label");
  if (ticketCountEl) ticketCountEl.textContent = `${totalCount} tickets`;

  const prevBtn = document.getElementById("support-prev-page");
  const nextBtn = document.getElementById("support-next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;

  // Reset select all
  const selectAllCb = document.getElementById("select-all-tickets");
  if (selectAllCb) selectAllCb.checked = false;

  if (allTickets.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="8" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No tickets match filters.</td></tr>';
    return;
  }

  tbody.innerHTML = allTickets
    .map((t) => {
      const isChecked = selectedTicketIds.has(t.id) ? "checked" : "";
      return `
        <tr>
            <td><input type="checkbox" class="ticket-checkbox" value="${esc(t.id)}" style="accent-color:var(--admin-accent);" ${isChecked} aria-label="Select ticket ${esc(t.subject)}"></td>
            <td><div style="font-weight:600;color:var(--admin-text-primary);margin-bottom:2px;">${esc(t.subject)}</div><div style="font-size:11px;color:var(--admin-text-muted);max-width:250px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(t.message.substring(0, 80))}</div></td>
            <td><div class="admin-user-inline"><div><div class="admin-user-inline-name">${esc(t.user_name)}</div><div class="admin-user-inline-email">${esc(t.user_email)}</div></div></div></td>
            <td>${priorityBadge(t.priority)}</td>
            <td>${statusBadge(t.status)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${fmtDate(t.created_at)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${fmtDate(t.updated_at)}</td>
            <td><a class="admin-btn admin-btn--primary admin-btn--sm" href="/admin/support-ticket.html?id=${esc(t.id)}">
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true"><path d="M1 8s3-6 7-6 7 6 7 6-3 6-7 6-7-6-7-6z"/><circle cx="8" cy="8" r="2.5"/></svg> View</a></td>
        </tr>
    `;
    })
    .join("");

  // Attach event listeners to new checkboxes — supports Shift+Click batch selection
  const checkboxes = Array.from(document.querySelectorAll(".ticket-checkbox"));
  checkboxes.forEach((cb, index) => {
    cb.addEventListener("click", (e) => {
      if (e.shiftKey && lastCheckedIndex !== null) {
        const start = Math.min(lastCheckedIndex, index);
        const end = Math.max(lastCheckedIndex, index);
        for (let i = start; i <= end; i++) {
          checkboxes[i].checked = true;
          selectedTicketIds.add(checkboxes[i].value);
        }
      }
      lastCheckedIndex = index;
    });
    cb.addEventListener("change", (e) => {
      if (e.target.checked) selectedTicketIds.add(e.target.value);
      else selectedTicketIds.delete(e.target.value);
      updateBulkUI();
    });
  });
}

async function applyBulkActions() {
  if (selectedTicketIds.size === 0) return;

  const status = document.getElementById("bulk-status").value;
  const priority = document.getElementById("bulk-priority").value;

  if (!status && !priority) {
    showSupportToast("warning", "Please select a status or priority to apply.");
    return;
  }

  const btn = document.getElementById("btn-apply-bulk");
  btn.textContent = "Applying...";
  btn.disabled = true;

  try {
    const resp = await fetch("/api/admin/support/bulk", {
      method: "PATCH",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Token": getCsrfToken()
      },
      body: JSON.stringify({
        ticket_ids: Array.from(selectedTicketIds),
        status: status || null,
        priority: priority || null,
      }),
    });

    if (resp.ok) {
      const count = selectedTicketIds.size;
      selectedTicketIds.clear();
      lastCheckedIndex = null;
      document.getElementById("bulk-status").value = "";
      document.getElementById("bulk-priority").value = "";
      updateBulkUI();
      showSupportToast("success", `Updated ${count} ticket${count > 1 ? 's' : ''} successfully.`);
      loadTickets();
    } else {
      showSupportToast("error", "Failed to apply bulk actions. Please try again.");
    }
  } catch (e) {
    showSupportToast("error", "Network error during bulk action.");
  } finally {
    btn.textContent = "Apply to Selected";
    btn.disabled = false;
  }
}

// ── Utilities ──

function esc(s) {
  if (s == null) return "";
  const d = document.createElement("div");
  d.textContent = String(s);
  return d.innerHTML;
}

function fmtDate(iso) {
  if (!iso) return "—";
  return new Date(iso).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function debounce(fn, ms) {
  let t;
  return function (...a) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, a), ms);
  };
}

function showToast(message, type = "info") {
  const container = document.getElementById("admin-toast-container");
  if (!container) return;
  const toast = document.createElement("div");
  toast.className = `admin-toast admin-toast--${type}`;
  toast.textContent = message;
  toast.setAttribute("role", "alert");
  container.appendChild(toast);
  setTimeout(() => {
    toast.style.opacity = "0";
    toast.style.transition = "opacity 0.3s ease";
    setTimeout(() => toast.remove(), 300);
  }, 4000);
}

function statusBadge(s) {
  const m = {
    open: ["admin-badge--warning", "Open"],
    in_progress: ["admin-badge--info", "In Progress"],
    resolved: ["admin-badge--success", "Resolved"],
    closed: ["admin-badge--neutral", "Closed"],
  };
  const [c, l] = m[s] || ["admin-badge--neutral", s];
  return `<span class="admin-badge ${c}">${l}</span>`;
}

function priorityBadge(p) {
  const m = {
    urgent: ["admin-badge--danger", "Urgent"],
    high: ["admin-badge--warning", "High"],
    normal: ["admin-badge--neutral", "Normal"],
    low: ["admin-badge--info", "Low"],
  };
  const [c, l] = m[p] || ["admin-badge--neutral", p];
  return `<span class="admin-badge ${c}">${l}</span>`;
}

// ─── Toast Notification ───────────────────────────────────────────────────────
function showSupportToast(type, message) {
  if (window.showPooolToast) {
    window.showPooolToast(null, message, type);
    return;
  }
  let container = document.getElementById("admin-toast-container");
  if (!container) {
    container = document.createElement("div");
    container.id = "admin-toast-container";
    container.className = "admin-toast-container";
    container.setAttribute("aria-live", "polite");
    container.style.cssText = "position:fixed;top:24px;right:24px;z-index:10000;display:flex;flex-direction:column;gap:8px;";
    document.body.appendChild(container);
  }
  const colors = { success: "#059669", error: "#dc2626", warning: "#d97706", info: "#2563eb" };
  const toast = document.createElement("div");
  toast.style.cssText = `padding:12px 20px;border-radius:8px;background:${colors[type] || colors.info};color:#fff;font-size:13px;font-weight:600;box-shadow:0 4px 12px rgba(0,0,0,0.15);animation:admin-fadeIn 0.2s ease;`;
  toast.textContent = message;
  container.appendChild(toast);
  setTimeout(() => { toast.style.opacity = "0"; toast.style.transition = "opacity 0.3s"; }, 3500);
  setTimeout(() => toast.remove(), 4000);
}

// ─── CSV Export ───────────────────────────────────────────────────────────────
function exportTicketsCsv() {
  if (allTickets.length === 0) {
    showSupportToast("warning", "No tickets to export.");
    return;
  }
  const headers = ["ID", "Subject", "User", "Email", "Priority", "Status", "Created", "Updated"];
  const rows = allTickets.map(t => [
    t.id || "",
    (t.subject || "").replace(/"/g, '""'),
    (t.user_name || "").replace(/"/g, '""'),
    t.user_email || "",
    t.priority || "",
    t.status || "",
    t.created_at || "",
    t.updated_at || ""
  ]);
  const csvContent = [headers, ...rows].map(r => r.map(c => `"${c}"`).join(",")).join("\n");
  const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `support-tickets-${new Date().toISOString().split("T")[0]}.csv`;
  a.click();
  URL.revokeObjectURL(url);
  showSupportToast("success", `Exported ${allTickets.length} tickets.`);
}
