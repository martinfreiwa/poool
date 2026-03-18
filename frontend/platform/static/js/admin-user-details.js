/**
 * Admin User Details JS — Loads comprehensive user data from the API
 * and populates all tabs (Overview, Wallets, KYC, Investments, Orders, Sessions, Audit).
 */

let userData = null;
let allTransactions = [];
const userId = new URLSearchParams(window.location.search).get("id");

document.addEventListener("DOMContentLoaded", () => {
  if (!userId) {
    document.getElementById("user-loading").innerHTML =
      '<p style="color:var(--admin-danger);font-weight:600;">No user ID provided. <a href="/admin/users" class="admin-link">Back to Users</a></p>';
    return;
  }

  setupTabs();
  setupFilters();
  setupModals();
  loadUserDetails();
});

// ─── Tab Switching ──────────────────────────────────────────────

function setupTabs() {
  document.querySelectorAll(".admin-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      document
        .querySelectorAll(".admin-tab")
        .forEach((t) => t.classList.remove("active"));
      document
        .querySelectorAll(".admin-tab-panel")
        .forEach((p) => (p.style.display = "none"));
      tab.classList.add("active");
      const panel = document.getElementById("tab-" + tab.dataset.tab);
      if (panel) panel.style.display = "";
    });
  });
}

function setupFilters() {
  ["filter-tx-type", "filter-tx-status"].forEach((id) => {
    const el = document.getElementById(id);
    if (el) el.addEventListener("change", renderTransactions);
  });
}

// ─── Modals & Actions ─────────────────────────────────────────────

function setupModals() {
  // Shared functionality for closing modals
  document.querySelectorAll('[id$="-modal"]').forEach((modal) => {
    modal.addEventListener("click", (e) => {
      if (e.target === modal) modal.style.display = "none";
    });
  });

  // Suspend / Activate Action
  const suspendTrigger = document.getElementById("btn-suspend-trigger");
  const suspendModal = document.getElementById("suspend-modal");
  if (suspendTrigger && suspendModal) {
    suspendTrigger.addEventListener("click", () => {
      if (!userData) return;
      const isSuspended = userData.status === "suspended";

      // Update modal text based on current status
      const title = document.getElementById("suspend-modal-title");
      const text = document.getElementById("suspend-modal-text");
      const confirmBtn = document.getElementById("btn-confirm-suspend");

      if (isSuspended) {
        title.textContent = "Activate User?";
        text.textContent =
          "The user will regain access to all platform features immediately.";
        confirmBtn.textContent = "Confirm Activation";
        confirmBtn.className = "admin-btn admin-btn--primary";
      } else {
        title.textContent = "Suspend User?";
        text.textContent =
          "This user will be immediately logged out and blocked from all platform features.";
        confirmBtn.textContent = "Confirm Suspension";
        confirmBtn.className = "admin-btn admin-btn--danger";
      }

      suspendModal.style.display = "flex";
    });

    document
      .getElementById("suspend-modal-cancel")
      .addEventListener("click", () => {
        suspendModal.style.display = "none";
      });

    document
      .getElementById("btn-confirm-suspend")
      .addEventListener("click", async () => {
        const isSuspended = userData.status === "suspended";
        const newStatus = isSuspended ? "active" : "suspended";
        const btn = document.getElementById("btn-confirm-suspend");

        btn.disabled = true;
        btn.textContent = "Processing...";

        try {
          const res = await fetch(`/api/admin/users/${userId}/status`, {
            method: "POST",
            credentials: "same-origin",
            headers: { 
              "Content-Type": "application/json", 
              "X-CSRF-Token": getCsrfToken() 
            },
            body: JSON.stringify({ status: newStatus }),
          });
          if (res.ok) {
            suspendModal.style.display = "none";
            loadUserDetails();
          } else {
            let errorMsg = "Failed to update status";
            try {
              const err = await res.json();
              errorMsg = err.error || errorMsg;
            } catch (jsonErr) {
              errorMsg = `${res.status} ${res.statusText}`;
            }
            console.error("Status update failed:", errorMsg);
            alert(errorMsg);
          }
        } catch (err) {
          console.error("Network error during status update:", err);
          alert(`Error: ${err.message}`);
        } finally {
          btn.disabled = false;
        }
      });
  }

  // Freeze / Unfreeze Action
  const freezeBtnEl = document.getElementById("btn-freeze-trigger");
  if (freezeBtnEl) {
    freezeBtnEl.addEventListener("click", async () => {
      if (!userData) return;
      const isFrozen = userData.status === "frozen";
      const action = isFrozen ? "Unfreeze" : "Freeze";
      if (!confirm(`Are you sure you want to ${action} this user?`)) return;

      try {
        const res = await fetch(`/api/admin/users/${userId}/status`, {
          method: "POST",
          credentials: "same-origin",
          headers: { 
            "Content-Type": "application/json", 
            "X-CSRF-Token": getCsrfToken() 
          },
          body: JSON.stringify({ status: isFrozen ? "active" : "frozen" }),
        });
        if (res.ok) {
          loadUserDetails();
        } else {
          let errorMsg = "Failed to update status";
          try {
            const err = await res.json();
            errorMsg = err.error || errorMsg;
          } catch (jsonErr) {
            errorMsg = `${res.status} ${res.statusText}`;
          }
          console.error("Freeze update failed:", errorMsg);
          alert(errorMsg);
        }
      } catch (err) {
        console.error("Network error during freeze update:", err);
        alert(`Error: ${err.message}`);
      }
    });
  }

  // Logout All Action
  const logoutAction = async () => {
    if (
      !confirm(
        "Are you sure you want to revoke all active sessions for this user? They will be logged out immediately.",
      )
    )
      return;
    try {
      const res = await fetch(`/api/admin/users/${userId}/sessions`, {
        method: "DELETE",
        credentials: "same-origin",
        headers: { "X-CSRF-Token": getCsrfToken() },
      });
      if (res.ok) {
        alert("All sessions wiped successfully.");
        loadUserDetails();
      } else {
        let errorMsg = "Failed to revoke sessions";
        try {
          const err = await res.json();
          errorMsg = err.error || errorMsg;
        } catch (jsonErr) {
          errorMsg = `${res.status} ${res.statusText}`;
        }
        console.error("Session logout failed:", errorMsg);
        alert(errorMsg);
      }
    } catch (err) {
      console.error("Network error during session revocation:", err);
      alert(`Error: ${err.message}`);
    }
  };

  const logoutBtn = document.getElementById("btn-logout-all");
  if (logoutBtn) logoutBtn.addEventListener("click", logoutAction);

  // There is also a btn-revoke-all-sessions in the sessions tab
  const revokeBtn = document.getElementById("btn-revoke-all-sessions");
  if (revokeBtn) revokeBtn.addEventListener("click", logoutAction);

  // Profile Edit Modal
  const profileModal = document.getElementById("edit-profile-modal");
  if (profileModal) {
    document
      .getElementById("edit-profile-close")
      .addEventListener("click", () => (profileModal.style.display = "none"));
    document
      .getElementById("edit-profile-cancel")
      .addEventListener("click", () => (profileModal.style.display = "none"));

    // Open Modal and populate data
    document.addEventListener("click", (e) => {
      const btn = e.target.closest("#btn-edit-profile");
      if (!btn) return;
      const p = userData.profile || {};
      document.getElementById("edit-first-name").value = p.first_name || "";
      document.getElementById("edit-last-name").value = p.last_name || "";
      document.getElementById("edit-dob").value = p.date_of_birth || "";
      document.getElementById("edit-nationality").value = p.nationality || "";
      document.getElementById("edit-phone").value = p.phone_number || "";
      document.getElementById("edit-tax-id").value = p.tax_id || "";
      document.getElementById("edit-address-1").value = p.address_line_1 || "";
      document.getElementById("edit-address-2").value = p.address_line_2 || "";
      document.getElementById("edit-city").value = p.city || "";
      document.getElementById("edit-state").value = p.state_province || "";
      document.getElementById("edit-postal").value = p.postal_code || "";
      document.getElementById("edit-country").value = p.country || "";
      profileModal.style.display = "flex";
    });

    // Submit Profile Save
    document
      .getElementById("edit-profile-form")
      .addEventListener("submit", async (e) => {
        e.preventDefault();
        const btn = document.getElementById("edit-profile-submit");
        btn.textContent = "Saving...";
        btn.disabled = true;

        const payload = {
          first_name: document.getElementById("edit-first-name").value || null,
          last_name: document.getElementById("edit-last-name").value || null,
          date_of_birth: document.getElementById("edit-dob").value || null,
          nationality:
            document.getElementById("edit-nationality").value || null,
          phone_number: document.getElementById("edit-phone").value || null,
          tax_id: document.getElementById("edit-tax-id").value || null,
          address_line_1:
            document.getElementById("edit-address-1").value || null,
          address_line_2:
            document.getElementById("edit-address-2").value || null,
          city: document.getElementById("edit-city").value || null,
          state_province: document.getElementById("edit-state").value || null,
          postal_code: document.getElementById("edit-postal").value || null,
          country: document.getElementById("edit-country").value || null,
        };

        try {
          const res = await fetch(`/api/admin/users/${userId}/profile`, {
            method: "POST",
            credentials: "same-origin",
            headers: { 
              "Content-Type": "application/json", 
              "X-CSRF-Token": getCsrfToken() 
            },
            body: JSON.stringify(payload),
          });
          if (res.ok) {
            profileModal.style.display = "none";
            loadUserDetails(); // reload to reflect changes
          } else {
            let errorMsg = "Failed to update profile";
            try {
              const err = await res.json();
              errorMsg = err.error || errorMsg;
            } catch (jsonErr) {
              errorMsg = `${res.status} ${res.statusText}`;
            }
            console.error("Profile update failed:", errorMsg);
            alert(errorMsg);
          }
        } catch (err) {
          console.error("Network error during profile update:", err);
          alert("Error updating profile");
        } finally {
          btn.textContent = "Save Changes";
          btn.disabled = false;
        }
      });
  }

  // Balance Edit Modal
  const balanceModal = document.getElementById("edit-balance-modal");
  if (balanceModal) {
    document
      .getElementById("edit-balance-close")
      .addEventListener("click", () => (balanceModal.style.display = "none"));
    document
      .getElementById("edit-balance-cancel")
      .addEventListener("click", () => (balanceModal.style.display = "none"));

    // Open Modal
    document.addEventListener("click", (e) => {
      const btn = e.target.closest("#btn-edit-balance");
      if (!btn) return;
      document.getElementById("edit-balance-form").reset();
      balanceModal.style.display = "flex";
    });

    // Submit Balance Adj
    document
      .getElementById("edit-balance-form")
      .addEventListener("submit", async (e) => {
        e.preventDefault();
        const btn = document.getElementById("edit-balance-submit");
        btn.textContent = "Saving...";
        btn.disabled = true;

        const amountDollar = parseFloat(
          document.getElementById("edit-balance-amount").value,
        );
        if (isNaN(amountDollar) || amountDollar === 0) {
          alert("Please enter a valid amount.");
          btn.textContent = "Adjust Balance";
          btn.disabled = false;
          return;
        }

        const payload = {
          wallet_type: document.getElementById("edit-balance-wallet").value,
          amount_cents: Math.round(amountDollar * 100),
          reason: document.getElementById("edit-balance-reason").value,
        };

        try {
          const res = await fetch(`/api/admin/users/${userId}/balance`, {
            method: "POST",
            credentials: "same-origin",
            headers: { 
              "Content-Type": "application/json", 
              "X-CSRF-Token": getCsrfToken() 
            },
            body: JSON.stringify(payload),
          });
          if (res.ok) {
            balanceModal.style.display = "none";
            loadUserDetails();
          } else {
            let errorMsg = "Failed to update balance";
            try {
              const err = await res.json();
              errorMsg = err.error || errorMsg;
            } catch (jsonErr) {
              errorMsg = `${res.status} ${res.statusText}`;
            }
            console.error("Balance update failed:", errorMsg);
            alert(errorMsg);
          }
        } catch (err) {
          console.error("Network error during balance update:", err);
          alert("Error updating balance");
        } finally {
          btn.textContent = "Adjust Balance";
          btn.disabled = false;
        }
      });
  }

  // Roles Edit Modal
  const rolesModal = document.getElementById("edit-roles-modal");
  if (rolesModal) {
    document
      .getElementById("edit-roles-close")
      .addEventListener("click", () => (rolesModal.style.display = "none"));
    document
      .getElementById("edit-roles-cancel")
      .addEventListener("click", () => (rolesModal.style.display = "none"));

    // Open Modal
    document.addEventListener("click", (e) => {
      const btn = e.target.closest("#btn-edit-roles");
      if (!btn) return;

      // Reset checkboxes
      const checkboxes = document.querySelectorAll(
        '#edit-roles-form input[name="role"]',
      );
      checkboxes.forEach((cb) => {
        cb.checked =
          userData && userData.roles && userData.roles.includes(cb.value);
      });

      rolesModal.style.display = "flex";
    });

    // Submit Roles Save
    document
      .getElementById("edit-roles-form")
      .addEventListener("submit", async (e) => {
        e.preventDefault();
        const btn = document.getElementById("edit-roles-submit");
        btn.textContent = "Saving...";
        btn.disabled = true;

        const selectedRoles = Array.from(
          document.querySelectorAll(
            '#edit-roles-form input[name="role"]:checked',
          ),
        ).map((cb) => cb.value);

        try {
          const res = await fetch(`/api/admin/users/${userId}/roles`, {
            method: "POST",
            credentials: "same-origin",
            headers: { 
              "Content-Type": "application/json", 
              "X-CSRF-Token": getCsrfToken() 
            },
            body: JSON.stringify({ roles: selectedRoles }),
          });
          if (res.ok) {
            rolesModal.style.display = "none";
            loadUserDetails(); // reload to reflect changes
          } else {
            let errorMsg = "Failed to update roles";
            try {
              const err = await res.json();
              errorMsg = err.error || errorMsg;
            } catch (jsonErr) {
              errorMsg = `${res.status} ${res.statusText}`;
            }
            console.error("Roles update failed:", errorMsg);
            alert(errorMsg);
          }
        } catch (err) {
          console.error("Network error during roles update:", err);
          alert("Error updating roles");
        } finally {
          btn.textContent = "Save Roles";
          btn.disabled = false;
        }
      });
  }
}

// ─── Main Data Load ─────────────────────────────────────────────

async function loadUserDetails() {
  try {
    const resp = await fetch(`/api/admin/users/${userId}`, { credentials: "same-origin" });
    if (resp.ok) {
      userData = await resp.json();
      renderAll();
      return;
    }
    if (resp.status === 404) {
      showError("User not found.");
      return;
    }
    showError("Failed to load user details.");
  } catch (e) {
    showError("Network error while loading user details.");
  }
}

function renderAll() {
  document.getElementById("user-loading").style.display = "none";
  document.getElementById("user-content").style.display = "";

  renderHeader();
  renderPersonalInfo();
  renderAccountInfo();
  renderPaymentMethods();
  renderWallets();
  renderTransactions();
  renderKYC();
  renderInvestments();
  renderOrders();
  renderSessions();
  renderOAuth();
  renderAudit();
}

function showError(msg) {
  document.getElementById("user-loading").innerHTML =
    `<p style="color:var(--admin-danger);font-weight:600;">${msg} <a href="/admin/users" class="admin-link">Back to Users</a></p>`;
}

// ─── Render: Header ─────────────────────────────────────────────

function renderHeader() {
  const d = userData;
  const name = `${d.first_name || ""} ${d.last_name || ""}`.trim() || d.email;
  document.getElementById("user-fullname").textContent = name;
  document.getElementById("user-email").textContent = d.email;
  document.getElementById("breadcrumb-name").textContent = name;
  document.title = `${name} - POOOL Admin`;

  // Avatar
  const initials = getInitials(d.first_name, d.last_name);
  const av = document.getElementById("user-avatar");
  av.textContent = initials;
  av.style.background = getAvatarColor(d.email);

  // Badges
  const badgesEl = document.getElementById("user-badges");
  let badges = "";
  (d.roles || []).forEach((r) => {
    badges += `<span class="admin-badge ${getRoleBadgeClass(r)}">${esc(r)}</span>`;
  });
  badges += getStatusBadge(d.status);
  badges += getKYCBadge(d.kyc_status);
  badgesEl.innerHTML = badges;

  // Quick stats
  document.getElementById("user-cash-balance").textContent = formatUSD(
    d.cash_balance_cents || 0,
  );
  document.getElementById("user-rewards-balance").textContent = formatUSD(
    d.rewards_balance_cents || 0,
  );
  document.getElementById("user-joined").textContent = formatDate(d.created_at);

  // Suspend button
  const suspendBtn = document.getElementById("btn-suspend-trigger");
  const suspendLabel = document.getElementById("btn-suspend-label");
  if (suspendBtn) {
    if (d.status === "suspended") {
      suspendLabel.textContent = "Activate";
      suspendBtn.style.borderColor = "var(--admin-success)";
      suspendBtn.style.color = "var(--admin-success)";
    } else {
      suspendLabel.textContent = "Suspend";
      suspendBtn.style.borderColor = "";
      suspendBtn.style.color = "";
    }
  }

  // Freeze button
  const freezeBtn = document.getElementById("btn-freeze-trigger");
  const freezeLabel = document.getElementById("btn-freeze-label");
  if (freezeBtn) {
    if (d.status === "frozen") {
      freezeLabel.textContent = "Unfreeze";
      freezeBtn.style.borderColor = "var(--admin-success)";
      freezeBtn.style.color = "var(--admin-success)";
    } else {
      freezeLabel.textContent = "Freeze";
      freezeBtn.style.borderColor =
        d.status === "frozen" ? "var(--admin-success)" : "var(--admin-danger)";
      freezeBtn.style.color =
        d.status === "frozen" ? "var(--admin-success)" : "var(--admin-danger)";
    }
  }
}

// ─── Render: Personal Info ──────────────────────────────────────

function renderPersonalInfo() {
  const d = userData;
  const profile = d.profile || {};
  document.getElementById("personal-info").innerHTML = `
        ${detailRow("First Name", profile.first_name)}
        ${detailRow("Last Name", profile.last_name)}
        ${detailRow("Date of Birth", profile.date_of_birth ? formatDate(profile.date_of_birth) : "—")}
        ${detailRow("Nationality", profile.nationality || "—")}
        ${detailRow("Phone", profile.phone_number || "—")}
        ${detailRow("Address", [profile.address_line_1, profile.address_line_2, profile.city, profile.state_province, profile.postal_code, profile.country].filter(Boolean).join(", ") || "—")}
        ${detailRow("Tax ID", profile.tax_id ? "••••" + (profile.tax_id.slice(-4) || "") : "—")}
    `;
}

// ─── Render: Account Info ───────────────────────────────────────

function renderAccountInfo() {
  const d = userData;
  const settings = d.settings || {};
  document.getElementById("account-info").innerHTML = `
        ${detailRowHtml("User ID", `<code style="font-size:11px;color:var(--admin-text-muted);background:rgba(255,255,255,0.05);padding:2px 6px;border-radius:4px;">${esc(d.id)}</code>`)}
        ${detailRowHtml("Email Verified", d.email_verified ? '<span class="admin-badge admin-badge--success">Yes</span>' : '<span class="admin-badge admin-badge--danger">No</span>')}
        ${detailRowHtml("2FA Enabled", settings.totp_enabled ? '<span class="admin-badge admin-badge--success">Enabled</span>' : '<span class="admin-badge admin-badge--neutral">Disabled</span>')}
        ${detailRow("Language", settings.language || "en")}
        ${detailRow("Currency", settings.currency || "USD")}
        ${detailRow("Timezone", settings.timezone || "—")}
        ${detailRow("Last Updated", formatDateTime(d.updated_at))}
    `;
}

// ─── Render: Payment Methods ────────────────────────────────────

function renderPaymentMethods() {
  const methods = userData.payment_methods || [];
  const tbody = document.getElementById("payment-methods-body");
  if (methods.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="5" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No payment methods found.</td></tr>';
    return;
  }
  tbody.innerHTML = methods
    .map(
      (m) => `
        <tr>
            <td>${esc(m.method_type === "card" ? "Card" : "Bank")}</td>
            <td>
                <div><strong>${esc(m.brand || m.bank_name || "—")}</strong> •••• ${esc(m.last_four || "—")}</div>
                ${m.holder_name ? `<div style="font-size:11px;color:var(--admin-text-muted);">${esc(m.holder_name)}</div>` : ""}
            </td>
            <td>${getMethodStatusBadge(m.status)}</td>
            <td>${m.is_default ? '<span class="admin-badge admin-badge--success">Default</span>' : "—"}</td>
            <td style="color:var(--admin-text-muted);font-size:12px;">${formatDate(m.created_at)}</td>
        </tr>
    `,
    )
    .join("");
}

// ─── Render: Wallets ────────────────────────────────────────────

function renderWallets() {
  const wallets = userData.wallets || [];
  const container = document.getElementById("wallet-cards");
  if (wallets.length === 0) {
    container.innerHTML =
      '<div class="admin-kpi-card" style="padding:20px;"><div class="admin-kpi-label">No wallets found</div></div>';
    return;
  }
  container.innerHTML = wallets
    .map(
      (w) => `
        <div class="admin-kpi-card">
            <div class="admin-kpi-header">
                <span class="admin-kpi-label">${esc(w.wallet_type)} Wallet</span>
                <div class="admin-kpi-icon ${w.wallet_type === "cash" ? "admin-kpi-icon--green" : "admin-kpi-icon--purple"}">${w.wallet_type === "cash" ? '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="var(--admin-success)" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="5" width="16" height="12" rx="2" /><path d="M14 10a2 2 0 11-4 0 2 2 0 014 0z" /><path d="M2 5l4-3h8l4 3" /></svg>' : '<svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="var(--admin-accent)" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M10 2l2.4 4.8L18 7.6l-4 3.9.9 5.5L10 14.6 5.1 17l.9-5.5-4-3.9 5.6-.8L10 2z"/></svg>'}</div>
            </div>
            <div class="admin-kpi-value">${formatUSD(w.balance_cents)}</div>
            <span class="admin-kpi-subtext">${esc(w.currency || "USD")}</span>
        </div>
    `,
    )
    .join("");

  allTransactions = userData.transactions || [];
}

// ─── Render: Transactions ───────────────────────────────────────

function renderTransactions() {
  const typeFilter = document.getElementById("filter-tx-type")?.value || "";
  const statusFilter = document.getElementById("filter-tx-status")?.value || "";

  let txs = allTransactions;
  if (typeFilter) txs = txs.filter((t) => t.type === typeFilter);
  if (statusFilter) txs = txs.filter((t) => t.status === statusFilter);

  const tbody = document.getElementById("transactions-body");
  if (txs.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="6" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No transactions found.</td></tr>';
    return;
  }

  tbody.innerHTML = txs
    .map(
      (t) => `
        <tr>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDateTime(t.created_at)}</td>
            <td>${getTxTypeBadge(t.type)}</td>
            <td style="font-weight:600;font-variant-numeric:tabular-nums;${t.amount_cents >= 0 ? "color:var(--admin-success);" : "color:var(--admin-danger);"}">${t.amount_cents >= 0 ? "+" : ""}${formatUSD(t.amount_cents)}</td>
            <td>${getStatusBadge(t.status)}</td>
            <td style="max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--admin-text-muted);font-size:12px;">${esc(t.description || "—")}</td>
            <td style="font-size:11px;color:var(--admin-text-muted);font-family:monospace;">${esc(t.external_ref_id || "—")}</td>
        </tr>
    `,
    )
    .join("");
}

// ─── Render: KYC ────────────────────────────────────────────────

function renderKYC() {
  const records = userData.kyc_records || [];
  const tbody = document.getElementById("kyc-records-body");
  if (records.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="8" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No KYC records.</td></tr>';
    return;
  }
  tbody.innerHTML = records
    .map(
      (k) => `
        <tr>
            <td>${esc(k.provider || "sumsub")}</td>
            <td>${getKYCBadge(k.status)}</td>
            <td>${esc(k.document_type || "—")}</td>
            <td>${k.pep_check_passed === true ? '<span class="admin-badge admin-badge--success">Passed</span>' : k.pep_check_passed === false ? '<span class="admin-badge admin-badge--danger">Flagged</span>' : "—"}</td>
            <td>${k.sanctions_check === true ? '<span class="admin-badge admin-badge--success">Clear</span>' : k.sanctions_check === false ? '<span class="admin-badge admin-badge--danger">Hit</span>' : "—"}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${k.verified_at ? formatDate(k.verified_at) : "—"}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${k.expires_at ? formatDate(k.expires_at) : "—"}</td>
            <td>
                ${k.provider === "sumsub"
          ? `<a href="https://cockpit.sumsub.com/checkus#/applicant/${esc(k.provider_ref_id)}" target="_blank" rel="noopener" class="admin-btn admin-btn--secondary admin-btn--sm">View in SumSub</a>`
          : k.provider === "didit"
            ? `<a href="https://business.didit.me/sessions/${esc(k.provider_ref_id)}" target="_blank" rel="noopener" class="admin-btn admin-btn--secondary admin-btn--sm">View in Didit</a>`
            : k.provider_ref_id
              ? `<span class="admin-badge admin-badge--neutral">Ref: ${esc(k.provider_ref_id.slice(0, 8))}…</span>`
              : "—"
        }
            </td>
        </tr>
    `,
    )
    .join("");
}

// ─── Render: Investments ────────────────────────────────────────

function renderInvestments() {
  const investments = userData.investments || [];
  const tbody = document.getElementById("investments-body");
  if (investments.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="7" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No investments.</td></tr>';
    return;
  }
  tbody.innerHTML = investments
    .map(
      (inv) => `
        <tr>
            <td><a href="/admin/asset-details?id=${esc(inv.asset_id)}" class="admin-link">${esc(inv.asset_title || inv.asset_id)}</a></td>
            <td style="font-weight:600;">${inv.tokens_owned}</td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(inv.purchase_value_cents)}</td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(inv.current_value_cents)}</td>
            <td style="font-variant-numeric:tabular-nums;">${formatUSD(inv.total_rental_cents)}</td>
            <td>${getStatusBadge(inv.status)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${formatDate(inv.purchased_at)}</td>
        </tr>
    `,
    )
    .join("");
}

// ─── Render: Orders ─────────────────────────────────────────────

function renderOrders() {
  const orders = userData.orders || [];
  const tbody = document.getElementById("orders-body");
  if (orders.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="6" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No orders.</td></tr>';
    return;
  }
  tbody.innerHTML = orders
    .map(
      (o) => `
        <tr>
            <td><a href="/admin/orders?id=${esc(o.id)}" class="admin-link">${esc(o.order_number)}</a></td>
            <td style="font-weight:600;font-variant-numeric:tabular-nums;">${formatUSD(o.total_cents)}</td>
            <td>${getStatusBadge(o.status)}</td>
            <td>${esc(o.payment_method || "—")}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${formatDateTime(o.created_at)}</td>
            <td>
                <div style="display:flex;align-items:center;gap:8px;">
                    <span style="font-size:12px;color:var(--admin-text-muted);">${o.completed_at ? formatDateTime(o.completed_at) : "—"}</span>
                    ${o.status === "pending"
          ? `
                        <button onclick="approveOrder('${o.id}', '${esc(o.order_number)}')" class="admin-btn admin-btn--success admin-btn--sm" style="background:#12B76A;border-color:#12B76A;color:white;padding:2px 8px;font-size:11px;">Approve</button>
                        <button onclick="rejectOrder('${o.id}', '${esc(o.order_number)}')" class="admin-btn admin-btn--danger admin-btn--sm" style="background:#F04438;border-color:#F04438;color:white;padding:2px 8px;font-size:11px;">Reject</button>
                    `
          : ""
        }
                </div>
            </td>
        </tr>
    `,
    )
    .join("");
}

async function approveOrder(id, num) {
  if (
    !confirm(
      `Are you sure you want to APPROVE Order ${num}? This will confirm payment and activate the user's investments.`,
    )
  )
    return;
  try {
    const res = await fetch(`/api/admin/orders/${id}/approve`, {
      method: "POST",
      credentials: "same-origin",
      headers: { "x-csrf-token": getCsrfToken() },
    });
    const data = await res.json();
    if (res.ok) {
      alert(`Order ${num} approved successfully.`);
      loadUserDetails();
    } else {
      alert(data.error || "Approval failed");
    }
  } catch (e) {
    alert("Network error during approval.");
  }
}

async function rejectOrder(id, num) {
  if (
    !confirm(
      `Are you sure you want to REJECT Order ${num}? This will FAIL the order and return the reserved tokens to availability.`,
    )
  )
    return;
  try {
    const res = await fetch(`/api/admin/orders/${id}/reject`, {
      method: "POST",
      credentials: "same-origin",
      headers: { "x-csrf-token": getCsrfToken() },
    });
    const data = await res.json();
    if (res.ok) {
      alert(`Order ${num} rejected and tokens returned.`);
      loadUserDetails();
    } else {
      alert(data.error || "Rejection failed");
    }
  } catch (e) {
    alert("Network error during rejection.");
  }
}

// ─── Render: Sessions ───────────────────────────────────────────

function renderSessions() {
  const sessions = userData.sessions || [];
  const tbody = document.getElementById("sessions-body");
  if (sessions.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="5" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No active sessions.</td></tr>';
    return;
  }
  tbody.innerHTML = sessions
    .map(
      (s) => `
        <tr>
            <td style="font-family:monospace;font-size:12px;">${esc(s.ip_address || "—")}</td>
            <td style="max-width:250px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:12px;color:var(--admin-text-muted);" title="${esc(s.user_agent || "")}">${esc(parseUserAgent(s.user_agent))}</td>
            <td>${s.remember_me ? '<span class="admin-badge admin-badge--info">Yes</span>' : "No"}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${formatDateTime(s.created_at)}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${formatDateTime(s.expires_at)}</td>
        </tr>
    `,
    )
    .join("");
}

// ─── Render: OAuth ──────────────────────────────────────────────

function renderOAuth() {
  const accounts = userData.oauth_accounts || [];
  const tbody = document.getElementById("oauth-body");
  if (accounts.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="3" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No linked OAuth accounts.</td></tr>';
    return;
  }
  tbody.innerHTML = accounts
    .map(
      (a) => `
        <tr>
            <td>
                <span class="admin-badge ${a.provider === "google" ? "admin-badge--info" : "admin-badge--neutral"}">
                    ${esc(a.provider)}
                </span>
            </td>
            <td>${esc(a.provider_email || "—")}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);">${formatDate(a.created_at)}</td>
        </tr>
    `,
    )
    .join("");
}

// ─── Render: Audit ──────────────────────────────────────────────

function renderAudit() {
  const logs = userData.audit_logs || [];
  const tbody = document.getElementById("audit-body");
  if (logs.length === 0) {
    tbody.innerHTML =
      '<tr><td colspan="6" style="text-align:center;padding:24px;color:var(--admin-text-muted);">No audit entries.</td></tr>';
    return;
  }
  tbody.innerHTML = logs
    .map(
      (l) => `
        <tr>
            <td style="font-size:11px;color:var(--admin-text-muted);font-family:monospace;">#${l.id}</td>
            <td><span class="admin-badge admin-badge--neutral">${esc(l.action)}</span></td>
            <td style="font-size:12px;">${esc(l.entity_type)}${l.entity_id ? ` <code style="font-size:10px;">${esc(String(l.entity_id).slice(0, 8))}…</code>` : ""}</td>
            <td style="font-family:monospace;font-size:11px;color:var(--admin-text-muted);">${esc(l.ip_address || "—")}</td>
            <td style="font-size:12px;color:var(--admin-text-muted);white-space:nowrap;">${formatDateTime(l.created_at)}</td>
            <td>
                ${l.previous_state || l.new_state ? `<button class="admin-btn admin-btn--secondary admin-btn--sm" onclick="toggleAuditDetail(this, '${esc(JSON.stringify({ prev: l.previous_state, next: l.new_state }))}')">View</button>` : "—"}
            </td>
        </tr>
    `,
    )
    .join("");
}

function toggleAuditDetail(btn, jsonStr) {
  const row = btn.closest("tr");
  const existing = row.nextElementSibling;
  if (existing && existing.classList.contains("audit-detail-row")) {
    existing.remove();
    btn.textContent = "View";
    return;
  }
  const data = JSON.parse(jsonStr);
  const detailRow = document.createElement("tr");
  detailRow.className = "audit-detail-row";
  detailRow.innerHTML = `<td colspan="6" style="background:rgba(255,255,255,0.02);padding:12px 16px;">
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px;">
            <div><strong style="font-size:11px;color:var(--admin-text-muted);text-transform:uppercase;">Previous State</strong><pre style="margin:4px 0;font-size:11px;color:var(--admin-text-secondary);white-space:pre-wrap;max-height:150px;overflow:auto;">${data.prev ? JSON.stringify(data.prev, null, 2) : "N/A"}</pre></div>
            <div><strong style="font-size:11px;color:var(--admin-text-muted);text-transform:uppercase;">New State</strong><pre style="margin:4px 0;font-size:11px;color:var(--admin-text-secondary);white-space:pre-wrap;max-height:150px;overflow:auto;">${data.next ? JSON.stringify(data.next, null, 2) : "N/A"}</pre></div>
        </div>
    </td>`;
  row.after(detailRow);
  btn.textContent = "Hide";
}

// ─── Helpers ────────────────────────────────────────────────────

function detailRow(label, value) {
  const safeValue = value ? esc(String(value)) : "—";
  return `<div class="admin-detail-row"><span class="admin-detail-label">${label}</span><span class="admin-detail-value">${safeValue}</span></div>`;
}

/** Like detailRow but value is trusted HTML (badges, code blocks, etc.) */
function detailRowHtml(label, htmlValue) {
  return `<div class="admin-detail-row"><span class="admin-detail-label">${label}</span><span class="admin-detail-value">${htmlValue || "—"}</span></div>`;
}

function esc(str) {
  if (typeof str !== "string") return str || "";
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function getInitials(first, last) {
  return ((first || "?")[0] + (last || "?")[0]).toUpperCase();
}

function getAvatarColor(email) {
  let hash = 0;
  for (let i = 0; i < (email || "").length; i++)
    hash = email.charCodeAt(i) + ((hash << 5) - hash);
  const colors = [
    "#6366F1",
    "#EC4899",
    "#F59E0B",
    "#10B981",
    "#3B82F6",
    "#8B5CF6",
    "#EF4444",
    "#06B6D4",
  ];
  return colors[Math.abs(hash) % colors.length];
}

function getRoleBadgeClass(role) {
  if (role === "admin" || role === "super_admin") return "admin-badge--danger";
  if (role === "developer") return "admin-badge--info";
  return "admin-badge--neutral";
}

function getKYCBadge(status) {
  if (!status)
    return '<span class="admin-badge admin-badge--neutral"><span class="admin-badge-dot"></span>No KYC</span>';
  if (status === "approved")
    return '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Verified</span>';
  if (status === "pending" || status === "in_review")
    return '<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>Pending</span>';
  if (status === "rejected")
    return '<span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>Rejected</span>';
  if (status === "expired")
    return '<span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>Expired</span>';
  return `<span class="admin-badge admin-badge--neutral">${esc(status)}</span>`;
}

function getStatusBadge(status) {
  if (status === "active" || status === "completed" || status === "paid")
    return `<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>${esc(status)}</span>`;
  if (
    status === "pending" ||
    status === "processing" ||
    status === "in_process" ||
    status === "scheduled"
  )
    return `<span class="admin-badge admin-badge--warning"><span class="admin-badge-dot"></span>${esc(status)}</span>`;
  if (
    status === "suspended" ||
    status === "failed" ||
    status === "cancelled" ||
    status === "rejected"
  )
    return `<span class="admin-badge admin-badge--danger"><span class="admin-badge-dot"></span>${esc(status)}</span>`;
  return `<span class="admin-badge admin-badge--neutral"><span class="admin-badge-dot"></span>${esc(status || "unknown")}</span>`;
}

function getMethodStatusBadge(status) {
  if (status === "active")
    return '<span class="admin-badge admin-badge--success"><span class="admin-badge-dot"></span>Active</span>';
  if (status === "inactive")
    return '<span class="admin-badge admin-badge--neutral"><span class="admin-badge-dot"></span>Inactive</span>';
  return `<span class="admin-badge admin-badge--neutral">${esc(status || "—")}</span>`;
}

function getTxTypeBadge(type) {
  const colors = {
    deposit: "admin-badge--success",
    withdrawal: "admin-badge--warning",
    purchase: "admin-badge--info",
    sale: "admin-badge--info",
    dividend: "admin-badge--success",
    reward: "admin-badge--success",
    refund: "admin-badge--warning",
    fee: "admin-badge--danger",
  };
  return `<span class="admin-badge ${colors[type] || "admin-badge--neutral"}">${esc(type)}</span>`;
}

function formatUSD(cents) {
  if (typeof cents !== "number") return "$0.00";
  const neg = cents < 0;
  const abs = Math.abs(cents);
  return (
    (neg ? "-" : "") +
    "$" +
    (abs / 100).toLocaleString("en-US", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })
  );
}

function formatDate(isoString) {
  if (!isoString) return "—";
  const d = new Date(isoString);
  return d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function formatDateTime(isoString) {
  if (!isoString) return "—";
  const d = new Date(isoString);
  return (
    d.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    }) +
    " " +
    d.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    })
  );
}

function parseUserAgent(ua) {
  if (!ua) return "—";
  if (ua.includes("Chrome")) return "Chrome";
  if (ua.includes("Firefox")) return "Firefox";
  if (ua.includes("Safari")) return "Safari";
  if (ua.includes("Edge")) return "Edge";
  return ua.slice(0, 40) + "…";
}
