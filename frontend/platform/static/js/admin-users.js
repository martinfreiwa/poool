/**
 * Admin Users Page JS — Loads user list from API and handles filtering, search, pagination.
 */

let allUsers = [];
let filteredUsers = [];
let currentPage = 1;
const PAGE_SIZE = 20;
let sortField = "created_at";
let sortOrder = "desc"; // 'asc' or 'desc'

document.addEventListener("DOMContentLoaded", () => {
  loadUsers();

  // Search with debounce
  const searchInput = document.getElementById("user-search-input");
  if (searchInput) {
    let timeout;
    searchInput.addEventListener("input", () => {
      clearTimeout(timeout);
      timeout = setTimeout(applyFilters, 300);
    });
  }

  // Filter change listeners
  ["filter-role", "filter-kyc", "filter-status"].forEach((id) => {
    const el = document.getElementById(id);
    if (el) el.addEventListener("change", applyFilters);
  });

  // Pagination
  document.getElementById("prev-page")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      renderTable();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    const maxPage = Math.ceil(filteredUsers.length / PAGE_SIZE);
    if (currentPage < maxPage) {
      currentPage++;
      renderTable();
    }
  });

  // Select all
  document
    .getElementById("select-all-users")
    ?.addEventListener("change", (e) => {
      document
        .querySelectorAll(".user-checkbox")
        .forEach((cb) => (cb.checked = e.target.checked));
    });

  // Export CSV
  document
    .getElementById("export-users-btn")
    ?.addEventListener("click", exportCSV);

  // Sorting Listeners
  setupSorting();
});

function setupSorting() {
  const table = document.getElementById("users-table");
  if (!table) return;
  const headers = table.querySelectorAll("th[data-sort]");
  headers.forEach((th) => {
    th.style.cursor = "pointer";
    th.addEventListener("click", () => {
      const field = th.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      applyFilters();
    });
  });
}

async function loadUsers() {
  try {
    const resp = await fetch("/api/admin/users");
    if (resp.ok) {
      allUsers = await resp.json();
      applyFilters();
      updateStats();
    } else {
      console.error('Admin users API error:', resp.status);
    }
  } catch (e) {
    console.error('Admin users fetch failed:', e);
    if (window.Sentry) Sentry.captureException(e);
  }
}

function applyFilters() {
  const search = (
    document.getElementById("user-search-input")?.value || ""
  ).toLowerCase();
  const roleFilter = document.getElementById("filter-role")?.value || "";
  const kycFilter = document.getElementById("filter-kyc")?.value || "";
  const statusFilter = document.getElementById("filter-status")?.value || "";

  let result = allUsers.filter((u) => {
    // Search
    if (search) {
      const name = `${u.first_name || ""} ${u.last_name || ""}`.toLowerCase();
      const match =
        name.includes(search) ||
        u.email.toLowerCase().includes(search) ||
        u.id.toLowerCase().includes(search);
      if (!match) return false;
    }
    // Role
    if (roleFilter && !u.roles.includes(roleFilter)) return false;
    // KYC
    if (kycFilter) {
      if (kycFilter === "none" && u.kyc_status) return false;
      if (kycFilter !== "none" && u.kyc_status !== kycFilter) return false;
    }
    // Status
    if (statusFilter && u.status !== statusFilter) return false;
    return true;
  });

  // Sort Result
  result.sort((a, b) => {
    let valA = a[sortField];
    let valB = b[sortField];

    // Special handling for nested or calculated fields
    if (sortField === "name") {
      valA = `${a.first_name} ${a.last_name}`.toLowerCase();
      valB = `${b.first_name} ${b.last_name}`.toLowerCase();
    }

    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredUsers = result;
  currentPage = 1;
  renderTable();
}

function renderTable() {
  const tbody = document.getElementById("users-table-body");
  if (!tbody) return;

  const start = (currentPage - 1) * PAGE_SIZE;
  const pageUsers = filteredUsers.slice(start, start + PAGE_SIZE);
  const totalPages = Math.max(1, Math.ceil(filteredUsers.length / PAGE_SIZE));

  if (pageUsers.length === 0) {
    tbody.innerHTML = `<tr><td colspan="8" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No users found matching your filters.</td></tr>`;
  } else {
    tbody.innerHTML = pageUsers
      .map(
        (u) => `
      <tr>
        <td><input type="checkbox" class="user-checkbox" value="${u.id}" style="accent-color:var(--admin-accent);"></td>
        <td>
          <div class="admin-user-inline">
            <div style="width:32px;height:32px;border-radius:50%;background:${getAvatarColor(u.email)};display:flex;align-items:center;justify-content:center;font-size:13px;font-weight:700;color:#fff;flex-shrink:0;">
              ${getInitials(u.first_name, u.last_name)}
            </div>
            <div>
              <div class="admin-user-inline-name">${escapeHtml(u.first_name || "")} ${escapeHtml(u.last_name || "")}</div>
              <div class="admin-user-inline-email">${escapeHtml(u.email)}</div>
            </div>
          </div>
        </td>
        <td>${(u.roles || []).map((r) => `<span class="admin-badge ${getRoleBadgeClass(r)}" style="margin-right:4px;">${r}</span>`).join("")}</td>
        <td>${getKYCBadge(u.kyc_status)}</td>
        <td style="font-weight:600;font-variant-numeric:tabular-nums;">${formatUSD(u.balance_cents || 0)}</td>
        <td>${getStatusBadge(u.status)}</td>
        <td style="color:var(--admin-text-muted);font-size:12px;">${formatDate(u.created_at)}</td>
        <td>
          <a href="/admin/user-details.html?id=${u.id}" class="admin-btn admin-btn--secondary admin-btn--sm" title="View Details">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M1 8s3-5.5 7-5.5S15 8 15 8s-3 5.5-7 5.5S1 8 1 8z"/><circle cx="8" cy="8" r="2.5"/></svg>
          </a>
          <button class="admin-btn admin-btn--sm ${u.status === "suspended" ? "admin-btn--primary" : "admin-btn--secondary"}" 
                  onclick="toggleUserStatus('${u.id}', '${u.status}')" 
                  title="${u.status === "suspended" ? "Activate" : "Suspend"}">
            ${u.status === "suspended" ? "✓" : "✕"}
          </button>
        </td>
      </tr>
    `,
      )
      .join("");
  }

  // Update pagination
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages}(${filteredUsers.length} users)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;

  // Count label
  const countLabel = document.getElementById("user-count-label");
  if (countLabel)
    countLabel.textContent = `Showing ${pageUsers.length} of ${filteredUsers.length} users`;
}

async function toggleUserStatus(userId, currentStatus) {
  const newStatus = currentStatus === "suspended" ? "active" : "suspended";
  if (
    !await pooolConfirm({
      title: newStatus === 'active' ? 'Activate user' : 'Suspend user',
      message: newStatus === 'active'
        ? 'This user will regain access to the platform.'
        : 'This user will be immediately suspended and locked out.',
      confirmText: newStatus === 'active' ? 'Activate' : 'Suspend',
      type: newStatus === 'active' ? 'success' : 'danger',
    })
  )
    return;

  try {
    const resp = await fetch(`/api/admin/users/${userId}/status`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status: newStatus }),
    });
    if (resp.ok) {
      loadUsers();
    } else {
      alert("Failed to update status");
    }
  } catch (e) {
    alert("Error: " + e.message);
  }
}

function updateStats() {
  setTextById("stat-total", String(allUsers.length));
  setTextById(
    "stat-investors",
    String(allUsers.filter((u) => u.roles?.includes("investor")).length),
  );
  setTextById(
    "stat-developers",
    String(allUsers.filter((u) => u.roles?.includes("developer")).length),
  );
  setTextById(
    "stat-verified",
    String(allUsers.filter((u) => u.kyc_status === "approved").length),
  );
  setTextById(
    "stat-suspended",
    String(allUsers.filter((u) => u.status === "suspended").length),
  );
}

// ── Helpers ──

function getInitials(first, last) {
  return ((first || "?")[0] + (last || "?")[0]).toUpperCase();
}

function getAvatarColor(email) {
  let hash = 0;
  for (let i = 0; i < email.length; i++)
    hash = email.charCodeAt(i) + ((hash << 5) - hash);
  const colors = [
    "#6366F1",
    "#EC4899",
    "#F59E0B",
    "#10B981",
    "#3B82F6",
    "#8B5CF6",
    "#EF4444",
    "#06B6D4",
  ];
  return colors[Math.abs(hash) % colors.length];
}

function getRoleBadgeClass(role) {
  if (role === "admin") return "admin-badge--danger";
  if (role === "developer") return "admin-badge--info";
  return "admin-badge--neutral";
}

function getKYCBadge(status) {
  if (!status)
    return '<span class="admin-badge admin-badge--neutral"><span class="admin-badge-dot"></span>None</span>';
  if (status === "approved")
    return '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Verified</span>';
  if (status === "pending")
    return '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Pending</span>';
  if (status === "rejected")
    return '<span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>Rejected</span>';
  return `<span class="admin-badge admin-badge--neutral">${escapeHtml(status)}</span>`;
}

function getStatusBadge(status) {
  if (status === "active")
    return '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Active</span>';
  if (status === "suspended")
    return '<span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>Suspended</span>';
  if (status === "pending")
    return '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Pending</span>';
  return `<span class="admin-badge admin-badge--neutral">${escapeHtml(status || "Unknown")}</span>`;
}

function formatUSD(cents) {
  return (
    "$" +
    (cents / 100).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
}

function formatDate(isoString) {
  if (!isoString) return "—";
  const d = new Date(isoString);
  return d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}

function setTextById(id, text) {
  const el = document.getElementById(id);
  if (el) el.textContent = text;
}

function exportCSV() {
  const headers = [
    "ID",
    "Email",
    "First Name",
    "Last Name",
    "Roles",
    "KYC Status",
    "Balance (cents)",
    "Status",
    "Joined",
  ];
  const rows = filteredUsers.map((u) => [
    u.id,
    u.email,
    u.first_name || "",
    u.last_name || "",
    (u.roles || []).join(";"),
    u.kyc_status || "none",
    u.balance_cents || 0,
    u.status,
    u.created_at || "",
  ]);
  const csv = [headers, ...rows]
    .map((r) => r.map((c) => `"${String(c).replace(/"/g, '""')}"`).join(","))
    .join("\n");
  const blob = new Blob([csv], { type: "text/csv" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `poool-users-${new Date().toISOString().split("T")[0]}.csv`;
  a.click();
  URL.revokeObjectURL(url);
}
