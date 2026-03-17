/**
 * Admin Live Assets JS
 * Manages published marketplace assets with funding progress.
 */

let allAssets = [];
let filteredAssets = [];
let currentPage = 1;
const PAGE_SIZE = 10;
let sortField = "title";
let sortOrder = "asc";

document.addEventListener("DOMContentLoaded", () => {
  loadAssets();
  document
    .getElementById("asset-search")
    ?.addEventListener("input", debounce(applyFilters, 200));
  document
    .getElementById("filter-type")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("filter-status")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("filter-featured")
    ?.addEventListener("change", applyFilters);

  setupSorting();
  setupPagination();
});

function setupSorting() {
  const table = document.querySelector(".admin-table");
  if (!table) return;
  const headers = table.querySelectorAll("th[data-sort]");
  headers.forEach((th) => {
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
    const maxPage = Math.ceil(filteredAssets.length / PAGE_SIZE);
    if (currentPage < maxPage) {
      currentPage++;
      renderTable();
    }
  });
}

async function loadAssets() {
  try {
    const resp = await fetch("/api/admin/assets");
    if (resp.ok) {
      const data = await resp.json();
      allAssets = data.assets || data;
      applyFilters();
      updateStats();
    } else {
    }
  } catch (e) {
    console.error("Error loading assets", e);
    if (typeof Sentry !== 'undefined') Sentry.captureException(e);
  }
}

function updateStats() {
  const totalEl = document.getElementById("stat-total");
  if (totalEl) totalEl.textContent = allAssets.length;

  const fundingEl = document.getElementById("stat-funding");
  if (fundingEl)
    fundingEl.textContent = allAssets.filter(
      (a) =>
        a.funding_status === "funding_open" ||
        a.funding_status === "funding_in_progress",
    ).length;

  const fundedEl = document.getElementById("stat-funded");
  if (fundedEl)
    fundedEl.textContent = allAssets.filter((a) =>
      ["funded", "rented", "exited"].includes(a.funding_status),
    ).length;

  const aumEl = document.getElementById("stat-aum");
  if (aumEl) {
    const aum = allAssets.reduce((s, a) => s + (a.total_value_cents || 0), 0);
    aumEl.textContent = formatUSD(aum);
  }

  const soldEl = document.getElementById("stat-tokens-sold");
  if (soldEl) {
    const sold = allAssets.reduce(
      (s, a) => s + ((a.tokens_total || 0) - (a.tokens_available || 0)),
      0,
    );
    soldEl.textContent = sold.toLocaleString();
  }
}

function applyFilters() {
  const search = (
    document.getElementById("asset-search")?.value || ""
  ).toLowerCase();
  const type = document.getElementById("filter-type")?.value || "";
  const status = document.getElementById("filter-status")?.value || "";
  const featured = document.getElementById("filter-featured")?.checked || false;

  let result = allAssets.filter((a) => {
    if (type && a.asset_type !== type) return false;
    if (status && a.funding_status !== status) return false;
    if (featured && !a.featured) return false;
    if (
      search &&
      !`${a.title} ${a.location_city} ${a.slug}`.toLowerCase().includes(search)
    )
      return false;
    return true;
  });

  // Sort Result
  result.sort((a, b) => {
    let valA = a[sortField];
    let valB = b[sortField];

    if (sortField === "funding_progress") {
      valA =
        a.tokens_total > 0
          ? (a.tokens_total - a.tokens_available) / a.tokens_total
          : 0;
      valB =
        b.tokens_total > 0
          ? (b.tokens_total - b.tokens_available) / b.tokens_total
          : 0;
    }

    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredAssets = result;
  currentPage = 1;
  document.getElementById("asset-count-label").textContent =
    `${filteredAssets.length} assets`;
  renderTable();
}

function renderTable() {
  const tbody = document.getElementById("assets-table-body");
  if (!tbody) return;

  const totalPages = Math.max(1, Math.ceil(filteredAssets.length / PAGE_SIZE));
  currentPage = Math.min(currentPage, totalPages);
  const start = (currentPage - 1) * PAGE_SIZE;
  const slice = filteredAssets.slice(start, start + PAGE_SIZE);

  if (slice.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="9" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No assets match your filters.</td></tr>';
    return;
  }

  // Update Pagination Info
  const info = document.getElementById("pagination-info");
  if (info)
    info.textContent = `Page ${currentPage} of ${totalPages} (${filteredAssets.length} total)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages;

  tbody.innerHTML = slice
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
                <div style="font-weight:600;color:var(--admin-text-primary);margin-bottom:2px;">${esc(a.title)}</div>
                <div style="font-size:11px;color:var(--admin-text-muted);">${esc(a.slug)}</div>
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
                <div style="font-size:11px;color:var(--admin-text-muted);margin-top:2px;">${sold.toLocaleString()} / ${(a.tokens_total || 0).toLocaleString()} tokens</div>
            </td>
            <td style="font-variant-numeric:tabular-nums;">${a.annual_yield_bps ? (a.annual_yield_bps / 100).toFixed(1) + "%" : "—"}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${esc(a.location_city || "—")}</td>
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
                    <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="toggleFeatured('${esc(a.id)}')" title="Toggle featured">
                        <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M8 1l2 4 4.5.7-3.3 3.2.8 4.6L8 11.3 3.9 13.5l.8-4.6L1.5 5.7 6 5z"/></svg>
                    </button>
                    <a href="/property/${esc(a.slug)}" target="_blank" class="admin-btn admin-btn--secondary admin-btn--sm" title="View on marketplace">
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
  const asset = allAssets.find((a) => a.id === id);
  if (!asset) return;
  try {
    const resp = await fetch(`/api/admin/assets/${id}/toggle-featured`, {
      method: "POST",
    });
    if (resp.ok) {
      loadAssets();
      return;
    } else {
      console.error("Failed to toggle featured status");
    }
  } catch (e) {
    console.error("Error toggling featured status", e);
    if (typeof Sentry !== 'undefined') Sentry.captureException(e);
  }
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
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    })
  );
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
