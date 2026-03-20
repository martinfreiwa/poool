/**
 * Admin Users Page JS — Server-side paginated, filtered, and sorted user list.
 *
 * Production-hardened: XSS-safe rendering, error states, toast notifications,
 * CSV formula injection protection, aria-live announcements, sort indicators.
 */

let currentUsers = [];      // Current page's users (from server)
let totalCount = 0;         // Total users matching filters (from server)
let totalPages = 1;
let currentPage = 1;
const PAGE_SIZE = 20;
let sortField = "created_at";
let sortOrder = "desc";
let _searchTimeout;

// ── Escape HTML (string-only, no DOM allocation) ──
const ESC_MAP = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#x27;' };
function escapeHtml(str) {
  return String(str || "").replace(/[&<>"']/g, c => ESC_MAP[c]);
}

document.addEventListener("DOMContentLoaded", () => {
  loadUsers();

  // Search with debounce — sends to server
  const searchInput = document.getElementById("user-search-input");
  if (searchInput) {
    searchInput.addEventListener("input", () => {
      clearTimeout(_searchTimeout);
      _searchTimeout = setTimeout(() => {
        currentPage = 1;
        loadUsers();
      }, 400);
    });
  }

  // Filter change listeners — sends to server
  ["filter-role", "filter-kyc", "filter-status"].forEach((id) => {
    const el = document.getElementById(id);
    if (el) el.addEventListener("change", () => {
      currentPage = 1;
      loadUsers();
    });
  });

  // Pagination
  document.getElementById("prev-page")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      loadUsers();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    if (currentPage < totalPages) {
      currentPage++;
      loadUsers();
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
      currentPage = 1;
      loadUsers();
    });
  });
}

// ── Sort direction indicator ──
function updateSortIndicators() {
  document.querySelectorAll('#users-table th[data-sort]').forEach(th => {
    let arrow = th.querySelector('.sort-arrow');
    if (!arrow) {
      arrow = document.createElement('span');
      arrow.className = 'sort-arrow';
      arrow.style.marginLeft = '4px';
      arrow.style.opacity = '0.35';
      arrow.style.fontSize = '11px';
      th.appendChild(arrow);
    }
    if (th.dataset.sort === sortField) {
      arrow.textContent = sortOrder === 'asc' ? ' ↑' : ' ↓';
      arrow.style.opacity = '1';
    } else {
      arrow.textContent = ' ↕';
      arrow.style.opacity = '0.35';
    }
  });
}

/**
 * Load users from the API with current filters, search, sort, and pagination.
 */
async function loadUsers() {
  const tbody = document.getElementById("users-table-body");

  const search = (document.getElementById("user-search-input")?.value || "").trim();
  const roleFilter = document.getElementById("filter-role")?.value || "";
  const kycFilter = document.getElementById("filter-kyc")?.value || "";
  const statusFilter = document.getElementById("filter-status")?.value || "";

  const params = new URLSearchParams();
  params.set("page", String(currentPage));
  params.set("limit", String(PAGE_SIZE));
  if (search) params.set("search", search);
  if (roleFilter) params.set("role", roleFilter);
  if (kycFilter) params.set("kyc_status", kycFilter);
  if (statusFilter) params.set("status", statusFilter);
  params.set("sort_by", sortField);
  params.set("sort_dir", sortOrder);

  try {
    const resp = await fetch(`/api/admin/users?${params.toString()}`);
    if (resp.ok) {
      const result = await resp.json();
      currentUsers = result.data || [];
      totalCount = result.total_count || 0;
      totalPages = result.total_pages || 1;
      currentPage = result.page || 1;
      renderTable();
      updateStats();
    } else {
      if (window.Sentry) Sentry.captureMessage(`Admin users API error: ${resp.status}`, 'error');
      if (tbody) {
        tbody.innerHTML = `<tr><td colspan="8" style="text-align:center;padding:40px;">
          <div style="color:var(--admin-danger);font-weight:600;margin-bottom:8px;">Failed to load users</div>
          <div style="color:var(--admin-text-muted);font-size:12px;margin-bottom:12px;">Server returned status ${escapeHtml(String(resp.status))}</div>
          <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="loadUsers()">Retry</button>
        </td></tr>`;
      }
    }
  } catch (e) {
    if (window.Sentry) Sentry.captureException(e);
    if (tbody) {
      tbody.innerHTML = `<tr><td colspan="8" style="text-align:center;padding:40px;">
        <div style="color:var(--admin-danger);font-weight:600;margin-bottom:8px;">Failed to load users</div>
        <div style="color:var(--admin-text-muted);font-size:12px;margin-bottom:12px;">${escapeHtml(e.message)}</div>
        <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="loadUsers()">Retry</button>
      </td></tr>`;
    }
  }
}

function renderTable() {
  const tbody = document.getElementById("users-table-body");
  if (!tbody) return;

  // Check PII permission for balance masking
  const canViewPII = window.adminPermissions?.has('pii.view') !== false;

  if (currentUsers.length === 0) {
    tbody.innerHTML = `<tr><td colspan="8" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No users found matching your filters.</td></tr>`;
  } else {
    tbody.innerHTML = currentUsers
      .map(
        (u) => `
      <tr>
        <td><input type="checkbox" class="user-checkbox" value="${escapeHtml(u.id)}" style="accent-color:var(--admin-accent);"></td>
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
        <td>${(u.roles || []).map((r) => `<span class="admin-badge ${getRoleBadgeClass(r)}" style="margin-right:4px;">${escapeHtml(r)}</span>`).join("")}</td>
        <td>${getKYCBadge(u.kyc_status)}</td>
        <td style="font-weight:600;font-variant-numeric:tabular-nums;">${canViewPII ? formatUSD(u.balance_cents || 0) : '•••'}</td>
        <td>${getStatusBadge(u.status)}</td>
        <td style="color:var(--admin-text-muted);font-size:12px;">${formatDate(u.created_at)}</td>
        <td>
          <a href="/admin/user-details.html?id=${escapeHtml(u.id)}" class="admin-btn admin-btn--secondary admin-btn--sm" title="View Details">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M1 8s3-5.5 7-5.5S15 8 15 8s-3 5.5-7 5.5S1 8 1 8z"/><circle cx="8" cy="8" r="2.5"/></svg>
          </a>
          <button class="admin-btn admin-btn--sm ${u.status === "suspended" ? "admin-btn--primary" : "admin-btn--secondary"} toggle-status-btn"
                  data-user-id="${escapeHtml(u.id)}"
                  data-user-status="${escapeHtml(u.status)}"
                  title="${u.status === "suspended" ? "Activate" : "Suspend"}">
            ${u.status === "suspended" ? "✓" : "✕"}
          </button>
        </td>
      </tr>
    `,
      )
      .join("");

    // Bind toggle-status buttons via addEventListener (no inline onclick)
    tbody.querySelectorAll('.toggle-status-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        toggleUserStatus(btn.dataset.userId, btn.dataset.userStatus);
      });
    });
  }

  // Update pagination
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${totalCount} users)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;

  // Count label
  const countLabel = document.getElementById("user-count-label");
  if (countLabel)
    countLabel.textContent = `Showing ${currentUsers.length} of ${totalCount} users`;

  // Update sort direction indicators
  updateSortIndicators();

  // Announce to screen readers
  const announcer = document.getElementById('table-announcer');
  if (announcer) {
    announcer.textContent = `Showing ${currentUsers.length} of ${totalCount} users, page ${currentPage} of ${totalPages}`;
  }
}

async function toggleUserStatus(userId, currentStatus) {
  const newStatus = currentStatus === "suspended" ? "active" : "suspended";

  // Guard against pooolConfirm not yet loaded
  const confirmFn = typeof pooolConfirm === 'function'
    ? pooolConfirm
    : (opts) => Promise.resolve(window.confirm(opts.message));

  if (
    !await confirmFn({
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
    const resp = await fetch(`/api/admin/users/${encodeURIComponent(userId)}/status`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status: newStatus }),
    });
    if (resp.ok) {
      showToast(newStatus === 'active' ? 'User activated' : 'User suspended', 'success');
      loadUsers();
    } else {
      showToast("Failed to update user status", "error");
    }
  } catch (e) {
    if (window.Sentry) Sentry.captureException(e);
    showToast("Network error — please try again", "error");
  }
}

function updateStats() {
  // With server-side pagination, stats are computed from the current page data
  // For accurate counts, we could add a dedicated stats endpoint, but for now
  // we show the total from the server
  setTextById("stat-total", String(totalCount));

  // These are approximations from the current page — for exact counts a
  // dedicated /api/admin/users/stats endpoint would be needed
  const investors = currentUsers.filter((u) => u.roles?.includes("investor")).length;
  const developers = currentUsers.filter((u) => u.roles?.includes("developer")).length;
  const verified = currentUsers.filter((u) => u.kyc_status === "approved").length;
  const suspended = currentUsers.filter((u) => u.status === "suspended").length;

  setTextById("stat-investors", String(investors));
  setTextById("stat-developers", String(developers));
  setTextById("stat-verified", String(verified));
  setTextById("stat-suspended", String(suspended));
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

function setTextById(id, text) {
  const el = document.getElementById(id);
  if (el) el.textContent = text;
}

// ── Toast Notification System ──
function showToast(message, type = 'info') {
  const toast = document.createElement('div');
  toast.style.cssText = `
    position: fixed; bottom: 24px; right: 24px; z-index: 10000;
    padding: 12px 20px; border-radius: 8px; font-size: 13px; font-weight: 600;
    color: #fff; box-shadow: 0 4px 12px rgba(0,0,0,0.15);
    transform: translateY(20px); opacity: 0;
    transition: transform 0.25s ease, opacity 0.25s ease;
  `;
  toast.style.background = type === 'success' ? '#059669' : type === 'error' ? '#DC2626' : '#3B82F6';
  toast.textContent = message;
  document.body.appendChild(toast);
  // Trigger animation
  requestAnimationFrame(() => {
    toast.style.transform = 'translateY(0)';
    toast.style.opacity = '1';
  });
  setTimeout(() => {
    toast.style.transform = 'translateY(20px)';
    toast.style.opacity = '0';
    setTimeout(() => toast.remove(), 300);
  }, 3500);
}

// ── CSV Formula Injection Protection ──
function sanitizeCsvCell(value) {
  const str = String(value);
  if (/^[=+\-@\t\r]/.test(str)) return "'" + str;
  return str;
}

function exportCSV() {
  // Export current page only (for full export, a server-side endpoint would be needed)
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
  const rows = currentUsers.map((u) => [
    sanitizeCsvCell(u.id),
    sanitizeCsvCell(u.email),
    sanitizeCsvCell(u.first_name || ""),
    sanitizeCsvCell(u.last_name || ""),
    sanitizeCsvCell((u.roles || []).join(";")),
    sanitizeCsvCell(u.kyc_status || "none"),
    u.balance_cents || 0,
    sanitizeCsvCell(u.status),
    sanitizeCsvCell(u.created_at || ""),
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
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
