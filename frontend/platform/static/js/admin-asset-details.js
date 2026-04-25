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
        <div style="display:flex;justify-content:space-between;align-items:center;">
            <span style="font-size:13px;color:var(--admin-text-muted);">${l}</span>
            <span style="font-size:14px;font-weight:600;font-variant-numeric:tabular-nums;">${esc(v)}</span>
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
    }
  }
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
function renderCapTable(a) {
  const investors = a.investors || [];
  if (investors.length === 0) return;

  document.getElementById("captable-tbody").innerHTML = investors
    .map(
      (inv) => `
        <tr>
            <td>
                <a href="/admin/user-details?id=${esc(inv.user_id)}" style="color:var(--admin-accent);text-decoration:none;font-weight:600;">
                    ${esc(inv.name)}
                </a>
            </td>
            <td style="font-weight:600;font-variant-numeric:tabular-nums;">${inv.tokens_owned.toLocaleString()}</td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(inv.purchase_value_cents)}</td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(inv.current_value_cents)}</td>
            <td style="font-variant-numeric:tabular-nums;color:var(--admin-success);">${formatUSD(inv.total_rental_cents)}</td>
            <td><span class="admin-badge admin-badge--${inv.status === "active" ? "success" : "neutral"}"><span class="admin-badge-dot"></span>${inv.status}</span></td>
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
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDate(o.created_at)}</td>
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
