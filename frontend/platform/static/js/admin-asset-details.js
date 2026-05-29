/**
 * Admin Asset Details — Full tabbed interface
 * Fetches from GET /api/admin/assets/:id/detail
 */

let assetData = null;
let assetId = null;
let featuredPending = false;
let publishedPending = false;
let fundingStatusPending = false;

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
    const tab = e.target.closest(".asset-tab");
    if (!tab) return;
    document
      .querySelectorAll(".asset-tab")
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
  document
    .getElementById("select-funding-status")
    ?.addEventListener("change", updateFundingStatus);

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
    const resp = await fetch(`/api/admin/assets/${encodeURIComponent(assetId)}/detail`);
    if (!resp.ok) throw new Error(await responseError(resp));
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
        <div style="color:var(--admin-danger);font-weight:700;margin-bottom:12px;">${esc(msg)}</div>
        <a href="/admin/assets" class="admin-btn admin-btn--secondary">← Back to Assets</a>
    `;
}

// ─── Render All ───────────────────────────────────────────────
function renderAll(a) {
  document.getElementById("loading-overlay").style.display = "none";
  document.getElementById("asset-content").style.display = "block";

  // Header
  document.getElementById("breadcrumb-asset-title").textContent = a.title;
  document.getElementById("asset-title-main").innerHTML = `${esc(a.title)} <code style="font-family:monospace;font-size:14px;padding:2px 8px;background:var(--admin-border);border-radius:4px;color:var(--admin-text-secondary);font-weight:500;margin-left:8px;vertical-align:middle;">#APP-${(a.id || '').substring(0, 6).toUpperCase()}</code>`;
  document.getElementById("asset-location").textContent =
    [a.city, formatCountry(a.country)].filter(Boolean).join(", ") || "Location not specified";

  // Status badges
  const statusBadge = document.getElementById("asset-status-badge");
  statusBadge.textContent = a.published ? "LIVE" : "DRAFT";
  statusBadge.className = `admin-badge ${a.published ? "admin-badge--success" : "admin-badge--warning"}`;

  const fundingBadge = document.getElementById("asset-funding-badge");
  const fundingMap = {
    upcoming: ["Upcoming", "admin-badge--neutral"],
    funding_open: ["Funding Open", "admin-badge--info"],
    funding_in_progress: ["Funding In Progress", "admin-badge--info"],
    funded: ["Funded", "admin-badge--success"],
    rented: ["Rented", "admin-badge--success"],
    payout_pending: ["Payout Pending", "admin-badge--warning"],
    exited: ["Exited", "admin-badge--danger"],
  };
  const [fLabel, fClass] = fundingMap[a.funding_status] || [
    a.funding_status || "Unknown",
    "admin-badge--neutral",
  ];
  fundingBadge.textContent = fLabel;
  fundingBadge.className = `admin-badge ${fClass}`;

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
  // Editable property page content + milestones (shared module)
  if (window.PropertyPageEditor && a.id) {
    PropertyPageEditor.init({ assetId: a.id, asset: a, milestones: a.milestones || [] });
  }
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

  // Dividend warning banner — funded asset with zero dividends ever distributed
  const existingBanner = document.getElementById("dividend-warning-banner");
  if (existingBanner) existingBanner.remove();
  const isFunded = ["funded", "rented"].includes(a.funding_status);
  const totalDividends = (a.investors || []).reduce((sum, inv) => sum + (inv.total_rental_cents || 0), 0);
  if (isFunded && totalDividends === 0 && (a.investors || []).length > 0) {
    const banner = document.createElement("div");
    banner.id = "dividend-warning-banner";
    banner.style.cssText = "background:rgba(245,158,11,0.1);border:1px solid rgba(245,158,11,0.4);border-radius:8px;padding:12px 16px;display:flex;align-items:center;gap:10px;margin-bottom:16px;";
    banner.innerHTML = `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#f59e0b" stroke-width="2" style="flex-shrink:0;"><path d="M8 1l7 14H1L8 1z"/><path d="M8 6v4"/><circle cx="8" cy="12" r=".5" fill="#f59e0b"/></svg><span style="font-size:13px;color:#92400e;font-weight:500;">No dividends distributed yet — asset is funded with <strong>${bpsToPercent(a.annual_yield_bps)}</strong> annual yield.</span>`;
    const overviewPanel = document.getElementById("panel-overview");
    overviewPanel.insertBefore(banner, overviewPanel.firstChild);
  }

  // Milestones warning on overview when live + 0 milestones
  const existingMilestoneBanner = document.getElementById("milestone-warning-banner");
  if (existingMilestoneBanner) existingMilestoneBanner.remove();
  if (a.published && (a.milestones || []).length === 0) {
    const mb = document.createElement("div");
    mb.id = "milestone-warning-banner";
    mb.style.cssText = "background:rgba(99,102,241,0.08);border:1px solid rgba(99,102,241,0.25);border-radius:8px;padding:12px 16px;display:flex;align-items:center;gap:10px;margin-bottom:16px;";
    mb.innerHTML = `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="var(--admin-accent)" stroke-width="2" style="flex-shrink:0;"><circle cx="8" cy="8" r="7"/><path d="M8 5v3l2 2"/></svg><span style="font-size:13px;color:var(--admin-text-secondary);font-weight:500;">No milestones added — investors cannot track property progress.</span>`;
    const overviewPanel = document.getElementById("panel-overview");
    overviewPanel.insertBefore(mb, overviewPanel.firstChild);
  }

  // Financial Summary
  const resaleAvailable = a.resale_tokens_available || 0;
  const summaryRows = [
    ["Total Valuation", formatUSD(a.total_value_cents)],
    ["Token Price", formatUSD(a.token_price_cents)],
    ["Tokens Total", (a.tokens_total || 0).toLocaleString()],
    ["Primary Available", (a.tokens_available || 0).toLocaleString()],
    ["Resale Available", resaleAvailable > 0
      ? `<span style="color:var(--admin-success);font-weight:700;">${resaleAvailable.toLocaleString()}</span>`
      : `<span style="color:var(--admin-text-muted);">0</span>`],
    ["Annual Yield", bpsToPercent(a.annual_yield_bps)],
  ];
  document.getElementById("financial-summary").innerHTML = summaryRows
    .map(
      ([l, v]) => `
        <div style="display:flex;justify-content:space-between;align-items:center;">
            <span style="font-size:13px;color:var(--admin-text-muted);">${l}</span>
            <span style="font-size:14px;font-weight:600;font-variant-numeric:tabular-nums;">${typeof v === "string" && v.startsWith("<") ? v : esc(String(v))}</span>
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
        <div style="display:flex;justify-content:space-between;align-items:center;">
            <span style="font-size:13px;color:var(--admin-text-muted);">${l}</span>
            <span style="font-size:14px;font-weight:700;">${v}</span>
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
    const grid = document.getElementById("media-grid");
    grid.textContent = "";
    images.forEach((img) => {
      const item = document.createElement("div");
      item.className = "media-item";
      const url = safeUrl(img.url, { allowExternal: true });
      if (url) item.style.backgroundImage = `url("${url}")`;
      if (img.is_cover) {
        const badge = document.createElement("span");
        badge.className = "cover-badge";
        badge.textContent = "Cover";
        item.appendChild(badge);
      }
      grid.appendChild(item);
    });
  }

  if (a.video_url) {
    const videoUrl = safeUrl(a.video_url, { allowExternal: true });
    if (videoUrl) {
      document.getElementById("video-section").hidden = false;
      document.getElementById("video-link").href = videoUrl;
      const embed = document.getElementById("video-embed");
      embed.textContent = "";
      const embedSrc = toEmbedUrl(videoUrl);
      if (embedSrc) {
        const iframe = document.createElement("iframe");
        iframe.src = embedSrc;
        iframe.style.cssText = "width:100%;height:100%;border:0;";
        iframe.setAttribute("allow", "accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture");
        iframe.setAttribute("allowfullscreen", "");
        iframe.setAttribute("loading", "lazy");
        embed.appendChild(iframe);
      } else {
        const video = document.createElement("video");
        video.src = videoUrl;
        video.controls = true;
        video.style.cssText = "width:100%;height:100%;background:#000;";
        embed.appendChild(video);
      }
    }
  }
}

function toEmbedUrl(rawUrl) {
  try {
    const u = new URL(rawUrl);
    const host = u.hostname.replace(/^www\./, "");
    if (host === "youtube.com" || host === "m.youtube.com") {
      const id = u.searchParams.get("v");
      if (id) return `https://www.youtube.com/embed/${encodeURIComponent(id)}`;
      const m = u.pathname.match(/^\/(?:embed|shorts)\/([\w-]+)/);
      if (m) return `https://www.youtube.com/embed/${m[1]}`;
    }
    if (host === "youtu.be") {
      const id = u.pathname.replace(/^\//, "");
      if (id) return `https://www.youtube.com/embed/${encodeURIComponent(id)}`;
    }
    if (host === "vimeo.com") {
      const id = u.pathname.replace(/^\//, "").split("/")[0];
      if (/^\d+$/.test(id)) return `https://player.vimeo.com/video/${id}`;
    }
    if (host === "player.vimeo.com") return rawUrl;
  } catch (_) {}
  return null;
}

// ─── Tab: Documents ───────────────────────────────────────────
function renderDocuments(a) {
  const docs = a.documents || [];
  if (docs.length === 0) return;

  document.getElementById("documents-list").innerHTML = docs
    .map((d) => {
      const size = d.file_size
        ? `${(d.file_size / 1024 / 1024).toFixed(2)} MB`
        : "—";
      const url = safeUrl(d.url);
      const label = d.title || formatAssetType(d.document_type || "document");
      return `
            <div class="document-item">
                <div style="display:flex;align-items:center;gap:12px;">
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="color:var(--admin-text-muted);" aria-hidden="true"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
                    <div>
                        <span class="document-type-badge">${esc((d.document_type || "document").replace(/_/g, " "))}</span>
                        <span style="font-size:13px;color:var(--admin-text-primary);margin-left:8px;">${esc(label)}</span>
                        <span style="font-size:12px;color:var(--admin-text-muted);margin-left:8px;">${size}</span>
                    </div>
                </div>
                ${url ? `<a href="${esc(url)}" target="_blank" rel="noopener" class="admin-btn admin-btn--secondary admin-btn--sm">View</a>` : '<button type="button" class="admin-btn admin-btn--secondary admin-btn--sm" disabled>Unavailable</button>'}
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
            <td style="color:var(--admin-success);font-weight:600;font-variant-numeric:tabular-nums;">${formatUSD(f.rental_income_cents)}</td>
            <td style="color:var(--admin-danger);font-variant-numeric:tabular-nums;">${formatUSD(f.expenses_cents)}</td>
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
                <div style="font-size:14px;font-weight:600;color:var(--admin-text-primary);margin-bottom:2px;">${esc(m.title)}</div>
                <div style="font-size:12px;color:var(--admin-text-muted);">${esc(m.description || "")}</div>
                ${m.month_index != null ? `<span style="font-size:11px;color:var(--admin-text-muted);margin-top:4px;display:inline-block;">Month ${m.month_index}</span>` : ""}
            </div>
            <span class="admin-badge ${m.is_completed ? "admin-badge--success" : "admin-badge--neutral"}">
                ${m.is_completed ? "Completed" : "Pending"}
            </span>
        </div>
    `,
    )
    .join("");
}

// ─── Tab: Cap Table ───────────────────────────────────────────
function investorInitials(name) {
  const parts = (name || "").trim().split(/\s+/);
  if (parts.length >= 2) return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  return (name || "?").substring(0, 2).toUpperCase();
}

function investorAvatarColor(userId) {
  const colors = ["#6366f1","#0ea5e9","#10b981","#f59e0b","#ef4444","#8b5cf6","#ec4899","#14b8a6"];
  let hash = 0;
  for (let i = 0; i < (userId || "").length; i++) hash = (hash * 31 + userId.charCodeAt(i)) >>> 0;
  return colors[hash % colors.length];
}

function renderCapTable(a) {
  const investors = a.investors || [];
  const totalTokens = a.tokens_total || 0;
  if (investors.length === 0) return;

  // Concentration warning: any investor > 50%
  const existingConc = document.getElementById("concentration-warning");
  if (existingConc) existingConc.remove();
  const topHolder = investors[0];
  const topPct = totalTokens > 0 ? (topHolder.tokens_owned / totalTokens) * 100 : 0;
  if (topPct > 50) {
    const warn = document.createElement("div");
    warn.id = "concentration-warning";
    warn.style.cssText = "background:rgba(239,68,68,0.08);border:1px solid rgba(239,68,68,0.3);border-radius:8px;padding:12px 16px;display:flex;align-items:center;gap:10px;margin-bottom:16px;";
    warn.innerHTML = `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="#ef4444" stroke-width="2" style="flex-shrink:0;"><path d="M8 1l7 14H1L8 1z"/><path d="M8 6v4"/><circle cx="8" cy="12" r=".5" fill="#ef4444"/></svg><span style="font-size:13px;color:#991b1b;font-weight:500;">Concentration risk: <strong>${esc(topHolder.name)}</strong> holds <strong>${topPct.toFixed(1)}%</strong> of total supply.</span>`;
    document.getElementById("panel-captable").insertBefore(warn, document.getElementById("panel-captable").firstChild);
  }

  document.getElementById("captable-tbody").innerHTML = investors
    .map((inv) => {
      const pct = totalTokens > 0 ? ((inv.tokens_owned / totalTokens) * 100).toFixed(1) : "0.0";
      const color = investorAvatarColor(inv.user_id);
      const initials = investorInitials(inv.name);
      const emailDomain = (inv.email || "").split("@")[1] || "";
      const isExternalDomain = emailDomain && !["poool.app", "poool.com"].includes(emailDomain);
      return `
        <tr>
            <td>
                <div style="display:flex;align-items:center;gap:10px;">
                    <div style="width:32px;height:32px;border-radius:50%;background:${color};display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:700;color:#fff;flex-shrink:0;">${esc(initials)}</div>
                    <div>
                        <a href="/admin/user-details?id=${esc(inv.user_id)}" style="color:var(--admin-accent);text-decoration:none;font-weight:600;font-size:13px;display:block;">${esc(inv.name)}</a>
                        <span style="font-size:11px;color:${isExternalDomain ? "#f59e0b" : "var(--admin-text-muted)"};">${esc(inv.email || "")}</span>
                    </div>
                </div>
            </td>
            <td style="font-weight:600;font-variant-numeric:tabular-nums;">${inv.tokens_owned.toLocaleString()}</td>
            <td style="font-variant-numeric:tabular-nums;color:var(--admin-text-muted);font-size:13px;">${pct}%</td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(inv.purchase_value_cents)}</td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(inv.current_value_cents)}</td>
            <td style="font-variant-numeric:tabular-nums;color:${inv.total_rental_cents > 0 ? "var(--admin-success)" : "var(--admin-text-muted)"};">${formatUSD(inv.total_rental_cents)}</td>
            <td><span class="admin-badge admin-badge--${inv.status === "active" ? "success" : "neutral"}"><span class="admin-badge-dot"></span>${inv.status}</span></td>
        </tr>
    `;
    })
    .join("");
}

// ─── Tab: Orders ──────────────────────────────────────────────
function renderOrders(a) {
  const orders = a.orders || [];
  if (orders.length === 0) return;

  const tokenPriceCents = a.token_price_cents || 0;
  const totalTokens = a.tokens_total || 1;
  const LARGE_ORDER_CENTS = 10_000_00; // $100k
  const LARGE_PCT = 20; // >20% of supply

  document.getElementById("orders-tbody").innerHTML = orders
    .map((o) => {
      const expectedCents = tokenPriceCents * o.tokens;
      const mismatch = expectedCents > 0 && Math.abs(o.subtotal_cents - expectedCents) > 100;
      const pctOfSupply = (o.tokens / totalTokens) * 100;
      const isLarge = o.subtotal_cents >= LARGE_ORDER_CENTS || pctOfSupply >= LARGE_PCT;
      const emailDomain = (o.user_email || "").split("@")[1] || "";
      const isExternalDomain = emailDomain && !["poool.app", "poool.com"].includes(emailDomain);

      const anomalyFlag = isLarge
        ? `<span title="Large order: ${pctOfSupply.toFixed(1)}% of supply / ${formatUSD(o.subtotal_cents)}" style="display:inline-flex;align-items:center;gap:3px;font-size:10px;font-weight:700;color:#f59e0b;background:rgba(245,158,11,0.1);border:1px solid rgba(245,158,11,0.3);border-radius:4px;padding:1px 5px;margin-left:6px;">⚠ LARGE</span>`
        : "";

      const expectedCell = mismatch
        ? `<td style="font-variant-numeric:tabular-nums;color:var(--admin-danger);font-weight:600;" title="Expected ${formatUSD(expectedCents)}">${formatUSD(expectedCents)} ⚠</td>`
        : `<td style="font-variant-numeric:tabular-nums;color:var(--admin-text-muted);font-size:12px;">${formatUSD(expectedCents)}</td>`;

      return `
        <tr${isLarge ? ' style="background:rgba(245,158,11,0.04);"' : ""}>
            <td style="font-weight:600;font-family:monospace;font-size:12px;">${esc(o.order_number)}${anomalyFlag}</td>
            <td>
                <span style="color:${isExternalDomain ? "#f59e0b" : "var(--admin-text-primary)"};">${esc(o.user_email)}</span>
                ${isExternalDomain ? `<span style="font-size:10px;color:#92400e;background:rgba(245,158,11,0.1);border-radius:3px;padding:1px 4px;margin-left:4px;">external</span>` : ""}
            </td>
            <td style="font-variant-numeric:tabular-nums;">${o.tokens.toLocaleString()}</td>
            <td style="font-weight:600;font-variant-numeric:tabular-nums;">${formatUSD(o.subtotal_cents)}</td>
            ${expectedCell}
            <td>${orderStatusBadge(o.status)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDate(o.created_at)}</td>
        </tr>
    `;
    })
    .join("");
}

// ─── Tab: Settings ────────────────────────────────────────────
function renderSettings(a) {
  const featuredEl = document.getElementById("toggle-featured");
  const publishedEl = document.getElementById("toggle-published");
  const fundingSelect = document.getElementById("select-funding-status");

  setSwitchState(featuredEl, Boolean(a.featured));
  setSwitchState(publishedEl, Boolean(a.published));
  if (fundingSelect && a.funding_status) fundingSelect.value = a.funding_status;
}

// ─── Actions ──────────────────────────────────────────────────
async function toggleFeatured() {
  if (featuredPending) return;
  const toggle = document.getElementById("toggle-featured");
  featuredPending = true;
  if (toggle) toggle.disabled = true;
  try {
    const resp = await fetch(`/api/admin/assets/${encodeURIComponent(assetId)}/toggle-featured`, {
      method: "POST",
      headers: csrfHeaders(),
    });
    if (!resp.ok) throw new Error(await responseError(resp));
    const data = await resp.json();
    assetData = { ...assetData, featured: Boolean(data.featured) };
    setSwitchState(toggle, assetData.featured);
    showToast("Featured status updated", "success");
  } catch (e) {
    console.error(e);
    showToast(e.message || "Failed to update featured status", "error");
    setSwitchState(toggle, Boolean(assetData?.featured));
  } finally {
    featuredPending = false;
    if (toggle) toggle.disabled = false;
  }
}

async function togglePublished() {
  if (publishedPending) return;
  await setPublished(!Boolean(assetData?.published));
}

async function setPublished(nextPublished, options = {}) {
  if (publishedPending) return;
  const toggle = document.getElementById("toggle-published");
  if (options.confirm) {
    const label = nextPublished ? "publish" : "unpublish";
    const confirmed = await pooolConfirm({
      title: `${nextPublished ? "Publish" : "Unpublish"} asset`,
      message: `Are you sure you want to ${label} "${assetData?.title}"?`,
      confirmText: nextPublished ? "Publish" : "Unpublish",
      type: nextPublished ? "default" : "danger",
    });
    if (!confirmed) return;
  }

  publishedPending = true;
  if (toggle) toggle.disabled = true;
  try {
    const resp = await fetch(`/api/admin/assets/${encodeURIComponent(assetId)}/publication`, {
      method: "PATCH",
      headers: csrfHeaders({ "Content-Type": "application/json" }),
      body: JSON.stringify({ published: Boolean(nextPublished) }),
    });
    if (!resp.ok) throw new Error(await responseError(resp));
    const data = await resp.json();
    assetData = { ...assetData, published: Boolean(data.published) };
    setSwitchState(toggle, assetData.published);
    renderAll(assetData);
    showToast(assetData.published ? "Asset published" : "Asset unpublished", "success");
  } catch (e) {
    console.error(e);
    showToast(e.message || "Failed to update publication status", "error");
    setSwitchState(toggle, Boolean(assetData?.published));
  } finally {
    publishedPending = false;
    if (toggle) toggle.disabled = false;
  }
}

async function updateFundingStatus(e) {
  if (fundingStatusPending) return;
  const select = e.currentTarget;
  const nextStatus = select.value;
  const previousStatus = assetData?.funding_status;

  fundingStatusPending = true;
  select.disabled = true;
  try {
    const resp = await fetch(`/api/admin/assets/${encodeURIComponent(assetId)}/funding-status`, {
      method: "PATCH",
      headers: csrfHeaders({ "Content-Type": "application/json" }),
      body: JSON.stringify({ funding_status: nextStatus }),
    });
    if (!resp.ok) throw new Error(await responseError(resp));
    const data = await resp.json();
    assetData = { ...assetData, funding_status: data.funding_status };
    renderAll(assetData);
    showToast("Funding status updated", "success");
  } catch (err) {
    console.error(err);
    select.value = previousStatus || "upcoming";
    showToast(err.message || "Failed to update funding status", "error");
  } finally {
    fundingStatusPending = false;
    select.disabled = false;
  }
}

async function dangerAction(action) {
  if (action === "unpublish") {
    await setPublished(false, { confirm: true });
    return;
  }

  showToast("This action is unavailable until its backend workflow is implemented.", "warning");
}

// ─── Helpers ──────────────────────────────────────────────────
function esc(s) {
  if (typeof s !== "string") return s ?? "";
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

function setSwitchState(el, active) {
  if (!el) return;
  el.classList.toggle("active", active);
  el.setAttribute("aria-checked", active ? "true" : "false");
}

function safeUrl(value, options = {}) {
  if (typeof value !== "string" || !value.trim()) return "";
  try {
    const url = new URL(value, window.location.origin);
    if (!["http:", "https:"].includes(url.protocol)) return "";
    if (!options.allowExternal && url.origin !== window.location.origin) return "";
    return url.href;
  } catch (_) {
    return "";
  }
}

function csrfHeaders(headers = {}) {
  const token = getCsrfToken();
  return token ? { ...headers, "X-CSRF-Token": token } : headers;
}

function getCsrfToken() {
  const value = `; ${document.cookie}`;
  const parts = value.split("; csrf_token=");
  if (parts.length !== 2) return "";
  return decodeURIComponent(parts.pop().split(";").shift() || "");
}

async function responseError(resp) {
  try {
    const data = await resp.json();
    if (data && data.error) return data.error;
  } catch (_) {
    // Fall through to status text below.
  }
  return resp.statusText || `HTTP ${resp.status}`;
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
    pending: "admin-badge--warning",
    processing: "admin-badge--info",
    completed: "admin-badge--success",
    failed: "admin-badge--danger",
    cancelled: "admin-badge--danger",
    refunded: "admin-badge--neutral",
  };
  const cls = map[status] || "admin-badge--neutral";
  return `<span class="admin-badge ${cls}"><span class="admin-badge-dot"></span>${status || "—"}</span>`;
}

function showToast(msg, type = "info") {
  if(window.showPooolToast) {
    window.showPooolToast(null, msg, type);
  } else if (window.showToast) {
    window.showToast(msg, type);
  }
}
