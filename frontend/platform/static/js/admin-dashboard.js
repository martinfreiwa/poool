/**
 * Admin Dashboard JS - loads KPI data and renders the dashboard safely.
 */

document.addEventListener("DOMContentLoaded", () => {
  const dateEl = document.getElementById("dashboard-date");
  if (dateEl) {
    const now = new Date();
    dateEl.textContent = now.toLocaleDateString("en-US", {
      weekday: "long",
      year: "numeric",
      month: "long",
      day: "numeric",
    });
  }

  const notificationButton = document.getElementById("admin-notification-button");
  if (notificationButton) {
    notificationButton.addEventListener("click", () => {
      window.location.href = "/admin/notifications.html";
    });
  }

  loadDashboardStats();
  loadSystemHealth();
  setActiveNav();

  const rangeSelector = document.getElementById("dashboard-range");
  if (rangeSelector) {
    rangeSelector.addEventListener("change", loadDashboardStats);
  }

  setInterval(loadDashboardStats, 30000);
  setInterval(loadSystemHealth, 60000);
});

async function loadDashboardStats() {
  try {
    const range = document.getElementById("dashboard-range")?.value || "30d";
    const resp = await fetch(`/api/admin/stats/overview?range=${encodeURIComponent(range)}`);
    if (!resp.ok) {
      renderDashboardError(`Dashboard stats unavailable (${resp.status}).`);
      return;
    }
    const data = await resp.json();
    populateKPIs(data);
  } catch (e) {
    renderDashboardError("Dashboard stats unavailable. Check your connection and retry.");
    if (window.Sentry) Sentry.captureException(e);
  }
}

function populateKPIs(data) {
  const label = data.range_label || "last 30 days";
  setTextById("kpi-total-users", formatNumber(data.total_users));
  setTextById("kpi-new-users", `+${data.new_users_range} ${label}`);
  setTextById("kpi-aum", formatUSD(data.aum_cents));
  setTextById("kpi-deposits-label", "Deposits");
  setTextById("kpi-deposits-24h", formatUSD(data.deposits_range_cents));
  setTextById("kpi-deposits-count", `${data.deposits_range_count} transactions - ${label}`);
  setTextById("kpi-pending-kyc", String(data.pending_kyc));
  setTextById("kpi-live-assets", String(data.live_assets));
  setTextById("kpi-funded-assets", `${data.funded_assets} fully funded`);
  setTextById("kpi-pending-deposits", String(data.pending_deposits));
  setTextById("kpi-open-tickets", String(data.open_tickets));
  setTextById("kpi-rewards-liability", formatUSD(data.rewards_liability_cents));

  setTextById("badge-kyc", String(data.pending_kyc));
  setTextById("badge-deposits", String(data.pending_deposits));
  setTextById("badge-support", String(data.open_tickets));
  updateNotificationBadge(data.unread_notifications);

  const activityFeed = document.getElementById("activity-feed");
  if (activityFeed && Array.isArray(data.recent_activity)) {
    if (data.recent_activity.length === 0) renderEmptyBlock(activityFeed, "No recent activity.");
    else renderActivityFeed(activityFeed, data.recent_activity);
  }

  const ordersTable = document.getElementById("recent-orders-table");
  if (ordersTable && Array.isArray(data.recent_orders)) {
    if (data.recent_orders.length === 0) renderEmptyRow(ordersTable, 4, "No recent orders.");
    else renderRecentOrders(ordersTable, data.recent_orders);
  }

  const depositsTable = document.getElementById("pending-deposits-table");
  if (depositsTable && Array.isArray(data.pending_deposits_list)) {
    if (data.pending_deposits_list.length === 0) renderEmptyRow(depositsTable, 4, "No pending deposits.");
    else renderPendingDeposits(depositsTable, data.pending_deposits_list);
  }

  if (data.user_trend) {
    renderSparkline("trend-users", data.user_trend, "var(--admin-accent)");
  }
  if (data.deposit_trend) {
    renderSparkline("trend-deposits", data.deposit_trend, "var(--admin-success)");
  }
}

function updateNotificationBadge(count) {
  const badge = document.getElementById("notification-count");
  if (!badge) return;
  const safeCount = Number.isFinite(Number(count)) ? Number(count) : 0;
  if (safeCount > 0) {
    badge.textContent = String(safeCount > 99 ? "99+" : safeCount);
    badge.style.display = "";
  } else {
    badge.textContent = "";
    badge.style.display = "none";
  }
}

function renderDashboardError(message) {
  [
    "kpi-total-users",
    "kpi-aum",
    "kpi-deposits-24h",
    "kpi-pending-kyc",
    "kpi-live-assets",
    "kpi-pending-deposits",
    "kpi-open-tickets",
    "kpi-rewards-liability",
  ].forEach((id) => setTextById(id, "Unavailable"));

  renderErrorBlock(document.getElementById("activity-feed"), message);
  renderErrorRow(document.getElementById("recent-orders-table"), 4, message);
  renderErrorRow(document.getElementById("pending-deposits-table"), 4, message);
}

function renderActivityFeed(container, activities) {
  container.replaceChildren();
  activities.forEach((act) => {
    const item = document.createElement("div");
    item.className = "admin-activity-item";

    const dot = document.createElement("div");
    dot.className = `admin-activity-dot admin-activity-dot--${getActivityType(act.action)}`;

    const content = document.createElement("div");
    content.className = "admin-activity-content";

    const text = document.createElement("div");
    text.className = "admin-activity-text";
    const action = document.createElement("strong");
    action.textContent = formatAction(String(act.action || "activity"));
    text.append(
      action,
      document.createTextNode(` - ${act.entity_type || "entity"} ID: ${act.entity_id || "N/A"}`),
    );

    const time = document.createElement("div");
    time.className = "admin-activity-time";
    time.textContent = fmtRelativeTime(act.created_at);

    content.append(text, time);
    item.append(dot, content);
    container.appendChild(item);
  });
}

function renderRecentOrders(table, orders) {
  table.replaceChildren();
  orders.forEach((order) => {
    const row = document.createElement("tr");

    const link = document.createElement("a");
    link.href = `/admin/orders.html?id=${encodeURIComponent(order.order_number || "")}`;
    link.className = "admin-link";
    link.textContent = order.order_number || "Order";
    appendCell(row, link);

    const user = document.createElement("div");
    user.className = "admin-user-inline";
    const name = document.createElement("span");
    name.className = "admin-user-inline-name";
    name.textContent = order.user_email || "Unknown user";
    user.appendChild(name);
    appendCell(row, user);

    appendCell(row, formatUSD(order.total_cents));
    appendCell(row, buildStatusBadge(order.status));
    table.appendChild(row);
  });
}

function renderPendingDeposits(table, deposits) {
  table.replaceChildren();
  deposits.forEach((deposit) => {
    const row = document.createElement("tr");

    const user = document.createElement("div");
    user.className = "admin-user-inline";
    const name = document.createElement("span");
    name.className = "admin-user-inline-name";
    name.textContent = deposit.user_email || "Unknown user";
    user.appendChild(name);
    appendCell(row, user);

    appendCell(row, formatUSD(deposit.amount_cents));

    const provider = document.createElement("span");
    provider.className = "admin-badge admin-badge--neutral";
    provider.textContent = deposit.provider || "unknown";
    appendCell(row, provider);

    const review = document.createElement("a");
    review.href = "/admin/deposits.html";
    review.className = "admin-btn admin-btn--primary admin-btn--sm";
    review.textContent = "Review";
    appendCell(row, review);
    table.appendChild(row);
  });
}

function appendCell(row, content) {
  const cell = document.createElement("td");
  if (content instanceof Node) cell.appendChild(content);
  else cell.textContent = String(content ?? "");
  row.appendChild(cell);
}

function buildStatusBadge(status) {
  const badge = document.createElement("span");
  badge.className = `admin-badge admin-badge--${getStatusClass(status)}`;
  const dot = document.createElement("span");
  dot.className = "admin-badge-dot";
  badge.append(dot, document.createTextNode(status || "unknown"));
  return badge;
}

function renderEmptyBlock(container, message) {
  if (!container) return;
  container.replaceChildren();
  const block = document.createElement("div");
  block.style.cssText = "text-align:center;padding:20px;color:var(--admin-text-muted);";
  block.textContent = message;
  container.appendChild(block);
}

function renderErrorBlock(container, message) {
  if (!container) return;
  container.replaceChildren();
  const block = document.createElement("div");
  block.setAttribute("role", "alert");
  block.style.cssText = "text-align:center;padding:20px;color:var(--admin-danger);";
  const text = document.createElement("div");
  text.textContent = message;
  const retry = buildRetryButton();
  block.append(text, retry);
  container.appendChild(block);
}

function renderEmptyRow(table, colspan, message) {
  if (!table) return;
  table.replaceChildren();
  const row = document.createElement("tr");
  const cell = document.createElement("td");
  cell.colSpan = colspan;
  cell.style.cssText = "text-align:center;padding:20px;";
  cell.textContent = message;
  row.appendChild(cell);
  table.appendChild(row);
}

function renderErrorRow(table, colspan, message) {
  if (!table) return;
  table.replaceChildren();
  const row = document.createElement("tr");
  const cell = document.createElement("td");
  cell.colSpan = colspan;
  cell.setAttribute("role", "alert");
  cell.style.cssText = "text-align:center;padding:20px;color:var(--admin-danger);";
  const text = document.createElement("div");
  text.textContent = message;
  const retry = buildRetryButton();
  cell.append(text, retry);
  row.appendChild(cell);
  table.appendChild(row);
}

function buildRetryButton() {
  const retry = document.createElement("button");
  retry.type = "button";
  retry.className = "admin-btn admin-btn--secondary admin-btn--sm";
  retry.style.marginTop = "10px";
  retry.textContent = "Retry";
  retry.addEventListener("click", loadDashboardStats);
  return retry;
}

function renderSparkline(containerId, data, color) {
  const container = document.getElementById(containerId);
  if (!container || !Array.isArray(data) || data.length < 2) return;

  const width = 120;
  const height = 30;
  const values = data.map((value) => Number(value) || 0);
  const max = Math.max(...values, 1);
  const points = values.map((val, i) => {
    const x = (i / (values.length - 1)) * width;
    const y = height - (val / max) * height - 2;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });

  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("width", "100%");
  svg.setAttribute("height", "100%");
  svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
  svg.setAttribute("preserveAspectRatio", "none");
  svg.style.overflow = "visible";

  const line = document.createElementNS("http://www.w3.org/2000/svg", "path");
  line.setAttribute("d", `M ${points.join(" L ")}`);
  line.setAttribute("fill", "none");
  line.setAttribute("stroke", color);
  line.setAttribute("stroke-width", "2");
  line.setAttribute("stroke-linecap", "round");
  line.setAttribute("stroke-linejoin", "round");

  const area = document.createElementNS("http://www.w3.org/2000/svg", "path");
  area.setAttribute("d", `M ${points.join(" L ")} L ${width},${height} L 0,${height} Z`);
  area.setAttribute("fill", color);
  area.setAttribute("fill-opacity", "0.1");
  area.setAttribute("stroke", "none");

  svg.append(line, area);
  container.replaceChildren(svg);
}

function getActivityType(action) {
  const safeAction = String(action || "").toLowerCase();
  if (safeAction.includes("deposit")) return "deposit";
  if (safeAction.includes("kyc")) return "kyc";
  if (safeAction.includes("order") || safeAction.includes("purchase")) return "order";
  if (safeAction.includes("error") || safeAction.includes("fail")) return "alert";
  if (safeAction.includes("withdrawal")) return "withdrawal";
  return "neutral";
}

function formatAction(action) {
  return String(action || "")
    .replace(/\./g, " ")
    .replace(/_/g, " ")
    .replace(/\b\w/g, (l) => l.toUpperCase());
}

function getStatusClass(status) {
  const safeStatus = String(status || "").toLowerCase();
  if (safeStatus === "completed" || safeStatus === "paid" || safeStatus === "success") return "success";
  if (safeStatus === "pending" || safeStatus === "processing") return "warning";
  if (safeStatus === "failed" || safeStatus === "cancelled") return "danger";
  return "neutral";
}

function fmtRelativeTime(iso) {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "Unknown time";
  const diff = Date.now() - d.getTime();
  const secs = Math.floor(diff / 1000);
  if (secs < 60) return "Just now";
  if (secs < 3600) return Math.floor(secs / 60) + "m ago";
  if (secs < 86400) return Math.floor(secs / 3600) + "h ago";
  return d.toLocaleDateString();
}

function formatUSD(cents) {
  const safeCents = Number(cents) || 0;
  const dollars = safeCents / 100;
  if (Math.abs(dollars) >= 1000000) return `$${(dollars / 1000000).toFixed(2)}M`;
  if (Math.abs(dollars) >= 1000) return `$${(dollars / 1000).toFixed(1)}K`;
  return `$${dollars.toFixed(2)}`;
}

function formatNumber(num) {
  return (Number(num) || 0).toLocaleString("en-US");
}

function setTextById(id, text) {
  const el = document.getElementById(id);
  if (el) el.textContent = text;
}

function setActiveNav() {
  const path = window.location.pathname;
  const navItems = document.querySelectorAll(".admin-nav-item");

  navItems.forEach((item) => {
    item.classList.remove("active");
    const href = item.getAttribute("href");
    if (href && path === href) item.classList.add("active");
  });

  if (path === "/admin/" || path === "/admin/index.html") {
    const dashNav = document.getElementById("nav-dashboard");
    if (dashNav) dashNav.classList.add("active");
  }
}

async function loadSystemHealth() {
  try {
    const resp = await fetch("/api/admin/system");
    if (!resp.ok) {
      setAllHealthUnknown("System health unavailable");
      return;
    }
    const data = await resp.json();

    setHealthDot(
      "health-db",
      data.db_healthy ? "ok" : "error",
      `Database: ${data.db_healthy ? "Connected" : "Degraded"}`,
    );

    setHealthDot(
      "health-psp",
      data.psp_connected ? "ok" : "warn",
      `PSP: ${data.psp_connected ? "Configured" : "Not configured"}`,
    );

    setHealthDot(
      "health-kyc",
      data.kyc_provider ? "ok" : "warn",
      `KYC: ${data.kyc_provider || "Not configured"}`,
    );

    setHealthDot(
      "health-email",
      data.email_configured ? "ok" : "warn",
      `Email: ${data.email_configured ? "Configured" : "Not configured"}`,
    );

    const container = document.getElementById("health-indicators");
    if (container) {
      container.title = data.api_healthy
        ? "System Health: Core services OK"
        : "System Health: Some checks are degraded";
    }
  } catch {
    setAllHealthUnknown("System health unavailable");
  }
}

function setAllHealthUnknown(message) {
  setHealthDot("health-db", "error", message);
  setHealthDot("health-psp", "unknown", "PSP: Unknown");
  setHealthDot("health-kyc", "unknown", "KYC: Unknown");
  setHealthDot("health-email", "unknown", "Email: Unknown");
}

function setHealthDot(id, status, tooltip) {
  const el = document.getElementById(id);
  if (!el) return;
  el.className = "admin-health-dot";
  if (status === "ok") el.classList.add("admin-health-dot--ok");
  else if (status === "warn") el.classList.add("admin-health-dot--warn");
  else if (status === "error") el.classList.add("admin-health-dot--error");
  if (tooltip) {
    el.title = tooltip;
    el.setAttribute("aria-label", tooltip);
  }
}
