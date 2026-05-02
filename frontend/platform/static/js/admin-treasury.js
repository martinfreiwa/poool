// State
let treasuryData = {};
let allTx = [];
let filteredTx = [];
let currentPage = 1;
const PAGE_SIZE = 15;
let sortField = "created_at";
let sortOrder = "desc";

document.addEventListener("DOMContentLoaded", () => {
  if (window.AdminPageKit) AdminPageKit.injectScopedCss();
  loadTreasury();
  if (window.AdminPageKit) {
    AdminPageKit.setupAutoRefresh({
      refreshFn: () => loadTreasury(),
      intervalMs: 60000,
    });
  }
  document
    .getElementById("tx-type-filter")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("tx-status-filter")
    ?.addEventListener("change", applyFilters);

  setupSorting();
  setupPagination();
});

async function loadTreasury() {
  try {
    const resp = await fetch("/api/admin/treasury");
    if (resp.ok) {
      treasuryData = await resp.json();
      allTx = treasuryData.recent_transactions || [];
      applyFilters();
      renderKPIs();
      renderTypeBreakdown();
      renderDividends();
    } else {
      console.error('Treasury API error:', resp.status);
    }
  } catch (e) {
    console.error('Treasury fetch failed:', e);
    if (window.Sentry) Sentry.captureException(e);
  }
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
      renderTransactions();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    const totalPages = Math.ceil(filteredTx.length / PAGE_SIZE);
    if (currentPage < totalPages) {
      currentPage++;
      renderTransactions();
    }
  });
}

function applyFilters() {
  const typeFilter = document.getElementById("tx-type-filter")?.value || "";
  const statusFilter = document.getElementById("tx-status-filter")?.value || "";
  let result = allTx;
  if (typeFilter) result = result.filter((t) => t.type === typeFilter);
  if (statusFilter) result = result.filter((t) => t.status === statusFilter);

  // Sort
  result.sort((a, b) => {
    let valA = a[sortField];
    let valB = b[sortField];
    if (sortField === "created_at") {
      valA = new Date(valA || 0).getTime();
      valB = new Date(valB || 0).getTime();
    }
    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredTx = result;
  currentPage = 1;
  renderTransactions();
}

function renderKPIs() {
  const s = treasuryData.stats || {};
  const el = (id) => document.getElementById(id);
  if (el("stat-total-balance"))
    el("stat-total-balance").textContent = formatUSD(
      s.total_balance_cents || 0,
    );
  if (el("stat-wallet-count"))
    el("stat-wallet-count").textContent =
      `across ${s.wallet_count || 0} wallets`;
  if (el("stat-total-deposits"))
    el("stat-total-deposits").textContent = formatUSD(
      s.total_deposits_cents || 0,
    );
  if (el("stat-deposit-count"))
    el("stat-deposit-count").textContent =
      `${s.deposit_count || 0} transactions`;
  if (el("stat-total-withdrawals"))
    el("stat-total-withdrawals").textContent = formatUSD(
      s.total_withdrawals_cents || 0,
    );
  if (el("stat-withdraw-count"))
    el("stat-withdraw-count").textContent =
      `${s.withdrawal_count || 0} transactions`;
  if (el("stat-net-revenue"))
    el("stat-net-revenue").textContent = formatUSD(s.net_revenue_cents || 0);
}

function renderTypeBreakdown() {
  const breakdown = treasuryData.type_breakdown || [];
  const container = document.getElementById("tx-type-breakdown");
  if (!breakdown.length) {
    container.innerHTML =
      '<div style="font-size:13px;color:var(--admin-text-muted);padding:12px 0;">No transaction data yet.</div>';
    return;
  }

  const maxVal = Math.max(...breakdown.map((b) => Math.abs(b.total_cents)));

  container.innerHTML = breakdown
    .map((b) => {
      const pct =
        maxVal > 0 ? Math.round((Math.abs(b.total_cents) / maxVal) * 100) : 0;
      const color =
        b.type === "deposit"
          ? "var(--admin-success)"
          : b.type === "withdrawal"
            ? "var(--admin-danger)"
            : b.type === "purchase"
              ? "var(--admin-info)"
              : b.type === "dividend"
                ? "var(--admin-warning)"
                : "var(--admin-text-muted)";
      return `
        <div style="margin-bottom:12px;">
            <div style="display:flex;justify-content:space-between;font-size:12px;margin-bottom:4px;">
                <span style="font-weight:500;color:var(--admin-text-primary);text-transform:capitalize;">${b.type}</span>
                <span style="font-weight:600;font-variant-numeric:tabular-nums;color:var(--admin-text-secondary);">${formatUSD(b.total_cents)} · ${b.count} txns</span>
            </div>
            <div style="height:6px;background:var(--admin-border);border-radius:3px;overflow:hidden;">
                <div style="width:${pct}%;height:100%;background:${color};border-radius:3px;transition:width 0.4s;"></div>
            </div>
        </div>`;
    })
    .join("");
}

function renderDividends() {
  const d = treasuryData.dividend_stats || {};
  const container = document.getElementById("dividend-stats");

  container.innerHTML = `
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px;">
            <div style="padding:16px;background:var(--admin-bg);border:1px solid var(--admin-border);border-radius:var(--admin-radius-md);">
                <div style="font-size:11px;color:var(--admin-text-muted);margin-bottom:4px;">Total Paid</div>
                <div style="font-size:20px;font-weight:700;color:var(--admin-success);">${formatUSD(d.total_paid_cents || 0)}</div>
                <div style="font-size:11px;color:var(--admin-text-muted);margin-top:2px;">${d.paid_count || 0} payouts</div>
            </div>
            <div style="padding:16px;background:var(--admin-bg);border:1px solid var(--admin-border);border-radius:var(--admin-radius-md);">
                <div style="font-size:11px;color:var(--admin-text-muted);margin-bottom:4px;">Scheduled</div>
                <div style="font-size:20px;font-weight:700;color:var(--admin-warning);">${formatUSD(d.scheduled_cents || 0)}</div>
                <div style="font-size:11px;color:var(--admin-text-muted);margin-top:2px;">${d.scheduled_count || 0} upcoming</div>
            </div>
            <div style="padding:16px;background:var(--admin-bg);border:1px solid var(--admin-border);border-radius:var(--admin-radius-md);">
                <div style="font-size:11px;color:var(--admin-text-muted);margin-bottom:4px;">Processing</div>
                <div style="font-size:20px;font-weight:700;color:var(--admin-info);">${formatUSD(d.processing_cents || 0)}</div>
                <div style="font-size:11px;color:var(--admin-text-muted);margin-top:2px;">${d.processing_count || 0} in progress</div>
            </div>
            <div style="padding:16px;background:var(--admin-bg);border:1px solid var(--admin-border);border-radius:var(--admin-radius-md);">
                <div style="font-size:11px;color:var(--admin-text-muted);margin-bottom:4px;">Failed</div>
                <div style="font-size:20px;font-weight:700;color:var(--admin-danger);">${d.failed_count || 0}</div>
                <div style="font-size:11px;color:var(--admin-text-muted);margin-top:2px;">${formatUSD(d.failed_cents || 0)}</div>
            </div>
        </div>
    `;
}

function renderTransactions() {
  const tbody = document.getElementById("tx-table-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(filteredTx.length / PAGE_SIZE));
  currentPage = Math.min(currentPage, totalPages);
  const start = (currentPage - 1) * PAGE_SIZE;
  const slice = filteredTx.slice(start, start + PAGE_SIZE);

  if (!slice.length) {
    tbody.innerHTML =
      '<tr><td colspan="6" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No transactions found.</td></tr>';
    return;
  }

  // Pagination UI
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${filteredTx.length} total)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;

  tbody.innerHTML = slice
    .map((tx) => {
      const amountColor =
        tx.amount_cents >= 0 ? "var(--admin-success)" : "var(--admin-danger)";
      const sign = tx.amount_cents >= 0 ? "+" : "-";
      return `
        <tr>
            <td>
                <div class="admin-user-inline">
                    <div>
                        <div class="admin-user-inline-name">${esc(tx.user_name)}</div>
                        <div class="admin-user-inline-email">${esc(tx.user_email)}</div>
                    </div>
                </div>
            </td>
            <td>${typeBadge(tx.type)}</td>
            <td style="font-weight:700;font-variant-numeric:tabular-nums;color:${amountColor};">${sign}${formatUSD(tx.amount_cents)}</td>
            <td>${statusBadge(tx.status)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${esc(tx.description || "—")}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDate(tx.created_at)}</td>
        </tr>`;
    })
    .join("");
}

// ─── Export ─────────────────────────────────────────────────────

function exportTreasuryCSV() {
  if (!allTx || !allTx.length) {
    alert("No data to export.");
    return;
  }
  const headers = [
    "id",
    "type",
    "amount_cents",
    "status",
    "description",
    "user_name",
    "user_email",
    "created_at",
  ];
  const csvRows = [
    headers.join(","),
    ...allTx.map((tx) =>
      headers
        .map((h) => `"${String(tx[h] ?? "").replace(/"/g, '""')}"`)
        .join(","),
    ),
  ];
  const csv = csvRows.join("\n");
  const blob = new Blob([csv], { type: "text/csv" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  const date = new Date().toISOString().split("T")[0];
  a.download = `poool_treasury_export_${date}.csv`;
  a.click();
}

// ─── Helpers ────────────────────────────────────────────────────

function esc(s) {
  if (typeof s !== "string") return s || "";
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}
function formatUSD(c) {
  return (
    "$" +
    (Math.abs(c || 0) / 100).toLocaleString("en-US", {
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

function typeBadge(t) {
  const m = {
    deposit: ["admin-badge--success", "Deposit"],
    withdrawal: ["admin-badge--danger", "Withdrawal"],
    purchase: ["admin-badge--info", "Purchase"],
    sale: ["admin-badge--neutral", "Sale"],
    dividend: ["admin-badge--warning", "Dividend"],
    reward: ["admin-badge--info", "Reward"],
    refund: ["admin-badge--neutral", "Refund"],
    fee: ["admin-badge--neutral", "Fee"],
  };
  const [cls, label] = m[t] || ["admin-badge--neutral", t];
  return `<span class="admin-badge ${cls}">${label}</span>`;
}

function statusBadge(s) {
  const m = {
    pending: ["admin-badge--warning", "Pending"],
    processing: ["admin-badge--info", "Processing"],
    completed: ["admin-badge--success", "Completed"],
    failed: ["admin-badge--danger", "Failed"],
    cancelled: ["admin-badge--neutral", "Cancelled"],
  };
  const [cls, label] = m[s] || ["admin-badge--neutral", s];
  return `<span class="admin-badge ${cls}">${label}</span>`;
}
