/**
 * Admin Dashboard JS — Loads KPI data from the API and populates the dashboard.
 * Uses HTMX polling pattern for real-time updates.
 */

document.addEventListener("DOMContentLoaded", () => {
  // Set current date
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

  // Load KPI data
  loadDashboardStats();

  // Load system health status
  loadSystemHealth();

  // Set active nav item based on URL
  setActiveNav();

  // Event listener for date range change
  const rangeSelector = document.getElementById("dashboard-range");
  if (rangeSelector) {
    rangeSelector.addEventListener("change", () => {
      loadDashboardStats();
    });
  }

  // Refresh every 30 seconds
  setInterval(loadDashboardStats, 30000);

  // Refresh system health every 60 seconds
  setInterval(loadSystemHealth, 60000);
});

/**
 * Fetch dashboard stats from backend and populate KPI cards.
 */
async function loadDashboardStats() {
  try {
    const range = document.getElementById("dashboard-range")?.value || "30d";
    const resp = await fetch(`/api/admin/stats/overview?range=${range}`);
    if (resp.ok) {
      const data = await resp.json();
      populateKPIs(data);
    } else {
      console.error('Dashboard stats API error:', resp.status);
    }
  } catch (e) {
    console.error('Dashboard stats fetch failed:', e);
    if (window.Sentry) Sentry.captureException(e);
  }
}

/**
 * Populate KPI card elements with data.
 */
function populateKPIs(data) {
  const label = data.range_label || "last 30 days";
  setTextById("kpi-total-users", formatNumber(data.total_users));
  setTextById("kpi-new-users", `+${data.new_users_range} ${label}`);
  setTextById("kpi-aum", formatUSD(data.aum_cents));
  setTextById("kpi-deposits-24h", formatUSD(data.deposits_range_cents));
  setTextById(
    "kpi-deposits-count",
    `${data.deposits_range_count} transactions`,
  );
  setTextById("kpi-pending-kyc", String(data.pending_kyc));
  setTextById("kpi-live-assets", String(data.live_assets));
  setTextById("kpi-funded-assets", `${data.funded_assets} fully funded`);
  setTextById("kpi-pending-deposits", String(data.pending_deposits));
  setTextById("kpi-open-tickets", String(data.open_tickets));
  setTextById("kpi-rewards-liability", formatUSD(data.rewards_liability_cents));

  // Update sidebar badges
  setTextById("badge-kyc", String(data.pending_kyc));
  setTextById("badge-deposits", String(data.pending_deposits));
  setTextById("badge-support", String(data.open_tickets));

  // Render Recent Activity
  const activityFeed = document.getElementById("activity-feed");
  if (activityFeed && data.recent_activity) {
    if (data.recent_activity.length === 0) {
      activityFeed.innerHTML =
        '<div style="text-align:center;padding:20px;color:var(--admin-text-muted);">No recent activity.</div>';
    } else {
      activityFeed.innerHTML = data.recent_activity
        .map(
          (act) => `
                <div class="admin-activity-item">
                    <div class="admin-activity-dot admin-activity-dot--${getActivityType(act.action)}"></div>
                    <div class="admin-activity-content">
                        <div class="admin-activity-text"><strong>${formatAction(act.action)}</strong> — ${act.entity_type} ID: ${act.entity_id || "N/A"}</div>
                        <div class="admin-activity-time">${fmtRelativeTime(act.created_at)}</div>
                    </div>
                </div>
            `,
        )
        .join("");
    }
  }

  // Render Recent Orders
  const ordersTable = document.getElementById("recent-orders-table");
  if (ordersTable && data.recent_orders) {
    if (data.recent_orders.length === 0) {
      ordersTable.innerHTML =
        '<tr><td colspan="4" style="text-align:center;padding:20px;">No recent orders.</td></tr>';
    } else {
      ordersTable.innerHTML = data.recent_orders
        .map(
          (o) => `
                <tr>
                    <td><a href="/admin/orders.html?id=${o.order_number}" class="admin-link">${o.order_number}</a></td>
                    <td><div class="admin-user-inline"><span class="admin-user-inline-name">${o.user_email}</span></div></td>
                    <td>${formatUSD(o.total_cents)}</td>
                    <td><span class="admin-badge admin-badge--${getStatusClass(o.status)}"><span class="admin-badge-dot"></span>${o.status}</span></td>
                </tr>
            `,
        )
        .join("");
    }
  }

  // Render Pending Deposits
  const depositsTable = document.getElementById("pending-deposits-table");
  if (depositsTable && data.pending_deposits_list) {
    if (data.pending_deposits_list.length === 0) {
      depositsTable.innerHTML =
        '<tr><td colspan="4" style="text-align:center;padding:20px;">No pending deposits.</td></tr>';
    } else {
      depositsTable.innerHTML = data.pending_deposits_list
        .map(
          (d) => `
                <tr>
                    <td><div class="admin-user-inline"><span class="admin-user-inline-name">${d.user_email}</span></div></td>
                    <td>${formatUSD(d.amount_cents)}</td>
                    <td><span class="admin-badge admin-badge--neutral">${d.provider}</span></td>
                    <td><a href="/admin/deposits.html" class="admin-btn admin-btn--primary admin-btn--sm">Review</a></td>
                </tr>
            `,
        )
        .join("");
    }
  }

  // Render Sparklines
  if (data.user_trend) {
    renderSparkline("trend-users", data.user_trend, "var(--admin-accent)");
  }
  if (data.deposit_trend) {
    renderSparkline(
      "trend-deposits",
      data.deposit_trend,
      "var(--admin-success)",
    );
  }
}

/**
 * Render a simple SVG sparkline into a container.
 */
function renderSparkline(containerId, data, color) {
  const container = document.getElementById(containerId);
  if (!container || !data || data.length < 2) return;

  const width = 120; // Fixed width for sparkline
  const height = 30; // Fixed height
  const max = Math.max(...data, 1);

  // Normalize points
  const points = data.map((val, i) => {
    const x = (i / (data.length - 1)) * width;
    const y = height - (val / max) * height - 2; // -2 for stroke margin
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });

  const svg = `
        <svg width="100%" height="100%" viewBox="0 0 ${width} ${height}" preserveAspectRatio="none" style="overflow:visible">
            <path d="M ${points.join(" L ")}" fill="none" stroke="${color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
            <path d="M ${points.join(" L ")} L ${width},${height} L 0,${height} Z" fill="${color}" fill-opacity="0.1" stroke="none" />
        </svg>
    `;
  container.innerHTML = svg;
}

function getActivityType(action) {
  if (action.includes("deposit")) return "deposit";
  if (action.includes("kyc")) return "kyc";
  if (action.includes("order") || action.includes("purchase")) return "order";
  if (action.includes("error") || action.includes("fail")) return "alert";
  if (action.includes("withdrawal")) return "withdrawal";
  return "neutral";
}

function formatAction(action) {
  return action
    .replace(/\./g, " ")
    .replace(/_/g, " ")
    .replace(/\b\w/g, (l) => l.toUpperCase());
}

function getStatusClass(status) {
  status = status.toLowerCase();
  if (status === "completed" || status === "paid" || status === "success")
    return "success";
  if (status === "pending" || status === "processing") return "warning";
  if (status === "failed" || status === "cancelled") return "danger";
  return "neutral";
}

function fmtRelativeTime(iso) {
  const d = new Date(iso);
  const diff = Date.now() - d.getTime();
  const secs = Math.floor(diff / 1000);
  if (secs < 60) return "Just now";
  if (secs < 3600) return Math.floor(secs / 60) + "m ago";
  if (secs < 86400) return Math.floor(secs / 3600) + "h ago";
  return d.toLocaleDateString();
}

/**
 * Format cents to USD display string (e.g. 485000000 → "$4.85M")
 */
function formatUSD(cents) {
  const dollars = cents / 100;
  if (dollars >= 1000000) {
    return `$${(dollars / 1000000).toFixed(2)}M`;
  } else if (dollars >= 1000) {
    return `$${(dollars / 1000).toFixed(1)}K`;
  }
  return `$${dollars.toFixed(2)}`;
}

/**
 * Format numbers with comma separators
 */
function formatNumber(num) {
  return num.toLocaleString("en-US");
}

/**
 * Safe text setter
 */
function setTextById(id, text) {
  const el = document.getElementById(id);
  if (el) el.textContent = text;
}

/**
 * Set active nav item based on current URL
 */
function setActiveNav() {
  const path = window.location.pathname;
  const navItems = document.querySelectorAll(".admin-nav-item");

  navItems.forEach((item) => {
    item.classList.remove("active");
    const href = item.getAttribute("href");
    if (href && path === href) {
      item.classList.add("active");
    }
  });

  // Default: dashboard
  if (path === "/admin/" || path === "/admin/index.html") {
    const dashNav = document.getElementById("nav-dashboard");
    if (dashNav) dashNav.classList.add("active");
  }
}

/**
 * Fetch system health status and update the header health indicator dots.
 */
async function loadSystemHealth() {
  try {
    const resp = await fetch("/api/admin/system");
    if (!resp.ok) {
      setHealthDot("health-db", "unknown");
      setHealthDot("health-psp", "unknown");
      setHealthDot("health-kyc", "unknown");
      setHealthDot("health-email", "unknown");
      return;
    }
    const data = await resp.json();

    // DB health — if we got a response, the DB is up
    setHealthDot("health-db", "ok", "Database: Connected");

    // PSP health
    const pspStatus = data.psp_connected ? "ok" : "warn";
    setHealthDot(
      "health-psp",
      pspStatus,
      `PSP: ${data.psp_connected ? "Connected" : "Not Configured"}`,
    );

    // KYC health
    const kycStatus = data.kyc_provider ? "ok" : "warn";
    setHealthDot(
      "health-kyc",
      kycStatus,
      `KYC: ${data.kyc_provider || "Not Configured"}`,
    );

    // Email health
    const emailStatus = data.email_configured ? "ok" : "warn";
    setHealthDot(
      "health-email",
      emailStatus,
      `Email: ${data.email_configured ? "Configured" : "Not Configured"}`,
    );

    // Update container title
    const container = document.getElementById("health-indicators");
    if (container) {
      const allOk =
        pspStatus === "ok" && kycStatus === "ok" && emailStatus === "ok";
      container.title = allOk
        ? "System Health: All Services OK"
        : "System Health: Some services need attention";
    }
  } catch (e) {
    // Network error — mark all as unknown
    setHealthDot("health-db", "error", "Database: Unreachable");
    setHealthDot("health-psp", "unknown", "PSP: Unknown");
    setHealthDot("health-kyc", "unknown", "KYC: Unknown");
    setHealthDot("health-email", "unknown", "Email: Unknown");
  }
}

function setHealthDot(id, status, tooltip) {
  const el = document.getElementById(id);
  if (!el) return;
  el.className = "admin-health-dot";
  if (status === "ok") el.classList.add("admin-health-dot--ok");
  else if (status === "warn") el.classList.add("admin-health-dot--warn");
  else if (status === "error") el.classList.add("admin-health-dot--error");
  if (tooltip) el.title = tooltip;
}
