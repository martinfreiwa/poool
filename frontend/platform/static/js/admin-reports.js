/**
 * Admin Reports & Exports JS — v2
 * Categorised report cards with date range support, preview table, and CSV/JSON export.
 */

// ─── Report Definitions ───────────────────────────────────────────────────────
const REPORTS = [
  // Financial
  {
    id: "monthly-financial",
    group: "financial",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="5" width="16" height="12" rx="2"/><path d="M14 10a2 2 0 11-4 0 2 2 0 014 0z"/><path d="M2 5l4-3h8l4 3"/></svg>',
    title: "Monthly Financial Summary",
    format: "CSV",
    accentColor: "var(--admin-success)",
    desc: "Total deposits, withdrawals, investments, dividends, and fees grouped by month.",
    endpoint: "/api/admin/reports/financial-summary",
  },
  {
    id: "wallet-transactions",
    group: "financial",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="5" width="16" height="12" rx="2" /><path d="M14 10a2 2 0 11-4 0 2 2 0 014 0z" /><path d="M2 5l4-3h8l4 3" /></svg>',
    title: "Wallet Transactions Ledger",
    format: "CSV",
    accentColor: "var(--admin-success)",
    desc: "All platform wallet transactions — deposits, withdrawals, purchases, rewards, and fees.",
    endpoint: "/api/admin/reports/wallet-transactions",
  },
  {
    id: "invoices",
    group: "financial",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M16 2H4a1 1 0 00-1 1v14a1 1 0 001 1h12a1 1 0 001-1V3a1 1 0 00-1-1z" /><path d="M7 6h6M7 10h6M7 14h3" /></svg>',
    title: "Invoice Summary",
    format: "CSV",
    accentColor: "var(--admin-success)",
    desc: "All issued invoices with status, amounts, taxes, and linked orders.",
    endpoint: "/api/admin/reports/invoice-summary",
  },
  {
    id: "multi-currency",
    group: "financial",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><circle cx="10" cy="10" r="8" /><path d="M10 5v10" /><path d="M7 7.5c0-1 1.5-2 3-2s3 1 3 2-1.5 1.5-3 2-3 1-3 2 1.5 2 3 2 3-1 3-2" /></svg>',
    title: "Multi-Currency Exposure",
    format: "CSV",
    accentColor: "var(--admin-success)",
    desc: "Aggregated wallet balances by currency (USD, IDR, EUR) across all user accounts.",
    endpoint: "/api/admin/reports/multi-currency",
  },
  // Compliance
  {
    id: "users",
    group: "compliance",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M14 17v-1.5A3.5 3.5 0 0010.5 12h-4A3.5 3.5 0 003 15.5V17" /><circle cx="8.5" cy="6.5" r="3.5" /><path d="M17 17v-1.5a3.5 3.5 0 00-2.5-3.37" /><path d="M13 2.13a3.5 3.5 0 010 6.74" /></svg>',
    title: "User Growth Report",
    format: "CSV",
    accentColor: "var(--admin-info)",
    desc: "All registered users with signup dates, verification status, and growth trends.",
    endpoint: "/api/admin/reports/user-growth",
  },
  {
    id: "kyc",
    group: "compliance",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M10 1L2 5v4.5c0 5 3.5 9 8 10.5 4.5-1.5 8-5.5 8-10.5V5L10 1z" /><path d="M7 10l2 2 4-4" /></svg>',
    title: "KYC Status Report",
    format: "CSV",
    accentColor: "var(--admin-info)",
    desc: "All KYC/AML submissions with status, document type, PEP flags, and review timestamps.",
    endpoint: "/api/admin/reports/kyc-status",
  },
  {
    id: "audit",
    group: "compliance",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3h14v14H3z" /><path d="M7 7h6M7 10h6M7 13h3" /></svg>',
    title: "Audit Trail Export",
    format: "JSON",
    accentColor: "var(--admin-info)",
    desc: "Immutable audit log — every action with actor, entity, IP address, and state changes.",
    endpoint: "/api/admin/reports/audit-summary",
  },
  {
    id: "aml",
    group: "compliance",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M10 1L2 5v4.5c0 5 3.5 9 8 10.5 4.5-1.5 8-5.5 8-10.5V5L10 1z" /><path d="M10 7v4" /><circle cx="10" cy="14" r="0.5" fill="currentColor" /></svg>',
    title: "AML / PEP Flags Report",
    format: "CSV",
    accentColor: "var(--admin-danger)",
    desc: "All KYC records with PEP hits or sanctions flags — critical for regulatory reporting.",
    endpoint: "/api/admin/reports/aml-compliance",
  },
  // Assets & Investments
  {
    id: "investments",
    group: "assets",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 17 8 9 12 13 17 6" /><path d="M17 6h-4M17 6v4" /></svg>',
    title: "Investment Register",
    format: "CSV",
    accentColor: "var(--admin-accent)",
    desc: "All active, completed, and exited investments with token counts, values, and yield data.",
    endpoint: "/api/admin/reports/investment-summary",
  },
  {
    id: "assets",
    group: "assets",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M2 7l8-5 8 5v9a2 2 0 01-2 2H4a2 2 0 01-2-2V7z" /><path d="M7 18V10h6v8" /></svg>',
    title: "Asset Performance",
    format: "CSV",
    accentColor: "var(--admin-accent)",
    desc: "Per-asset funding progress, yield, occupancy, and token availability.",
    endpoint: "/api/admin/reports/asset-performance",
  },
  {
    id: "orders",
    group: "assets",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M16 2H4a1 1 0 00-1 1v14a1 1 0 001 1h12a1 1 0 001-1V3a1 1 0 00-1-1z" /><path d="M7 6h6M7 10h6M7 14h3" /></svg>',
    title: "Order History",
    format: "CSV",
    accentColor: "var(--admin-accent)",
    desc: "Full order log with user details, items, payment methods, amounts, and status.",
    endpoint: "/api/admin/reports/order-summary",
  },
  // Operational
  {
    id: "rewards-liability",
    group: "operational",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M10 2l2.4 4.8L18 7.6l-4 3.9.9 5.5L10 14.6 5.1 17l.9-5.5-4-3.9 5.6-.8L10 2z" /></svg>',
    title: "Rewards Liability",
    format: "CSV",
    accentColor: "var(--admin-warning)",
    desc: "Total outstanding reward balances across cashback, referrals, and promotions.",
    endpoint: "/api/admin/reports/rewards-liability",
  },
  {
    id: "referral-effectiveness",
    group: "operational",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M17 11a5 5 0 00-10 0" /><path d="M3 11a5 5 0 0010 0" /><circle cx="12" cy="7" r="3" /><circle cx="8" cy="13" r="3" /></svg>',
    title: "Referral Programme Effectiveness",
    format: "CSV",
    accentColor: "var(--admin-warning)",
    desc: "Referral conversion rates, pending qualifications, and total rewards paid.",
    endpoint: "/api/admin/reports/referral-effectiveness",
  },
  {
    id: "support",
    group: "operational",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M18 10c0 4.418-3.582 8-8 8a8.07 8.07 0 01-3.2-.66L2 18l.66-4.8A8.07 8.07 0 012 10c0-4.418 3.582-8 8-8s8 3.582 8 8z" /></svg>',
    title: "Support Ticket Log",
    format: "CSV",
    accentColor: "var(--admin-warning)",
    desc: "Open and closed support tickets with priority, status, and resolution timestamps.",
    endpoint: "/api/admin/reports/support-summary",
  },
  // Tax & Fiscal
  {
    id: "investor-pl",
    group: "tax",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v16m-6-16v16M2 10h16M2 6h16M2 14h16" /></svg>',
    title: "Annual Investor P&L",
    format: "PDF",
    accentColor: "var(--admin-accent)",
    desc: "Calculated annual P&L, capital gains, and dividends for all investors. Tax-ready format.",
    endpoint: "/api/admin/reports/tax-pl",
  },
  {
    id: "withholding-tax",
    group: "tax",
    icon: '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M10 2v16m-4-16v16M2 10h16M2 6h16M2 14h16" /></svg>',
    title: "Withholding Tax Summary",
    format: "CSV",
    accentColor: "var(--admin-accent)",
    desc: "Summary of all tax withheld from dividend payouts for regulatory reporting.",
    endpoint: "/api/admin/reports/tax-withholding",
  },
];

// ─── Boot ─────────────────────────────────────────────────────────────────────
document.addEventListener("DOMContentLoaded", () => {
  setPreset("30d");
  renderAllGrids();
});

function setPreset(preset) {
  const to = new Date();
  const from = new Date();
  if (preset === "30d") from.setDate(to.getDate() - 30);
  else if (preset === "90d") from.setDate(to.getDate() - 90);
  else if (preset === "ytd") from.setMonth(0, 1);
  const isoFrom = from.toISOString().split("T")[0];
  const isoTo = to.toISOString().split("T")[0];
  const fromEl = document.getElementById("range-from");
  const toEl = document.getElementById("range-to");
  if (fromEl) fromEl.value = isoFrom;
  if (toEl) toEl.value = isoTo;
}

// ─── Render Report Cards ──────────────────────────────────────────────────────
function renderAllGrids() {
  const groups = {
    financial: "grid-financial",
    compliance: "grid-compliance",
    assets: "grid-assets",
    operational: "grid-operational",
    tax: "grid-tax",
  };
  Object.entries(groups).forEach(([group, gridId]) => {
    const grid = document.getElementById(gridId);
    if (!grid) return;
    const reports = REPORTS.filter((r) => r.group === group);
    grid.innerHTML = reports.map((r) => reportCard(r)).join("");
  });
}

function reportCard(r) {
  return `
        <div class="admin-card" id="report-card-${r.id}" style="padding:20px;display:flex;flex-direction:column;gap:12px;transition:box-shadow 0.2s;">
            <div style="display:flex;align-items:flex-start;gap:12px;">
                <div style="width:44px;height:44px;border-radius:var(--admin-radius-md);background:${r.accentColor}15;display:flex;align-items:center;justify-content:center;color:${r.accentColor};flex-shrink:0;">${r.icon}</div>
                <div style="flex:1;min-width:0;">
                    <div style="font-size:14px;font-weight:700;color:var(--admin-text-primary);margin-bottom:2px;">${r.title}</div>
                    <span style="display:inline-block;padding:1px 6px;border-radius:4px;font-size:10px;font-weight:700;background:${r.accentColor}20;color:${r.accentColor};">${r.format}</span>
                </div>
            </div>
            <p style="font-size:12px;color:var(--admin-text-muted);margin:0;line-height:1.6;flex:1;">${r.desc}</p>
            <div style="display:flex;gap:8px;margin-top:auto;">
                <button class="admin-btn admin-btn--primary admin-btn--sm" style="flex:1;" id="dl-btn-${r.id}" onclick="downloadReport('${r.id}')">
                    <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M8 2v9M4 8l4 4 4-4"/><path d="M2 14h12"/></svg>
                    Download ${r.format}
                </button>
                <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="previewReport('${r.id}')" title="Preview first 5 rows">
                    <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M1 8s3-6 7-6 7 6 7 6-3 6-7 6-7-6-7-6z"/><circle cx="8" cy="8" r="2.5"/></svg>
                </button>
            </div>
        </div>
    `;
}

// ─── Download ─────────────────────────────────────────────────────────────────
async function downloadReport(id) {
  const report = REPORTS.find((r) => r.id === id);
  if (!report) return;
  const btn = document.getElementById(`dl-btn-${id}`);
  const origHTML = btn.innerHTML;
  btn.innerHTML =
    '<div style="width:12px;height:12px;border:2px solid currentColor;border-top-color:transparent;border-radius:50%;animation:spin 0.8s linear infinite;display:inline-block;vertical-align:middle;margin-right:4px;"></div>Generating…';
  btn.disabled = true;
  const dateFrom = document.getElementById("range-from")?.value || "";
  const dateTo = document.getElementById("range-to")?.value || "";
  try {
    const url = `${report.endpoint}${dateFrom ? `?from=${dateFrom}&to=${dateTo}` : ""}`;
    let resp = await fetch(url);
    if (!resp.ok && report.fallbackEndpoint) {
      resp = await fetch(
        `${report.fallbackEndpoint}${dateFrom ? `?from=${dateFrom}&to=${dateTo}` : ""}`,
      );
    }
    if (report.format === "PDF") {
      window.open(url, "_blank");
      showToast(`Generating PDF... check your browser downloads.`, "info");
    } else if (report.format === "JSON") {
      const data = await resp.json();
      downloadJSON(data, `poool_${id}_${dateFrom}_${dateTo}.json`);
      showToast(`${report.title} downloaded!`, "success");
    } else {
      const data = await resp.json();
      const rows = extractRows(data);
      downloadCSV(rows, `poool_${id}_${dateFrom}_${dateTo}.csv`);
      showToast(`${report.title} downloaded!`, "success");
    }
  } catch (e) {
    showToast(`Failed to generate ${report.title}.`, "danger");
  }
  btn.innerHTML = origHTML;
  btn.disabled = false;
}

// ─── Preview ──────────────────────────────────────────────────────────────────
async function previewReport(id) {
  const report = REPORTS.find((r) => r.id === id);
  if (!report) return;
  document.getElementById("preview-title").textContent =
    `Preview: ${report.title}`;
  const dateFrom = document.getElementById("range-from")?.value || "";
  const dateTo = document.getElementById("range-to")?.value || "";
  let rows = [];
  try {
    const url = `${report.endpoint}${dateFrom ? `?from=${dateFrom}&to=${dateTo}` : ""}`;
    let resp = await fetch(url);
    if (!resp.ok && report.fallbackEndpoint) {
      const fallbackUrl = `${report.fallbackEndpoint}${dateFrom ? `?from=${dateFrom}&to=${dateTo}` : ""}`;
      resp = await fetch(fallbackUrl);
    }
    if (!resp.ok) throw new Error();
    const data = await resp.json();
    rows = extractRows(data).slice(0, 5);
  } catch (e) {
    showToast(`Failed to load preview for ${report.title}.`, "danger");
    return;
  }
  const section = document.getElementById("preview-section");
  if (rows.length > 0) {
    const headers = Object.keys(rows[0]);
    document.getElementById("preview-thead").innerHTML =
      `<tr>${headers.map((h) => `<th>${esc(h.replace(/_/g, " "))}</th>`).join("")}</tr>`;
    document.getElementById("preview-tbody").innerHTML = rows
      .map(
        (r) =>
          `<tr>${headers.map((h) => `<td style="max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(String(r[h] ?? "—"))}</td>`).join("")}</tr>`,
      )
      .join("");
    document.getElementById("preview-footer").textContent =
      `Showing ${rows.length} of ${report.title} records. Set date range above before downloading.`;
    section.style.display = "";
    section.scrollIntoView({ behavior: "smooth", block: "nearest" });
  } else {
    showToast(`No data available for ${report.title}.`, "warning");
  }
}

// ─── Data helpers ─────────────────────────────────────────────────────────────
function extractRows(data) {
  // The new /api/admin/reports/:type API returns { report_type, date_from, date_to, rows }
  if (data && Array.isArray(data.rows)) return data.rows;
  if (Array.isArray(data)) return data;
  const key = Object.keys(data).find(
    (k) => Array.isArray(data[k]) && data[k].length > 0,
  );
  return key ? data[key] : [];
}

function downloadCSV(rows, filename) {
  if (!rows.length) {
    showToast("No data to export.", "warning");
    return;
  }
  const headers = Object.keys(rows[0]);
  const csv = [
    headers.join(","),
    ...rows.map((r) =>
      headers
        .map((h) => `"${String(r[h] ?? "").replace(/"/g, '""')}"`)
        .join(","),
    ),
  ].join("\n");
  triggerDownload(new Blob([csv], { type: "text/csv" }), filename);
}

function downloadJSON(data, filename) {
  triggerDownload(
    new Blob([JSON.stringify(data, null, 2)], { type: "application/json" }),
    filename,
  );
}

function triggerDownload(blob, filename) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = filename;
  a.click();
}

function showToast(msg, type = "success") {
  const colors = {
    success: "var(--admin-success)",
    info: "var(--admin-info)",
    warning: "var(--admin-warning)",
    danger: "var(--admin-danger)",
  };
  const t = document.createElement("div");
  t.style.cssText = `position:fixed;bottom:24px;right:24px;z-index:9999;padding:12px 20px;border-radius:8px;font-size:13px;font-weight:500;background:${colors[type] || colors.success};color:#fff;box-shadow:0 4px 20px rgba(0,0,0,0.2);animation:admin-fadeIn 0.25s ease;`;
  t.textContent = msg;
  document.body.appendChild(t);
  setTimeout(() => t.remove(), 3000);
}

function esc(s) {
  if (typeof s !== "string") return s || "";
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}
