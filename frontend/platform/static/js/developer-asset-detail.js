/**
 * Admin Asset Details — Full tabbed interface
 * Fetches from GET /api/admin/assets/:id/detail
 */

let assetData = null;
let assetId = null;

// ─── Init ─────────────────────────────────────────────────────
document.addEventListener("DOMContentLoaded", () => {
  const params = new URLSearchParams(window.location.search);
  assetId = params.get("id");

  if (!assetId) {
    showError("No Asset ID provided.");
    return;
  }

  loadAsset();

  // Tab switching
  document.getElementById("asset-tabs")?.addEventListener("click", (e) => {
    const tab = e.target.closest(".ad-tabs__item");
    if (!tab) return;
    document
      .querySelectorAll(".ad-tabs__item")
      .forEach((t) => t.classList.remove("active"));
    document
      .querySelectorAll(".tab-panel")
      .forEach((p) => p.classList.remove("active"));
    tab.classList.add("active");
    const panel = document.getElementById(`panel-${tab.dataset.tab}`);
    if (panel) panel.classList.add("active");
  });

  // Refresh
  document.getElementById("btn-refresh")?.addEventListener("click", loadAsset);

  // Settings Toggles
  document
    .getElementById("toggle-featured")
    ?.addEventListener("click", toggleFeatured);
  document
    .getElementById("toggle-published")
    ?.addEventListener("click", togglePublished);

  // Danger zone
  document
    .getElementById("btn-freeze")
    ?.addEventListener("click", () => dangerAction("freeze"));
  document
    .getElementById("btn-unpublish")
    ?.addEventListener("click", () => dangerAction("unpublish"));
  document
    .getElementById("btn-archive")
    ?.addEventListener("click", () => dangerAction("archive"));
});

// ─── Load ─────────────────────────────────────────────────────
async function loadAsset() {
  try {
    const resp = await fetch(`/api/developer/assets/${assetId}`);
    if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
    assetData = await resp.json();
    renderAll(assetData);
  } catch (err) {
    showError(`Failed to load asset details: ${err.message}`);
  }
}

function showError(msg) {
  const el = document.getElementById("loading-overlay");
  if (el)
    el.innerHTML = `
        <div style="color:var(--btn-danger-bg, #D92D20);font-weight:700;margin-bottom:12px;">${esc(msg)}</div>
        <a href="/developer/dashboard" class="admin-btn admin-btn--secondary">← Back to Dashboard</a>
    `;
}

// ─── Render All ───────────────────────────────────────────────
function renderAll(a) {
  document.getElementById("loading-overlay").style.display = "none";
  document.getElementById("asset-content").style.display = "block";

  // Header
  document.getElementById("breadcrumb-asset-title").textContent = a.title;
  document.getElementById("asset-title-main").innerHTML = `${esc(a.title)} <code style="font-family:monospace;font-size:14px;padding:2px 8px;background:var(--bg-surface, #f4f4f5);border:1px solid var(--border-color, #e4e4e7);border-radius:4px;color:var(--text-secondary, #52525b);font-weight:500;margin-left:8px;vertical-align:middle;">#APP-${(a.id || '').substring(0, 6).toUpperCase()}</code>`;
  const pageHeading = document.getElementById("page-heading-title");
  if (pageHeading) pageHeading.textContent = a.title;
  document.getElementById("asset-location").textContent =
    [a.city, formatCountry(a.country)].filter(Boolean).join(", ") || "Location not specified";

  // Status badges
  const statusBadge = document.getElementById("asset-status-badge");
  statusBadge.textContent = a.published ? "LIVE" : "DRAFT";
  statusBadge.className = `ad-badge ${a.published ? "ad-badge--success" : "ad-badge--warning"}`;

  const fundingBadge = document.getElementById("asset-funding-badge");
  const fundingMap = {
    pre_funding: ["Pre-Funding", "ad-badge--neutral"],
    funding_open: ["Funding Open", "ad-badge--info"],
    fully_funded: ["Fully Funded", "ad-badge--success"],
    closed: ["Closed", "ad-badge--danger"],
    exited: ["Exited", "ad-badge--danger"],
  };
  const [fLabel, fClass] = fundingMap[a.funding_status] || [
    a.funding_status || "Unknown",
    "ad-badge--neutral",
  ];
  fundingBadge.textContent = fLabel;
  fundingBadge.className = `ad-badge ${fClass}`;

  // Funding progress
  const sold = (a.tokens_total || 0) - (a.tokens_available || 0);
  const pct = a.tokens_total ? Math.round((sold / a.tokens_total) * 100) : 0;
  document.getElementById("funding-label").textContent =
    `${sold.toLocaleString()} of ${(a.tokens_total || 0).toLocaleString()} tokens sold`;
  document.getElementById("funding-pct").textContent = `${pct}%`;
  document.getElementById("funding-bar").style.width = `${pct}%`;

  // Header stats
  document.getElementById("asset-valuation").textContent = formatUSD(
    a.total_value_cents,
  );
  document.getElementById("asset-token-price").textContent = formatUSD(
    a.token_price_cents,
  );
  document.getElementById("asset-yield").textContent = bpsToPercent(
    a.annual_yield_bps,
  );
  document.getElementById("asset-type").textContent = formatAssetType(
    a.asset_type,
  );

  // Image
  if (a.images && a.images.length > 0) {
    const cover = a.images.find((i) => i.is_cover) || a.images[0];
    if (cover.url) document.getElementById("asset-image").src = cover.url;
  }

  // Tab badges
  const badgeInvestors = document.getElementById("badge-investors");
  if (badgeInvestors) badgeInvestors.textContent = (a.investors || []).length;
  const badgeOrders = document.getElementById("badge-orders");
  if (badgeOrders) badgeOrders.textContent = (a.orders || []).length;

  renderOverview(a);
  renderMedia(a);
  renderDocuments(a);
  renderFinancials(a);
  renderMilestones(a);
  renderCapTable(a);
  renderOrders(a);
  renderSettings(a);
}

// ─── Tab: Overview ────────────────────────────────────────────
function renderOverview(a) {
  document.getElementById("asset-description").textContent =
    a.description || "No description provided.";

  // Property Details grid
  const details = [
    ["Asset Type", formatAssetType(a.asset_type)],
    ["Property Type", a.property_type || "—"],
    ["Construction", a.construction_status || "—"],
    ["Slug", a.slug || "—"],
    ["Appreciation", bpsToPercent(a.capital_appreciation_bps)],
    ["Occupancy", bpsToPercent(a.occupancy_rate_bps)],
  ];
  document.getElementById("property-details").innerHTML = details
    .map(
      ([label, value]) => `
        <div class="info-item">
            <span class="info-label">${label}</span>
            <span class="info-value">${esc(value)}</span>
        </div>
    `,
    )
    .join("");

  // Financial Summary
  document.getElementById("financial-summary").innerHTML = [
    ["Total Valuation", formatUSD(a.total_value_cents)],
    ["Token Price", formatUSD(a.token_price_cents)],
    ["Tokens Total", (a.tokens_total || 0).toLocaleString()],
    ["Tokens Available", (a.tokens_available || 0).toLocaleString()],
    ["Annual Yield", bpsToPercent(a.annual_yield_bps)],
  ]
    .map(
      ([l, v]) => `
        <div class="summary-row">
            <span class="summary-label">${l}</span>
            <span class="summary-value">${esc(v)}</span>
        </div>
    `,
    )
    .join("");

  // Quick Stats
  document.getElementById("quick-stats").innerHTML = [
    ["Investors", (a.investors || []).length],
    ["Documents", (a.documents || []).length],
    ["Images", (a.images || []).length],
    ["Milestones", (a.milestones || []).length],
    ["Orders", (a.orders || []).length],
  ]
    .map(
      ([l, v]) => `
        <div class="summary-row">
            <span class="summary-label">${l}</span>
            <span class="summary-value">${v}</span>
        </div>
    `,
    )
    .join("");
}

// ─── Tab: Media ───────────────────────────────────────────────
function renderMedia(a) {
  const images = a.images || [];
  document.getElementById("media-count").textContent =
    `${images.length} image${images.length !== 1 ? "s" : ""}`;

  if (images.length > 0) {
    document.getElementById("media-grid").innerHTML = images
      .map(
        (img) => `
            <div class="media-item" style="background-image:url('${esc(img.url)}')">
                ${img.is_cover ? '<span class="cover-badge">Cover</span>' : ""}
            </div>
        `,
      )
      .join("");
  }

  if (a.video_url) {
    document.getElementById("video-section").hidden = false;
    document.getElementById("video-link").href = a.video_url;
  }
}

// ─── Tab: Documents ───────────────────────────────────────────
function renderDocuments(a) {
  const docs = a.documents || [];
  if (docs.length === 0) return;

  const docIcon = `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="color:var(--label-color, #475467);"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>`;

  document.getElementById("documents-list").innerHTML = docs
    .map((d) => {
      const size = d.file_size
        ? `${(d.file_size / 1024 / 1024).toFixed(2)} MB`
        : "—";
      return `
            <div class="document-item">
                <div style="display:flex;align-items:center;gap:12px;">
                    ${docIcon}
                    <div>
                        <span class="document-type-badge">${esc(d.document_type.replace(/_/g, " "))}</span>
                        <span style="font-size:12px;color:var(--label-color, #475467);margin-left:8px;">${size}</span>
                    </div>
                </div>
                <a href="#" class="admin-btn admin-btn--secondary admin-btn--sm">View</a>
            </div>
        `;
    })
    .join("");
}

// ─── Tab: Financials ──────────────────────────────────────────
function renderFinancials(a) {
  const financials = a.financials || [];
  document.getElementById("financials-count").textContent =
    `${financials.length} record${financials.length !== 1 ? "s" : ""}`;

  if (financials.length === 0) return;

  const months = [
    "",
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
  ];
  document.getElementById("financials-tbody").innerHTML = financials
    .map(
      (f) => `
        <tr>
            <td style="font-weight:600;">${months[f.period_month] || f.period_month} ${f.period_year}</td>
            <td style="color:var(--badge-success-color, #027A48);font-weight:600;font-variant-numeric:tabular-nums;">${formatUSD(f.rental_income_cents)}</td>
            <td style="color:var(--btn-danger-bg, #D92D20);font-variant-numeric:tabular-nums;">${formatUSD(f.expenses_cents)}</td>
            <td style="font-weight:700;font-variant-numeric:tabular-nums;">${formatUSD(f.net_income_cents)}</td>
            <td>${bpsToPercent(f.occupancy_rate_bps)}</td>
        </tr>
    `,
    )
    .join("");
}

// ─── Tab: Milestones ──────────────────────────────────────────
function renderMilestones(a) {
  const milestones = a.milestones || [];
  if (milestones.length === 0) return;

  document.getElementById("milestones-list").innerHTML = milestones
    .map(
      (m, i) => `
        <div class="milestone-item">
            <div class="milestone-dot ${m.is_completed ? "done" : "pending"}">
                ${m.is_completed
          ? '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M3 8l3.5 3.5L13 4"/></svg>'
          : i + 1
        }
            </div>
            <div style="flex:1;">
                <div style="font-size:14px;font-weight:600;color:var(--value-color, #101828);margin-bottom:2px;">${esc(m.title)}</div>
                <div style="font-size:12px;color:var(--label-color, #475467);">${esc(m.description || "")}</div>
                ${m.month_index != null ? `<span style="font-size:11px;color:var(--label-color, #475467);margin-top:4px;display:inline-block;">Month ${m.month_index}</span>` : ""}
            </div>
            <span class="ad-badge ${m.is_completed ? "ad-badge--success" : "ad-badge--neutral"}">
                ${m.is_completed ? "Completed" : "Pending"}
            </span>
        </div>
    `,
    )
    .join("");
}

// ─── Tab: Cap Table ───────────────────────────────────────────
function renderCapTable(a) {
  const investors = a.investors || [];
  if (investors.length === 0) return;

  document.getElementById("captable-tbody").innerHTML = investors
    .map(
      (inv) => `
        <tr>
            <td>
                <span style="font-weight:600;">${esc(inv.name)}</span>
            </td>
            <td style="font-weight:600;font-variant-numeric:tabular-nums;">${inv.tokens_owned.toLocaleString()}</td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(inv.purchase_value_cents)}</td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(inv.current_value_cents)}</td>
            <td style="font-variant-numeric:tabular-nums;color:var(--badge-success-color, #027A48);">${formatUSD(inv.total_rental_cents)}</td>
            <td><span class="ad-badge ad-badge--${inv.status === "active" ? "success" : "neutral"}">${inv.status}</span></td>
        </tr>
    `,
    )
    .join("");
}

// ─── Tab: Orders ──────────────────────────────────────────────
function renderOrders(a) {
  const orders = a.orders || [];
  if (orders.length === 0) return;

  document.getElementById("orders-tbody").innerHTML = orders
    .map(
      (o) => `
        <tr>
            <td style="font-weight:600;font-family:monospace;font-size:13px;">${esc(o.order_number)}</td>
            <td>${esc(o.user_email)}</td>
            <td style="font-variant-numeric:tabular-nums;">${o.tokens}</td>
            <td style="font-weight:600;font-variant-numeric:tabular-nums;">${formatUSD(o.subtotal_cents)}</td>
            <td>${orderStatusBadge(o.status)}</td>
            <td style="font-size:12px;color:var(--label-color, #475467);white-space:nowrap;">${formatDate(o.created_at)}</td>
        </tr>
    `,
    )
    .join("");
}

// ─── Tab: Settings ────────────────────────────────────────────
function renderSettings(a) {
  const featuredEl = document.getElementById("toggle-featured");
  const publishedEl = document.getElementById("toggle-published");
  const fundingSelect = document.getElementById("select-funding-status");

  if (a.featured) featuredEl?.classList.add("active");
  if (a.published) publishedEl?.classList.add("active");
  if (fundingSelect && a.funding_status) fundingSelect.value = a.funding_status;
}

// ─── Actions ──────────────────────────────────────────────────
async function toggleFeatured() {
  showToast("Only admins can feature assets on the landing page.");
}

async function togglePublished() {
  showToast("Publishing status is managed by admins. Contact support to request a change.", "warning");
}

async function dangerAction(action) {
  const names = {
    freeze: "freeze trading on",
    unpublish: "unpublish",
    archive: "archive",
  };
  showToast("This action is not yet available. Contact support to request changes.", "warning");
}

// ─── Helpers ──────────────────────────────────────────────────
function esc(s) {
  if (typeof s !== "string") return s ?? "";
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

function formatUSD(cents) {
  if (typeof cents !== "number") return "$0.00";
  return (
    "$" +
    (Math.abs(cents) / 100).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
}

function formatCountry(code) {
  if (!code || code.trim().length !== 2) return typeof code === 'string' ? code.toUpperCase() : (code || "");
  try {
    const regionNames = new Intl.DisplayNames(['en'], { type: 'region' });
    return regionNames.of(code.trim().toUpperCase());
  } catch(e) {
    return code.toUpperCase();
  }
}

function bpsToPercent(bps) {
  if (bps == null) return "—";
  return (bps / 100).toFixed(1) + "%";
}

function formatAssetType(type) {
  if (!type) return "—";
  return type.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function formatDate(iso) {
  if (!iso) return "—";
  const d = new Date(iso);
  return d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function orderStatusBadge(status) {
  const map = {
    pending: "ad-badge--warning",
    processing: "ad-badge--info",
    completed: "ad-badge--success",
    failed: "ad-badge--danger",
    cancelled: "ad-badge--danger",
    refunded: "ad-badge--neutral",
  };
  const cls = map[status] || "ad-badge--neutral";
  return `<span class="ad-badge ${cls}">${status || "—"}</span>`;
}

function showToast(msg) {
  if(window.showPooolToast) {
    window.showPooolToast(null, msg, "info");
  }
}
