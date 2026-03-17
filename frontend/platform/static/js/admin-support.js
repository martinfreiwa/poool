// State Management
let allTickets = [];
let totalCount = 0;
let currentPage = 1;
const PAGE_SIZE = 15;
let sortField = "created_at";
let sortOrder = "desc";
let selectedTicketIds = new Set();

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
});

function setupSorting() {
  const table = document.querySelector(".admin-table");
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
      // Update UI sort indicators (optional)
      table
        .querySelectorAll("th[data-sort]")
        .forEach(
          (el) => (el.textContent = el.textContent.replace(/ ↑| ↓/g, "")),
        );
      th.textContent += sortOrder === "asc" ? " ↑" : " ↓";
      loadTickets();
    });
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
            <td colspan="8" style="padding: 20px;">
                <div style="display: flex; flex-direction: column; gap: 12px;">
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
  tbody.innerHTML =
    '<tr><td colspan="8" style="text-align:center;padding:40px;color:var(--admin-danger);">Failed to load tickets. Please try again.</td></tr>';
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
            <td><input type="checkbox" class="ticket-checkbox" value="${esc(t.id)}" style="accent-color:var(--admin-accent);" ${isChecked}></td>
            <td><div style="font-weight:600;color:var(--admin-text-primary);margin-bottom:2px;">${esc(t.subject)}</div><div style="font-size:11px;color:var(--admin-text-muted);max-width:250px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(t.message.substring(0, 80))}</div></td>
            <td><div class="admin-user-inline"><div><div class="admin-user-inline-name">${esc(t.user_name)}</div><div class="admin-user-inline-email">${esc(t.user_email)}</div></div></div></td>
            <td>${priorityBadge(t.priority)}</td>
            <td>${statusBadge(t.status)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${fmtDate(t.created_at)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${fmtDate(t.updated_at)}</td>
            <td><a class="admin-btn admin-btn--primary admin-btn--sm" href="/admin/support-ticket.html?id=${esc(t.id)}">
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M1 8s3-6 7-6 7 6 7 6-3 6-7 6-7-6-7-6z"/><circle cx="8" cy="8" r="2.5"/></svg> View</a></td>
        </tr>
    `;
    })
    .join("");

  // Attach event listeners to new checkboxes
  document.querySelectorAll(".ticket-checkbox").forEach((cb) => {
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
    alert("Please select a status or priority to apply.");
    return;
  }

  const btn = document.getElementById("btn-apply-bulk");
  btn.textContent = "Applying...";
  btn.disabled = true;

  try {
    const resp = await fetch("/api/admin/support/bulk", {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        ticket_ids: Array.from(selectedTicketIds),
        status: status || null,
        priority: priority || null,
      }),
    });

    if (resp.ok) {
      selectedTicketIds.clear();
      document.getElementById("bulk-status").value = "";
      document.getElementById("bulk-priority").value = "";
      updateBulkUI();
      loadTickets();
    } else {
      alert("Failed to apply bulk actions");
    }
  } catch (e) {
    alert("Network error during bulk action");
  } finally {
    btn.textContent = "Apply to Selected";
    btn.disabled = false;
  }
}

function esc(s) {
  if (typeof s !== "string") return s || "";
  const d = document.createElement("div");
  d.textContent = s;
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
