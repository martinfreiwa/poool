/**
 * Admin Asset Details — Full tabbed interface
 * Fetches from GET /api/admin/assets/:id/detail
 */

let assetData = null;
let assetId = null;
let detailAssetImages = [];

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

  // Status badge (LIVE/DRAFT only — no funding stage badge)
  const statusBadge = document.getElementById("asset-status-badge");
  statusBadge.textContent = a.published ? "LIVE" : "DRAFT";
  statusBadge.className = `ad-badge ${a.published ? "ad-badge--success" : "ad-badge--warning"}`;
  const fundingBadge = document.getElementById("asset-funding-badge");
  fundingBadge.style.display = "none";

  // Funding progress — only show when live/funded
  const fundingEl = document.getElementById("asset-funding-section");
  const isDraft = !a.published;
  if (fundingEl) fundingEl.hidden = isDraft;
  if (!isDraft) {
    const sold = (a.tokens_total || 0) - (a.tokens_available || 0);
    const pct = a.tokens_total ? Math.round((sold / a.tokens_total) * 100) : 0;
    document.getElementById("funding-label").textContent =
      `${sold.toLocaleString()} of ${(a.tokens_total || 0).toLocaleString()} tokens sold`;
    document.getElementById("funding-pct").textContent = `${pct}%`;
    document.getElementById("funding-bar").style.width = `${pct}%`;
  }

  // Header stats
  document.getElementById("asset-valuation").textContent = formatUSD(a.total_value_cents);
  document.getElementById("asset-token-price").textContent = formatUSD(a.token_price_cents);
  document.getElementById("asset-yield").textContent = bpsToPercent(a.annual_yield_bps);
  document.getElementById("asset-type").textContent = formatAssetType(a.asset_type);

  // Image — fallback to POOOL logo placeholder
  const heroImg = document.getElementById("asset-image");
  const heroWrap = document.getElementById("asset-image-wrap");

  function showHeroFallback() {
    heroImg.style.display = "none";
    if (heroWrap) {
      heroWrap.style.cssText = "background:#F5F7FF;display:flex;align-items:center;justify-content:center;";
      const logo = document.createElement("img");
      logo.src = "/static/images/logos/logo-blue.svg";
      logo.style.cssText = "width:80px;opacity:0.3;pointer-events:none;";
      heroWrap.appendChild(logo);
    }
  }

  if (a.images && a.images.length > 0) {
    const cover = a.images.find((i) => i.is_cover) || a.images[0];
    if (cover.url) {
      heroImg.src = cover.url;
      heroImg.onerror = showHeroFallback;
      heroImg.onload = () => { if (heroImg.naturalWidth <= 1) showHeroFallback(); };
    } else {
      showHeroFallback();
    }
  } else {
    showHeroFallback();
  }

  // Tab badges
  const investorCount = (a.investors || []).length;
  const orderCount = (a.orders || []).length;
  const badgeInvestors = document.getElementById("badge-investors");
  if (badgeInvestors) { badgeInvestors.textContent = investorCount; badgeInvestors.style.display = investorCount === 0 ? "none" : ""; }
  const badgeOrders = document.getElementById("badge-orders");
  if (badgeOrders) { badgeOrders.textContent = orderCount; badgeOrders.style.display = orderCount === 0 ? "none" : ""; }

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
const POOOL_LOGO_URL = "/static/images/logos/logo-blue.svg";

function renderMedia(a) {
  const images = (a.images || [])
    .filter((i) => i.url)
    .map((img, index) => ({
      id: img.id || "",
      url: img.url,
      is_cover: !!img.is_cover,
      sort_order: Number.isFinite(Number(img.sort_order)) ? Number(img.sort_order) : index,
    }))
    .sort((left, right) => left.sort_order - right.sort_order);

  if (images.length > 0 && !images.some((img) => img.is_cover)) {
    images[0].is_cover = true;
  }
  detailAssetImages = images.map((img, index) => ({
    ...img,
    sort_order: index,
    is_cover: index === 0 ? true : img.is_cover && images.findIndex((item) => item.is_cover) === index,
  }));

  document.getElementById("media-count").textContent =
    `${detailAssetImages.length} image${detailAssetImages.length !== 1 ? "s" : ""}`;

  renderMediaGrid();

  const mediaBody = document.querySelector("#panel-media .ad-card__body");
  let hint = document.getElementById("media-order-hint");
  if (mediaBody && !hint) {
    hint = document.createElement("p");
    hint.id = "media-order-hint";
    hint.className = "media-order-hint";
    mediaBody.insertBefore(hint, document.getElementById("media-grid"));
  }
  if (hint) {
    hint.hidden = detailAssetImages.length < 2;
    hint.textContent = "Drag images or use the arrow buttons to set the display order. Image 1 is used as the cover.";
  }

  if (a.video_url) {
    document.getElementById("video-section").hidden = false;
    document.getElementById("video-link").href = a.video_url;
  }
}

function renderMediaGrid() {
  const grid = document.getElementById("media-grid");
  if (!grid) return;

  grid.textContent = "";
  if (detailAssetImages.length === 0) {
    const empty = document.createElement("p");
    empty.className = "ad-empty";
    empty.textContent = "No images uploaded.";
    grid.appendChild(empty);
    return;
  }

  detailAssetImages.forEach((img, index) => {
    img.sort_order = index;
    img.is_cover = index === 0;

    const item = document.createElement("div");
    item.className = "media-item media-item--sortable";
    item.draggable = detailAssetImages.length > 1 && !!img.id;
    item.dataset.index = String(index);
    item.dataset.id = img.id;
    item.style.backgroundImage = `url("${cssUrl(img.url)}")`;
    item.setAttribute("aria-label", `Image ${index + 1} of ${detailAssetImages.length}`);
    item.addEventListener("dragstart", handleMediaDragStart);
    item.addEventListener("dragover", handleMediaDragOver);
    item.addEventListener("drop", handleMediaDrop);
    item.addEventListener("dragend", handleMediaDragEnd);

    const orderBadge = document.createElement("span");
    orderBadge.className = "media-order-badge";
    orderBadge.textContent = `#${index + 1}`;
    item.appendChild(orderBadge);

    if (img.is_cover) {
      const coverBadge = document.createElement("span");
      coverBadge.className = "cover-badge";
      coverBadge.textContent = "Cover";
      item.appendChild(coverBadge);
    }

    if (detailAssetImages.length > 1 && img.id) {
      const controls = document.createElement("div");
      controls.className = "media-order-controls";
      controls.appendChild(createMediaOrderButton("up", index, index === 0));
      controls.appendChild(createMediaOrderButton("down", index, index === detailAssetImages.length - 1));
      if (index !== 0) {
        const coverButton = document.createElement("button");
        coverButton.type = "button";
        coverButton.className = "media-order-cover";
        coverButton.textContent = "Make cover";
        coverButton.addEventListener("click", () => moveDetailImage(index, 0));
        controls.appendChild(coverButton);
      }
      item.appendChild(controls);
    }

    grid.appendChild(item);
  });
}

function createMediaOrderButton(direction, index, disabled) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "media-order-btn";
  button.disabled = disabled;
  button.setAttribute("aria-label", direction === "up" ? "Move image earlier" : "Move image later");
  button.textContent = direction === "up" ? "↑" : "↓";
  button.addEventListener("click", () => {
    moveDetailImage(index, direction === "up" ? index - 1 : index + 1);
  });
  return button;
}

let draggedMediaIndex = null;

function handleMediaDragStart(event) {
  draggedMediaIndex = Number(this.dataset.index);
  event.dataTransfer.effectAllowed = "move";
  event.dataTransfer.setData("text/plain", String(draggedMediaIndex));
  this.classList.add("media-item--dragging");
}

function handleMediaDragOver(event) {
  event.preventDefault();
  event.dataTransfer.dropEffect = "move";
}

function handleMediaDrop(event) {
  event.preventDefault();
  const targetIndex = Number(this.dataset.index);
  if (Number.isInteger(draggedMediaIndex) && draggedMediaIndex !== targetIndex) {
    moveDetailImage(draggedMediaIndex, targetIndex);
  }
}

function handleMediaDragEnd() {
  this.classList.remove("media-item--dragging");
  draggedMediaIndex = null;
}

async function moveDetailImage(fromIndex, toIndex) {
  if (fromIndex < 0 || toIndex < 0 || fromIndex >= detailAssetImages.length || toIndex >= detailAssetImages.length) {
    return;
  }
  if (!detailAssetImages.every((img) => img.id)) {
    showToast("Image order cannot be saved until all images have server IDs.", "warning");
    return;
  }

  const previousImages = detailAssetImages.map((img) => ({ ...img }));
  const [moved] = detailAssetImages.splice(fromIndex, 1);
  detailAssetImages.splice(toIndex, 0, moved);
  normalizeDetailImageOrder();
  renderMediaGrid();

  try {
    await syncDetailImageOrder();
    showToast("Image order saved.", "success");
  } catch (err) {
    detailAssetImages = previousImages;
    renderMediaGrid();
    showToast(`Could not save image order: ${err.message}`, "error");
  }
}

function normalizeDetailImageOrder() {
  detailAssetImages.forEach((img, index) => {
    img.sort_order = index;
    img.is_cover = index === 0;
  });
}

async function syncDetailImageOrder() {
  if (!assetId || detailAssetImages.length === 0) return;
  const payload = detailAssetImages.map((img, index) => ({
    id: img.id,
    sort_order: index,
    is_cover: index === 0,
  }));
  const response = await fetch(`/api/developer/draft/${assetId}/images/reorder`, {
    method: "PUT",
    headers: {
      "Content-Type": "application/json",
      "X-CSRF-Token": getCsrfToken(),
    },
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.error || `HTTP ${response.status}`);
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

function cssUrl(value) {
  return String(value || "").replace(/["\\\n\r\f]/g, "");
}

function getCsrfToken() {
  const value = `; ${document.cookie}`;
  const parts = value.split(`; csrf_token=`);
  if (parts.length === 2) return parts.pop().split(";").shift();
  return "";
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

function showToast(msg, type = "info") {
  if(window.showPooolToast) {
    window.showPooolToast(null, msg, type);
  }
}
