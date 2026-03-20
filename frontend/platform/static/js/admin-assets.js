/**
 * Admin Live Assets JS
 * Manages published marketplace assets with funding progress.
 */

let currentData = [];
let currentPage = 1;
let totalAssets = 0;
const PAGE_SIZE = 10;
let sortField = "created_at";
let sortOrder = "desc";

document.addEventListener("DOMContentLoaded", () => {
  // Sync Variables from URL Params on Load
  const params = new URLSearchParams(window.location.search);
  if (params.has("search")) {
    const searchEl = document.getElementById("asset-search");
    if (searchEl) searchEl.value = params.get("search");
  }
  if (params.has("type")) {
    const typeEl = document.getElementById("filter-type");
    if (typeEl) typeEl.value = params.get("type");
  }
  if (params.has("status")) {
    const statusEl = document.getElementById("filter-status");
    if (statusEl) statusEl.value = params.get("status");
  }
  if (params.has("featured")) {
    const featEl = document.getElementById("filter-featured");
    if (featEl) featEl.checked = params.get("featured") === "true";
  }
  if (params.has("page")) {
    currentPage = parseInt(params.get("page"), 10) || 1;
  }
  if (params.has("sort")) sortField = params.get("sort");
  if (params.has("order")) sortOrder = params.get("order");

  loadAssets();
  
  document
    .getElementById("asset-search")
    ?.addEventListener("input", debounce(() => { currentPage = 1; fetchAssets(); }, 300));
  document
    .getElementById("filter-type")
    ?.addEventListener("change", () => { currentPage = 1; fetchAssets(); });
  document
    .getElementById("filter-status")
    ?.addEventListener("change", () => { currentPage = 1; fetchAssets(); });
  document
    .getElementById("filter-featured")
    ?.addEventListener("change", () => { currentPage = 1; fetchAssets(); });

  // Setup Event Delegation for Buttons
  const tbody = document.getElementById("assets-table-body");
  if (tbody) {
      tbody.addEventListener("click", e => {
          const featBtn = e.target.closest(".action-toggle-featured");
          if (featBtn) {
              const id = featBtn.dataset.assetId;
              if (id) toggleFeatured(id);
          }
      });
  }

  setupSorting();
  setupPagination();
});

function setupSorting() {
  const table = document.querySelector(".admin-table");
  if (!table) return;
  const headers = table.querySelectorAll("th[data-sort]");
  headers.forEach((th) => {
    th.style.cursor = "pointer";
    th.setAttribute("tabindex", "0");
    th.setAttribute("role", "button");
    const handleClick = () => {
      const field = th.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      currentPage = 1;
      fetchAssets();
    };
    th.addEventListener("click", handleClick);
    th.addEventListener("keydown", (e) => {
        if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            handleClick();
        }
    });
  });
}

function setupPagination() {
  document.getElementById("prev-page")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      fetchAssets();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    const maxPage = Math.ceil(totalAssets / PAGE_SIZE);
    if (currentPage < maxPage) {
      currentPage++;
      fetchAssets();
    }
  });
}

function fetchAssets() {
  // Read current filters
  const search = (document.getElementById("asset-search")?.value || "").trim();
  const type = document.getElementById("filter-type")?.value || "";
  const status = document.getElementById("filter-status")?.value || "";
  const featured = document.getElementById("filter-featured")?.checked || false;

  // Update URL
  const params = new URLSearchParams(window.location.search);
  if (search) params.set("search", search); else params.delete("search");
  if (type) params.set("type", type); else params.delete("type");
  if (status) params.set("status", status); else params.delete("status");
  if (featured) params.set("featured", "true"); else params.delete("featured");
  params.set("page", currentPage.toString());
  params.set("sort", sortField);
  params.set("order", sortOrder);
  window.history.replaceState({}, "", `${window.location.pathname}?${params.toString()}`);

  loadAssets();
}

async function loadAssets() {
  const tbody = document.getElementById("assets-table-body");
  if (tbody && currentData.length === 0) {
     tbody.innerHTML = `<tr><td colspan="9" style="text-align:center;padding:40px;color:var(--admin-text-muted);">
            <div style="margin: 0 auto 12px; width: 24px; height: 24px; border: 2px solid var(--admin-border); border-top-color: var(--admin-accent); border-radius: 50%; animation: spin 0.8s linear infinite;"></div>
            Loading assets…
        </td></tr>`;
  } else if (tbody) {
      tbody.style.opacity = "0.5";
  }

  try {
    const params = new URLSearchParams(window.location.search);
    const resp = await fetch(`/api/admin/assets?${params.toString()}`);
    if (resp.ok) {
      const data = await resp.json();
      currentData = data.assets || [];
      const stats = data.stats || {};
      totalAssets = stats.stat_total || 0;
      
      const countLabel = document.getElementById("asset-count-label");
      if (countLabel) countLabel.textContent = `${totalAssets} assets`;
      
      updateStats(stats);
      renderTable();
    } else {
        throw new Error("Failed network response.");
    }
  } catch (e) {
    console.error("Error loading assets", e);
    if (tbody) {
        tbody.style.opacity = "1";
        tbody.innerHTML = `<tr><td colspan="9" style="text-align:center;color:var(--admin-error);padding:40px;">Failed to load assets. <button class="admin-btn admin-btn--secondary" onclick="loadAssets()" style="margin-left: 12px">Retry</button></td></tr>`;
    }
    if (typeof Sentry !== 'undefined') Sentry.captureException(e);
  }
}

function updateStats(stats) {
  const totalEl = document.getElementById("stat-total");
  if (totalEl) totalEl.textContent = formatNumber(stats.stat_total || 0);

  const fundingEl = document.getElementById("stat-funding");
  if (fundingEl) fundingEl.textContent = formatNumber(stats.stat_funding || 0);

  const fundedEl = document.getElementById("stat-funded");
  if (fundedEl) fundedEl.textContent = formatNumber(stats.stat_funded || 0);

  const aumEl = document.getElementById("stat-aum");
  if (aumEl) aumEl.textContent = formatUSD(stats.stat_aum || 0);

  const soldEl = document.getElementById("stat-tokens-sold");
  if (soldEl) soldEl.textContent = formatNumber(stats.stat_tokens_sold || 0);
}

function renderTable() {
  const tbody = document.getElementById("assets-table-body");
  if (!tbody) return;
  tbody.style.opacity = "1";

  if (currentData.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="9" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No assets match your filters.</td></tr>';
    
    // Reset pagination info
    const info = document.getElementById("pagination-info");
    if (info) info.textContent = `Page 1 of 1 (0 total)`;
    const prevBtn = document.getElementById("prev-page");
    const nextBtn = document.getElementById("next-page");
    if (prevBtn) prevBtn.disabled = true;
    if (nextBtn) nextBtn.disabled = true;
    return;
  }

  const totalPages = Math.max(1, Math.ceil(totalAssets / PAGE_SIZE));
  
  // Update Pagination Info
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${totalAssets} total)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;

  tbody.innerHTML = currentData
    .map((a) => {
      const sold = (a.tokens_total || 0) - (a.tokens_available || 0);
      const pct =
        a.tokens_total > 0 ? Math.round((sold / a.tokens_total) * 100) : 0;
      const progressColor =
        pct >= 100
          ? "var(--admin-success)"
          : pct >= 50
            ? "var(--admin-info)"
            : "var(--admin-warning)";

      return `
        <tr>
            <td>
                <div style="font-weight:600;color:var(--admin-text-primary);margin-bottom:2px;">${escapeHtml(a.title)}</div>
                <div style="font-size:11px;color:var(--admin-text-muted);">${escapeHtml(a.slug)}</div>
            </td>
            <td>${typeBadge(a.asset_type)}</td>
            <td style="font-weight:700;font-variant-numeric:tabular-nums;">${formatUSD(a.total_value_cents)}</td>
            <td style="min-width:140px;">
                <div style="display:flex;align-items:center;gap:8px;">
                    <div style="flex:1;height:6px;background:var(--admin-border);border-radius:3px;overflow:hidden;">
                        <div style="width:${pct}%;height:100%;background:${progressColor};border-radius:3px;transition:width 0.4s;"></div>
                    </div>
                    <span style="font-size:11px;font-weight:600;color:var(--admin-text-secondary);width:36px;text-align:right;">${pct}%</span>
                </div>
                <div style="font-size:11px;color:var(--admin-text-muted);margin-top:2px;">${formatNumber(sold)} / ${formatNumber(a.tokens_total || 0)} tokens</div>
            </td>
            <td style="font-variant-numeric:tabular-nums;">${a.annual_yield_bps ? (a.annual_yield_bps / 100).toFixed(1) + "%" : "—"}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${escapeHtml(a.location_city || "—")}</td>
            <td>${statusBadge(a.funding_status)}</td>
            <td style="text-align:center;">
                ${a.featured
          ? `
                    <svg width="16" height="16" viewBox="0 0 20 20" fill="var(--admin-warning)" stroke="var(--admin-warning)" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" title="Featured">
                        <path d="M10 2l2.4 4.8L18 7.6l-4 3.9.9 5.5L10 14.6 5.1 17l.9-5.5-4-3.9 5.6-.8L10 2z" />
                    </svg>
                `
          : `
                    <svg width="16" height="16" viewBox="0 0 20 20" fill="none" stroke="var(--admin-text-muted)" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M10 2l2.4 4.8L18 7.6l-4 3.9.9 5.5L10 14.6 5.1 17l.9-5.5-4-3.9 5.6-.8L10 2z" />
                    </svg>
                `
        }
            </td>
            <td>
                <div style="display:flex;gap:4px;">
                    <button class="admin-btn admin-btn--secondary admin-btn--sm action-toggle-featured" data-asset-id="${escapeHtml(a.id)}" title="Toggle featured">
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M8 1l2 4 4.5.7-3.3 3.2.8 4.6L8 11.3 3.9 13.5l.8-4.6L1.5 5.7 6 5z"/></svg>
                    </button>
                    <a href="/property/${encodeURIComponent(a.slug)}" target="_blank" class="admin-btn admin-btn--secondary admin-btn--sm" title="View on marketplace">
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M6 2H3a1 1 0 00-1 1v10a1 1 0 001 1h10a1 1 0 001-1v-3"/><path d="M9 2h5v5M14 2L7 9"/></svg>
                    </a>
                </div>
            </td>
        </tr>
        `;
    })
    .join("");
}

async function toggleFeatured(id) {
  const asset = currentData.find((a) => a.id === id);
  if (!asset) return;

  // Optimistic UI update
  asset.featured = !asset.featured;
  renderTable();

  try {
    const resp = await fetch(`/api/admin/assets/${encodeURIComponent(id)}/toggle-featured`, {
      method: "POST",
    });
    if (resp.ok) {
      showToast("success", "Asset featured status updated");
      return;
    } else {
      console.error("Failed to toggle featured status");
      asset.featured = !asset.featured; // Revert
      renderTable();
      showToast("error", "Failed to update asset.");
    }
  } catch (e) {
    console.error("Error toggling featured status", e);
    asset.featured = !asset.featured; // Revert
    renderTable();
    showToast("error", "Network error updating asset.");
    if (typeof Sentry !== 'undefined') Sentry.captureException(e);
  }
}

// Simple Toast Helper to provide user feedback
function showToast(type, msg) {
    const d = document.createElement("div");
    d.style.position = "fixed";
    d.style.bottom = "20px";
    d.style.right = "20px";
    d.style.padding = "12px 20px";
    d.style.borderRadius = "8px";
    d.style.background = type === "success" ? "var(--admin-success)" : "var(--admin-error)";
    d.style.color = "white";
    d.style.fontWeight = "600";
    d.style.zIndex = "9999";
    d.style.boxShadow = "0 10px 15px -3px rgba(0,0,0,0.1)";
    d.textContent = msg;
    document.body.appendChild(d);
    setTimeout(() => {d.style.opacity="0"; d.style.transition="opacity 0.4s"; setTimeout(()=>d.remove(),400)}, 3000);
}

// ─── Helpers ────────────────────────────────────────────────────

function formatUSD(c) {
  return (
    "$" +
    (Math.abs(c || 0) / 100).toLocaleString("en-US", {
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    })
  );
}

function formatNumber(num) {
  return Number(num || 0).toLocaleString("en-US");
}

function debounce(fn, ms) {
  let t;
  return function (...a) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, a), ms);
  };
}

function typeBadge(t) {
  const m = {
    real_estate: "Real Estate",
    commercial_property: "Commercial",
    commodity: "Commodity",
    business: "Business",
    startup: "Startup",
    land_plot: "Land",
  };
  return `<span class="admin-badge admin-badge--neutral">${m[t] || t}</span>`;
}

function statusBadge(s) {
  const m = {
    upcoming: ["admin-badge--neutral", "Upcoming"],
    funding_open: ["admin-badge--success", "Funding Open"],
    funding_in_progress: ["admin-badge--info", "In Progress"],
    funded: ["admin-badge--info", "Funded"],
    rented: ["admin-badge--success", "Rented"],
    payout_pending: ["admin-badge--warning", "Payout"],
    exited: ["admin-badge--neutral", "Exited"],
  };
  const [cls, label] = m[s] || ["admin-badge--neutral", s];
  return `<span class="admin-badge ${cls}">${label}</span>`;
}
