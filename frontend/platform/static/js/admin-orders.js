/**
 * Admin Orders & Investments JS
 * Lists orders and investments with filters.
 */

// State
let allOrders = [];
let filteredOrders = [];
let allInvestments = [];
let filteredInvestments = [];

let orderPage = 1;
let invPage = 1;
const PAGE_SIZE = 15;

let orderSortField = "created_at";
let orderSortOrder = "desc";
let invSortField = "purchased_at";
let invSortOrder = "desc";

document.addEventListener("DOMContentLoaded", () => {
  // Check for ID parameter from global search
  const urlParams = new URLSearchParams(window.location.search);
  const searchId = urlParams.get("id");
  if (searchId) {
    const searchInput = document.getElementById("order-search");
    if (searchInput) searchInput.value = searchId;
  }

  loadData();

  // Search with debounce
  document
    .getElementById("order-search")
    ?.addEventListener("input", debounce(applyOrderFilters, 200));
  document
    .getElementById("inv-search")
    ?.addEventListener("input", debounce(applyInvFilters, 200));

  // Filter change listeners
  document
    .getElementById("order-filter-status")
    ?.addEventListener("change", applyOrderFilters);
  document
    .getElementById("inv-filter-status")
    ?.addEventListener("change", applyInvFilters);

  setupTabSystem();
  setupSorting();
  setupPagination();
});

// ─── Tabs ───────────────────────────────────────────────────────

function setupTabSystem() {
  const tabs = document.querySelectorAll(".admin-tab");
  const panels = document.querySelectorAll(".admin-tab-panel");

  tabs.forEach((tab) => {
    tab.addEventListener("click", () => {
      tabs.forEach((t) => t.classList.remove("active"));
      tab.classList.add("active");

      const target = tab.dataset.tab;
      panels.forEach((p) => {
        p.style.display = p.id === `tab-${target}` ? "block" : "none";
      });
    });
  });
}

function setupSorting() {
  // Orders sorting
  const orderTable = document.querySelector("#tab-orders .admin-table");
  orderTable?.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    th.addEventListener("click", () => {
      const field = th.dataset.sort;
      if (orderSortField === field) {
        orderSortOrder = orderSortOrder === "asc" ? "desc" : "asc";
      } else {
        orderSortField = field;
        orderSortOrder = "asc";
      }
      applyOrderFilters();
    });
  });

  // Investments sorting
  const invTable = document.querySelector("#tab-investments .admin-table");
  invTable?.querySelectorAll("th[data-sort]").forEach((th) => {
    th.style.cursor = "pointer";
    th.addEventListener("click", () => {
      const field = th.dataset.sort;
      if (invSortField === field) {
        invSortOrder = invSortOrder === "asc" ? "desc" : "asc";
      } else {
        invSortField = field;
        invSortOrder = "asc";
      }
      applyInvFilters();
    });
  });
}

function setupPagination() {
  // Orders pagination
  document.getElementById("order-prev-page")?.addEventListener("click", () => {
    if (orderPage > 1) {
      orderPage--;
      renderOrders();
    }
  });
  document.getElementById("order-next-page")?.addEventListener("click", () => {
    const maxPage = Math.ceil(filteredOrders.length / PAGE_SIZE);
    if (orderPage < maxPage) {
      orderPage++;
      renderOrders();
    }
  });

  // Investments pagination
  document.getElementById("inv-prev-page")?.addEventListener("click", () => {
    if (invPage > 1) {
      invPage--;
      renderInvestments();
    }
  });
  document.getElementById("inv-next-page")?.addEventListener("click", () => {
    const maxPage = Math.ceil(filteredInvestments.length / PAGE_SIZE);
    if (invPage < maxPage) {
      invPage++;
      renderInvestments();
    }
  });
}

async function loadData() {
  try {
    const orderResp = await fetch("/api/admin/orders");
    const invResp = await fetch("/api/admin/investments");

    if (orderResp.ok) {
      const data = await orderResp.json();
      allOrders = Array.isArray(data) ? data : data.orders || [];
    }
    if (invResp.ok) {
      const data = await invResp.json();
      allInvestments = Array.isArray(data) ? data : data.investments || [];
    }
  } catch (e) {
    console.error('Failed to load orders/investments:', e);
    if (window.Sentry) Sentry.captureException(e);
  }

  applyOrderFilters(false);
  applyInvFilters(false);
  updateStats();
}

function applyOrderFilters(resetPage = true) {
  const search = (
    document.getElementById("order-search")?.value || ""
  ).toLowerCase();
  const status = document.getElementById("order-filter-status")?.value || "";

  let result = allOrders.filter((o) => {
    if (status && o.status !== status) return false;
    if (
      search &&
      !`${o.order_number} ${o.user_name} ${o.user_email} ${o.id}`
        .toLowerCase()
        .includes(search)
    )
      return false;
    return true;
  });

  // Sort
  result.sort((a, b) => {
    let valA = a[orderSortField];
    let valB = b[orderSortField];
    if (orderSortField === "user_name") {
      valA = `${a.user_name}`.toLowerCase();
      valB = `${b.user_name}`.toLowerCase();
    }
    if (valA < valB) return orderSortOrder === "asc" ? -1 : 1;
    if (valA > valB) return orderSortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredOrders = result;
  if (resetPage === true || resetPage instanceof Event) orderPage = 1;
  const orderCountEl = document.getElementById("order-count-label");
  if (orderCountEl)
    orderCountEl.textContent = `${filteredOrders.length} orders`;
  renderOrders();
}

function applyInvFilters(resetPage = true) {
  const search = (
    document.getElementById("inv-search")?.value || ""
  ).toLowerCase();
  const status = document.getElementById("inv-filter-status")?.value || "";

  let result = allInvestments.filter((i) => {
    if (status && i.status !== status) return false;
    if (
      search &&
      !`${i.user_name} ${i.user_email} ${i.asset_title}`
        .toLowerCase()
        .includes(search)
    )
      return false;
    return true;
  });

  // Sort
  result.sort((a, b) => {
    let valA = a[invSortField];
    let valB = b[invSortField];
    if (invSortField === "user_name") {
      valA = `${a.user_name}`.toLowerCase();
      valB = `${b.user_name}`.toLowerCase();
    }
    if (valA < valB) return invSortOrder === "asc" ? -1 : 1;
    if (valA > valB) return invSortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredInvestments = result;
  if (resetPage === true || resetPage instanceof Event) invPage = 1;
  const invCountEl = document.getElementById("inv-count-label");
  if (invCountEl)
    invCountEl.textContent = `${filteredInvestments.length} investments`;
  renderInvestments();
}

function updateStats() {
  const elStatOrders = document.getElementById("stat-total-orders");
  if (elStatOrders) elStatOrders.textContent = allOrders.length;
  const completedRevenue = allOrders
    .filter((o) => o.status === "completed")
    .reduce((s, o) => s + (o.total_cents || 0), 0);
  const elRevenue = document.getElementById("stat-revenue");
  if (elRevenue) elRevenue.textContent = formatUSD(completedRevenue);
  const elInv = document.getElementById("stat-investments");
  if (elInv)
    elInv.textContent = allInvestments.filter(
      (i) => i.status === "active" || i.status === "rented",
    ).length;
  const elPending = document.getElementById("stat-pending-orders");
  if (elPending)
    elPending.textContent = allOrders.filter(
      (o) => o.status === "pending",
    ).length;
}

// ─── Render: Orders ─────────────────────────────────────────────

function renderOrders() {
  const tbody = document.getElementById("orders-table-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(filteredOrders.length / PAGE_SIZE));
  orderPage = Math.min(orderPage, totalPages);
  const start = (orderPage - 1) * PAGE_SIZE;
  const slice = filteredOrders.slice(start, start + PAGE_SIZE);

  if (slice.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="8" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No orders found.</td></tr>';
    return;
  }

  // Pagination Info
  const orderPagInfo = document.getElementById("order-pagination-info");
  if (orderPagInfo)
    orderPagInfo.textContent = `Page ${orderPage} of ${totalPages} (${filteredOrders.length} total)`;
  const orderPrev = document.getElementById("order-prev-page");
  const orderNext = document.getElementById("order-next-page");
  if (orderPrev) orderPrev.disabled = orderPage <= 1;
  if (orderNext) orderNext.disabled = orderPage >= totalPages;

  tbody.innerHTML = slice
    .map(
      (o) => `
        <tr>
            <td style="font-family:monospace;font-size:12px;font-weight:600;color:var(--admin-accent);">${esc(o.order_number)}</td>
            <td>
                <div class="admin-user-inline">
                    <div>
                        <div class="admin-user-inline-name"><a href="/admin/user-details?id=${esc(o.user_id)}" class="admin-link">${esc(o.user_name)}</a></div>
                        <div class="admin-user-inline-email">${esc(o.user_email)}</div>
                    </div>
                </div>
            </td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${o.item_count} item${o.item_count !== 1 ? "s" : ""}</td>
            <td style="font-weight:700;font-variant-numeric:tabular-nums;">${formatUSD(o.total_cents)}</td>
            <td>${getPaymentBadge(o.payment_method)}</td>
            <td>${getOrderStatusBadge(o.status)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDate(o.created_at)}</td>
            <td>
                <div style="display:flex;gap:6px;">
                    <a href="/admin/user-details?id=${esc(o.user_id)}" class="admin-btn admin-btn--secondary admin-btn--sm" title="View User Account">User</a>
                    ${o.status === "pending"
          ? `
                        <button onclick="approveOrder('${o.id}', '${esc(o.order_number)}')" class="admin-btn admin-btn--success admin-btn--sm" style="background:#12B76A;border-color:#12B76A;color:white;">Approve</button>
                        <button onclick="rejectOrder('${o.id}', '${esc(o.order_number)}')" class="admin-btn admin-btn--danger admin-btn--sm" style="background:#F04438;border-color:#F04438;color:white;">Reject</button>
                    `
          : ""
        }
                </div>
            </td>
        </tr>
    `,
    )
    .join("");
}

async function approveOrder(id, num) {
  if (
    !confirm(
      `Are you sure you want to APPROVE Order ${num}? This will confirm payment and activate the user's investments.`,
    )
  )
    return;
  try {
    const res = await fetch(`/api/admin/orders/${id}/approve`, {
      method: "POST",
    });
    const data = await res.json();
    if (res.ok) {
      showNotification(
        "Order Approved",
        `Order ${num} has been successfully completed.`,
        "success",
      );
      loadData();
    } else {
      showNotification(
        "Approval Failed",
        data.error || "Unknown error",
        "error",
      );
    }
  } catch (e) {
    showNotification("Network Error", "Failed to reach server.", "error");
  }
}

async function rejectOrder(id, num) {
  if (
    !confirm(
      `Are you sure you want to REJECT Order ${num}? This will FAIL the order and return the reserved tokens to availability.`,
    )
  )
    return;
  try {
    const res = await fetch(`/api/admin/orders/${id}/reject`, {
      method: "POST",
    });
    const data = await res.json();
    if (res.ok) {
      showNotification(
        "Order Rejected",
        `Order ${num} has been failed and tokens returned.`,
        "success",
      );
      loadData();
    } else {
      showNotification(
        "Rejection Failed",
        data.error || "Unknown error",
        "error",
      );
    }
  } catch (e) {
    showNotification("Network Error", "Failed to reach server.", "error");
  }
}

function showNotification(title, message, type = "success") {
  // Basic fallback if global notify isn't present
  if (window.showToast) {
    window.showToast(message, type);
  } else {
    alert(`${title}\n\n${message}`);
  }
}

// ─── Render: Investments ────────────────────────────────────────

function renderInvestments() {
  const tbody = document.getElementById("investments-table-body");
  if (!tbody) return;

  const totalPages = Math.max(
    1,
    Math.ceil(filteredInvestments.length / PAGE_SIZE),
  );
  invPage = Math.min(invPage, totalPages);
  const start = (invPage - 1) * PAGE_SIZE;
  const slice = filteredInvestments.slice(start, start + PAGE_SIZE);

  if (slice.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="8" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No investments found.</td></tr>';
    return;
  }

  // Pagination Info
  const invPagInfo = document.getElementById("inv-pagination-info");
  if (invPagInfo)
    invPagInfo.textContent = `Page ${invPage} of ${totalPages} (${filteredInvestments.length} total)`;
  const invPrev = document.getElementById("inv-prev-page");
  const invNext = document.getElementById("inv-next-page");
  if (invPrev) invPrev.disabled = invPage <= 1;
  if (invNext) invNext.disabled = invPage >= totalPages;

  tbody.innerHTML = slice
    .map((inv) => {
      const change = inv.current_value_cents - inv.purchase_value_cents;
      const changePct =
        inv.purchase_value_cents > 0
          ? ((change / inv.purchase_value_cents) * 100).toFixed(1)
          : "0.0";
      const changeColor =
        change >= 0 ? "var(--admin-success)" : "var(--admin-danger)";

      return `
        <tr>
            <td>
                <div class="admin-user-inline">
                    <div>
                        <div class="admin-user-inline-name"><a href="/admin/user-details?id=${esc(inv.user_id)}" class="admin-link">${esc(inv.user_name)}</a></div>
                        <div class="admin-user-inline-email">${esc(inv.user_email)}</div>
                    </div>
                </div>
            </td>
            <td style="font-weight:600;color:var(--admin-text-primary);">${esc(inv.asset_title)}</td>
            <td style="font-variant-numeric:tabular-nums;">${(inv.tokens_owned || 0).toLocaleString()}</td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(inv.purchase_value_cents)}</td>
            <td>
                <span style="font-variant-numeric:tabular-nums;font-weight:600;">${formatUSD(inv.current_value_cents)}</span>
                <span style="font-size:11px;color:${changeColor};margin-left:4px;">${change >= 0 ? "+" : ""}${changePct}%</span>
            </td>
            <td style="font-variant-numeric:tabular-nums;color:var(--admin-success);">${formatUSD(inv.total_rental_cents)}</td>
            <td>${getInvStatusBadge(inv.status)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDate(inv.purchased_at)}</td>
        </tr>
        `;
    })
    .join("");
}

// ─── Helpers ────────────────────────────────────────────────────

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
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
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

function getOrderStatusBadge(status) {
  const map = {
    pending: ["admin-badge--warning", "Pending"],
    processing: ["admin-badge--info", "Processing"],
    completed: ["admin-badge--success", "Completed"],
    failed: ["admin-badge--danger", "Failed"],
    cancelled: ["admin-badge--danger", "Cancelled"],
    refunded: ["admin-badge--neutral", "Refunded"],
  };
  const [cls, label] = map[status] || ["admin-badge--neutral", status];
  return `<span class="admin-badge ${cls}">${label}</span>`;
}

function getPaymentBadge(method) {
  const map = {
    wallet: ["admin-badge--info", "Wallet"],
    bank_transfer: ["admin-badge--neutral", "Bank"],
    card: ["admin-badge--neutral", "Card"],
  };
  const [cls, label] = map[method] || ["admin-badge--neutral", method || "—"];
  return `<span class="admin-badge ${cls}">${label}</span>`;
}

function getInvStatusBadge(status) {
  const map = {
    active: ["admin-badge--success", "Active"],
    funded: ["admin-badge--info", "Funded"],
    rented: ["admin-badge--success", "Rented"],
    payout_pending: ["admin-badge--warning", "Payout Pending"],
    in_process: ["admin-badge--info", "In Process"],
    funding_in_progress: ["admin-badge--warning", "Funding"],
    exited: ["admin-badge--neutral", "Exited"],
  };
  const [cls, label] = map[status] || ["admin-badge--neutral", status];
  return `<span class="admin-badge ${cls}">${label}</span>`;
}

function debounce(fn, ms) {
  let t;
  return function (...args) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, args), ms);
  };
}
