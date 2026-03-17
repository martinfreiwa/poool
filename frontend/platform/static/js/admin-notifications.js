/**
 * Admin Notifications Center JS
 */
let allNotifs = [];
let filteredNotifs = [];
let currentPage = 1;
const PAGE_SIZE = 15;
let sortField = "created_at";
let sortOrder = "desc";

document.addEventListener("DOMContentLoaded", () => {
  loadNotifications();
  document
    .getElementById("notif-search")
    ?.addEventListener("input", debounce(applyFilters, 200));
  document
    .getElementById("filter-type")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("filter-read")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("broadcast-send-btn")
    ?.addEventListener("click", sendBroadcast);

  setupSorting();
  setupPagination();
});

async function loadNotifications() {
  try {
    const r = await fetch("/api/admin/notifications");
    if (r.ok) {
      const d = await r.json();
      allNotifs = d.notifications || d;
    } else {
      console.error('Notifications API error:', r.status);
    }
  } catch (e) {
    console.error('Notifications fetch failed:', e);
    if (window.Sentry) Sentry.captureException(e);
  }
  updateStats();
  applyFilters();
}

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
      applyFilters();
    });
  });
}

function setupPagination() {
  document.getElementById("prev-page")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      renderTable();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    const totalPages = Math.ceil(filteredNotifs.length / PAGE_SIZE);
    if (currentPage < totalPages) {
      currentPage++;
      renderTable();
    }
  });
}

function applyFilters() {
  const search = (
    document.getElementById("notif-search")?.value || ""
  ).toLowerCase();
  const type = document.getElementById("filter-type")?.value || "";
  const read = document.getElementById("filter-read")?.value;

  let result = allNotifs;
  if (type) result = result.filter((n) => n.type === type);
  if (read === "true") result = result.filter((n) => n.is_read);
  else if (read === "false") result = result.filter((n) => !n.is_read);
  if (search)
    result = result.filter((n) =>
      `${n.title} ${n.message} ${n.user_email} ${n.user_name || ""}`
        .toLowerCase()
        .includes(search),
    );

  // Sort
  result.sort((a, b) => {
    let valA = a[sortField];
    let valB = b[sortField];
    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredNotifs = result;
  currentPage = 1;
  const notifCountEl = document.getElementById("notif-count-label");
  if (notifCountEl)
    notifCountEl.textContent = `${filteredNotifs.length} notifications`;
  renderTable();
}

function updateStats() {
  const total = allNotifs.length;
  const unread = allNotifs.filter((n) => !n.is_read).length;
  const today = allNotifs.filter((n) => {
    const d = new Date(n.created_at);
    const t = new Date();
    return d.toDateString() === t.toDateString();
  }).length;
  const statTotal = document.getElementById("stat-total");
  const statUnread = document.getElementById("stat-unread");
  const statReadRate = document.getElementById("stat-read-rate");
  const statToday = document.getElementById("stat-today");
  if (statTotal) statTotal.textContent = total;
  if (statUnread) statUnread.textContent = unread;
  if (statReadRate)
    statReadRate.textContent =
      total > 0 ? Math.round(((total - unread) / total) * 100) + "%" : "—";
  if (statToday) statToday.textContent = today;
}

function renderTable() {
  const tbody = document.getElementById("notif-table-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(filteredNotifs.length / PAGE_SIZE));
  currentPage = Math.min(currentPage, totalPages);
  const start = (currentPage - 1) * PAGE_SIZE;
  const slice = filteredNotifs.slice(start, start + PAGE_SIZE);

  if (!slice.length) {
    tbody.innerHTML =
      '<tr><td colspan="6" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No notifications found.</td></tr>';
    return;
  }

  // Pagination UI
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${filteredNotifs.length} total)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;

  tbody.innerHTML = slice
    .map(
      (n) => `
        <tr style="${!n.is_read ? "background:var(--admin-accent-bg);" : ""}">
            <td><div class="admin-user-inline"><div><div class="admin-user-inline-name">${esc(n.user_name || "")}</div><div class="admin-user-inline-email">${esc(n.user_email || "")}</div></div></div></td>
            <td>${typeBadge(n.type)}</td>
            <td><div style="font-weight:${n.is_read ? "400" : "600"};color:var(--admin-text-primary);margin-bottom:2px;">${esc(n.title)}</div><div style="font-size:11px;color:var(--admin-text-muted);max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(n.message || "")}</div></td>
            <td>${n.is_read ? '<span class="admin-badge admin-badge--neutral">Read</span>' : '<span class="admin-badge admin-badge--warning">Unread</span>'}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${fmtDate(n.created_at)}</td>
        </tr>
    `,
    )
    .join("");
}

async function sendBroadcast() {
  const type = document.getElementById("broadcast-type").value;
  const title = document.getElementById("broadcast-title").value.trim();
  const message = document.getElementById("broadcast-message").value.trim();
  if (!title) {
    alert("Please enter a notification title.");
    return;
  }
  try {
    const r = await fetch("/api/admin/notifications/broadcast", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ type, title, message }),
    });
    if (r.ok) {
      document.getElementById("broadcast-title").value = "";
      document.getElementById("broadcast-message").value = "";
      loadNotifications();
      return;
    } else {
      const err = await r.json();
      alert(err.error || "Failed to send broadcast");
    }
  } catch (e) {
    alert("Network error sending broadcast");
  }
}

function typeBadge(t) {
  const m = {
    system: ["admin-badge--neutral", "System"],
    kyc: ["admin-badge--warning", "KYC"],
    investment: ["admin-badge--info", "Investment"],
    payout: ["admin-badge--success", "Payout"],
    promo: ["admin-badge--info", "Promo"],
  };
  const [c, l] = m[t] || ["admin-badge--neutral", t];
  return `<span class="admin-badge ${c}">${l}</span>`;
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
    hour: "2-digit",
    minute: "2-digit",
  });
}
function debounce(fn, ms) {
  let t;
  return function (...a) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, a), ms);
  };
}
