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

  setupNotificationPanel();

  loadDashboardStats();
  loadSystemHealth();
  setActiveNav();
  setupEnvBadge();
  setupRewardsPopover();
  setupRefreshButton();
  setupWeekendBanner();
  setupActivityTabs();
  setupExportButtons();
  setupBulkPendingDeposits();
  setupAvatarMenu();
  setupSidebarSearchAndCollapse();

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
  const rangeTag = rangeShortLabel(document.getElementById("dashboard-range")?.value || "30d");
  setTextById("kpi-deposits-range-tag", rangeTag);
  setTextById("kpi-users-range-tag", rangeTag);

  setTextById("kpi-total-users", formatNumber(data.total_users));
  setTextById("kpi-new-users", `+${formatNumber(data.new_users_range)} new`);
  renderDelta("kpi-users-delta", data.new_users_range, data.new_users_prev);
  setTextById("kpi-aum", formatUSD(data.aum_cents));
  setTextById("kpi-deposits-label", `Deposits`);
  setTextById("kpi-deposits-24h", formatUSD(data.deposits_range_cents));
  setTextById("kpi-deposits-count", `${formatNumber(data.deposits_range_count)} txns ${label}`);
  renderDelta("kpi-deposits-delta", data.deposits_range_cents, data.deposits_prev_cents);
  setTextById("kpi-pending-kyc", String(data.pending_kyc));
  setTextById("kpi-live-assets", String(data.live_assets));
  setTextById("kpi-funded-assets", `${data.funded_assets} fully funded`);
  setTextById("kpi-pending-deposits", String(data.pending_deposits));
  setTextById("kpi-open-tickets", String(data.open_tickets));
  setTextById("kpi-rewards-liability", formatUSD(data.rewards_liability_cents));
  populateRewardsBreakdown(data.rewards_liability_breakdown, data.rewards_liability_cents);

  // Aging subtexts (SLA visibility)
  applyAgingSubtext("kpi-pending-deposits-aging", data.oldest_pending_deposit_secs, "Awaiting confirmation");
  applyAgingSubtext("kpi-pending-kyc-aging", data.oldest_pending_kyc_secs, "Awaiting review");
  applyAgingSubtext("kpi-open-tickets-aging", data.oldest_open_ticket_secs, "Active support requests");

  flagAnomaly('[data-kpi="deposits-range"]', data.deposits_range_cents, data.deposits_prev_cents, 1.0);
  flagAnomaly('[data-kpi="users"]', data.new_users_range, data.new_users_prev, 1.0);
  renderActionRequired(data);
  updateLastUpdated();

  setTextById("badge-kyc", String(data.pending_kyc));
  setTextById("badge-deposits", String(data.pending_deposits));
  setTextById("badge-support", String(data.open_tickets));
  updateNotificationBadge(data.unread_notifications);

  activityCache = Array.isArray(data.recent_activity) ? data.recent_activity : [];
  const activityFeed = document.getElementById("activity-feed");
  if (activityFeed) {
    if (activityCache.length === 0) renderEmptyBlock(activityFeed, "No recent activity.");
    else renderActivityFeed(activityFeed, applyActivityFilter(activityCache));
  }

  const ordersTable = document.getElementById("recent-orders-table");
  if (ordersTable && Array.isArray(data.recent_orders)) {
    if (data.recent_orders.length === 0) renderEmptyRow(ordersTable, 4, "No recent orders.");
    else renderRecentOrders(ordersTable, data.recent_orders);
  }

  const depositsTable = document.getElementById("pending-deposits-table");
  if (depositsTable && Array.isArray(data.pending_deposits_list)) {
    if (data.pending_deposits_list.length === 0) renderEmptyRow(depositsTable, 6, "No pending deposits.");
    else renderPendingDeposits(depositsTable, data.pending_deposits_list);
  }

  if (data.user_trend) {
    renderSparkline("trend-users", data.user_trend, "var(--admin-accent)", {
      format: (v) => `${formatNumber(v)} new`,
      labels: buildSparklineLabels(data.user_trend.length),
    });
  }
  if (data.deposit_trend) {
    renderSparkline("trend-deposits", data.deposit_trend, "var(--admin-success)", {
      format: (v) => formatUSD(v),
      labels: buildSparklineLabels(data.deposit_trend.length),
    });
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
  renderErrorRow(document.getElementById("pending-deposits-table"), 6, message);
}

function renderActivityFeed(container, activities) {
  container.replaceChildren();
  activities.forEach((act) => {
    const item = document.createElement("div");
    item.className = "admin-activity-item";

    const type = getActivityType(act.action);
    const dot = document.createElement("div");
    dot.className = `admin-activity-dot admin-activity-dot--${type}`;
    dot.setAttribute("role", "img");
    dot.setAttribute("aria-label", activityTypeLabel(type));
    dot.title = activityTypeLabel(type);

    const content = document.createElement("div");
    content.className = "admin-activity-content";

    const text = document.createElement("div");
    text.className = "admin-activity-text";
    const action = document.createElement("strong");
    action.textContent = formatAction(String(act.action || "activity"));
    text.append(action);
    text.append(document.createTextNode(` · ${act.entity_type || "entity"} `));
    text.append(buildEntityIdNode(act.entity_id));

    const time = document.createElement("div");
    time.className = "admin-activity-time";
    const tsAbs = act.created_at ? new Date(act.created_at).toLocaleString() : "";
    time.textContent = fmtRelativeTime(act.created_at);
    if (tsAbs) time.title = tsAbs;

    content.append(text, time);
    item.append(dot, content);
    container.appendChild(item);
  });
}

function activityTypeLabel(type) {
  return ({
    deposit: "Deposit event",
    withdrawal: "Withdrawal event",
    kyc: "KYC event",
    order: "Order event",
    alert: "Error or failure",
    neutral: "Informational",
  })[type] || "Activity";
}

function renderRecentOrders(table, orders) {
  currentOrdersCache = orders;
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
  currentDepositsCache = deposits;
  selectedDepositIds.clear();
  updateBulkBar();
  const headerCheck = document.getElementById("pending-deposits-checkall");
  if (headerCheck) headerCheck.checked = false;

  table.replaceChildren();
  deposits.forEach((deposit) => {
    const row = document.createElement("tr");

    const checkCell = document.createElement("td");
    if (deposit.id) {
      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.className = "admin-deposit-check";
      cb.dataset.id = deposit.id;
      cb.setAttribute("aria-label", `Select deposit from ${deposit.user_email || "user"}`);
      cb.addEventListener("change", () => {
        if (cb.checked) selectedDepositIds.add(deposit.id);
        else selectedDepositIds.delete(deposit.id);
        updateBulkBar();
      });
      checkCell.appendChild(cb);
    }
    row.appendChild(checkCell);

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

    const ageCell = buildAgeCell(deposit.created_at);
    row.appendChild(ageCell);

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

function renderSparkline(containerId, data, color, options = {}) {
  const container = document.getElementById(containerId);
  if (!container || !Array.isArray(data) || data.length < 2) return;

  const width = 120;
  const height = 30;
  const values = data.map((value) => Number(value) || 0);
  const max = Math.max(...values, 1);
  const min = Math.min(...values);
  const points = values.map((val, i) => {
    const x = (i / (values.length - 1)) * width;
    const y = height - (val / max) * height - 2;
    return { x, y, val };
  });
  const pathStr = points.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(" L ");

  container.replaceChildren();
  container.classList.add("admin-sparkline-host");

  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("width", "100%");
  svg.setAttribute("height", "100%");
  svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
  svg.setAttribute("preserveAspectRatio", "none");
  svg.style.overflow = "visible";

  const area = document.createElementNS("http://www.w3.org/2000/svg", "path");
  area.setAttribute("d", `M ${pathStr} L ${width},${height} L 0,${height} Z`);
  area.setAttribute("fill", color);
  area.setAttribute("fill-opacity", "0.1");
  area.setAttribute("stroke", "none");

  const line = document.createElementNS("http://www.w3.org/2000/svg", "path");
  line.setAttribute("d", `M ${pathStr}`);
  line.setAttribute("fill", "none");
  line.setAttribute("stroke", color);
  line.setAttribute("stroke-width", "2");
  line.setAttribute("stroke-linecap", "round");
  line.setAttribute("stroke-linejoin", "round");

  // Min / Max markers
  const maxIdx = values.indexOf(max);
  const minIdx = values.indexOf(min);
  [maxIdx, minIdx].forEach((idx, i) => {
    if (idx < 0 || (i === 1 && minIdx === maxIdx)) return;
    const c = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    c.setAttribute("cx", points[idx].x.toFixed(1));
    c.setAttribute("cy", points[idx].y.toFixed(1));
    c.setAttribute("r", "1.8");
    c.setAttribute("fill", color);
    svg.appendChild(c);
  });

  svg.append(area, line);

  // Hover overlay
  const overlay = document.createElementNS("http://www.w3.org/2000/svg", "rect");
  overlay.setAttribute("x", "0");
  overlay.setAttribute("y", "0");
  overlay.setAttribute("width", String(width));
  overlay.setAttribute("height", String(height));
  overlay.setAttribute("fill", "transparent");
  overlay.style.cursor = "crosshair";

  const cursor = document.createElementNS("http://www.w3.org/2000/svg", "line");
  cursor.setAttribute("y1", "0");
  cursor.setAttribute("y2", String(height));
  cursor.setAttribute("stroke", color);
  cursor.setAttribute("stroke-width", "0.5");
  cursor.setAttribute("stroke-dasharray", "2,2");
  cursor.style.display = "none";

  const tip = document.createElement("div");
  tip.className = "admin-sparkline-tip";
  tip.style.display = "none";
  container.appendChild(tip);

  const formatter = options.format || ((v) => formatNumber(v));
  const labels = options.labels;

  function onMove(e) {
    const rect = svg.getBoundingClientRect();
    const ratio = (e.clientX - rect.left) / rect.width;
    const i = Math.max(0, Math.min(points.length - 1, Math.round(ratio * (points.length - 1))));
    cursor.setAttribute("x1", String(points[i].x));
    cursor.setAttribute("x2", String(points[i].x));
    cursor.style.display = "";
    const labelText = labels?.[i] || `Point ${i + 1}`;
    tip.textContent = `${labelText}: ${formatter(values[i])}`;
    tip.style.display = "";
    tip.style.left = `${(points[i].x / width) * 100}%`;
  }
  function onLeave() {
    cursor.style.display = "none";
    tip.style.display = "none";
  }
  overlay.addEventListener("mousemove", onMove);
  overlay.addEventListener("mouseleave", onLeave);

  svg.append(cursor, overlay);
  container.appendChild(svg);
}

function buildSparklineLabels(count) {
  const labels = [];
  const today = new Date();
  for (let i = count - 1; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(today.getDate() - i);
    labels.push(d.toLocaleDateString(undefined, { month: "short", day: "numeric" }));
  }
  return labels;
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
    .split(/\s+/)
    .filter(Boolean)
    .map((word) => {
      const upper = word.toUpperCase();
      if (ACRONYMS.has(upper)) return upper;
      return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase();
    })
    .join(" ");
}

// UUID truncate + click-to-copy ------------------------------------------------
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
function buildEntityIdNode(rawId) {
  if (!rawId || rawId === "N/A" || rawId === "null") {
    const span = document.createElement("span");
    span.className = "admin-entity-id admin-entity-id--missing";
    span.textContent = "—";
    return span;
  }
  const isUuid = UUID_RE.test(rawId);
  const display = isUuid ? `${rawId.slice(0, 8)}…${rawId.slice(-4)}` : rawId;
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "admin-entity-id";
  btn.textContent = display;
  btn.title = `${rawId} — click to copy`;
  btn.setAttribute("aria-label", `Entity ID ${rawId}, click to copy`);
  btn.addEventListener("click", (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (navigator.clipboard?.writeText) {
      navigator.clipboard.writeText(rawId).then(() => {
        btn.classList.add("admin-entity-id--copied");
        const orig = btn.textContent;
        btn.textContent = "✓ Copied";
        setTimeout(() => {
          btn.textContent = orig;
          btn.classList.remove("admin-entity-id--copied");
        }, 1200);
      });
    }
  });
  return btn;
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
  if (Math.abs(dollars) >= 1_000_000) return `$${stripTrailingZeros((dollars / 1_000_000).toFixed(2))}M`;
  if (Math.abs(dollars) >= 10_000) return `$${stripTrailingZeros((dollars / 1000).toFixed(1))}K`;
  if (Math.abs(dollars) >= 1000) return `$${dollars.toLocaleString("en-US", { maximumFractionDigits: 0 })}`;
  return `$${dollars.toFixed(2)}`;
}

function stripTrailingZeros(s) {
  return String(s).replace(/\.0+$/, "").replace(/(\.\d*?)0+$/, "$1");
}

// ACRONYMS — keep these uppercase when title-casing audit log actions.
const ACRONYMS = new Set(["PII", "KYC", "API", "URL", "ID", "IP", "CSV", "PDF", "JWT", "RBAC", "SSO", "OTP", "2FA", "SLA", "NDA", "ACH", "IBAN", "BIC", "AML", "CDD", "TOS", "OAUTH", "MFA"]);

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
  // Mirror to the named system-health card tile
  const service = id.replace(/^health-/, "");
  const tile = document.querySelector(`.admin-health-tile[data-service="${service}"]`);
  if (tile) {
    const dot = tile.querySelector(".admin-health-tile-dot");
    if (dot) {
      dot.className = "admin-health-tile-dot";
      if (status === "ok") dot.classList.add("admin-health-dot--ok");
      else if (status === "warn") dot.classList.add("admin-health-dot--warn");
      else if (status === "error") dot.classList.add("admin-health-dot--error");
      else dot.classList.add("admin-health-dot--unknown");
    }
    const statusEl = document.getElementById(`health-tile-${service}`);
    if (statusEl) {
      // Tooltip looks like "Database: Connected" — strip the prefix
      const text = (tooltip || "").includes(":") ? tooltip.split(":").slice(1).join(":").trim() : tooltip;
      statusEl.textContent = text || "Unknown";
      statusEl.dataset.status = status;
    }
  }
}

// ---- Action Required Zone ---------------------------------------------------

function renderActionRequired(data) {
  const zone = document.getElementById("action-required");
  const grid = document.getElementById("action-required-grid");
  const countEl = document.getElementById("action-required-count");
  if (!zone || !grid) return;

  const items = [];
  if ((data.pending_deposits | 0) > 0) {
    items.push({
      sev: agingSeverity(data.oldest_pending_deposit_secs, 24, 72),
      label: `${data.pending_deposits} pending deposit${data.pending_deposits === 1 ? "" : "s"}`,
      sub: agingSubtitle(data.oldest_pending_deposit_secs, "Oldest"),
      href: "/admin/deposits.html?status=pending",
      cta: "Review",
    });
  }
  if ((data.pending_kyc | 0) > 0) {
    items.push({
      sev: agingSeverity(data.oldest_pending_kyc_secs, 48, 168),
      label: `${data.pending_kyc} KYC submission${data.pending_kyc === 1 ? "" : "s"}`,
      sub: agingSubtitle(data.oldest_pending_kyc_secs, "Oldest"),
      href: "/admin/kyc.html?status=pending",
      cta: "Review",
    });
  }
  if ((data.open_tickets | 0) > 0) {
    items.push({
      sev: agingSeverity(data.oldest_open_ticket_secs, 12, 48),
      label: `${data.open_tickets} open ticket${data.open_tickets === 1 ? "" : "s"}`,
      sub: agingSubtitle(data.oldest_open_ticket_secs, "Oldest"),
      href: "/admin/support-tickets.html",
      cta: "Open",
    });
  }

  if (items.length === 0) {
    zone.hidden = true;
    grid.replaceChildren();
    if (countEl) countEl.textContent = "";
    return;
  }
  zone.hidden = false;
  if (countEl) countEl.textContent = `${items.length} item${items.length === 1 ? "" : "s"}`;

  grid.replaceChildren();
  items.forEach((it) => {
    const a = document.createElement("a");
    a.href = it.href;
    a.className = `admin-action-item admin-action-item--${it.sev}`;
    const main = document.createElement("div");
    main.className = "admin-action-item-main";
    const label = document.createElement("div");
    label.className = "admin-action-item-label";
    label.textContent = it.label;
    const sub = document.createElement("div");
    sub.className = "admin-action-item-sub";
    sub.textContent = it.sub;
    main.append(label, sub);
    const cta = document.createElement("span");
    cta.className = "admin-action-item-cta";
    cta.textContent = `${it.cta} →`;
    a.append(main, cta);
    grid.appendChild(a);
  });
}

function agingSeverity(secs, warnHours, dangerHours) {
  if (secs == null || !Number.isFinite(secs)) return "info";
  const hours = secs / 3600;
  if (hours >= dangerHours) return "danger";
  if (hours >= warnHours) return "warning";
  return "info";
}

function agingSubtitle(secs, prefix) {
  if (secs == null || !Number.isFinite(secs)) return `${prefix}: just now`;
  return `${prefix}: ${humanizeDuration(secs)}`;
}

function applyAgingSubtext(elId, secs, fallback) {
  const el = document.getElementById(elId);
  if (!el) return;
  if (secs == null || !Number.isFinite(secs)) {
    el.textContent = fallback;
    el.classList.remove("admin-aging-warn", "admin-aging-danger");
    return;
  }
  el.textContent = `Oldest: ${humanizeDuration(secs)}`;
  el.classList.toggle("admin-aging-danger", secs / 3600 >= 72);
  el.classList.toggle("admin-aging-warn", secs / 3600 >= 24 && secs / 3600 < 72);
}

function humanizeDuration(secs) {
  const s = Math.max(0, Math.floor(secs));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  if (s < 86400) return `${Math.floor(s / 3600)}h`;
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  return h > 0 ? `${d}d ${h}h` : `${d}d`;
}

function buildAgeCell(createdAtIso) {
  const cell = document.createElement("td");
  if (!createdAtIso) {
    cell.textContent = "—";
    return cell;
  }
  const t = new Date(createdAtIso).getTime();
  if (Number.isNaN(t)) {
    cell.textContent = "—";
    return cell;
  }
  const secs = (Date.now() - t) / 1000;
  const badge = document.createElement("span");
  badge.className = "admin-age-badge";
  if (secs / 3600 >= 72) badge.classList.add("admin-age-badge--danger");
  else if (secs / 3600 >= 24) badge.classList.add("admin-age-badge--warn");
  else badge.classList.add("admin-age-badge--ok");
  badge.textContent = humanizeDuration(secs);
  badge.title = new Date(createdAtIso).toLocaleString();
  cell.appendChild(badge);
  return cell;
}

// ---- Deltas / Range / Last-Updated -----------------------------------------

function renderDelta(elId, current, previous) {
  const el = document.getElementById(elId);
  if (!el) return;
  const cur = Number(current) || 0;
  const prev = Number(previous) || 0;
  el.classList.remove("admin-kpi-change--up", "admin-kpi-change--down", "admin-kpi-change--flat");
  if (prev === 0 && cur === 0) {
    el.textContent = "·";
    el.classList.add("admin-kpi-change--flat");
    return;
  }
  if (prev === 0) {
    el.textContent = "▲ new";
    el.classList.add("admin-kpi-change--up");
    return;
  }
  const pct = ((cur - prev) / Math.abs(prev)) * 100;
  const arrow = pct > 0 ? "▲" : pct < 0 ? "▼" : "·";
  const cls = pct > 0 ? "admin-kpi-change--up" : pct < 0 ? "admin-kpi-change--down" : "admin-kpi-change--flat";
  el.textContent = `${arrow} ${Math.abs(pct).toFixed(pct >= 100 ? 0 : 1)}%`;
  el.classList.add(cls);
}

function rangeShortLabel(range) {
  return ({ today: "24h", "7d": "7d", "30d": "30d", "90d": "90d", "1y": "1y", all: "all" })[range] || "30d";
}

function updateLastUpdated() {
  const el = document.getElementById("dashboard-last-updated");
  if (!el) return;
  el.dataset.ts = String(Date.now());
  el.textContent = "Updated just now";
}

setInterval(() => {
  const el = document.getElementById("dashboard-last-updated");
  if (!el || !el.dataset.ts) return;
  const secs = Math.floor((Date.now() - Number(el.dataset.ts)) / 1000);
  if (secs < 5) el.textContent = "Updated just now";
  else if (secs < 60) el.textContent = `Updated ${secs}s ago`;
  else el.textContent = `Updated ${Math.floor(secs / 60)}m ago`;
}, 5000);

// ---- Env Badge --------------------------------------------------------------

function populateRewardsBreakdown(breakdown, totalCents) {
  if (!breakdown) return;
  setTextById("rewards-bd-cashback", formatUSD(breakdown.cashback_cents));
  setTextById("rewards-bd-referrals", formatUSD(breakdown.referrals_cents));
  setTextById("rewards-bd-promotions", formatUSD(breakdown.promotions_cents));
  setTextById("rewards-bd-total", formatUSD(totalCents));
  setTextById("rewards-bd-users", formatNumber(breakdown.users_with_balance));

  const sub = document.getElementById("kpi-rewards-subtext");
  if (sub) {
    const users = Number(breakdown.users_with_balance) || 0;
    sub.textContent = users > 0
      ? `Across ${users} user${users === 1 ? "" : "s"}`
      : "No outstanding balances";
  }
}

function setupRewardsPopover() {
  const trigger = document.getElementById("rewards-info-trigger");
  const popover = document.getElementById("rewards-popover");
  const closeBtn = document.getElementById("rewards-popover-close");
  if (!trigger || !popover) return;

  function open(e) {
    if (e) { e.preventDefault(); e.stopPropagation(); }
    popover.hidden = false;
    trigger.setAttribute("aria-expanded", "true");
    positionPopover();
  }
  function positionPopover() {
    const r = trigger.getBoundingClientRect();
    popover.style.position = "fixed";
    const top = r.bottom + 6;
    let left = r.left;
    const maxLeft = window.innerWidth - popover.offsetWidth - 12;
    if (left > maxLeft) left = maxLeft;
    if (left < 12) left = 12;
    popover.style.top = `${top}px`;
    popover.style.left = `${left}px`;
  }
  function close() {
    popover.hidden = true;
    trigger.setAttribute("aria-expanded", "false");
  }
  trigger.addEventListener("click", open);
  if (closeBtn) closeBtn.addEventListener("click", close);
  document.addEventListener("click", (e) => {
    if (popover.hidden) return;
    if (!popover.contains(e.target) && e.target !== trigger) close();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !popover.hidden) close();
  });
  window.addEventListener("resize", () => { if (!popover.hidden) positionPopover(); });
  window.addEventListener("scroll", () => { if (!popover.hidden) positionPopover(); }, true);
}

// ---- Activity tabs (#18 + #21) ---------------------------------------------

let activityCache = [];
let activityFilter = "all";

function setupActivityTabs() {
  const tabs = document.querySelectorAll("#activity-tabs .admin-activity-tab");
  if (!tabs.length) return;
  tabs.forEach((tab) => {
    tab.addEventListener("click", () => {
      tabs.forEach((t) => {
        t.classList.remove("admin-activity-tab--active");
        t.setAttribute("aria-selected", "false");
      });
      tab.classList.add("admin-activity-tab--active");
      tab.setAttribute("aria-selected", "true");
      activityFilter = tab.dataset.filter;
      const feed = document.getElementById("activity-feed");
      if (!feed) return;
      const filtered = applyActivityFilter(activityCache);
      if (filtered.length === 0) renderEmptyBlock(feed, `No ${activityFilter === "all" ? "" : activityFilter + " "}activity.`);
      else renderActivityFeed(feed, filtered);
    });
  });
}

function applyActivityFilter(items) {
  if (activityFilter === "all") return items;
  return items.filter((act) => activityCategory(act) === activityFilter);
}

function activityCategory(act) {
  const action = String(act.action || "").toLowerCase();
  const entity = String(act.entity_type || "").toLowerCase();
  if (action.includes("kyc") || action.includes("fraud") || action.includes("pii") || action.includes("compliance"))
    return "compliance";
  if (action.startsWith("user.") || action.includes("login") || action.includes("logout") || entity === "users" || entity === "user")
    return "user";
  if (action.includes("market") || action.includes("order") || entity.startsWith("market_") || action.includes("orderbook"))
    return "marketplace";
  if (action.includes("system") || action.includes("orderbook_rebuilt") || action.includes("scheduler") || action.includes("worker") || action.includes("sync"))
    return "system";
  return "user";
}

// ---- CSV export (#19) ------------------------------------------------------

function setupExportButtons() {
  const a = document.getElementById("export-activity");
  if (a) a.addEventListener("click", () => exportCSV(applyActivityFilter(activityCache).map(activityToCsvRow), ["When", "Action", "Entity Type", "Entity ID"], `activity-${todayStamp()}.csv`));

  const o = document.getElementById("export-orders");
  if (o) o.addEventListener("click", () => exportCSV(currentOrdersCache.map(orderToCsvRow), ["Order", "User", "Amount (USD)", "Status", "Created"], `orders-${todayStamp()}.csv`));

  const d = document.getElementById("export-deposits");
  if (d) d.addEventListener("click", () => exportCSV(currentDepositsCache.map(depositToCsvRow), ["User", "Amount (USD)", "Provider", "Status", "Created"], `pending-deposits-${todayStamp()}.csv`));
}

function activityToCsvRow(a) {
  return [a.created_at || "", formatAction(a.action || ""), a.entity_type || "", a.entity_id || ""];
}
function orderToCsvRow(o) {
  return [o.order_number || "", o.user_email || "", ((Number(o.total_cents) || 0) / 100).toFixed(2), o.status || "", o.created_at || ""];
}
function depositToCsvRow(d) {
  return [d.user_email || "", ((Number(d.amount_cents) || 0) / 100).toFixed(2), d.provider || "", d.status || "", d.created_at || ""];
}

function exportCSV(rows, headers, filename) {
  const csv = [headers, ...rows]
    .map((r) => r.map(csvEscape).join(","))
    .join("\r\n");
  const blob = new Blob(["﻿" + csv], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  setTimeout(() => {
    URL.revokeObjectURL(url);
    a.remove();
  }, 0);
}

function csvEscape(val) {
  const s = String(val ?? "");
  if (/[",\r\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
  return s;
}

function todayStamp() {
  const d = new Date();
  const pad = (n) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}`;
}

let currentOrdersCache = [];
let currentDepositsCache = [];

// ---- Bulk pending deposits (#20) -------------------------------------------

let selectedDepositIds = new Set();

function setupBulkPendingDeposits() {
  const headerCheck = document.getElementById("pending-deposits-checkall");
  if (headerCheck) {
    headerCheck.addEventListener("change", () => {
      const checked = headerCheck.checked;
      document.querySelectorAll("#pending-deposits-table .admin-deposit-check").forEach((cb) => {
        cb.checked = checked;
        const id = cb.dataset.id;
        if (id) {
          if (checked) selectedDepositIds.add(id);
          else selectedDepositIds.delete(id);
        }
      });
      updateBulkBar();
    });
  }
  const confirmBtn = document.getElementById("bulk-confirm-deposits");
  const cancelBtn = document.getElementById("bulk-cancel-deposits");
  if (confirmBtn) confirmBtn.addEventListener("click", () => bulkDepositAction("confirm"));
  if (cancelBtn) cancelBtn.addEventListener("click", () => bulkDepositAction("cancel"));
}

function updateBulkBar() {
  const bar = document.getElementById("bulk-bar-deposits");
  const countEl = document.getElementById("bulk-bar-count");
  if (!bar) return;
  if (selectedDepositIds.size === 0) {
    bar.hidden = true;
    return;
  }
  bar.hidden = false;
  if (countEl) countEl.textContent = `${selectedDepositIds.size} selected`;
}

async function bulkDepositAction(kind) {
  const ids = Array.from(selectedDepositIds);
  if (ids.length === 0) return;
  const verb = kind === "confirm" ? "confirm" : "cancel";
  if (!confirm(`${verb.charAt(0).toUpperCase() + verb.slice(1)} ${ids.length} deposit${ids.length === 1 ? "" : "s"}?`)) return;
  const results = await Promise.allSettled(
    ids.map((id) =>
      fetch(`/api/admin/deposits/${encodeURIComponent(id)}/${kind}`, { method: "POST", headers: { "Content-Type": "application/json" } })
    )
  );
  const failed = results.filter((r) => r.status === "rejected" || (r.status === "fulfilled" && !r.value.ok)).length;
  selectedDepositIds.clear();
  updateBulkBar();
  if (failed > 0) alert(`${failed} of ${ids.length} requests failed. See deposits page for detail.`);
  loadDashboardStats();
}

// ---- Avatar menu (#33) -----------------------------------------------------

function setupAvatarMenu() {
  document.addEventListener("admin:sidebar-ready", attachAvatarMenu, { once: true });
  // Already-rendered case
  attachAvatarMenu();
}

function attachAvatarMenu() {
  const userBlock = document.querySelector(".admin-sidebar-user");
  if (!userBlock || userBlock.dataset.menuBound === "1") return;
  userBlock.dataset.menuBound = "1";
  userBlock.classList.add("admin-sidebar-user--menu");
  userBlock.setAttribute("role", "button");
  userBlock.setAttribute("tabindex", "0");
  userBlock.setAttribute("aria-haspopup", "menu");
  userBlock.setAttribute("aria-expanded", "false");

  const menu = document.createElement("div");
  menu.className = "admin-avatar-menu";
  menu.hidden = true;
  menu.setAttribute("role", "menu");
  menu.innerHTML = `
    <a href="/admin/settings.html" role="menuitem" class="admin-avatar-menu-item">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 11-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 11-4 0v-.09a1.65 1.65 0 00-1-1.51 1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 11-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 110-4h.09a1.65 1.65 0 001.51-1 1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 112.83-2.83l.06.06a1.65 1.65 0 001.82.33h.01a1.65 1.65 0 001-1.51V3a2 2 0 114 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 112.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82v.01a1.65 1.65 0 001.51 1H21a2 2 0 110 4h-.09a1.65 1.65 0 00-1.51 1z"/></svg>
      Settings
    </a>
    <a href="/admin/roles.html" role="menuitem" class="admin-avatar-menu-item">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2"/><circle cx="8.5" cy="7" r="4"/><path d="M20 8v6M23 11h-6"/></svg>
      Roles & permissions
    </a>
    <a href="/admin/audit-logs.html" role="menuitem" class="admin-avatar-menu-item">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><path d="M14 2v6h6"/></svg>
      My audit trail
    </a>
    <hr class="admin-avatar-menu-sep">
    <button type="button" role="menuitem" class="admin-avatar-menu-item admin-avatar-menu-item--danger" id="avatar-logout">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 01-2-2V5a2 2 0 012-2h4"/><path d="M16 17l5-5-5-5"/><path d="M21 12H9"/></svg>
      Sign out
    </button>
  `;
  userBlock.appendChild(menu);

  function open() { menu.hidden = false; userBlock.setAttribute("aria-expanded", "true"); }
  function close() { menu.hidden = true; userBlock.setAttribute("aria-expanded", "false"); }
  function toggle(e) {
    if (e) e.stopPropagation();
    if (menu.hidden) open(); else close();
  }
  userBlock.addEventListener("click", toggle);
  userBlock.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggle(); }
    if (e.key === "Escape") close();
  });
  document.addEventListener("click", (e) => {
    if (!menu.hidden && !userBlock.contains(e.target)) close();
  });

  const logout = menu.querySelector("#avatar-logout");
  if (logout) {
    logout.addEventListener("click", async (e) => {
      e.stopPropagation();
      try {
        await fetch("/api/auth/logout", { method: "POST" });
      } catch {}
      window.location.href = "/login.html";
    });
  }
}

// ---- Sidebar search + collapse (#36) ---------------------------------------

function setupSidebarSearchAndCollapse() {
  document.addEventListener("admin:sidebar-ready", enhanceSidebar, { once: true });
  enhanceSidebar();
}

function enhanceSidebar() {
  const nav = document.querySelector(".admin-sidebar-nav");
  if (!nav || nav.dataset.enhanced === "1") return;
  nav.dataset.enhanced = "1";

  // Search input
  const searchWrap = document.createElement("div");
  searchWrap.className = "admin-sidebar-search-wrap";
  searchWrap.innerHTML = `
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="6"/><path d="M21 21l-4.35-4.35"/></svg>
    <input type="text" id="admin-sidebar-search" placeholder="Filter nav…" aria-label="Filter sidebar navigation" />
  `;
  nav.parentNode.insertBefore(searchWrap, nav);
  const input = searchWrap.querySelector("input");
  input.addEventListener("input", () => filterSidebar(input.value));

  // Section collapse
  nav.querySelectorAll(".admin-nav-section-label").forEach((title) => {
    title.classList.add("admin-nav-section-label--collapsible");
    title.setAttribute("role", "button");
    title.setAttribute("tabindex", "0");
    title.setAttribute("aria-expanded", "true");
    const chevron = document.createElement("span");
    chevron.className = "admin-nav-section-chevron";
    chevron.innerHTML = `<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6"/></svg>`;
    title.appendChild(chevron);
    title.addEventListener("click", () => toggleNavSection(title));
    title.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggleNavSection(title); }
    });
  });
}

function toggleNavSection(title) {
  const collapsed = title.classList.toggle("admin-nav-section-label--collapsed");
  title.setAttribute("aria-expanded", collapsed ? "false" : "true");
  const section = title.closest(".admin-nav-section");
  if (!section) return;
  section.querySelectorAll(".admin-nav-item").forEach((item) => {
    item.style.display = collapsed ? "none" : "";
  });
}

function filterSidebar(query) {
  const q = query.trim().toLowerCase();
  document.querySelectorAll(".admin-sidebar-nav .admin-nav-item").forEach((item) => {
    const text = (item.textContent || "").toLowerCase();
    item.style.display = !q || text.includes(q) ? "" : "none";
  });
  document.querySelectorAll(".admin-sidebar-nav .admin-nav-section").forEach((section) => {
    const visibleItems = section.querySelectorAll(".admin-nav-item:not([style*='display: none'])");
    section.style.display = !q || visibleItems.length > 0 ? "" : "none";
  });
}

// ---- Anomaly card border (#13 simplified) ----------------------------------

function flagAnomaly(cardSelector, current, previous, threshold = 1.0) {
  const card = document.querySelector(cardSelector);
  if (!card) return;
  card.classList.remove("admin-kpi-card--anomaly");
  const cur = Number(current) || 0;
  const prev = Number(previous) || 0;
  if (prev === 0) return;
  const delta = (cur - prev) / Math.abs(prev);
  if (Math.abs(delta) >= threshold) {
    card.classList.add("admin-kpi-card--anomaly");
    card.dataset.anomalyDir = delta > 0 ? "up" : "down";
  }
}

// ---- Notifications dropdown panel (#8) -------------------------------------

let notifCache = null;
let notifFilter = "all";

function setupNotificationPanel() {
  const btn = document.getElementById("admin-notification-button");
  const panel = document.getElementById("notification-panel");
  if (!btn || !panel) return;

  btn.addEventListener("click", (e) => {
    e.stopPropagation();
    const open = !panel.hidden;
    if (open) closeNotifPanel();
    else openNotifPanel();
  });

  document.addEventListener("click", (e) => {
    if (panel.hidden) return;
    if (!panel.contains(e.target) && e.target !== btn) closeNotifPanel();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !panel.hidden) closeNotifPanel();
  });

  panel.querySelectorAll(".admin-notif-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      panel.querySelectorAll(".admin-notif-tab").forEach((t) => {
        t.classList.remove("admin-notif-tab--active");
        t.setAttribute("aria-selected", "false");
      });
      tab.classList.add("admin-notif-tab--active");
      tab.setAttribute("aria-selected", "true");
      notifFilter = tab.dataset.filter;
      renderNotifList();
    });
  });
}

async function openNotifPanel() {
  const panel = document.getElementById("notification-panel");
  const btn = document.getElementById("admin-notification-button");
  if (!panel || !btn) return;
  panel.hidden = false;
  btn.setAttribute("aria-expanded", "true");
  if (!notifCache) await fetchNotifications();
  renderNotifList();
}

function closeNotifPanel() {
  const panel = document.getElementById("notification-panel");
  const btn = document.getElementById("admin-notification-button");
  if (!panel || !btn) return;
  panel.hidden = true;
  btn.setAttribute("aria-expanded", "false");
}

async function fetchNotifications() {
  try {
    const resp = await fetch("/api/admin/notifications");
    if (!resp.ok) {
      notifCache = [];
      return;
    }
    const data = await resp.json();
    notifCache = Array.isArray(data.notifications) ? data.notifications : [];
  } catch {
    notifCache = [];
  }
}

function renderNotifList() {
  const body = document.getElementById("notification-panel-body");
  if (!body) return;
  body.replaceChildren();
  if (!notifCache || notifCache.length === 0) {
    const empty = document.createElement("div");
    empty.className = "admin-notif-empty";
    empty.textContent = "No notifications.";
    body.appendChild(empty);
    return;
  }
  const items = notifFilter === "unread" ? notifCache.filter((n) => !n.is_read) : notifCache;
  if (items.length === 0) {
    const empty = document.createElement("div");
    empty.className = "admin-notif-empty";
    empty.textContent = "All caught up.";
    body.appendChild(empty);
    return;
  }
  items.slice(0, 30).forEach((n) => {
    const row = document.createElement("div");
    row.className = `admin-notif-row${n.is_read ? "" : " admin-notif-row--unread"}`;
    const dot = document.createElement("span");
    dot.className = `admin-notif-dot admin-notif-dot--${notifTypeClass(n.type)}`;
    const main = document.createElement("div");
    main.className = "admin-notif-main";
    const title = document.createElement("div");
    title.className = "admin-notif-title";
    title.textContent = n.title || "(untitled)";
    const sub = document.createElement("div");
    sub.className = "admin-notif-sub";
    sub.textContent = n.message ? String(n.message).slice(0, 120) : (n.user_email || "");
    const meta = document.createElement("div");
    meta.className = "admin-notif-meta";
    meta.textContent = `${n.user_name || n.user_email || ""} · ${fmtRelativeTime(n.created_at)}`;
    main.append(title, sub, meta);
    row.append(dot, main);
    body.appendChild(row);
  });
}

function notifTypeClass(type) {
  const t = String(type || "").toLowerCase();
  if (t.includes("error") || t.includes("fail")) return "danger";
  if (t.includes("warn") || t.includes("alert")) return "warning";
  if (t.includes("success") || t.includes("ok")) return "success";
  return "info";
}

function setupRefreshButton() {
  const btn = document.getElementById("dashboard-refresh");
  if (!btn) return;
  btn.addEventListener("click", () => {
    btn.classList.add("admin-refresh-spinning");
    Promise.all([loadDashboardStats(), loadSystemHealth()]).finally(() => {
      setTimeout(() => btn.classList.remove("admin-refresh-spinning"), 400);
    });
  });
}

function setupWeekendBanner() {
  const banner = document.getElementById("weekend-banner");
  if (!banner) return;
  const day = new Date().getDay(); // 0 = Sun, 6 = Sat
  if (day === 0 || day === 6) {
    banner.hidden = false;
  }
}

function setupEnvBadge() {
  const el = document.getElementById("env-badge");
  if (!el) return;
  const host = window.location.hostname;
  let env = null;
  if (host === "localhost" || host === "127.0.0.1") env = { label: "LOCAL", cls: "admin-env-badge--local" };
  else if (host.includes("staging") || host.includes("stage.") || host.startsWith("dev.")) env = { label: "STAGING", cls: "admin-env-badge--staging" };
  if (env) {
    el.textContent = env.label;
    el.classList.add(env.cls);
    el.hidden = false;
  }
}
