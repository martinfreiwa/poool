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

let orderRangeDays = ""; // "", "1", "7", "30", "90"
let orderStaleOnly = false;
let lastLoadedAt = null;
const STALE_THRESHOLD_MS = 7 * 24 * 60 * 60 * 1000; // 7 days

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
    ?.addEventListener("change", (e) => {
      syncChipsFromDropdown(e.target.value);
      applyOrderFilters();
    });
  document
    .getElementById("inv-filter-status")
    ?.addEventListener("change", applyInvFilters);

  document
    .getElementById("order-filter-range")
    ?.addEventListener("change", (e) => {
      orderRangeDays = e.target.value;
      applyOrderFilters();
    });

  setupTabSystem();
  setupSorting();
  setupPagination();
  setupQuickChips();
  setupToolbar();
  setupOrdersKitFeatures();
});

let ordersAutoRefresh = null;
function setupOrdersKitFeatures() {
  if (!window.AdminPageKit) return;
  AdminPageKit.injectScopedCss();
  AdminPageKit.wireKpiClicks((card) => {
    const status = card.dataset.filterStatus;
    if (!status) return;
    const sel = document.getElementById("order-filter-status");
    if (sel) {
      sel.value = status;
      syncChipsFromDropdown(status);
      applyOrderFilters();
    }
  });
  ordersAutoRefresh = AdminPageKit.setupAutoRefresh({
    refreshFn: () => loadData(),
    intervalMs: 60000,
  });
}

function renderOrdersActionRequired() {
  if (!window.AdminPageKit) return;
  const failed = allOrders.filter((o) => o.status === "failed");
  const stalePending = allOrders.filter((o) => {
    if (o.status !== "pending") return false;
    return AdminPageKit.ageSeconds(o.created_at) > 86400;
  });
  const items = [];
  const goToStatus = (status) => () => {
    const sel = document.getElementById("order-filter-status");
    if (sel) { sel.value = status; syncChipsFromDropdown(status); applyOrderFilters(); }
  };
  if (failed.length) items.push({ label: "Failed orders", count: failed.length, color: "var(--admin-danger, #C2410C)", onClick: goToStatus("failed") });
  if (stalePending.length) items.push({ label: "Pending >24h", count: stalePending.length, color: "var(--admin-warning)", onClick: goToStatus("pending") });
  AdminPageKit.renderActionRequired(items, "#action-required-banner");
}

function setupQuickChips() {
  document.querySelectorAll("#order-quick-chips .admin-chip").forEach((chip) => {
    chip.addEventListener("click", () => {
      const status = chip.dataset.chipStatus;
      const stale = chip.dataset.chipStale;
      // Reset all chip active states
      document
        .querySelectorAll("#order-quick-chips .admin-chip")
        .forEach((c) => c.classList.remove("is-active"));
      chip.classList.add("is-active");
      if (stale) {
        orderStaleOnly = true;
        document.getElementById("order-filter-status").value = "";
      } else {
        orderStaleOnly = false;
        document.getElementById("order-filter-status").value = status || "";
      }
      applyOrderFilters();
    });
  });
}

function syncChipsFromDropdown(status) {
  orderStaleOnly = false;
  document
    .querySelectorAll("#order-quick-chips .admin-chip")
    .forEach((c) => c.classList.remove("is-active"));
  const match = document.querySelector(
    `#order-quick-chips .admin-chip[data-chip-status="${status || ""}"]`,
  );
  (match || document.querySelector('#order-quick-chips .admin-chip[data-chip-status=""]'))
    ?.classList.add("is-active");
}

function setupToolbar() {
  document.getElementById("btn-refresh")?.addEventListener("click", () => {
    loadData();
  });
  document.getElementById("btn-export-csv")?.addEventListener("click", () => {
    exportOrdersCsv();
  });
}

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
  const refreshBtn = document.getElementById("btn-refresh");
  const lastUpdatedEl = document.getElementById("data-last-updated");
  if (refreshBtn) {
    refreshBtn.disabled = true;
    refreshBtn.setAttribute("aria-busy", "true");
  }
  let ordersOk = false;
  let invsOk = false;
  try {
    const orderResp = await fetch("/api/admin/orders");
    const invResp = await fetch("/api/admin/investments");

    if (orderResp.ok) {
      const data = await orderResp.json();
      allOrders = Array.isArray(data) ? data : data.orders || [];
      ordersOk = true;
    }
    if (invResp.ok) {
      const data = await invResp.json();
      allInvestments = Array.isArray(data) ? data : data.investments || [];
      invsOk = true;
    }
    if (ordersOk && invsOk) lastLoadedAt = new Date();
    if (ordersOk) {
      renderOrdersActionRequired();
      if (ordersAutoRefresh) ordersAutoRefresh.markFetched();
    }
  } catch (e) {
    console.error('Failed to load orders/investments:', e);
    if (window.Sentry) Sentry.captureException(e);
  } finally {
    if (refreshBtn) {
      refreshBtn.disabled = false;
      refreshBtn.removeAttribute("aria-busy");
    }
  }

  applyOrderFilters(false);
  applyInvFilters(false);
  updateStats();
  updateLastUpdated();
  renderActionRequired();

  // Surface load errors so KPIs aren't trusted blindly
  if (!ordersOk || !invsOk) {
    if (lastUpdatedEl) {
      lastUpdatedEl.textContent = "Failed to refresh — showing last known data";
      lastUpdatedEl.style.color = "var(--admin-danger)";
    }
  } else if (lastUpdatedEl) {
    lastUpdatedEl.style.color = "var(--admin-text-muted)";
  }
}

function updateLastUpdated() {
  const el = document.getElementById("data-last-updated");
  if (!el || !lastLoadedAt) return;
  const t = lastLoadedAt;
  el.textContent = `Updated ${t.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })}`;
  el.title = `Last loaded: ${t.toLocaleString()}`;
}
// Refresh the relative timestamp every 30s
setInterval(updateLastUpdated, 30000);

function applyOrderFilters(resetPage = true) {
  const search = (
    document.getElementById("order-search")?.value || ""
  ).toLowerCase();
  const status = document.getElementById("order-filter-status")?.value || "";
  const rangeDays = orderRangeDays ? parseInt(orderRangeDays, 10) : null;
  const rangeCutoff = rangeDays
    ? Date.now() - rangeDays * 24 * 60 * 60 * 1000
    : null;
  const now = Date.now();

  let result = allOrders.filter((o) => {
    if (status && o.status !== status) return false;
    if (
      search &&
      !`${o.order_number} ${o.user_name} ${o.user_email} ${o.id}`
        .toLowerCase()
        .includes(search)
    )
      return false;
    if (rangeCutoff && o.created_at) {
      const t = Date.parse(o.created_at);
      if (Number.isFinite(t) && t < rangeCutoff) return false;
    }
    if (orderStaleOnly) {
      const t = Date.parse(o.created_at || "");
      const age = Number.isFinite(t) ? now - t : 0;
      const isStuck = o.status === "failed" || o.status === "pending";
      if (!isStuck || age < STALE_THRESHOLD_MS) return false;
    }
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
  // Total Orders
  const elStatOrders = document.getElementById("stat-total-orders");
  if (elStatOrders) elStatOrders.textContent = allOrders.length;
  const failedCount = allOrders.filter((o) => o.status === "failed").length;
  const failRate = allOrders.length
    ? ((failedCount / allOrders.length) * 100).toFixed(0)
    : 0;
  const elTotalSub = document.getElementById("stat-total-orders-sub");
  if (elTotalSub) {
    if (failedCount > 0) {
      elTotalSub.textContent = `${failedCount} failed (${failRate}%)`;
      elTotalSub.style.color =
        failedCount / allOrders.length > 0.1
          ? "var(--admin-danger)"
          : "var(--admin-text-muted)";
    } else {
      elTotalSub.textContent = "0 failed";
    }
  }

  // Revenue (30d) — use rolling 30d window of completed orders
  const cutoff30d = Date.now() - 30 * 24 * 60 * 60 * 1000;
  const completedIn30d = allOrders.filter(
    (o) =>
      o.status === "completed" &&
      o.created_at &&
      Date.parse(o.created_at) >= cutoff30d,
  );
  const completedRevenue30d = completedIn30d.reduce(
    (s, o) => s + (o.total_cents || 0),
    0,
  );
  const elRevenue = document.getElementById("stat-revenue");
  if (elRevenue) elRevenue.textContent = formatUSD(completedRevenue30d);
  const elRevSub = document.getElementById("stat-revenue-sub");
  if (elRevSub) {
    elRevSub.textContent = `${completedIn30d.length} completed orders · ${concentrationLine(completedIn30d)}`;
  }

  // Active Investments
  const activeInvs = allInvestments.filter(
    (i) => i.status === "active" || i.status === "rented",
  );
  const elInv = document.getElementById("stat-investments");
  if (elInv) elInv.textContent = activeInvs.length;
  const elInvSub = document.getElementById("stat-investments-sub");
  if (elInvSub) {
    elInvSub.textContent = `Total ${allInvestments.length} (active+rented shown)`;
  }

  // Pending Orders — color only when > 0
  const pendingCount = allOrders.filter((o) => o.status === "pending").length;
  const elPending = document.getElementById("stat-pending-orders");
  if (elPending) {
    elPending.textContent = pendingCount;
    elPending.style.color =
      pendingCount > 0 ? "var(--admin-warning)" : "var(--admin-text-primary)";
  }
  const elPendSub = document.getElementById("stat-pending-orders-sub");
  if (elPendSub) {
    if (pendingCount === 0) {
      elPendSub.textContent = "Inbox empty";
      elPendSub.style.color = "var(--admin-success)";
    } else {
      const pendingTimes = allOrders
        .filter((o) => o.status === "pending")
        .map((o) => Date.parse(o.created_at || ""))
        .filter(Number.isFinite);
      if (pendingTimes.length === 0) {
        elPendSub.textContent = `${pendingCount} pending (age unknown)`;
        elPendSub.style.color = "var(--admin-text-muted)";
      } else {
        const oldestPending = Math.min(...pendingTimes);
        const ageDays = Math.floor((Date.now() - oldestPending) / 86400000);
        elPendSub.textContent = `Oldest: ${ageDays}d`;
        elPendSub.style.color =
          ageDays > 7 ? "var(--admin-danger)" : "var(--admin-text-muted)";
      }
    }
  }
}

function concentrationLine(orders) {
  if (!orders.length) return "no data";
  const total = orders.reduce((s, o) => s + (o.total_cents || 0), 0);
  if (!total) return "no revenue";
  const byUser = new Map();
  for (const o of orders) {
    const k = o.user_email || o.user_id || "?";
    byUser.set(k, (byUser.get(k) || 0) + (o.total_cents || 0));
  }
  const sorted = Array.from(byUser.entries()).sort((a, b) => b[1] - a[1]);
  const topPct = ((sorted[0][1] / total) * 100).toFixed(0);
  return `top customer ${topPct}%`;
}

// ─── Action Required Zone ──────────────────────────────────────

function renderActionRequired() {
  const zone = document.getElementById("action-required-zone");
  const list = document.getElementById("action-required-list");
  const updated = document.getElementById("action-required-updated");
  if (!zone || !list) return;

  const now = Date.now();
  const items = [];

  const pending = allOrders.filter((o) => o.status === "pending");
  if (pending.length > 0) {
    const stalePending = pending.filter((o) => {
      const t = Date.parse(o.created_at || "");
      return Number.isFinite(t) && now - t > STALE_THRESHOLD_MS;
    }).length;
    items.push(
      `${pending.length} pending order${pending.length === 1 ? "" : "s"} awaiting approval` +
        (stalePending > 0
          ? ` <span style="color:var(--admin-danger);font-weight:600;">(${stalePending} stale &gt;7d)</span>`
          : "") +
        ` · <a href="#" class="admin-link" data-action-jump="pending">Review</a>`,
    );
  }

  const failed = allOrders.filter((o) => o.status === "failed");
  if (failed.length > 0) {
    const staleFailed = failed.filter((o) => {
      const t = Date.parse(o.created_at || "");
      return Number.isFinite(t) && now - t > STALE_THRESHOLD_MS;
    }).length;
    const failTotal = failed.reduce((s, o) => s + (o.total_cents || 0), 0);
    items.push(
      `${failed.length} failed order${failed.length === 1 ? "" : "s"} (${formatUSD(failTotal)})` +
        (staleFailed > 0
          ? ` <span style="color:var(--admin-danger);font-weight:600;">(${staleFailed} unresolved &gt;7d)</span>`
          : "") +
        ` · <a href="#" class="admin-link" data-action-jump="failed">Investigate</a>`,
    );
  }

  // Repeat-fail user clusters (≥3 fails by same user in last 7d)
  const recentFails = failed.filter((o) => {
    const t = Date.parse(o.created_at || "");
    return Number.isFinite(t) && now - t < 7 * 86400000;
  });
  const failsByUser = new Map();
  for (const o of recentFails) {
    const k = o.user_email || o.user_id || "?";
    failsByUser.set(k, (failsByUser.get(k) || 0) + 1);
  }
  for (const [user, count] of failsByUser.entries()) {
    if (count >= 3) {
      items.push(
        `<strong>${esc(user)}</strong> has ${count} failures in 7d — review for fraud or KYC issue`,
      );
    }
  }

  // Concentration risk on completed-in-30d
  const cutoff30d = now - 30 * 86400000;
  const completed30d = allOrders.filter(
    (o) =>
      o.status === "completed" &&
      o.created_at &&
      Date.parse(o.created_at) >= cutoff30d,
  );
  if (completed30d.length >= 3) {
    const total = completed30d.reduce((s, o) => s + (o.total_cents || 0), 0);
    const byUser = new Map();
    for (const o of completed30d) {
      const k = o.user_email || o.user_id || "?";
      byUser.set(k, (byUser.get(k) || 0) + (o.total_cents || 0));
    }
    const top = Array.from(byUser.entries()).sort((a, b) => b[1] - a[1])[0];
    if (top && total > 0 && top[1] / total > 0.5) {
      const pct = ((top[1] / total) * 100).toFixed(0);
      items.push(
        `Concentration risk: <strong>${esc(top[0])}</strong> = ${pct}% of 30d revenue`,
      );
    }
  }

  if (items.length === 0) {
    zone.style.display = "none";
    return;
  }
  zone.style.display = "block";
  list.innerHTML = items.map((i) => `<li>${i}</li>`).join("");
  if (updated && lastLoadedAt) {
    updated.textContent = `as of ${lastLoadedAt.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
  }

  // Wire jump-links (re-applies filter chip)
  list.querySelectorAll("[data-action-jump]").forEach((a) => {
    a.addEventListener("click", (e) => {
      e.preventDefault();
      const target = a.dataset.actionJump;
      // Ensure the Orders tab is active before applying filter chip
      const ordersTab = document.querySelector('.admin-tab[data-tab="orders"]');
      if (ordersTab && !ordersTab.classList.contains("active")) {
        ordersTab.click();
      }
      const chip = document.querySelector(
        `#order-quick-chips .admin-chip[data-chip-status="${target}"]`,
      );
      chip?.click();
      document
        .getElementById("tab-orders")
        ?.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  });
}

function getAgingBadge(o) {
  if (o.status !== "failed" && o.status !== "pending") return "";
  const t = Date.parse(o.created_at || "");
  if (!Number.isFinite(t)) return "";
  const days = Math.floor((Date.now() - t) / 86400000);
  if (days < 1) return "";
  let cls = "admin-badge--neutral";
  let prefix = "";
  if (days >= 7) {
    cls = "admin-badge--danger";
    prefix = "Stale ";
  } else if (days >= 3) {
    cls = "admin-badge--warning";
  }
  return `<span class="admin-badge ${cls}" style="margin-left:6px;font-size:10px;" title="Order is ${days} day${days === 1 ? "" : "s"} old">${prefix}${days}d</span>`;
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
      '<tr><td colspan="8" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No orders match the current filters. <a href="#" class="admin-link" id="orders-clear-filters">Clear filters</a></td></tr>';
    document
      .getElementById("orders-clear-filters")
      ?.addEventListener("click", (e) => {
        e.preventDefault();
        clearOrderFilters();
      });
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
            <td><a href="#" onclick="openOrderDetail('${esc(o.id)}');return false;" style="font-family:monospace;font-size:12px;font-weight:600;color:var(--admin-accent);text-decoration:none;cursor:pointer;" onmouseover="this.style.textDecoration='underline'" onmouseout="this.style.textDecoration='none'">${esc(o.order_number)}</a></td>
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
            <td style="white-space:nowrap;">${getOrderStatusBadge(o.status)}${getAgingBadge(o)}</td>
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
    !await pooolConfirm({
      title: `Approve Order ${num}`,
      message: `This will confirm payment and activate the user's investments.`,
      confirmText: 'Approve',
      type: 'success',
    })
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
    !await pooolConfirm({
      title: `Reject Order ${num}`,
      message: `This will FAIL the order and return the reserved tokens to availability.`,
      confirmText: 'Reject',
      type: 'danger',
    })
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

function clearOrderFilters() {
  document.getElementById("order-search").value = "";
  document.getElementById("order-filter-status").value = "";
  document.getElementById("order-filter-range").value = "";
  orderRangeDays = "";
  orderStaleOnly = false;
  syncChipsFromDropdown("");
  applyOrderFilters();
}

function exportOrdersCsv() {
  if (!filteredOrders.length) {
    alert("No orders to export. Adjust filters.");
    return;
  }
  const cols = [
    "order_number",
    "user_name",
    "user_email",
    "item_count",
    "total_usd",
    "payment_method",
    "status",
    "created_at",
    "completed_at",
  ];
  const header = cols.join(",");
  const rows = filteredOrders.map((o) =>
    cols
      .map((c) => {
        // total_usd is derived from total_cents; everything else maps 1:1
        let v = c === "total_usd" ? (o.total_cents || 0) / 100 : o[c];
        if (v === null || v === undefined) v = "";
        const s = String(v).replace(/"/g, '""');
        return /[",\n]/.test(s) ? `"${s}"` : s;
      })
      .join(","),
  );
  const csv = [header, ...rows].join("\n");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  const ts = new Date().toISOString().slice(0, 19).replace(/[:T]/g, "-");
  a.href = url;
  a.download = `orders-${ts}.csv`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function debounce(fn, ms) {
  let t;
  return function (...args) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, args), ms);
  };
}

// ─── Order Detail Modal ─────────────────────────────────────────

async function openOrderDetail(orderId) {
  const overlay = document.getElementById('order-detail-overlay');
  const body = document.getElementById('order-modal-body');
  const title = document.getElementById('order-modal-title');
  const subtitle = document.getElementById('order-modal-subtitle');

  // Show loading
  overlay.classList.add('active');
  body.innerHTML = `
    <div style="text-align:center;padding:40px;color:var(--admin-text-muted);">
      <div style="margin:0 auto 12px;width:24px;height:24px;border:2px solid var(--admin-border);border-top-color:var(--admin-accent);border-radius:50%;animation:spin .8s linear infinite;"></div>
      Loading order details…
    </div>`;

  try {
    const resp = await fetch(`/api/admin/orders/${orderId}`);
    if (!resp.ok) throw new Error('Failed to load order');
    const data = await resp.json();
    renderOrderDetail(data, title, subtitle, body);
  } catch (e) {
    body.innerHTML = `<div style="text-align:center;padding:40px;color:var(--admin-danger);">Failed to load order details.</div>`;
  }
}

function closeOrderDetail() {
  document.getElementById('order-detail-overlay').classList.remove('active');
}

// Close on overlay click
document.addEventListener('click', (e) => {
  if (e.target.id === 'order-detail-overlay') closeOrderDetail();
});
// Close on Escape
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') closeOrderDetail();
});

function renderOrderDetail(data, titleEl, subtitleEl, bodyEl) {
  const o = data.order;
  const items = data.items || [];
  const invoice = data.invoice;
  const walletTxs = data.wallet_transactions || [];

  titleEl.textContent = `Order ${o.order_number}`;
  subtitleEl.textContent = `Created ${formatDate(o.created_at)} • ${o.user_name} (${o.user_email})`;

  let html = '';

  // ── Order summary section ──
  html += `
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px 24px;margin-bottom:24px;">
      <div>
        <div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;color:var(--admin-text-muted);margin-bottom:2px;">Status</div>
        <div>${getOrderStatusBadge(o.status)}</div>
      </div>
      <div>
        <div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;color:var(--admin-text-muted);margin-bottom:2px;">Total</div>
        <div style="font-size:18px;font-weight:700;font-variant-numeric:tabular-nums;">${formatUSD(o.total_cents)}</div>
      </div>
      <div>
        <div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;color:var(--admin-text-muted);margin-bottom:2px;">Payment Method</div>
        <div>${getPaymentBadge(o.payment_method)}</div>
      </div>
      <div>
        <div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;color:var(--admin-text-muted);margin-bottom:2px;">Currency</div>
        <div style="font-weight:600;">${esc(o.currency)}${o.payment_currency && o.payment_currency !== o.currency ? ` → ${esc(o.payment_currency)}` : ''}</div>
      </div>
      ${o.fx_rate ? `
      <div>
        <div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;color:var(--admin-text-muted);margin-bottom:2px;">FX Rate</div>
        <div style="font-weight:500;">${esc(o.fx_rate)}${o.fx_provider ? ` <span style="font-size:11px;color:var(--admin-text-muted);">(${esc(o.fx_provider)})</span>` : ''}</div>
      </div>` : ''}
      ${o.payment_ref_id ? `
      <div>
        <div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;color:var(--admin-text-muted);margin-bottom:2px;">Payment Ref</div>
        <div style="font-family:monospace;font-size:12px;word-break:break-all;">${esc(o.payment_ref_id)}</div>
      </div>` : ''}
      ${o.completed_at ? `
      <div>
        <div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;color:var(--admin-text-muted);margin-bottom:2px;">Completed</div>
        <div style="font-size:13px;">${formatDate(o.completed_at)}</div>
      </div>` : ''}
      <div>
        <div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;color:var(--admin-text-muted);margin-bottom:2px;">Customer</div>
        <div><a href="/admin/user-details?id=${esc(o.user_id)}" class="admin-link" style="font-weight:600;">${esc(o.user_name)}</a></div>
        <div style="font-size:12px;color:var(--admin-text-muted);">${esc(o.user_email)}</div>
      </div>
    </div>`;

  // ── Proof of Transfer ──
  if (o.proof_of_transfer_url) {
    html += `
    <div style="margin-bottom:20px;padding:12px 16px;border-radius:var(--admin-radius-sm);border:1px solid var(--admin-border);background:var(--admin-hover-overlay);">
      <div style="font-size:11px;text-transform:uppercase;letter-spacing:.5px;color:var(--admin-text-muted);margin-bottom:6px;">Proof of Transfer</div>
      <a href="${esc(o.proof_of_transfer_url)}" target="_blank" class="admin-link" style="font-size:13px;word-break:break-all;">View uploaded proof →</a>
    </div>`;
  }

  // ── Line Items table ──
  html += `
    <div style="margin-bottom:20px;">
      <h3 style="font-size:13px;font-weight:600;color:var(--admin-text-primary);margin:0 0 8px;text-transform:uppercase;letter-spacing:.5px;">Line Items (${items.length})</h3>
      <div style="border:1px solid var(--admin-border);border-radius:var(--admin-radius-sm);overflow:hidden;">
        <table class="admin-table" style="margin:0;">
          <thead>
            <tr>
              <th>Asset</th>
              <th style="text-align:right;">Qty</th>
              <th style="text-align:right;">Price/Token</th>
              <th style="text-align:right;">Subtotal</th>
            </tr>
          </thead>
          <tbody>`;

  if (items.length === 0) {
    html += `<tr><td colspan="4" style="text-align:center;padding:20px;color:var(--admin-text-muted);">No line items</td></tr>`;
  } else {
    for (const item of items) {
      html += `
            <tr>
              <td style="font-weight:600;color:var(--admin-text-primary);">
                <a href="/admin/asset-details?id=${esc(item.asset_id)}" class="admin-link">${esc(item.asset_title)}</a>
              </td>
              <td style="text-align:right;font-variant-numeric:tabular-nums;">${item.tokens_quantity.toLocaleString()}</td>
              <td style="text-align:right;font-variant-numeric:tabular-nums;">${formatUSD(item.token_price_cents)}</td>
              <td style="text-align:right;font-weight:700;font-variant-numeric:tabular-nums;">${formatUSD(item.subtotal_cents)}</td>
            </tr>`;
    }
  }

  html += `
          </tbody>
        </table>
      </div>
    </div>`;

  // ── Invoice section ──
  if (invoice) {
    html += `
    <div style="margin-bottom:20px;">
      <h3 style="font-size:13px;font-weight:600;color:var(--admin-text-primary);margin:0 0 8px;text-transform:uppercase;letter-spacing:.5px;">Invoice</h3>
      <div style="padding:14px 16px;border:1px solid var(--admin-border);border-radius:var(--admin-radius-sm);background:var(--admin-hover-overlay);">
        <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px 16px;">
          <div>
            <div style="font-size:11px;color:var(--admin-text-muted);text-transform:uppercase;letter-spacing:.4px;">Invoice #</div>
            <div style="font-weight:600;font-family:monospace;font-size:13px;">${esc(invoice.invoice_number)}</div>
          </div>
          <div>
            <div style="font-size:11px;color:var(--admin-text-muted);text-transform:uppercase;letter-spacing:.4px;">Status</div>
            <div><span class="admin-badge ${invoice.status === 'issued' ? 'admin-badge--success' : 'admin-badge--neutral'}">${esc(invoice.status)}</span></div>
          </div>
          <div>
            <div style="font-size:11px;color:var(--admin-text-muted);text-transform:uppercase;letter-spacing:.4px;">Total</div>
            <div style="font-weight:700;font-variant-numeric:tabular-nums;">${formatUSD(invoice.total_cents)}</div>
          </div>
        </div>
        ${invoice.pdf_url ? `<div style="margin-top:10px;"><a href="${esc(invoice.pdf_url)}" target="_blank" class="admin-link" style="font-size:13px;">📄 Download PDF</a></div>` : ''}
      </div>
    </div>`;
  }

  // ── Wallet Transactions ──
  if (walletTxs.length > 0) {
    html += `
    <div style="margin-bottom:12px;">
      <h3 style="font-size:13px;font-weight:600;color:var(--admin-text-primary);margin:0 0 8px;text-transform:uppercase;letter-spacing:.5px;">Wallet Transactions (${walletTxs.length})</h3>
      <div style="border:1px solid var(--admin-border);border-radius:var(--admin-radius-sm);overflow:hidden;">
        <table class="admin-table" style="margin:0;">
          <thead><tr><th>Type</th><th>Status</th><th style="text-align:right;">Amount</th><th>Description</th><th>Date</th></tr></thead>
          <tbody>`;
    for (const tx of walletTxs) {
      const txStatusMap = {
        completed: 'admin-badge--success',
        pending: 'admin-badge--warning',
        failed: 'admin-badge--danger',
      };
      html += `
            <tr>
              <td style="font-weight:600;text-transform:capitalize;">${esc(tx.type.replace('_', ' '))}</td>
              <td><span class="admin-badge ${txStatusMap[tx.status] || 'admin-badge--neutral'}">${esc(tx.status)}</span></td>
              <td style="text-align:right;font-variant-numeric:tabular-nums;font-weight:600;">${formatUSD(tx.amount_cents)}</td>
              <td style="font-size:12px;color:var(--admin-text-muted);max-width:160px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(tx.description || '—')}</td>
              <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDate(tx.created_at)}</td>
            </tr>`;
    }
    html += `
          </tbody>
        </table>
      </div>
    </div>`;
  }

  bodyEl.innerHTML = html;
}
