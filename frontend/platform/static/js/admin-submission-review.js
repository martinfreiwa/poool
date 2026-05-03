/**
 * admin-submission-review.js
 * Developer Submission Deep-Dive Review
 * Fetches from /api/admin/developer-projects/:id
 */

let projectId = null;
let projectData = null;

document.addEventListener("DOMContentLoaded", () => {
  const urlParams = new URLSearchParams(window.location.search);
  projectId = urlParams.get("id");

  if (!projectId) {
    document.getElementById("loading-overlay").innerHTML =
      `<div style="color:var(--admin-danger);padding:40px;text-align:center;">
                <div style="font-size:24px;margin-bottom:8px;">⚠</div>
                <div>No project ID provided in URL.</div>
                <a href="/admin/developer-submissions" class="admin-btn admin-btn--secondary" style="margin-top:16px;display:inline-block;">← Back to Submissions</a>
             </div>`;
    return;
  }

  // Validation Checkboxes → enable/disable Approve button
  document.querySelectorAll(".validation-chk").forEach((chk) => {
    chk.addEventListener("change", updateApproveButtonState);
  });

  loadSubmission(projectId);
});

function updateApproveButtonState() {
  // Checklist is advisory — the approve button is always enabled.
  // We show a warning indicator if some items are unchecked.
  const checkboxes = Array.from(document.querySelectorAll(".validation-chk")).filter((c) => {
    const row = c.closest(".checklist-item");
    return !row || row.style.display !== "none";
  });
  const uncheckedCount = checkboxes.filter((c) => !c.checked).length;
  const btn = document.getElementById("btn-approve");
  if (!btn) return;
  // Always keep button enabled
  btn.disabled = false;

  // Show/update warning badge
  let badge = document.getElementById("approve-warning-badge");
  if (uncheckedCount > 0) {
    if (!badge) {
      badge = document.createElement("span");
      badge.id = "approve-warning-badge";
      badge.style.cssText = "display:block;text-align:center;font-size:11px;color:var(--admin-text-muted);margin-top:4px;";
      btn.insertAdjacentElement("afterend", badge);
    }
    badge.textContent = `⚠ ${uncheckedCount} checklist item${uncheckedCount > 1 ? "s" : ""} unchecked`;
  } else if (badge) {
    badge.remove();
  }

  // Auto-save checklist state (debounced)
  _scheduleChecklistSave();
}

// ─── Checklist Persistence ────────────────────────────────────────────────────
let _checklistSaveTimer = null;
let _checklistInitialized = false;

function _scheduleChecklistSave() {
  if (!_checklistInitialized) return; // Don't save during initial auto-checks
  if (_checklistSaveTimer) clearTimeout(_checklistSaveTimer);
  _checklistSaveTimer = setTimeout(_saveChecklist, 500);
}

async function _saveChecklist() {
  if (!projectId) return;
  const checkboxes = document.querySelectorAll(".validation-chk");
  const state = {};
  checkboxes.forEach((chk) => {
    if (chk.id) state[chk.id] = chk.checked;
  });
  try {
    await fetch(`/api/admin/developer-projects/${projectId}/checklist`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Token": getCsrfToken()
      },
      body: JSON.stringify({ checklist: state }),
    });
  } catch (e) {
    // Silent fail — checklist persistence is best-effort
    console.warn("Failed to save checklist:", e);
  }
}

async function _restoreChecklist() {
  if (!projectId) return;
  try {
    const res = await fetch(`/api/admin/developer-projects/${projectId}/checklist`);
    if (!res.ok) return;
    const data = await res.json();
    const saved = data.checklist || {};
    Object.entries(saved).forEach(([id, checked]) => {
      const chk = document.getElementById(id);
      if (chk && checked) {
        chk.checked = true;
      }
    });
    _checklistInitialized = true; // Now allow saves on user interaction
    updateApproveButtonState();
  } catch (e) {
    _checklistInitialized = true; // Even on error, allow saves
    console.warn("Failed to restore checklist:", e);
  }
}

// ─── Data Loading ─────────────────────────────────────────────────────────────
async function loadSubmission(id) {
  try {
    const response = await fetch(`/api/admin/developer-projects/${id}`);
    if (response.ok) {
      projectData = await response.json();
      renderSubmission(projectData);
    } else {
      const err = await response.json().catch(() => ({}));
      throw new Error(err.error || `HTTP ${response.status}`);
    }
  } catch (error) {
    document.getElementById("loading-overlay").innerHTML =
      `<div style="color:var(--admin-danger);padding:40px;text-align:center;">
                <div style="font-size:24px;margin-bottom:8px;">✗</div>
                <div>Failed to load project: ${esc(error.message || "Unknown error")}</div>
                <a href="/admin/developer-submissions" class="admin-btn admin-btn--secondary" style="margin-top:16px;display:inline-block;">← Back to List</a>
             </div>`;
  }
}

// ─── Main Render ─────────────────────────────────────────────────────────────
function renderSubmission(data) {
  document.getElementById("loading-overlay").style.display = "none";
  document.getElementById("review-content").style.display = "grid";

  // Store globally so image helpers can look up the asset ID
  window.pageData = data;

  const { project, developer, asset, documents, images, milestones } = data;

  // Update breadcrumb title
  const mainTitle = project.project_name || asset.title || "Untitled";
  document.getElementById("sub-title").innerHTML = `${esc(mainTitle)} <code style="font-family:monospace;font-size:12px;padding:2px 8px;background:var(--admin-border);border-radius:4px;color:var(--admin-text-secondary);font-weight:500;margin-left:8px;vertical-align:middle;">#APP-${(asset.id || project.id || '').substring(0, 6).toUpperCase()}</code>`;
  document.title = `Review: ${mainTitle} — POOOL Admin`;

  // Status badge in header
  const statusBadgeEl = document.getElementById("sub-status-badge");
  if (statusBadgeEl)
    statusBadgeEl.innerHTML = getProjectStatusBadge(project.status);

  // Toast: auto-transitioned to in_review
  if (project.was_transitioned_to_in_review) {
    showToast(
      'Status automatically updated to "In Review". Developer has been notified.',
      "info",
    );
  }

  // Developer profile card
  renderDeveloperCard(developer);

  // Asset info panels
  renderAssetInfo(asset);

  // Document data room (categorised)
  renderDocuments(documents || []);

  // Image gallery
  renderImages(images || []);

  // Editable property-page content
  hydratePageContentForm(asset);
  wirePageContentSave(asset);

  // Milestones (editable)
  renderMilestones(milestones || [], asset.id);
  wireMilestoneAddButton(asset.id);

  // Admin notes history
  loadNotes();

  // Token math auto-validation
  validateTokenMath(asset);

  // Auto-check KYC checkbox if developer KYC is approved
  if (developer.kyc_status === "approved") {
    const kycChk = document.getElementById("chk-kyc");
    if (kycChk) {
      kycChk.checked = true;
      updateApproveButtonState();
    }
  }

  // Conditionally show/hide video and maps checklist items
  toggleConditionalChecklist(asset);

  // Restore saved checklist state from DB (overrides auto-checks where applicable)
  _restoreChecklist();

  // Wire up action buttons
  const btnApprove = document.getElementById("btn-approve");
  const btnTokenize = document.getElementById("btn-tokenize");

  if (btnTokenize) {
    btnTokenize.style.display = "inline-flex";
    btnTokenize.onclick = () => {
      window.location.href = `/admin/asset-tokenize.html?id=${asset.id}`;
    };
  }

  if (btnApprove) {
    btnApprove.onclick = () => handleDecision("approve");
  }

  const btnReject = document.getElementById("btn-reject");
  if (btnReject) btnReject.onclick = () => handleDecision("reject");
  
  const btnRevise = document.getElementById("btn-revise");
  if (btnRevise) btnRevise.onclick = () => handleDecision("request_revision");
  
  const inReviewBtn = document.getElementById("btn-in-review");
  if (inReviewBtn) {
    inReviewBtn.onclick = () => handleDecision("in_review");
  }
}

// ─── Developer Card ──────────────────────────────────────────────────────────
function renderDeveloperCard(dev) {
  const container = document.getElementById("dev-profile-card");
  if (!container) return;

  const name =
    [dev.first_name, dev.last_name].filter(Boolean).join(" ") || dev.email;

  container.innerHTML = `
        <div style="display:flex;align-items:center;gap:16px;">
            <div style="width:48px;height:48px;border-radius:50%;background:linear-gradient(135deg,var(--admin-primary),var(--admin-accent));
                        display:flex;align-items:center;justify-content:center;color:white;font-weight:700;font-size:18px;flex-shrink:0;">
                ${esc(name.charAt(0).toUpperCase())}
            </div>
            <div style="flex:1;min-width:0;">
                <div style="font-weight:700;font-size:16px;color:var(--admin-text-primary);">${esc(name)}</div>
                <div style="font-size:13px;color:var(--admin-text-muted);margin-top:2px;">${esc(dev.email)}</div>
                <div style="margin-top:8px;display:flex;gap:8px;align-items:center;flex-wrap:wrap;">
                    ${getKycBadge(dev.kyc_status)}
                    <span style="font-size:12px;color:var(--admin-text-muted);">
                        ${dev.other_projects_count || 0} project(s) submitted total
                    </span>
                </div>
            </div>
            <a href="/admin/user-details?id=${esc(dev.user_id)}"
               class="admin-btn admin-btn--secondary admin-btn--sm"
               style="flex-shrink:0;">
               View Profile →
            </a>
        </div>
    `;
}

function getKycBadge(status) {
  const map = {
    approved: ["admin-badge--success", "✓ KYC Approved"],
    pending: ["admin-badge--warning", "⏳ KYC Pending"],
    rejected: ["admin-badge--danger", "✗ KYC Rejected"],
    in_review: ["admin-badge--info", "🔍 KYC In Review"],
    expired: ["admin-badge--danger", "⌛ KYC Expired"],
  };
  const [cls, label] = map[status] || ["admin-badge--neutral", "— No KYC"];
  return `<span class="admin-badge ${cls}">${label}</span>`;
}

// ─── Asset Info ───────────────────────────────────────────────────────────────
function renderAssetInfo(asset) {
  // Basic description
  document.getElementById("sub-description").textContent =
    asset.description || "No description provided.";

  // Financial
  document.getElementById("sub-valuation").textContent = formatUSD(
    asset.total_value_cents,
  );
  document.getElementById("sub-token-price").textContent = formatUSD(
    asset.token_price_cents,
  );
  document.getElementById("sub-tokens-total").textContent = (
    asset.tokens_total || 0
  ).toLocaleString();
  document.getElementById("sub-yield").textContent = asset.annual_yield_bps
    ? (asset.annual_yield_bps / 100).toFixed(2) + "%"
    : "—";
  document.getElementById("sub-cap-app").textContent =
    asset.capital_appreciation_bps
      ? (asset.capital_appreciation_bps / 100).toFixed(2) + "%"
      : "—";
  document.getElementById("sub-occupancy").textContent =
    asset.occupancy_rate_bps
      ? (asset.occupancy_rate_bps / 100).toFixed(2) + "%"
      : "—";

  // Property details
  document.getElementById("sub-type").textContent = formatLabel(
    asset.asset_type,
  );
  document.getElementById("sub-prop-type").textContent =
    formatLabel(asset.property_type) || "—";
  document.getElementById("sub-area").textContent = asset.area || "—";
  document.getElementById("sub-lease").textContent = asset.lease_type
    ? `${formatLabel(asset.lease_type)}${asset.lease_term_years ? `, ${asset.lease_term_years} yrs` : ""}`
    : "—";
  document.getElementById("sub-land-size").textContent = asset.land_size_sqm
    ? `${asset.land_size_sqm} sqm`
    : "—";
  document.getElementById("sub-bldg-size").textContent = asset.building_size_sqm
    ? `${asset.building_size_sqm} sqm`
    : "—";
  document.getElementById("sub-rooms").textContent =
    `${asset.bedrooms || 0} Bed / ${asset.bathrooms || 0} Bath`;
  document.getElementById("sub-year-built").textContent =
    asset.year_built || "—";
  document.getElementById("sub-construction").textContent =
    formatLabel(asset.construction_status) || "—";

  // Location
  document.getElementById("sub-location").textContent =
    [asset.location_city, asset.location_country].filter(Boolean).join(", ") ||
    "—";
  document.getElementById("sub-address").textContent =
    asset.location_address || "—";

  // Links
  const gmapEl = document.getElementById("sub-gmap");
  const videoEl = document.getElementById("sub-video");
  if (gmapEl) {
    if (asset.google_maps_url) {
      gmapEl.href = asset.google_maps_url;
      gmapEl.style.display = "";
    } else {
      gmapEl.closest(".attribute-row")?.style.setProperty("display", "none");
    }
  }
  if (videoEl) {
    if (asset.video_url) {
      videoEl.href = asset.video_url;
      videoEl.style.display = "";
    } else {
      videoEl.closest(".attribute-row")?.style.setProperty("display", "none");
    }
  }

  // Commodity section — show/hide based on type
  const commoditySection = document.getElementById("commodity-section");
  if (commoditySection) {
    if (asset.asset_type === "commodity") {
      commoditySection.style.display = "";
      document.getElementById("sub-operator").textContent =
        asset.operator_name || "—";
      document.getElementById("sub-term").textContent = asset.term_months
        ? `${asset.term_months} months`
        : "—";
      document.getElementById("sub-fixed-roi").textContent = asset.fixed_roi_bps
        ? (asset.fixed_roi_bps / 100).toFixed(2) + "%"
        : "—";
      document.getElementById("sub-rev-min").textContent = formatUSD(
        asset.revenue_min_cents,
      );
      document.getElementById("sub-rev-max").textContent = formatUSD(
        asset.revenue_max_cents,
      );
      document.getElementById("sub-expenses").textContent = formatUSD(
        asset.expenses_cents,
      );
      document.getElementById("sub-profit-min").textContent = formatUSD(
        asset.net_profit_min_cents,
      );
      document.getElementById("sub-profit-max").textContent = formatUSD(
        asset.net_profit_max_cents,
      );
      document.getElementById("sub-inv-payout").textContent = formatUSD(
        asset.investor_payout_cents,
      );
      document.getElementById("sub-op-split").textContent =
        asset.operator_split_pct ? `${asset.operator_split_pct}%` : "—";
      document.getElementById("sub-poool-split").textContent =
        asset.poool_split_pct ? `${asset.poool_split_pct}%` : "—";
    } else {
      commoditySection.style.display = "none";
    }
  }
}

// ─── Token Math Auto-Validation ───────────────────────────────────────────────
function validateTokenMath(asset) {
  const resultEl = document.getElementById("chk-math-result");
  if (!resultEl) return;

  const computed = (asset.token_price_cents || 0) * (asset.tokens_total || 0);
  const total = asset.total_value_cents || 0;
  const diff = Math.abs(computed - total);
  const ok = diff <= 2; // allow 1 cent rounding

  if (ok) {
    resultEl.innerHTML = `<span style="color:var(--admin-success);font-size:11px;display:block;margin-top:4px;">
            ✓ ${formatUSD(asset.token_price_cents)} × ${(asset.tokens_total || 0).toLocaleString()} = ${formatUSD(computed)}
        </span>`;
    const mathChk = document.getElementById("chk-math");
    if (mathChk) {
      mathChk.checked = true;
      updateApproveButtonState();
    }
  } else {
    resultEl.innerHTML = `<span style="color:var(--admin-danger);font-size:11px;display:block;margin-top:4px;">
            ✗ ${formatUSD(asset.token_price_cents)} × ${(asset.tokens_total || 0).toLocaleString()} = ${formatUSD(computed)},
            but total_value = ${formatUSD(total)} — <strong>MISMATCH</strong>
        </span>`;
  }
}

// ─── Conditional Checklist Items ──────────────────────────────────────────────
function toggleConditionalChecklist(asset) {
  const videoRow = document.getElementById("chk-video-row");
  const gmapRow = document.getElementById("chk-gmap-row");
  if (videoRow) videoRow.style.display = asset.video_url ? "" : "none";
  if (gmapRow) gmapRow.style.display = asset.google_maps_url ? "" : "none";
}

// ─── Document Data Room ───────────────────────────────────────────────────────
const DOC_CATEGORIES = {
  Legal: [
    "proof_of_title",
    "legal_basis",
    "building_permit",
    "license_nib",
    "id_card",
  ],
  Tax: ["tax_npwp", "tax_pbb", "tax_bphtb", "owner_npwp"],
  Property: ["site_plan", "floor_plan", "expose"],
  Financial: ["appraisal", "financial"],
  Other: ["other"],
};

function renderDocuments(docs) {
  const container = document.getElementById("documents-container");
  if (!container) return;

  if (docs.length === 0) {
    container.innerHTML = `<div style="color:var(--admin-danger);font-size:13px;padding:16px;
            border:1px solid var(--admin-danger);border-radius:8px;background:rgba(239,68,68,.05);">
            ⚠ No documents uploaded for this submission.
        </div>`;
    return;
  }

  // Group ALL docs by type (array, not single item)
  const docsByType = {};
  docs.forEach((d) => {
    if (!docsByType[d.document_type]) docsByType[d.document_type] = [];
    docsByType[d.document_type].push(d);
  });

  // Track which types have been rendered
  const renderedTypes = new Set();
  const catCounts = {};

  const renderDocItem = (d) => `
    <div class="document-item">
        <div class="document-info">
            <span class="document-type">${(d.document_type || "").replace(/_/g, " ").toUpperCase()}</span>
            <span class="document-meta">${esc(d.title || "")}${d.file_size_bytes ? " · " + formatFileSize(d.file_size_bytes) : ""}</span>
        </div>
        <div style="display:flex;gap:6px;">
            <a href="${esc(d.file_url || `/api/documents/${d.id}/download`)}" target="_blank" rel="noopener"
               class="admin-btn admin-btn--secondary admin-btn--sm">
               📄 View
            </a>
            <a href="${esc(d.file_url || `/api/documents/${d.id}/download`)}" download
               class="admin-btn admin-btn--secondary admin-btn--sm">
               ↓
            </a>
        </div>
    </div>`;

  const categorySections = Object.entries(DOC_CATEGORIES)
    .map(([catName, types]) => {
      const catDocs = types.flatMap((t) => {
        renderedTypes.add(t);
        return docsByType[t] || [];
      });
      const hasDocs = catDocs.length > 0;
      catCounts[catName] = catDocs.length;

      return `
        <div style="margin-bottom:20px;">
            <div style="font-size:11px;font-weight:700;color:var(--admin-text-muted);
                        text-transform:uppercase;letter-spacing:.6px;margin-bottom:8px;
                        display:flex;align-items:center;gap:8px;">
                ${catName}
                ${hasDocs
                  ? `<span style="background:var(--admin-success);color:white;font-size:10px;padding:1px 6px;border-radius:4px;">${catDocs.length}</span>`
                  : `<span style="background:var(--admin-danger);color:white;font-size:10px;padding:1px 6px;border-radius:4px;">MISSING</span>`
                }
            </div>
            <div class="document-list">
                ${!hasDocs
                  ? `<div style="font-size:12px;color:var(--admin-text-muted);padding:8px 12px;
                                 border:1px dashed var(--admin-border);border-radius:6px;">
                        No ${catName.toLowerCase()} documents uploaded
                    </div>`
                  : catDocs.map(renderDocItem).join("")
                }
            </div>
        </div>`;
    })
    .join("");

  // Catch-all: any docs with types not in DOC_CATEGORIES
  const extraDocs = docs.filter((d) => !renderedTypes.has(d.document_type));
  const extraSection = extraDocs.length > 0 ? `
    <div style="margin-bottom:20px;">
        <div style="font-size:11px;font-weight:700;color:var(--admin-text-muted);
                    text-transform:uppercase;letter-spacing:.6px;margin-bottom:8px;
                    display:flex;align-items:center;gap:8px;">
            Other Documents
            <span style="background:var(--admin-primary);color:white;font-size:10px;padding:1px 6px;border-radius:4px;">${extraDocs.length}</span>
        </div>
        <div class="document-list">
            ${extraDocs.map(renderDocItem).join("")}
        </div>
    </div>` : "";

  container.innerHTML = categorySections + extraSection;
  autoCheckDocs(catCounts);
}

function autoCheckDocs(counts) {
  if (counts.Legal > 0) {
    const chk = document.getElementById("chk-legal");
    if (chk) { chk.checked = true; updateApproveButtonState(); }
  }
  if (counts.Tax > 0) {
    const chk = document.getElementById("chk-tax");
    if (chk) { chk.checked = true; updateApproveButtonState(); }
  }
  if (counts.Financial > 0) {
     const chk = document.getElementById("chk-fin");
     if (chk) { chk.checked = true; updateApproveButtonState(); }
  }
}

// ─── Image Gallery (Admin-editable) ──────────────────────────────────────────
let _adminImages = [];
let _adminImageEditMode = false;

function renderImages(images) {
  _adminImages = images.slice();
  _renderImageGrid();
  _initAdminImageUpload();
}

function _renderImageGrid() {
  const container = document.getElementById("images-container");
  const hint = document.getElementById("images-count-hint");
  if (!container) return;

  if (_adminImages.length === 0) {
    container.innerHTML = `<span style="color:var(--admin-text-muted);font-size:13px;">No images uploaded.</span>`;
    if (hint) hint.textContent = "";
    return;
  }

  if (hint) hint.textContent = `${_adminImages.length} image${_adminImages.length !== 1 ? "s" : ""}`;

  container.innerHTML = _adminImages.map((img, index) => {
    // normalise the URL — API may use image_url, url, or file_url
    const url = img.image_url || img.url || img.file_url || "";
    
    // Cover badge
    const coverControls = _adminImageEditMode ? `
        <button
          onclick="adminSetCover('${esc(img.id)}')"
          title="${img.is_cover ? 'Current Cover' : 'Set as cover'}"
          style="position:absolute;top:6px;left:6px;background:${img.is_cover ? 'var(--admin-primary)' : 'rgba(0,0,0,.70)'};border:none;border-radius:6px;padding:3px 7px;cursor:pointer;color:#fff;font-size:10px;font-weight:700;white-space:nowrap;z-index:4;">
          ★ COVER
        </button>
    ` : (img.is_cover ? `<span class="cover-badge" style="z-index:4;">COVER</span>` : "");

    // The order number
    const orderBadge = `
        <div style="position:absolute;bottom:6px;left:6px;background:rgba(0,0,0,.70);border-radius:4px;padding:2px 6px;color:#fff;font-size:11px;font-weight:700;z-index:3;pointer-events:none;">
          ${index + 1}
        </div>
    `;

    // Delete button
    const editControls = _adminImageEditMode ? `
        <button
          onclick="adminDeleteImage('${esc(img.id)}')"
          title="Delete"
          style="position:absolute;top:6px;right:6px;z-index:3;background:rgba(200,30,30,.85);border:none;border-radius:6px;padding:4px 6px;cursor:pointer;display:flex;align-items:center;">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>` : "";

    return `
      <div class="image-item" title="${esc(img.alt_text || "")}" style="position:relative;background:#f1f5f9;${_adminImageEditMode ? 'cursor:grab;' : ''}"
           ${_adminImageEditMode ? `draggable="true" ondragstart="adminImageDragStart(event, '${img.id}')" ondragover="adminImageDragOver(event)" ondrop="adminImageDrop(event, '${img.id}')" ondragenter="adminImageDragEnter(event)" ondragleave="adminImageDragLeave(event)"` : ''}>
          ${url
            ? `<img src="${esc(url)}" alt="Property Image"
               style="width:100%;height:100%;object-fit:cover;display:block;pointer-events:none;"
               onerror="this.style.display='none';this.nextElementSibling.style.display='flex';this.onerror=null;" />
               <div style="display:none;width:100%;height:100%;align-items:center;justify-content:center;flex-direction:column;gap:6px;color:#94a3b8;pointer-events:none;">
                 <svg width='28' height='28' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.5'><rect x='3' y='3' width='18' height='18' rx='2'/><circle cx='8.5' cy='8.5' r='1.5'/><polyline points='21 15 16 10 5 21'/></svg>
                 <span style='font-size:10px;'>Image unavailable</span>
               </div>`
            : `<div style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;flex-direction:column;gap:6px;color:#94a3b8;pointer-events:none;">
                 <svg width='28' height='28' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.5'><rect x='3' y='3' width='18' height='18' rx='2'/><circle cx='8.5' cy='8.5' r='1.5'/><polyline points='21 15 16 10 5 21'/></svg>
                 <span style='font-size:10px;'>No URL</span>
               </div>`
          }
          ${coverControls}
          ${orderBadge}
          ${editControls}
          ${url && !_adminImageEditMode ? `<a href="${esc(url)}" target="_blank" rel="noopener"
             style="position:absolute;inset:0;display:flex;align-items:flex-end;justify-content:flex-start;
                    padding:6px;background:linear-gradient(transparent,rgba(0,0,0,.4));opacity:0;transition:.2s;"
             class="img-overlay-link">
             <span style="font-size:10px;color:white;">Open ↗</span>
          </a>` : ""}
      </div>`;
  }).join("");

  container.querySelectorAll(".image-item").forEach((el) => {
    const link = el.querySelector(".img-overlay-link");
    el.addEventListener("mouseenter", () => { if (link) link.style.opacity = "1"; });
    el.addEventListener("mouseleave", () => { if (link) link.style.opacity = "0"; });
  });
}

function toggleImageEdit() {
  _adminImageEditMode = !_adminImageEditMode;
  const btn = document.getElementById("toggle-image-edit-btn");
  const zone = document.getElementById("admin-image-upload-zone");
  if (btn) {
    btn.innerHTML = _adminImageEditMode
      ? `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg> Done Editing`
      : `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg> Edit Images`;
    btn.style.background = _adminImageEditMode ? "var(--admin-accent,#4f46e5)" : "";
    btn.style.color = _adminImageEditMode ? "#fff" : "";
    btn.style.borderColor = _adminImageEditMode ? "transparent" : "";
  }
  if (zone) zone.style.display = _adminImageEditMode ? "block" : "none";
  _renderImageGrid();
}

function _getAssetIdForImages() {
  return (window.pageData && window.pageData.asset && window.pageData.asset.id) || null;
}

async function adminDeleteImage(imgId) {
  if (!await pooolConfirm({ title: 'Delete image', message: 'This cannot be undone.', confirmText: 'Delete', type: 'danger' })) return;
  const assetId = _getAssetIdForImages();
  if (!assetId) { showToast("Could not determine asset ID", "error"); return; }
  try {
    const res = await fetch(`/api/admin/assets/${assetId}/images/${imgId}`, {
      method: "DELETE",
      headers: { "X-CSRF-Token": getCsrfToken() }
    });
    if (!res.ok) throw new Error("Delete failed");
    _adminImages = _adminImages.filter(i => i.id !== imgId);
    _renderImageGrid();
    showToast("Image deleted", "success");
  } catch (e) {
    showToast("Failed to delete image: " + e.message, "error");
  }
}

// ─── Drag & Drop Reordering ────────────────────────────────────────────────

let _draggedImageId = null;

function adminImageDragStart(e, imgId) {
  _draggedImageId = imgId;
  e.dataTransfer.effectAllowed = 'move';
  setTimeout(() => {
    const el = e.target.closest('.image-item');
    if (el) el.style.opacity = '0.4';
  }, 0);
}

function adminImageDragOver(e) {
  e.preventDefault();
  e.dataTransfer.dropEffect = 'move';
}

function adminImageDragEnter(e) {
  e.preventDefault();
  const item = e.target.closest('.image-item');
  if (item) item.style.border = '2px dashed var(--admin-primary)';
}

function adminImageDragLeave(e) {
  const item = e.target.closest('.image-item');
  if (item) item.style.border = '';
}

async function adminImageDrop(e, targetImgId) {
  e.preventDefault();
  const targetItem = e.target.closest('.image-item');
  if (targetItem) targetItem.style.border = '';
  document.querySelectorAll('.image-item').forEach(el => el.style.opacity = '1');

  if (!_draggedImageId || String(_draggedImageId) === String(targetImgId)) return;

  const dragIndex = _adminImages.findIndex(i => String(i.id) === String(_draggedImageId));
  const dropIndex = _adminImages.findIndex(i => String(i.id) === String(targetImgId));

  if (dragIndex === -1 || dropIndex === -1) return;

  const previousImages = _adminImages.map((img) => ({ ...img }));

  // Move array item
  const [draggedItem] = _adminImages.splice(dragIndex, 1);
  _adminImages.splice(dropIndex, 0, draggedItem);

  // The first image is always the cover
  _adminImages.forEach((img, idx) => {
    img.sort_order = idx;
    img.is_cover = (idx === 0);
  });

  _renderImageGrid(); // Optimistic rendering

  const assetId = _getAssetIdForImages();
  if (!assetId) return;

  const payload = _adminImages.map((img) => ({
    id: img.id, sort_order: img.sort_order, is_cover: img.is_cover
  }));

  try {
    const res = await fetch(`/api/admin/assets/${assetId}/images/reorder`, {
      method: "PUT",
      headers: { "Content-Type": "application/json", "X-CSRF-Token": getCsrfToken() },
      body: JSON.stringify(payload)
    });
    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      throw new Error(err.error || "Failed to save order");
    }
  } catch (err) {
    _adminImages = previousImages;
    _renderImageGrid();
    showToast("Failed to save new order: " + err.message, "error");
  }
}

async function adminSetCover(imgId) {
  const index = _adminImages.findIndex(i => String(i.id) === String(imgId));
  if (index === -1) return;

  const previousImages = _adminImages.map((img) => ({ ...img }));
  
  // Move it to index 0
  const [item] = _adminImages.splice(index, 1);
  _adminImages.unshift(item);

  // The first image is always the cover
  _adminImages.forEach((img, idx) => {
    img.sort_order = idx;
    img.is_cover = (idx === 0);
  });

  _renderImageGrid(); // Optimistic rendering
  
  const assetId = _getAssetIdForImages();
  if (!assetId) return;

  const payload = _adminImages.map((img) => ({
    id: img.id, sort_order: img.sort_order, is_cover: img.is_cover
  }));

  try {
    const res = await fetch(`/api/admin/assets/${assetId}/images/reorder`, {
      method: "PUT",
      headers: { "Content-Type": "application/json", "X-CSRF-Token": getCsrfToken() },
      body: JSON.stringify(payload)
    });
    if (!res.ok) throw new Error("Failed validation or unauthorized");
    showToast("Cover updated", "success");
  } catch (e) {
    _adminImages = previousImages;
    _renderImageGrid();
    showToast("Failed to set cover on server: " + e.message, "error");
  }
}

function _initAdminImageUpload() {
  const fileInput = document.getElementById("admin-image-file-input");
  const dropArea = document.getElementById("admin-image-drop-area");
  if (!fileInput || !dropArea || fileInput.dataset.adminInit) return;
  fileInput.dataset.adminInit = "1";

  fileInput.addEventListener("change", (e) => { _adminHandleFiles(e.target.files); fileInput.value = ""; });

  dropArea.addEventListener("dragover", (e) => {
    e.preventDefault();
    dropArea.style.borderColor = "var(--admin-accent,#4f46e5)";
    dropArea.style.background = "rgba(79,70,229,.05)";
  });
  dropArea.addEventListener("dragleave", () => {
    dropArea.style.borderColor = "var(--admin-border)";
    dropArea.style.background = "var(--admin-bg-secondary)";
  });
  dropArea.addEventListener("drop", (e) => {
    e.preventDefault();
    dropArea.style.borderColor = "var(--admin-border)";
    dropArea.style.background = "var(--admin-bg-secondary)";
    _adminHandleFiles(e.dataTransfer.files);
  });
  dropArea.addEventListener("click", (e) => {
    if (!e.target.closest("span[onclick]") && e.target.tagName !== "INPUT") fileInput.click();
  });
}

async function _adminHandleFiles(files) {
  const assetId = _getAssetIdForImages();
  if (!assetId) { showToast("Could not determine asset ID", "error"); return; }
  const progress = document.getElementById("admin-image-upload-progress");
  const arr = Array.from(files).filter(f => f.type.startsWith("image/"));
  if (!arr.length) { showToast("Please select image files only", "warning"); return; }

  for (const file of arr) {
    if (file.size > 20 * 1024 * 1024) { showToast(`${file.name} is too large (max 20 MB)`, "warning"); continue; }
    const rowId = "uprow-" + Date.now();
    if (progress) progress.insertAdjacentHTML("beforeend", `
      <div id="${rowId}" style="display:flex;align-items:center;gap:8px;font-size:12px;color:var(--admin-text-secondary);padding:4px 0;">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" style="flex-shrink:0;animation:spin 1s linear infinite"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
        <span>${esc(file.name)}</span>
      </div>`);
    const formData = new FormData();
    formData.append("file", file);
    formData.append("sort_order", String(_adminImages.length));
    formData.append("is_cover", _adminImages.length === 0 ? "true" : "false");
    try {
      const assetIdForUpload = _getAssetIdForImages();
      const uploadEndpoint = assetIdForUpload
        ? `/api/admin/assets/${assetIdForUpload}/images`
        : null;
      if (!uploadEndpoint) throw new Error("Cannot determine asset ID");
      const res = await fetch(uploadEndpoint, {
        method: "POST",
        headers: { "X-CSRF-Token": getCsrfToken() },
        body: formData
      });
      if (!res.ok) throw new Error(((await res.json().catch(() => ({}))).error) || "Upload failed");
      const data = await res.json();
      // Use the newly returned `image_id` instead of `id` since the API returns image_id
      _adminImages.push({ 
        id: data.image_id, 
        image_url: data.image_url, 
        url: data.image_url, 
        is_cover: data.is_cover, 
        sort_order: _adminImages.length 
      });
      document.getElementById(rowId)?.remove();
      _renderImageGrid();
      showToast(`${file.name} uploaded`, "success");
    } catch (e) {
      const row = document.getElementById(rowId);
      if (row) row.innerHTML = `<span style="color:#f04438;">✕ ${esc(file.name)}: ${esc(e.message)}</span>`;
      setTimeout(() => document.getElementById(rowId)?.remove(), 4000);
    }
  }
}

// ─── Milestones ───────────────────────────────────────────────────────────────
const SAMPLE_MILESTONES = [
  { month_index: 1, title: "Property Acquisition Complete", description: "Legal transfer of title deed and ownership confirmed.", milestone_date: null, is_completed: true },
  { month_index: 2, title: "SPV & Legal Structure Established", description: "Special Purpose Vehicle registered, ownership structure confirmed.", milestone_date: null, is_completed: true },
  { month_index: 3, title: "Token Offering Launch", description: "Tokens listed on POOOL marketplace, funding round opens to investors.", milestone_date: null, is_completed: false },
  { month_index: 6, title: "Funding Target Reached", description: "100% of tokens sold, funding round closed.", milestone_date: null, is_completed: false },
  { month_index: 9, title: "First Rental Distribution", description: "First quarterly dividend distributed to all token holders.", milestone_date: null, is_completed: false },
  { month_index: 24, title: "Performance Review & Revaluation", description: "Independent appraisal conducted, token value updated.", milestone_date: null, is_completed: false },
];

function renderMilestones(milestones, assetId) {
  const container = document.getElementById("milestones-container");
  if (!container) return;

  // Remove any prior sample-milestone-note (now that this list is editable)
  const card = container.closest(".review-card");
  card?.querySelector(".sample-milestone-note")?.remove();

  if (!milestones || milestones.length === 0) {
    container.innerHTML = `<tr><td colspan="6" style="padding:20px 28px;text-align:center;color:var(--admin-text-muted);font-size:12px;">
      No milestones yet. Click "Add milestone" to create the project roadmap.
    </td></tr>`;
    return;
  }

  container.innerHTML = milestones
    .map((m) => milestoneRowHtml(m))
    .join("");

  container.querySelectorAll("[data-milestone-id]").forEach((row) => {
    wireMilestoneRow(row, assetId);
  });
}

function milestoneRowHtml(m) {
  const id = esc(m.id || "");
  const month = m.month_index != null ? m.month_index : "";
  const dateVal = m.milestone_date ? toDateInputValue(m.milestone_date) : "";
  return `
    <tr data-milestone-id="${id}">
      <td><input class="pc-input ms-month" type="number" min="0" step="1" value="${esc(String(month))}" style="width:60px;padding:6px 8px;font-size:12px;" /></td>
      <td><input class="pc-input ms-title" value="${esc(m.title || "")}" maxlength="255" style="font-size:13px;font-weight:600;" /></td>
      <td><input class="pc-input ms-desc" value="${esc(m.description || "")}" style="font-size:12px;" /></td>
      <td><input class="pc-input ms-date" type="date" value="${esc(dateVal)}" style="font-size:12px;" /></td>
      <td style="text-align:center;"><input type="checkbox" class="ms-done" ${m.is_completed ? "checked" : ""} style="width:16px;height:16px;cursor:pointer;" /></td>
      <td style="text-align:right;">
        <button type="button" class="admin-btn admin-btn--secondary ms-save-btn" style="padding:4px 8px;font-size:11px;margin-right:4px;display:none;">Save</button>
        <button type="button" class="ms-delete-btn" title="Delete" style="background:transparent;border:none;color:var(--admin-danger);cursor:pointer;padding:4px;">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-2 14a2 2 0 0 1-2 2H9a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6M14 11v6"/></svg>
        </button>
      </td>
    </tr>
  `;
}

function wireMilestoneRow(row, assetId) {
  const milestoneId = row.dataset.milestoneId;
  const saveBtn = row.querySelector(".ms-save-btn");
  const showSave = () => { if (saveBtn) saveBtn.style.display = "inline-block"; };
  row.querySelectorAll("input").forEach((inp) => {
    inp.addEventListener("input", showSave);
    inp.addEventListener("change", showSave);
  });
  if (saveBtn) {
    saveBtn.addEventListener("click", async () => {
      const payload = collectMilestoneInputs(row);
      try {
        const res = await fetch(`/api/admin/assets/${assetId}/milestones/${milestoneId}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json", "X-CSRF-Token": getCsrfToken() },
          body: JSON.stringify(payload),
        });
        if (!res.ok) throw new Error((await res.json().catch(() => ({}))).error || `HTTP ${res.status}`);
        saveBtn.style.display = "none";
        showToast("Milestone updated", "success");
      } catch (e) {
        showToast(`Failed to save milestone: ${e.message}`, "error");
      }
    });
  }
  const deleteBtn = row.querySelector(".ms-delete-btn");
  if (deleteBtn) {
    deleteBtn.addEventListener("click", async () => {
      if (!confirm("Delete this milestone?")) return;
      try {
        const res = await fetch(`/api/admin/assets/${assetId}/milestones/${milestoneId}`, {
          method: "DELETE",
          headers: { "X-CSRF-Token": getCsrfToken() },
        });
        if (!res.ok) throw new Error((await res.json().catch(() => ({}))).error || `HTTP ${res.status}`);
        row.remove();
        showToast("Milestone deleted", "success");
      } catch (e) {
        showToast(`Failed to delete: ${e.message}`, "error");
      }
    });
  }
}

function collectMilestoneInputs(row) {
  const month = row.querySelector(".ms-month").value.trim();
  const title = row.querySelector(".ms-title").value.trim();
  const desc = row.querySelector(".ms-desc").value.trim();
  const date = row.querySelector(".ms-date").value;
  const done = row.querySelector(".ms-done").checked;
  return {
    title: title || null,
    description: desc === "" ? null : desc,
    month_index: month === "" ? null : Number(month),
    milestone_date: date ? new Date(date).toISOString() : null,
    is_completed: done,
  };
}

function wireMilestoneAddButton(assetId) {
  const btn = document.getElementById("btn-milestone-add");
  if (!btn || !assetId) return;
  btn.onclick = async () => {
    const title = prompt("Milestone title:");
    if (!title || !title.trim()) return;
    try {
      const res = await fetch(`/api/admin/assets/${assetId}/milestones`, {
        method: "POST",
        headers: { "Content-Type": "application/json", "X-CSRF-Token": getCsrfToken() },
        body: JSON.stringify({ title: title.trim() }),
      });
      if (!res.ok) throw new Error((await res.json().catch(() => ({}))).error || `HTTP ${res.status}`);
      const created = await res.json();
      const tbody = document.getElementById("milestones-container");
      // Clear empty-state row if present
      if (tbody.querySelector("td[colspan]")) tbody.innerHTML = "";
      tbody.insertAdjacentHTML("beforeend", milestoneRowHtml(created));
      const row = tbody.querySelector(`[data-milestone-id="${created.id}"]`);
      if (row) wireMilestoneRow(row, assetId);
      showToast("Milestone added", "success");
    } catch (e) {
      showToast(`Failed to add milestone: ${e.message}`, "error");
    }
  };
}

function toDateInputValue(iso) {
  // iso may be a postgres timestamp string; take YYYY-MM-DD prefix
  const m = String(iso).match(/^(\d{4}-\d{2}-\d{2})/);
  return m ? m[1] : "";
}

// ─── Property Page Content Form ──────────────────────────────────────────────
function hydratePageContentForm(asset) {
  const set = (id, v) => {
    const el = document.getElementById(id);
    if (el) el.value = v == null ? "" : v;
  };
  set("pc-location-description", asset.location_description);
  set("pc-investment-type", asset.investment_type);
  set("pc-investment-type-description", asset.investment_type_description);
  set("pc-leasing-strategy-type", asset.leasing_strategy_type);
  set("pc-leasing-strategy-description", asset.leasing_strategy_description);
  // Calculator: cents → USD; bps → percent
  set(
    "pc-default-investment-amount",
    asset.default_investment_amount_cents != null
      ? Math.round(asset.default_investment_amount_cents / 100)
      : ""
  );
  set(
    "pc-default-value-growth",
    asset.default_value_growth_bps != null ? asset.default_value_growth_bps / 100 : ""
  );
  set(
    "pc-default-rental-yield",
    asset.default_rental_yield_bps != null ? asset.default_rental_yield_bps / 100 : ""
  );
  set("pc-developer-name", asset.developer_name);
  set("pc-developer-logo-url", asset.developer_logo_url);
  set("pc-developer-description", asset.developer_description);
  set("pc-developer-website", asset.developer_website);
  set("pc-developer-facebook", asset.developer_facebook);
  set("pc-developer-instagram", asset.developer_instagram);
  set("pc-developer-youtube", asset.developer_youtube);
  set("pc-risk-notification", asset.risk_notification);
}

function wirePageContentSave(asset) {
  const btn = document.getElementById("btn-save-page-content");
  const status = document.getElementById("page-content-status");
  if (!btn || !asset.id) return;
  btn.onclick = async () => {
    const body = collectPageContentPayload();
    btn.disabled = true;
    if (status) status.textContent = "Saving…";
    try {
      const res = await fetch(`/api/admin/assets/${asset.id}/page-content`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json", "X-CSRF-Token": getCsrfToken() },
        body: JSON.stringify(body),
      });
      if (!res.ok) throw new Error((await res.json().catch(() => ({}))).error || `HTTP ${res.status}`);
      const data = await res.json();
      if (status) status.textContent = `Saved ${data.fields_updated?.length ?? 0} fields`;
      showToast("Property page content saved", "success");
    } catch (e) {
      if (status) status.textContent = "";
      showToast(`Save failed: ${e.message}`, "error");
    } finally {
      btn.disabled = false;
    }
  };
}

function collectPageContentPayload() {
  const form = document.getElementById("page-content-form");
  if (!form) return {};
  const out = {};
  form.querySelectorAll("[data-field]").forEach((el) => {
    const field = el.dataset.field;
    const unit = el.dataset.unit;
    const raw = el.value;
    if (unit === "cents") {
      out[field] = raw === "" ? null : Math.round(Number(raw) * 100);
    } else if (unit === "bps") {
      out[field] = raw === "" ? null : Math.round(Number(raw) * 100);
    } else {
      out[field] = raw.trim() === "" ? null : raw.trim();
    }
  });
  return out;
}

// ─── Actions ──────────────────────────────────────────────────────────────────

// For actions requiring a reason, show a modal; for others confirm directly
async function handleDecision(action) {
  const needsReason = action === "reject" || action === "request_revision";
  if (needsReason) {
    openReasonModal(action);
  } else {
    const actionLabels = {
      approve: "approve and publish this project to the marketplace",
      in_review: "mark this project as in review",
    };
    if (!await pooolConfirm({
      title: actionLabels[action] ? 'Confirm action' : action,
      message: `Are you sure you want to ${actionLabels[action] || action}? The developer will be notified immediately.`,
      confirmText: action === 'approve' ? 'Approve & Publish' : 'Confirm',
      type: action === 'approve' ? 'success' : 'default',
    })) return;
    submitDecision(action, "");
  }
}

let _pendingAction = null;
function openReasonModal(action) {
  _pendingAction = action;
  const titles = {
    reject: "Reject Submission",
    request_revision: "Request Revision",
  };
  const subtitles = {
    reject: "Please provide a reason for rejection. The developer will be notified with this message.",
    request_revision: "Describe what changes are required. The developer will be notified with this message.",
  };
  document.getElementById("reason-modal-title").textContent = titles[action] || action;
  document.getElementById("reason-modal-subtitle").textContent = subtitles[action] || "";
  document.getElementById("reason-modal-text").value = "";
  document.getElementById("reason-modal").style.display = "flex";
  setTimeout(() => document.getElementById("reason-modal-text").focus(), 50);
}

function closeReasonModal() {
  const modal = document.getElementById("reason-modal");
  if (modal) modal.style.display = "none";
  _pendingAction = null;
}

function trapReasonModalFocus(event) {
  if (event.key !== "Tab") return;
  const modal = document.getElementById("reason-modal");
  if (!modal || modal.style.display === "none") return;

  const focusable = Array.from(
    modal.querySelectorAll("textarea, button, [href], input, select, [tabindex]:not([tabindex='-1'])")
  ).filter((el) => !el.disabled && el.offsetParent !== null);
  if (!focusable.length) return;

  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

document.addEventListener("DOMContentLoaded", () => {
  const reasonModal = document.getElementById("reason-modal");
  reasonModal?.addEventListener("keydown", trapReasonModalFocus);
  reasonModal?.addEventListener("click", (event) => {
    if (event.target === reasonModal) closeReasonModal();
  });
  document.getElementById("reason-modal-cancel")?.addEventListener("click", closeReasonModal);
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && reasonModal && reasonModal.style.display !== "none") {
      closeReasonModal();
    }
  });
  document.getElementById("reason-modal-confirm")?.addEventListener("click", () => {
    const reason = document.getElementById("reason-modal-text").value.trim();
    if (!reason) {
      document.getElementById("reason-modal-text").style.borderColor = "var(--admin-danger)";
      document.getElementById("reason-modal-text").focus();
      return;
    }
    if (reasonModal) reasonModal.style.display = "none";
    submitDecision(_pendingAction, reason);
    _pendingAction = null;
  });
  document.getElementById("btn-add-note")?.addEventListener("click", addNote);
});

async function submitDecision(action, notes) {
  const btnMap = {
    approve: "btn-approve",
    reject: "btn-reject",
    request_revision: "btn-revise",
    in_review: "btn-in-review",
  };
  const btnEl = document.getElementById(btnMap[action] || "btn-approve");
  const originalHTML = btnEl ? btnEl.innerHTML : "";
  if (btnEl) { btnEl.disabled = true; btnEl.textContent = "Processing…"; }

  // Clear any previous error panel
  const existingError = document.getElementById("submission-error-panel");
  if (existingError) existingError.remove();

  try {
    const response = await fetch(`/api/admin/developer-projects/${projectId}/review`, {
      method: "POST",
      headers: { 
        "Content-Type": "application/json",
        "X-CSRF-Token": getCsrfToken()
      },
      body: JSON.stringify({ action, notes }),
    });

    if (response.ok) {
      const msgs = {
        approve: "✓ Project approved and published! Developer has been notified.",
        reject: "✓ Project rejected. Developer has been notified.",
        request_revision: "✓ Revision request sent. Project returned to queue.",
        in_review: "✓ Project marked as In Review. Developer has been notified.",
      };
      showToast(msgs[action] || "Action completed.", "success");
      setTimeout(() => { window.location.href = "/admin/developer-submissions"; }, 1500);
    } else {
      let errorMessage = "Unknown server error";
      let errorDetail = "";
      try {
        const err = await response.json();
        errorMessage = err.error || errorMessage;
        errorDetail = err.detail || err.details || "";
      } catch (_) {
        try { errorMessage = await response.text(); } catch (_) {}
      }

      const actionLabels = {
        approve: "Approve & Publish",
        reject: "Reject Submission",
        request_revision: "Request Revision",
        in_review: "Mark In Review",
      };

      // Show persistent error panel above the action buttons
      const errorPanel = document.createElement("div");
      errorPanel.id = "submission-error-panel";
      errorPanel.style.cssText = `
        margin-bottom:16px;padding:16px;border-radius:10px;
        background:rgba(239,68,68,.08);border:1px solid rgba(239,68,68,.3);
        animation:admin-fadeIn .2s ease-out;
      `;
      errorPanel.innerHTML = `
        <div style="display:flex;align-items:flex-start;gap:10px;">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="round" style="flex-shrink:0;margin-top:1px;">
            <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <div style="flex:1;min-width:0;">
            <div style="font-weight:700;font-size:13px;color:var(--admin-danger);margin-bottom:6px;">
              ${esc(actionLabels[action] || action)} Failed
              <span style="font-weight:500;font-size:11px;color:var(--admin-text-muted);margin-left:8px;">HTTP ${response.status}</span>
            </div>
            <div style="font-size:12px;color:var(--admin-text-primary);line-height:1.6;word-break:break-word;white-space:pre-wrap;">${esc(errorMessage)}</div>
            ${errorDetail ? `<div style="font-size:11px;color:var(--admin-text-muted);margin-top:6px;word-break:break-word;">${esc(errorDetail)}</div>` : ""}
            <div style="font-size:10px;color:var(--admin-text-muted);margin-top:8px;">
              ${new Date().toLocaleString()} · Project: ${esc(projectId)}
            </div>
          </div>
          <button onclick="this.closest('#submission-error-panel').remove()" style="background:none;border:none;cursor:pointer;color:var(--admin-text-muted);font-size:18px;line-height:1;padding:0 4px;" title="Dismiss">×</button>
        </div>
      `;

      // Insert the error panel above the action buttons in the sidebar
      const decisionCard = document.querySelector(".decision-card");
      const actionBtnContainer = decisionCard?.querySelector("div[style*='flex-direction: column']") || decisionCard?.querySelector("div[style*='flex-direction']");
      if (actionBtnContainer) {
        actionBtnContainer.insertAdjacentElement("beforebegin", errorPanel);
      } else {
        // Fallback: show at the top of the page content
        document.querySelector(".admin-content")?.prepend(errorPanel);
      }

      showToast("Action failed — see error details in the sidebar.", "error");
      if (btnEl) { btnEl.disabled = false; btnEl.innerHTML = originalHTML; }
    }
  } catch (err) {
    // Network / CORS / other JS error
    const errorPanel = document.createElement("div");
    errorPanel.id = "submission-error-panel";
    errorPanel.style.cssText = `
      margin-bottom:16px;padding:16px;border-radius:10px;
      background:rgba(239,68,68,.08);border:1px solid rgba(239,68,68,.3);
      animation:admin-fadeIn .2s ease-out;
    `;
    errorPanel.innerHTML = `
      <div style="display:flex;align-items:flex-start;gap:10px;">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#ef4444" stroke-width="2" stroke-linecap="round" style="flex-shrink:0;margin-top:1px;">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        <div style="flex:1;min-width:0;">
          <div style="font-weight:700;font-size:13px;color:var(--admin-danger);margin-bottom:6px;">Network Error</div>
          <div style="font-size:12px;color:var(--admin-text-primary);line-height:1.6;">${esc(err.message || "Could not reach the server. Please check that the backend is running and try again.")}</div>
          <div style="font-size:10px;color:var(--admin-text-muted);margin-top:8px;">${new Date().toLocaleString()}</div>
        </div>
        <button onclick="this.closest('#submission-error-panel').remove()" style="background:none;border:none;cursor:pointer;color:var(--admin-text-muted);font-size:18px;line-height:1;padding:0 4px;" title="Dismiss">×</button>
      </div>
    `;
    const decisionCard = document.querySelector(".decision-card");
    const actionBtnContainer = decisionCard?.querySelector("div[style*='flex-direction: column']") || decisionCard?.querySelector("div[style*='flex-direction']");
    if (actionBtnContainer) {
      actionBtnContainer.insertAdjacentElement("beforebegin", errorPanel);
    } else {
      document.querySelector(".admin-content")?.prepend(errorPanel);
    }
    showToast("Network error — see details in the sidebar.", "error");
    if (btnEl) { btnEl.disabled = false; btnEl.innerHTML = originalHTML; }
  }
}

// ─── Admin Notes ──────────────────────────────────────────────────────────────
async function loadNotes() {
  if (!projectId) return;
  try {
    const res = await fetch(`/api/admin/developer-projects/${projectId}/notes`);
    const data = await res.json();
    renderNotes(data.notes || []);
  } catch (e) {
    const el = document.getElementById("notes-history");
    if (el) el.innerHTML = `<div style="color:var(--admin-text-muted);font-size:13px;">Failed to load notes.</div>`;
  }
}

function renderNotes(notes) {
  const el = document.getElementById("notes-history");
  if (!el) return;
  if (!notes.length) {
    el.innerHTML = `<div style="color:var(--admin-text-muted);font-size:13px;padding:12px;text-align:center;">
      No notes yet. Be the first to add one.
    </div>`;
    return;
  }
  el.innerHTML = notes.map((n) => {
    const initials = (n.author_name || n.author_email || "A").split(" ").map(w => w[0]).join("").toUpperCase().slice(0, 2);
    const date = n.created_at ? new Date(n.created_at).toLocaleString("en-US", {
      month: "short", day: "numeric", year: "numeric", hour: "2-digit", minute: "2-digit"
    }) : "";
    return `
      <div style="padding:14px 16px;border:1px solid rgba(234, 179, 8, 0.3);border-radius:10px;
           background:rgba(234, 179, 8, 0.1);display:flex;gap:12px;align-items:flex-start;">
        <div style="width:34px;height:34px;border-radius:50%;background:var(--admin-accent, #4f46e5);
             color:white;display:flex;align-items:center;justify-content:center;
             font-size:12px;font-weight:700;flex-shrink:0;">${esc(initials)}</div>
        <div style="flex:1;min-width:0;">
          <div style="display:flex;justify-content:space-between;align-items:baseline;margin-bottom:6px;">
            <span style="font-size:13px;font-weight:600;color:var(--admin-text-primary);">${esc(n.author_name || n.author_email)}</span>
            <span style="font-size:11px;color:var(--admin-text-muted);white-space:nowrap;margin-left:10px;">${date}</span>
          </div>
          <p style="margin:0;font-size:13px;color:var(--admin-text-primary);line-height:1.6;
               white-space:pre-wrap;word-break:break-word;">${esc(n.content)}</p>
        </div>
      </div>`;
  }).join("");
}

async function addNote() {
  const textarea = document.getElementById("new-admin-note");
  const content = textarea?.value?.trim();
  if (!content) {
    if (textarea) { textarea.style.borderColor = "var(--admin-danger)"; textarea.focus(); }
    return;
  }
  const btn = document.getElementById("btn-add-note");
  if (btn) { btn.disabled = true; btn.textContent = "Saving…"; }

  try {
    const res = await fetch(`/api/admin/developer-projects/${projectId}/notes`, {
      method: "POST",
      headers: { 
        "Content-Type": "application/json",
        "X-CSRF-Token": getCsrfToken()
      },
      body: JSON.stringify({ content }),
    });
    if (!res.ok) throw new Error((await res.json().catch(() => ({}))).error || "Failed");
    if (textarea) { textarea.value = ""; textarea.style.borderColor = ""; }
    showToast("Note added.", "success");
    await loadNotes();
  } catch (e) {
    showToast("Failed to add note: " + e.message, "error");
  } finally {
    if (btn) { btn.disabled = false; btn.textContent = ""; btn.innerHTML = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg> Add Note`; }
  }
}



// ─── Helpers ──────────────────────────────────────────────────────────────────
function getProjectStatusBadge(status) {
  const map = {
    draft: ["admin-badge--neutral", "Draft"],
    submitted: ["admin-badge--warning", "Submitted"],
    in_review: ["admin-badge--info", "In Review"],
    approved: ["admin-badge--success", "Approved"],
    rejected: ["admin-badge--danger", "Rejected"],
    live: ["admin-badge--success", "Live"],
  };
  const [cls, label] = map[status] || ["admin-badge--neutral", status];
  return `<span class="admin-badge ${cls}">${label}</span>`;
}

function formatUSD(cents) {
  if (typeof cents !== "number" || cents == null) return "—";
  return (
    "$" +
    (Math.abs(cents) / 100).toLocaleString("en-US", {
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
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

function formatFileSize(bytes) {
  if (!bytes) return "Unknown size";
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / 1024 / 1024).toFixed(2) + " MB";
}

function formatLabel(str) {
  if (!str) return "";
  return str.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function esc(str) {
  if (typeof str !== "string") return str || "";
  const d = document.createElement("div");
  d.textContent = str;
  return d.innerHTML;
}

function showToast(message, type = "info") {
  if(window.showPooolToast) {
    window.showPooolToast(null, message, type);
  }
}
