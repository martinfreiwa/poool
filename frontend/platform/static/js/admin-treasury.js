// State
let treasuryData = {};
let allTx = []; // Holds the current page rows
let totalCount = 0;
let currentPage = 1;
const PAGE_SIZE = 15;
let sortField = "created_at";
let sortOrder = "desc";
let localSearchTimer = null;

document.addEventListener("DOMContentLoaded", () => {
  loadTreasury();
  document.getElementById("tx-type-filter")?.addEventListener("change", () => {
    currentPage = 1;
    loadTreasury();
  });
  document.getElementById("tx-status-filter")?.addEventListener("change", () => {
    currentPage = 1;
    loadTreasury();
  });
  document.getElementById("tx-start-date")?.addEventListener("change", () => {
    currentPage = 1;
    loadTreasury();
  });
  document.getElementById("tx-end-date")?.addEventListener("change", () => {
    currentPage = 1;
    loadTreasury();
  });
  document.getElementById("treasury-local-search")?.addEventListener("input", (e) => {
    clearTimeout(localSearchTimer);
    localSearchTimer = setTimeout(() => {
      currentPage = 1;
      loadTreasury();
    }, 400); // 400ms debounce
  });

  setupSorting();
  setupPagination();
});

async function loadTreasury() {
  const tbody = document.getElementById("tx-table-body");
  if (tbody) {
    tbody.innerHTML = `<tr><td colspan="6" style="text-align: center; padding: 40px; color: var(--admin-text-muted);">
      <div style="margin: 0 auto 12px; width: 24px; height: 24px; border: 2px solid var(--admin-border); border-top-color: var(--admin-accent); border-radius: 50%; animation: spin 0.8s linear infinite;"></div>
      Loading…
    </td></tr>`;
  }

  try {
    const typeFilter = document.getElementById("tx-type-filter")?.value || "";
    const statusFilter = document.getElementById("tx-status-filter")?.value || "";
    const startDate = document.getElementById("tx-start-date")?.value || "";
    const endDate = document.getElementById("tx-end-date")?.value || "";
    const search = document.getElementById("treasury-local-search")?.value || "";

    const params = new URLSearchParams({
      page: currentPage,
      limit: PAGE_SIZE,
      sort_by: sortField,
      sort_order: sortOrder,
    });
    
    if (typeFilter) params.append("tx_type", typeFilter);
    if (statusFilter) params.append("status", statusFilter);
    if (startDate) params.append("start_date", startDate);
    if (endDate) params.append("end_date", endDate);
    if (search) params.append("search", search);

    const resp = await fetch(`/api/admin/treasury?${params.toString()}`);
    if (resp.ok) {
      treasuryData = await resp.json();
      allTx = treasuryData.recent_transactions || [];
      totalCount = treasuryData.total_count || 0;
      
      renderKPIs();
      renderTypeBreakdown();
      renderDividends();
      renderTransactions();
    } else {
      console.error('Treasury API error:', resp.status);
      showErrorState();
    }
  } catch (e) {
    console.error('Treasury fetch failed:', e);
    if (window.Sentry) Sentry.captureException(e);
    showErrorState();
  }
}

function showErrorState() {
  const tbody = document.getElementById("tx-table-body");
  if (tbody) {
    tbody.innerHTML = `
      <tr><td colspan="6" style="text-align:center;padding:40px;color:var(--admin-danger);">
        Failed to load treasury data. 
        <button class="admin-btn admin-btn--sm" style="margin-top: 10px;" onclick="loadTreasury()">Try Again</button>
      </td></tr>`;
  }
}

function showToast(msg, type = "info") {
  const container = document.getElementById("admin-toast-container");
  if (!container) return;
  const tn = document.createElement("div");
  tn.style.padding = "10px 16px";
  tn.style.borderRadius = "4px";
  tn.style.color = "white";
  tn.style.fontSize = "14px";
  tn.style.background = type === "error" ? "var(--admin-danger)" : type === "success" ? "var(--admin-success)" : "var(--admin-info)";
  tn.style.boxShadow = "0 4px 12px rgba(0,0,0,0.15)";
  tn.style.transition = "opacity 0.3s";
  tn.textContent = msg;
  container.appendChild(tn);
  setTimeout(() => {
    tn.style.opacity = "0";
    setTimeout(() => tn.remove(), 300);
  }, 3000);
}

function setupSorting() {
  const table = document.querySelector(".admin-table");
  if (!table) return;
  table.querySelectorAll(".admin-sort-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const field = btn.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      
      // Update ARIA attributes
      table.querySelectorAll("th[aria-sort]").forEach(th => th.setAttribute("aria-sort", "none"));
      btn.closest("th").setAttribute("aria-sort", sortOrder === "asc" ? "ascending" : "descending");

      currentPage = 1;
      loadTreasury();
    });
  });
}

function setupPagination() {
  document.getElementById("prev-page")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      loadTreasury();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    const totalPages = Math.ceil(totalCount / PAGE_SIZE);
    if (currentPage < totalPages) {
      currentPage++;
      loadTreasury();
    }
  });
}

function renderKPIs() {
  const s = treasuryData.stats || {};
  const el = (id) => document.getElementById(id);
  if (el("stat-total-balance")) el("stat-total-balance").textContent = formatUSD(s.total_balance_cents || 0);
  if (el("stat-wallet-count")) el("stat-wallet-count").textContent = `across ${s.wallet_count || 0} wallets`;
  
  // Calculate filtered totals dynamically based on visible type breakdown
  const breakdown = treasuryData.type_breakdown || [];
  const dt = breakdown.find(b => b.type === 'deposit') || {total_cents: 0, count: 0};
  const wt = breakdown.find(b => b.type === 'withdrawal') || {total_cents: 0, count: 0};
  const ft = breakdown.find(b => b.type === 'fee') || {total_cents: 0, count: 0};

  if (el("stat-total-deposits")) el("stat-total-deposits").textContent = formatUSD(dt.total_cents);
  if (el("stat-deposit-count")) el("stat-deposit-count").textContent = `${dt.count} transactions`;
  
  if (el("stat-total-withdrawals")) el("stat-total-withdrawals").textContent = formatUSD(wt.total_cents);
  if (el("stat-withdraw-count")) el("stat-withdraw-count").textContent = `${wt.count} transactions`;
  
  if (el("stat-net-revenue")) el("stat-net-revenue").textContent = formatUSD(ft.total_cents);
}

function renderTypeBreakdown() {
  const breakdown = treasuryData.type_breakdown || [];
  const container = document.getElementById("tx-type-breakdown");
  if (!breakdown.length) {
    container.innerHTML = '<div style="font-size:13px;color:var(--admin-text-muted);padding:12px 0;">No transaction data yet.</div>';
    return;
  }

  const maxVal = Math.max(...breakdown.map((b) => Math.abs(b.total_cents)));

  container.innerHTML = breakdown
    .map((b) => {
      const pct = maxVal > 0 ? Math.round((Math.abs(b.total_cents) / maxVal) * 100) : 0;
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
    }).join("");
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

  const totalPages = Math.max(1, Math.ceil(totalCount / PAGE_SIZE));
  
  if (!allTx.length) {
    tbody.innerHTML = '<tr><td colspan="6" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No transactions found.</td></tr>';
  } else {
    tbody.innerHTML = allTx.map((tx) => {
      const amountColor = tx.amount_cents >= 0 ? "var(--admin-success)" : "var(--admin-danger)";
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
    }).join("");
  }

  // Pagination UI
  const info = document.getElementById("pagination-info");
  if (info) info.textContent = `Page ${currentPage} of ${totalPages} (${totalCount} total)`;
  
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;
}

// ─── Export ─────────────────────────────────────────────────────

function exportTreasuryCSV() {
  if (totalCount === 0) {
    showToast("No data to export.", "error");
    return;
  }
  
  showToast("Generating CSV Export...", "info");
  
  const typeFilter = document.getElementById("tx-type-filter")?.value || "";
  const statusFilter = document.getElementById("tx-status-filter")?.value || "";
  const startDate = document.getElementById("tx-start-date")?.value || "";
  const endDate = document.getElementById("tx-end-date")?.value || "";

  const params = new URLSearchParams();
  if (typeFilter) params.append("tx_type", typeFilter);
  if (statusFilter) params.append("status", statusFilter);
  if (startDate) params.append("start_date", startDate);
  if (endDate) params.append("end_date", endDate);

  const downloadUrl = `/api/admin/treasury/export?${params.toString()}`;
  
  const a = document.createElement("a");
  a.href = downloadUrl;
  a.click();
  
  setTimeout(() => showToast("Export Downloaded", "success"), 1000);
}

// ─── Helpers ────────────────────────────────────────────────────

function esc(s) {
  if (typeof s !== "string") return s || "";
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

function formatUSD(c) {
  return "$" + (Math.abs(c || 0) / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

function formatDate(iso) {
  if (!iso) return "—";
  return new Date(iso).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
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
