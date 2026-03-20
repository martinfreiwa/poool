/**
 * Admin Rewards & Referrals — Tabbed interface
 * Tabs: Tier Configuration, User Tiers, Reward Balances, Referral Programme
 */

let tiers = [],
  userTiers = [],
  balances = [],
  referrals = [],
  referralCodes = [];
let editingTier = null;

let utSortField = "invested_12m",
  utSortOrder = "desc";
let balSortField = "total",
  balSortOrder = "desc";
let refSortField = "created_at",
  refSortOrder = "desc";
let codeSortField = "code_created_at",
  codeSortOrder = "desc";

let utPage = 1,
  balPage = 1,
  refPage = 1,
  codePage = 1;
const PAGE_SIZE = 15;
let utFiltered = [],
  balFiltered = [],
  refFiltered = [],
  codeFiltered = [];

document.addEventListener("DOMContentLoaded", () => {
  const urlParams = new URLSearchParams(window.location.search);
  const tab = urlParams.get('tab') || 'tiers';
  switchTab(tab, false);

  loadAll();
  
  const bindSearch = (id, callback) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener("input", debounce((e) => {
        el.style.opacity = '0.5';
        callback();
        setTimeout(() => el.style.opacity = '1', 200);
    }, 250));
  };

  bindSearch("ut-search", () => { utPage = 1; renderUserTiers(); });
  document.getElementById("ut-filter-tier")?.addEventListener("change", () => {
    utPage = 1;
    renderUserTiers();
  });
  bindSearch("bal-search", () => { balPage = 1; renderBalances(); });
  bindSearch("ref-search", () => { refPage = 1; renderReferrals(); });
  document
    .getElementById("ref-filter-status")
    ?.addEventListener("change", () => {
      refPage = 1;
      renderReferrals();
    });
  bindSearch("code-search", () => { codePage = 1; renderCodes(); });

  // Pagination Listeners
  document.getElementById("ut-prev-page")?.addEventListener("click", () => {
    if (utPage > 1) {
      utPage--;
      renderUserTiers();
    }
  });
  document.getElementById("ut-next-page")?.addEventListener("click", () => {
    if (utPage < Math.ceil(utFiltered.length / PAGE_SIZE)) {
      utPage++;
      renderUserTiers();
    }
  });

  document.getElementById("bal-prev-page")?.addEventListener("click", () => {
    if (balPage > 1) {
      balPage--;
      renderBalances();
    }
  });
  document.getElementById("bal-next-page")?.addEventListener("click", () => {
    if (balPage < Math.ceil(balFiltered.length / PAGE_SIZE)) {
      balPage++;
      renderBalances();
    }
  });

  document.getElementById("ref-prev-page")?.addEventListener("click", () => {
    if (refPage > 1) {
      refPage--;
      renderReferrals();
    }
  });
  document.getElementById("ref-next-page")?.addEventListener("click", () => {
    if (refPage < Math.ceil(refFiltered.length / PAGE_SIZE)) {
      refPage++;
      renderReferrals();
    }
  });

  document.getElementById("code-prev-page")?.addEventListener("click", () => {
    if (codePage > 1) {
      codePage--;
      renderCodes();
    }
  });
  document.getElementById("code-next-page")?.addEventListener("click", () => {
    if (codePage < Math.ceil(codeFiltered.length / PAGE_SIZE)) {
      codePage++;
      renderCodes();
    }
  });

  setupSorting();

  // Modals
  document
    .getElementById("adjust-modal-close")
    ?.addEventListener("click", () => toggleModal("adjust-modal", false));
  document
    .getElementById("adjust-cancel")
    ?.addEventListener("click", () => toggleModal("adjust-modal", false));
  document
    .getElementById("adjust-confirm")
    ?.addEventListener("click", applyAdjustment);
  document
    .getElementById("tier-override-close")
    ?.addEventListener("click", () =>
      toggleModal("tier-override-modal", false),
    );
  document
    .getElementById("tier-override-cancel")
    ?.addEventListener("click", () =>
      toggleModal("tier-override-modal", false),
    );
  document
    .getElementById("tier-override-confirm")
    ?.addEventListener("click", applyTierOverride);
});

function setupSorting() {
  ["user-tiers", "balances", "referrals"].forEach((tab) => {
    const panel = document.getElementById("panel-" + tab);
    if (!panel) return;
    panel.querySelectorAll("th[data-sort]").forEach((th) => {
      th.style.cursor = "pointer";
      th.tabIndex = 0;
      th.addEventListener("keydown", (e) => {
          if (e.key === "Enter" || e.key === " ") { e.preventDefault(); th.click(); }
      });
      th.addEventListener("click", () => {
        const field = th.dataset.sort;
        if (tab === "user-tiers") {
          if (utSortField === field)
            utSortOrder = utSortOrder === "asc" ? "desc" : "asc";
          else {
            utSortField = field;
            utSortOrder = "asc";
          }
          renderUserTiers();
        } else if (tab === "balances") {
          if (balSortField === field)
            balSortOrder = balSortOrder === "asc" ? "desc" : "asc";
          else {
            balSortField = field;
            balSortOrder = "asc";
          }
          renderBalances();
        } else if (tab === "referrals") {
          if (field.startsWith("code_")) {
            if (codeSortField === field)
              codeSortOrder = codeSortOrder === "asc" ? "desc" : "asc";
            else {
              codeSortField = field;
              codeSortOrder = "asc";
            }
            renderCodes();
          } else {
            if (refSortField === field)
              refSortOrder = refSortOrder === "asc" ? "desc" : "asc";
            else {
              refSortField = field;
              refSortOrder = "asc";
            }
            renderReferrals();
          }
        }
      });
    });
  });
}

// ═══════════════ Tab Switching ═══════════════

window.switchTab = function (tab, updateUrl = true) {
  ["tiers", "user-tiers", "balances", "referrals"].forEach((t) => {
    const panel = document.getElementById("panel-" + t);
    const btn = document.getElementById("tab-" + t);
    if (panel) panel.style.display = t === tab ? "" : "none";
    if (btn) btn.classList.toggle("active", t === tab);
  });
  if (updateUrl) {
    const url = new URL(window.location);
    url.searchParams.set('tab', tab);
    window.history.pushState({}, '', url);
  }
};

// ═══════════════ Data Loading ═══════════════

async function loadAll() {
  try {
    const r = await fetch("/api/admin/rewards");
    if (r.ok) {
      const d = await r.json();
      tiers = d.tiers || [];
      userTiers = d.user_tiers || [];
      balances = d.balances || [];
      referrals = d.referrals || [];
      referralCodes = d.referral_codes || [];
    } else {
      throw new Error(`Failed to load data: HTTP ${r.status}`);
    }
  } catch (e) {
    console.error("Error loading rewards data", e);
    if (typeof Sentry !== 'undefined') Sentry.captureException(e);
    if (window.showPooolToast) window.showPooolToast("Error", "Failed to load rewards data.", "error");
    const errorMsg = '<tr><td colspan="7" style="text-align:center;padding:40px;color:var(--admin-danger);">Failed to load data. <button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="loadAll()" style="margin-left:8px;">Retry</button></td></tr>';
    const ids = ["tiers-table-body", "user-tiers-body", "balances-body", "referrals-body", "codes-body"];
    ids.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.innerHTML = errorMsg;
    });
    return;
  }
  updateKPIs();
  renderTiers();
  renderTierDistribution();
  renderUserTiers();
  renderBalances();
  renderReferrals();
  renderCodes();
}

function updateKPIs() {
  const totalBal = balances.reduce(
    (s, b) => s + (b.cashback + b.referrals_amt + b.promotions),
    0,
  );
  const pendingRefs = referrals.filter((r) => r.status === "pending").length;
  const paidOut = balances.reduce(
    (s, b) => s + b.cashback + b.referrals_amt,
    0,
  );
  const txnCount = referrals.filter((r) => r.status === "paid").length;

  el("kpi-total-balance").textContent = fmt(totalBal);
  el("kpi-wallet-count").textContent = balances.length + " wallets";
  el("kpi-referrals").textContent = pendingRefs;
  el("kpi-referral-sub").textContent =
    "pending " + (pendingRefs === 1 ? "qualification" : "qualifications");
  el("kpi-paid-out").textContent = fmt(paidOut);
  el("kpi-upgrades").textContent = Math.floor(txnCount * 0.1);
}

// ═══════════════ Tab 1: Tier Configuration ═══════════════

function renderTiers() {
  const tbody = el("tiers-table-body");
  tbody.innerHTML = tiers
    .map(
      (t) => `
        <tr style="cursor:pointer;" class="tier-clickable-row" data-name="${escapeHtml(t.name)}">
            <td><span style="display:inline-flex;align-items:center;gap:6px;">
                <span style="width:10px;height:10px;border-radius:50%;background:${escapeHtml(t.badge_color) || "#6366f1"};"></span>
                <strong>${esc(t.name)}</strong>
            </span></td>
            <td>${t.min_invest > 0 ? formatUSD(t.min_invest) : "Free"}</td>
            <td>${(t.cashback_pct || 0).toFixed(1)}%</td>
            <td>${formatUSD(t.referral_bonus || 0)}</td>
            <td><button class="admin-btn admin-btn--secondary admin-btn--sm">
                <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M11 2l3 3-9 9H2v-3l9-9z"/></svg>
            </button></td>
        </tr>
    `,
    )
    .join("");

  tbody.onclick = (e) => {
    const row = e.target.closest('.tier-clickable-row');
    if (row && row.dataset.name) {
        editTier(row.dataset.name);
    }
  };
}

window.editTier = function (name) {
  editingTier = tiers.find((t) => t.name === name);
  if (!editingTier) return;
  el("tier-edit-form").innerHTML = `
        <div style="display:flex;align-items:center;gap:8px;margin-bottom:20px;">
            <span style="width:16px;height:16px;border-radius:50%;background:${editingTier.badge_color || "#6366f1"};"></span>
            <span style="font-size:18px;font-weight:700;color:var(--admin-text-primary);">${esc(editingTier.name)}</span>
            <input type="hidden" id="edit-name" value="${esc(editingTier.name)}">
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px;">
            <div>
                <label class="edit-label">Min Investment (cents)</label>
                <input type="number" id="edit-min-invest" class="admin-input" style="width:100%;" value="${editingTier.min_invest}">
            </div>
            <div>
                <label class="edit-label">Max Investment (cents)</label>
                <input type="number" id="edit-max-invest" class="admin-input" style="width:100%;" value="${editingTier.max_invest || 0}">
            </div>
            <div>
                <label class="edit-label">Cashback (%)</label>
                <input type="number" step="0.1" id="edit-cashback" class="admin-input" style="width:100%;" value="${editingTier.cashback_pct || 0}">
            </div>
            <div>
                <label class="edit-label">Referral Bonus (cents)</label>
                <input type="number" id="edit-referral" class="admin-input" style="width:100%;" value="${editingTier.referral_bonus || 0}">
            </div>
            <div>
                <label class="edit-label">Badge Color</label>
                <input type="color" id="edit-color" class="admin-input" style="width:100%;height:38px;padding:4px;" value="${editingTier.badge_color || "#6366f1"}">
            </div>
            <div>
                <label class="edit-label">Sort Order</label>
                <input type="number" id="edit-sort" class="admin-input" style="width:100%;" value="${editingTier.sort_order || 0}">
            </div>
        </div>
        <div style="margin-top:20px;padding-top:16px;border-top:1px solid var(--admin-border);display:flex;gap:8px;justify-content:flex-end;">
            <button class="admin-btn admin-btn--secondary" onclick="cancelEdit()">Cancel</button>
            <button class="admin-btn admin-btn--primary" onclick="saveTier(false)">Save Changes</button>
        </div>
    `;
};

window.createTier = function () {
  editingTier = null;
  el("tier-edit-form").innerHTML = `
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px;">
            <div style="grid-column: span 2;">
                <label class="edit-label">Tier Name</label>
                <input type="text" id="edit-name" class="admin-input" style="width:100%;" placeholder="e.g. Platinum">
            </div>
            <div>
                <label class="edit-label">Min Investment (cents)</label>
                <input type="number" id="edit-min-invest" class="admin-input" style="width:100%;" value="0">
            </div>
            <div>
                <label class="edit-label">Max Investment (cents)</label>
                <input type="number" id="edit-max-invest" class="admin-input" style="width:100%;" value="0">
            </div>
            <div>
                <label class="edit-label">Cashback (%)</label>
                <input type="number" step="0.1" id="edit-cashback" class="admin-input" style="width:100%;" value="0">
            </div>
            <div>
                <label class="edit-label">Referral Bonus (cents)</label>
                <input type="number" id="edit-referral" class="admin-input" style="width:100%;" value="0">
            </div>
            <div>
                <label class="edit-label">Badge Color</label>
                <input type="color" id="edit-color" class="admin-input" style="width:100%;height:38px;padding:4px;" value="#6366f1">
            </div>
            <div>
                <label class="edit-label">Sort Order</label>
                <input type="number" id="edit-sort" class="admin-input" style="width:100%;" value="0">
            </div>
        </div>
        <div style="margin-top:20px;padding-top:16px;border-top:1px solid var(--admin-border);display:flex;gap:8px;justify-content:flex-end;">
            <button class="admin-btn admin-btn--secondary" onclick="cancelEdit()">Cancel</button>
            <button class="admin-btn admin-btn--primary" onclick="saveTier(true)">Create Tier</button>
        </div>
    `;
};

window.cancelEdit = function () {
  editingTier = null;
  el("tier-edit-form").innerHTML =
    `<div style="text-align:center;padding:40px 20px;color:var(--admin-text-muted);">
        <svg width="40" height="40" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.3" style="margin-bottom:8px;opacity:0.4;"><path d="M10 2l2.4 4.8L18 7.6l-4 3.9.9 5.5L10 14.6 5.1 17l.9-5.5-4-3.9 5.6-.8L10 2z"/></svg>
        <p style="margin:0;font-size:13px;">Select a tier to edit its configuration</p>
    </div>`;
};

window.saveTier = async function (isCreate) {
  const tierName = document.getElementById("edit-name").value;
  if (!tierName) return alert("Tier name is required.");

  const payload = {
    name: tierName,
    min_invest: Number(document.getElementById("edit-min-invest").value),
    max_invest:
      Number(document.getElementById("edit-max-invest").value) || null,
    cashback_pct: parseFloat(document.getElementById("edit-cashback").value),
    referral_bonus: Number(document.getElementById("edit-referral").value),
    badge_color: document.getElementById("edit-color").value,
    sort_order: Number(document.getElementById("edit-sort").value),
  };

  try {
    const url = isCreate
      ? `/api/admin/rewards/tiers`
      : `/api/admin/rewards/tiers/${encodeURIComponent(tierName)}`;
    const method = isCreate ? "POST" : "PATCH";

    const r = await fetch(url, {
      method,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (r.ok) {
      if (window.showPooolToast) window.showPooolToast("Success", `Tier "${tierName}" saved successfully.`, "success");
      loadAll();
      cancelEdit();
    } else {
      const err = await r.json();
      if (window.showPooolToast) window.showPooolToast("Failed", "Failed to save tier: " + (err.error || "Unknown error"), "error");
    }
  } catch (e) {
    if (window.showPooolToast) window.showPooolToast("Error", "Network error: " + e.message, "error");
  }
};

function renderTierDistribution() {
  const container = el("tier-distribution");
  const total = userTiers.length || 1;
  container.innerHTML = tiers
    .map((t) => {
      const count = userTiers.filter((u) => u.tier === t.name).length;
      const pct = Math.round((count / total) * 100);
      return `<div style="flex:1;min-width:120px;background:var(--admin-bg);border:1px solid var(--admin-border);border-radius:var(--admin-radius-md);padding:14px;text-align:center;">
            <div style="width:36px;height:36px;border-radius:50%;background:${t.badge_color || "#6366f1"}22;display:flex;align-items:center;justify-content:center;margin:0 auto 8px;">
                <span style="width:12px;height:12px;border-radius:50%;background:${t.badge_color || "#6366f1"};"></span>
            </div>
            <div style="font-size:13px;font-weight:700;color:var(--admin-text-primary);">${esc(t.name)}</div>
            <div style="font-size:20px;font-weight:800;color:var(--admin-text-primary);margin:4px 0;">${count}</div>
            <div style="font-size:11px;color:var(--admin-text-muted);">${pct}% of users</div>
        </div>`;
    })
    .join("");
}

// ═══════════════ Tab 2: User Tiers ═══════════════

function renderUserTiers() {
  const search = (el("ut-search")?.value || "").toLowerCase();
  const tierFilter = el("ut-filter-tier")?.value || "";
  let list = [...userTiers];
  if (tierFilter) list = list.filter((u) => u.tier === tierFilter);
  if (search)
    list = list.filter((u) =>
      `${u.name} ${u.email}`.toLowerCase().includes(search),
    );

  // Sort
  list.sort((a, b) => {
    let valA = a[utSortField],
      valB = b[utSortField];
    if (utSortField === "name") {
      valA = (a.name || "").toLowerCase();
      valB = (b.name || "").toLowerCase();
    }
    if (valA < valB) return utSortOrder === "asc" ? -1 : 1;
    if (valA > valB) return utSortOrder === "asc" ? 1 : -1;
    return 0;
  });

  utFiltered = list;
  const totalPages = Math.max(1, Math.ceil(utFiltered.length / PAGE_SIZE));
  utPage = Math.min(utPage, totalPages);
  const start = (utPage - 1) * PAGE_SIZE;
  const slice = utFiltered.slice(start, start + PAGE_SIZE);

  el("ut-count").textContent = utFiltered.length + " users";

  // Update Pagination UI
  const info = el("ut-pagination-info");
  if (info)
    info.textContent = `Page ${utPage} of ${totalPages} (${utFiltered.length} total)`;
  el("ut-prev-page").disabled = utPage <= 1;
  el("ut-next-page").disabled = utPage >= totalPages;

  const tbody = el("user-tiers-body");
  if (!slice.length) {
    tbody.innerHTML =
      '<tr><td colspan="5" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No users found.</td></tr>';
    return;
  }

  tbody.innerHTML = slice
    .map((u) => {
      const tier = tiers.find((t) => t.name === u.tier);
      const nextTier = tiers.find(
        (t) => t.sort_order === (tier?.sort_order || 0) + 1,
      );
      const progress = nextTier
        ? Math.min(
          100,
          Math.round((u.invested_12m / nextTier.min_invest) * 100),
        )
        : 100;
      return `<tr>
            <td><div class="admin-user-inline"><div><div class="admin-user-inline-name">${esc(u.name)}</div><div class="admin-user-inline-email">${esc(u.email)}</div></div></div></td>
            <td><span style="display:inline-flex;align-items:center;gap:5px;">
                <span style="width:8px;height:8px;border-radius:50%;background:${escapeHtml(tier?.badge_color) || "#888"};"></span>
                ${esc(u.tier)}</span></td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(u.invested_12m)}</td>
            <td><div style="display:flex;align-items:center;gap:8px;">
                <div style="flex:1;max-width:100px;height:6px;background:var(--admin-border);border-radius:3px;overflow:hidden;">
                    <div style="width:${progress}%;height:100%;background:${escapeHtml(tier?.badge_color) || "var(--admin-accent)"};border-radius:3px;"></div>
                </div>
                <span style="font-size:11px;color:var(--admin-text-muted);">${nextTier ? progress + "% → " + escapeHtml(nextTier.name) : "✓ Max"}</span>
            </div></td>
            <td><button class="admin-btn admin-btn--secondary admin-btn--sm override-btn" data-uid="${escapeHtml(u.user_id)}" data-name="${escapeHtml(u.name)}" data-email="${escapeHtml(u.email)}" data-tier="${escapeHtml(u.tier)}">Override</button></td>
        </tr>`;
    })
    .join("");

  tbody.querySelectorAll('.override-btn').forEach(btn => {
      btn.onclick = () => openTierOverride(btn.dataset.uid, btn.dataset.name, btn.dataset.email, btn.dataset.tier);
  });
}

// ═══════════════ Tab 3: Reward Balances ═══════════════

function renderBalances() {
  const search = (el("bal-search")?.value || "").toLowerCase();
  let list = [...balances];
  if (search)
    list = list.filter((b) =>
      `${b.name} ${b.email}`.toLowerCase().includes(search),
    );

  // Sort
  list.sort((a, b) => {
    let valA = a[balSortField],
      valB = b[balSortField];
    if (balSortField === "total") {
      valA = (a.cashback || 0) + (a.referrals_amt || 0) + (a.promotions || 0);
      valB = (b.cashback || 0) + (b.referrals_amt || 0) + (b.promotions || 0);
    }
    if (balSortField === "name") {
      valA = (a.name || "").toLowerCase();
      valB = (b.name || "").toLowerCase();
    }
    if (valA < valB) return balSortOrder === "asc" ? -1 : 1;
    if (valA > valB) return balSortOrder === "asc" ? 1 : -1;
    return 0;
  });

  balFiltered = list;
  const totalPages = Math.max(1, Math.ceil(balFiltered.length / PAGE_SIZE));
  balPage = Math.min(balPage, totalPages);
  const start = (balPage - 1) * PAGE_SIZE;
  const slice = balFiltered.slice(start, start + PAGE_SIZE);

  el("bal-count").textContent = balFiltered.length + " users";
  el("agg-cashback").textContent = fmt(
    balFiltered.reduce((s, b) => s + b.cashback, 0),
  );
  el("agg-referrals").textContent = fmt(
    balFiltered.reduce((s, b) => s + b.referrals_amt, 0),
  );
  el("agg-promotions").textContent = fmt(
    balFiltered.reduce((s, b) => s + b.promotions, 0),
  );

  // Update Pagination UI
  const info = el("bal-pagination-info");
  if (info)
    info.textContent = `Page ${balPage} of ${totalPages} (${balFiltered.length} total)`;
  el("bal-prev-page").disabled = balPage <= 1;
  el("bal-next-page").disabled = balPage >= totalPages;

  const tbody = el("balances-body");
  if (!slice.length) {
    tbody.innerHTML =
      '<tr><td colspan="6" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No users found.</td></tr>';
    return;
  }

  tbody.innerHTML = slice
    .map(
      (b) => `<tr>
        <td><div class="admin-user-inline"><div><div class="admin-user-inline-name">${esc(b.name)}</div><div class="admin-user-inline-email">${esc(b.email)}</div></div></div></td>
        <td style="font-variant-numeric:tabular-nums;">${fmt(b.cashback)}</td>
        <td style="font-variant-numeric:tabular-nums;">${fmt(b.referrals_amt)}</td>
        <td style="font-variant-numeric:tabular-nums;">${fmt(b.promotions)}</td>
        <td style="font-variant-numeric:tabular-nums;font-weight:700;">${fmt((b.cashback || 0) + (b.referrals_amt || 0) + (b.promotions || 0))}</td>
        <td><button class="admin-btn admin-btn--secondary admin-btn--sm adjust-btn" data-uid="${escapeHtml(b.user_id)}" data-name="${escapeHtml(b.name)}" data-email="${escapeHtml(b.email)}">Adjust</button></td>
    </tr>`,
    )
    .join("");

  tbody.querySelectorAll('.adjust-btn').forEach(btn => {
      btn.onclick = () => openAdjustModal(btn.dataset.uid, btn.dataset.name, btn.dataset.email);
  });
}

// ═══════════════ Tab 4: Referral Programme ═══════════════

function renderReferrals() {
  const search = (el("ref-search")?.value || "").toLowerCase();
  const statusFilter = el("ref-filter-status")?.value || "";
  let list = [...referrals];
  if (statusFilter) list = list.filter((r) => r.status === statusFilter);
  if (search)
    list = list.filter((r) =>
      `${r.referrer_name} ${r.referrer_email} ${r.referred_name} ${r.referred_email}`
        .toLowerCase()
        .includes(search),
    );

  // Sort
  list.sort((a, b) => {
    let valA = a[refSortField],
      valB = b[refSortField];
    if (refSortField === "referrer_name" || refSortField === "referred_name") {
      valA = (a[refSortField] || "").toLowerCase();
      valB = (b[refSortField] || "").toLowerCase();
    }
    if (valA < valB) return refSortOrder === "asc" ? -1 : 1;
    if (valA > valB) return refSortOrder === "asc" ? 1 : -1;
    return 0;
  });

  refFiltered = list;
  const totalPages = Math.max(1, Math.ceil(refFiltered.length / PAGE_SIZE));
  refPage = Math.min(refPage, totalPages);
  const start = (refPage - 1) * PAGE_SIZE;
  const slice = refFiltered.slice(start, start + PAGE_SIZE);

  el("ref-count").textContent = refFiltered.length + " referrals";
  el("ref-total").textContent = referrals.length;
  el("ref-qualified").textContent = referrals.filter(
    (r) => r.status === "qualified" || r.status === "paid",
  ).length;
  el("ref-pending").textContent = referrals.filter(
    (r) => r.status === "pending",
  ).length;
  const total = referrals.length || 1;
  const qualified = referrals.filter((r) => r.status !== "pending").length;
  el("ref-conversion").textContent =
    Math.round((qualified / total) * 100) + "%";

  // Update Pagination UI
  const info = el("ref-pagination-info");
  if (info)
    info.textContent = `Page ${refPage} of ${totalPages} (${refFiltered.length} total)`;
  el("ref-prev-page").disabled = refPage <= 1;
  el("ref-next-page").disabled = refPage >= totalPages;

  const tbody = el("referrals-body");
  if (!slice.length) {
    tbody.innerHTML =
      '<tr><td colspan="7" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No referrals found.</td></tr>';
    return;
  }

  tbody.innerHTML = slice
    .map(
      (r) => `<tr>
        <td><div class="admin-user-inline"><div><div class="admin-user-inline-name">${esc(r.referrer_name)}</div><div class="admin-user-inline-email">${esc(r.referrer_email)}</div></div></div></td>
        <td><div class="admin-user-inline"><div><div class="admin-user-inline-name">${esc(r.referred_name)}</div><div class="admin-user-inline-email">${esc(r.referred_email)}</div></div></div></td>
        <td>${refBadge(r.status)}</td>
        <td style="font-variant-numeric:tabular-nums;">${fmt(r.referrer_reward)}</td>
        <td style="font-variant-numeric:tabular-nums;">${fmt(r.referred_reward)}</td>
        <td style="font-size:12px;color:var(--admin-text-muted);">${fmtDate(r.created_at)}</td>
        <td><div style="display:flex;gap:4px;">
            ${r.status === "pending" ? `<button class="admin-btn admin-btn--primary admin-btn--sm action-btn" data-action="qualify" data-id="${escapeHtml(r.id)}">Qualify</button>` : ""}
            ${r.status === "pending" ? `<button class="admin-btn admin-btn--secondary admin-btn--sm action-btn" data-action="flag" data-id="${escapeHtml(r.id)}" title="Flag as fraud">⚑</button>` : ""}
            ${r.status === "qualified" ? `<button class="admin-btn admin-btn--primary admin-btn--sm action-btn" data-action="pay" data-id="${escapeHtml(r.id)}">Mark Paid</button>` : ""}
        </div></td>
    </tr>`,
    )
    .join("");

  tbody.querySelectorAll('.action-btn').forEach(btn => {
      btn.onclick = (e) => {
          const action = btn.dataset.action;
          const id = btn.dataset.id;
          if (action === 'qualify') qualifyRef(id, e.target);
          if (action === 'flag') flagRef(id, e.target);
          if (action === 'pay') payRef(id, e.target);
      };
  });
}

function renderCodes() {
  const search = (el("code-search")?.value || "").toLowerCase();
  let list = [...referralCodes];
  if (search)
    list = list.filter((c) =>
      `${c.code} ${c.user_name} ${c.user_email}`.toLowerCase().includes(search),
    );

  list.sort((a, b) => {
    let valA = a[codeSortField] || a[codeSortField.replace("code_", "")],
      valB = b[codeSortField] || b[codeSortField.replace("code_", "")];
    if (codeSortField === "code_user_name") {
      valA = (a.user_name || "").toLowerCase();
      valB = (b.user_name || "").toLowerCase();
    }
    if (valA < valB) return codeSortOrder === "asc" ? -1 : 1;
    if (valA > valB) return codeSortOrder === "asc" ? 1 : -1;
    return 0;
  });

  codeFiltered = list;
  const totalPages = Math.max(1, Math.ceil(codeFiltered.length / PAGE_SIZE));
  codePage = Math.min(codePage, totalPages);
  const start = (codePage - 1) * PAGE_SIZE;
  const slice = codeFiltered.slice(start, start + PAGE_SIZE);

  el("code-count").textContent = codeFiltered.length + " codes";

  const info = el("code-pagination-info");
  if (info)
    info.textContent = `Page ${codePage} of ${totalPages} (${codeFiltered.length} total)`;
  el("code-prev-page").disabled = codePage <= 1;
  el("code-next-page").disabled = codePage >= totalPages;

  const tbody = el("codes-body");
  if (!slice.length) {
    tbody.innerHTML =
      '<tr><td colspan="4" style="text-align:center;padding:40px;color:var(--admin-text-muted);">No codes found.</td></tr>';
    return;
  }

  tbody.innerHTML = slice
    .map(
      (c) => `<tr>
        <td><div class="admin-user-inline"><div><div class="admin-user-inline-name">${esc(c.user_name)}</div><div class="admin-user-inline-email">${esc(c.user_email)}</div></div></div></td>
        <td><strong style="color:var(--admin-accent);">${esc(c.code)}</strong></td>
        <td><div style="font-size:12px;color:var(--admin-text-muted);">https://poool.finance/r/${esc(c.code)}</div></td>
        <td style="font-size:12px;color:var(--admin-text-muted);">${fmtDate(c.created_at)}</td>
    </tr>`,
    )
    .join("");
}

async function updateRefStatus(id, status, btnElement) {
  let origText = "";
  if (btnElement) {
    btnElement.disabled = true;
    origText = btnElement.textContent;
    btnElement.textContent = 'Processing...';
  }
  try {
    const r = await fetch(`/api/admin/rewards/referrals/${id}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ status }),
    });
    if (r.ok) {
      if (window.showPooolToast) window.showPooolToast("Success", "Referral status updated.", "success");
      loadAll();
    } else {
      const err = await r.json();
      if (window.showPooolToast) window.showPooolToast("Failed", err.error || "Unknown error", "error");
      if (btnElement) { btnElement.disabled = false; btnElement.textContent = origText; }
    }
  } catch (e) {
    if (window.showPooolToast) window.showPooolToast("Error", "Err: " + e.message, "error");
    if (btnElement) { btnElement.disabled = false; btnElement.textContent = origText; }
  }
}

window.qualifyRef = function (id, btn) {
  updateRefStatus(id, "qualified", btn);
};
window.flagRef = async function (id, btn) {
  if (await pooolConfirm({ title: 'Flag as fraud', message: 'Flag this referral as fraud?', confirmText: 'Flag', type: 'danger' })) updateRefStatus(id, 'flagged', btn);
};
window.payRef = async function (id, btn) {
  if (await pooolConfirm({ title: 'Mark referral as paid', message: 'Rewards will be credited to both users.', confirmText: 'Mark Paid', type: 'success' }))
    updateRefStatus(id, 'paid', btn);
};

// ═══════════════ Modals ═══════════════

function toggleModal(id, show) {
  const m = document.getElementById(id);
  if (m) m.style.display = show ? "flex" : "none";
}

let adjustTarget = {};
window.openAdjustModal = function (user_id, name, email) {
  adjustTarget = { user_id, name, email };
  el("adjust-user-name").textContent = name + " (" + email + ")";
  document.getElementById("adjust-amount").value = "";
  document.getElementById("adjust-reason").value = "";
  toggleModal("adjust-modal", true);
};

async function applyAdjustment() {
  const amountVal = parseFloat(
    document.getElementById("adjust-amount")?.value || "0",
  );
  const direction = document.getElementById("adjust-direction").value;
  const category = document.getElementById("adjust-category").value;
  const reason = document.getElementById("adjust-reason")?.value?.trim();

  if (!amountVal || !reason) {
    if (window.showPooolToast) window.showPooolToast("Required", "Amount and reason are required.", "warning");
    return;
  }

  const amount_cents =
    Math.round(amountVal * 100) * (direction === "debit" ? -1 : 1);

  const payload = {
    cashback: category === "cashback" ? amount_cents : 0,
    referrals: category === "referrals" ? amount_cents : 0,
    promotions: category === "promotions" ? amount_cents : 0,
    reason,
  };

  const btnElement = document.getElementById("adjust-confirm");
  let origText = btnElement ? btnElement.textContent : "";
  if (btnElement) {
      btnElement.disabled = true;
      btnElement.textContent = "Processing...";
  }

  try {
    const r = await fetch(
      `/api/admin/rewards/balances/${adjustTarget.user_id}/adjust`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      },
    );
    if (r.ok) {
      if (window.showPooolToast) window.showPooolToast("Success", `Adjustment successful for ${adjustTarget.name}.`, "success");
      toggleModal("adjust-modal", false);
      loadAll();
    } else {
      const err = await r.json();
      if (window.showPooolToast) window.showPooolToast("Failed", err.error || "Unknown error", "error");
    }
  } catch (e) {
    if (window.showPooolToast) window.showPooolToast("Error", "Err: " + e.message, "error");
  } finally {
      if (btnElement) {
          btnElement.disabled = false;
          btnElement.textContent = origText;
      }
  }
}

let overrideTarget = {};
window.openTierOverride = function (user_id, name, email, currentTier) {
  overrideTarget = { user_id, name, email, currentTier };
  el("tier-override-user").textContent = name + " — currently " + currentTier;
  document.getElementById("tier-override-select").value = currentTier;
  toggleModal("tier-override-modal", true);
};

async function applyTierOverride() {
  const newTier = document.getElementById("tier-override-select").value;
  const btnElement = document.getElementById("tier-override-confirm");
  let origText = btnElement ? btnElement.textContent : "";
  if (btnElement) {
      btnElement.disabled = true;
      btnElement.textContent = "Processing...";
  }

  try {
    const r = await fetch(
      `/api/admin/users/${overrideTarget.user_id}/profile`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tier: newTier }),
      },
    );
    if (r.ok) {
      if (window.showPooolToast) window.showPooolToast("Success", `Tier overridden for ${overrideTarget.name}.`, "success");
      toggleModal("tier-override-modal", false);
      loadAll();
    } else {
      if (window.showPooolToast) window.showPooolToast("Failed", "Failed to override tier. Role permissions might be restricted.", "error");
    }
  } catch (e) {
    if (window.showPooolToast) window.showPooolToast("Error", "Err: " + e.message, "error");
  } finally {
      if (btnElement) {
          btnElement.disabled = false;
          btnElement.textContent = origText;
      }
  }
}

window.exportBalances = function() {
    exportToCSV(balances, 'rewards-balances.csv', ['user_id', 'name', 'email', 'cashback', 'referrals_amt', 'promotions']);
};
window.exportReferrals = function() {
    exportToCSV(referrals, 'referrals.csv', ['id', 'status', 'referrer_name', 'referrer_email', 'referred_name', 'referred_email', 'referrer_reward', 'referred_reward', 'created_at']);
};

function exportToCSV(data, filename, keys) {
    if (!data || !data.length) {
        if (window.showPooolToast) window.showPooolToast("Error", "No data to export", "error");
        return;
    }
    const csvRows = [];
    csvRows.push(keys.join(','));
    for (const row of data) {
        const values = keys.map(k => {
            const val = row[k] === null || row[k] === undefined ? '' : String(row[k]);
            return `"${val.replace(/"/g, '""')}"`;
        });
        csvRows.push(values.join(','));
    }
    const blob = new Blob([csvRows.join('\n')], { type: 'text/csv' });
    const url = window.URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.setAttribute('hidden', '');
    a.setAttribute('href', url);
    a.setAttribute('download', filename);
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
}

// ═══════════════ Helpers ═══════════════

function el(id) {
  return document.getElementById(id);
}
function esc(s) {
  if (typeof s !== "string") return "";
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}
function formatUSD(cents) {
  return (
    "$" +
    (Math.abs(cents || 0) / 100).toLocaleString(undefined, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
}
function fmt(cents) {
  return formatUSD(cents);
}
function fmtDate(iso) {
  if (!iso) return "—";
  return new Date(iso).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}
function debounce(fn, ms) {
  let t;
  return function (...a) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, a), ms);
  };
}

function refBadge(status) {
  const m = {
    pending: ["admin-badge--warning", "Pending"],
    qualified: ["admin-badge--info", "Qualified"],
    paid: ["admin-badge--success", "Paid"],
    flagged: ["admin-badge--danger", "Flagged"],
  };
  const [c, l] = m[status] || ["admin-badge--neutral", status];
  return `<span class="admin-badge ${c}">${l}</span>`;
}
