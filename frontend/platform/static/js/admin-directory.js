/**
 * Admin Directory JS - identity lifecycle management without Alpine.
 * The global CSP blocks Alpine's expression evaluator, so this page uses
 * plain DOM rendering and event delegation.
 */
(function () {
  "use strict";

  const fallbackRoles = [
    { id: "1", name: "super_admin", permissions: ["all"] },
    {
      id: "2",
      name: "compliance_officer",
      permissions: ["kyc.read", "kyc.write", "kyc.override", "users.view", "treasury.read"],
    },
    { id: "3", name: "support_agent", permissions: ["users.view", "support.read", "support.write"] },
    {
      id: "4",
      name: "finance_admin",
      permissions: ["treasury.read", "treasury.write", "financials.payout.draft"],
    },
    { id: "5", name: "auditor_read_only", permissions: ["users.view", "treasury.read", "kyc.read", "audit.read"] },
  ];

  const state = {
    admins: [],
    filteredAdmins: [],
    roles: [],
    pendingInvites: [],
    searchQuery: "",
    filterRole: "",
    filterStatus: "",
    activeTab: "admins",
    editTarget: null,
    editRoles: [],
    inviteForm: {
      email: "",
      role: "",
    },
    stats: {
      total: 0,
      active: 0,
      suspended: 0,
      noMfa: 0,
      activeSessions: 0,
      pendingInvites: 0,
    },
  };

  const els = {};

  document.addEventListener("DOMContentLoaded", init);

  async function init() {
    els.root = document.getElementById("admin-directory");
    if (!els.root) return;

    cacheElements();
    attachEvents();
    await loadData();
  }

  function cacheElements() {
    els.toast = document.getElementById("admin-directory-toast");
    els.adminsPanel = document.getElementById("admin-directory-admins-panel");
    els.invitesPanel = document.getElementById("admin-directory-invites-panel");
    els.invitesCard = document.getElementById("admin-directory-invites-card");
    els.search = document.getElementById("admin-directory-search");
    els.roleFilter = document.getElementById("admin-directory-role-filter");
    els.statusFilter = document.getElementById("admin-directory-status-filter");
    els.tableBody = document.querySelector(".admin-table tbody");
    els.inviteModal = document.getElementById("admin-directory-invite-modal");
    els.editModal = document.getElementById("admin-directory-edit-modal");
    els.inviteEmail = document.getElementById("admin-directory-invite-email");
    els.inviteRole = document.getElementById("admin-directory-invite-role");
    els.tabButtons = Array.from(document.querySelectorAll(".admin-tab--line"));
  }

  function attachEvents() {
    if (els.search) {
      els.search.addEventListener("input", function () {
        state.searchQuery = els.search.value;
        applyFilters();
        render();
      });
    }

    if (els.roleFilter) {
      els.roleFilter.addEventListener("change", function () {
        state.filterRole = els.roleFilter.value;
        applyFilters();
        render();
      });
    }

    if (els.statusFilter) {
      els.statusFilter.addEventListener("change", function () {
        state.filterStatus = els.statusFilter.value;
        applyFilters();
        render();
      });
    }

    els.tabButtons.forEach(function (button, index) {
      button.addEventListener("click", function () {
        state.activeTab = index === 0 ? "admins" : "invites";
        renderTabs();
      });
    });

    document.querySelectorAll("button").forEach(function (button) {
      const text = button.textContent.trim();
      if (text === "Invite Admin" || text === "New Invite") {
        button.addEventListener("click", openInviteModal);
      }
    });

    if (els.inviteModal) {
      els.inviteModal.addEventListener("click", function (event) {
        if (event.target === els.inviteModal) closeInviteModal();
        const button = event.target.closest("button");
        if (!button) return;
        const text = button.textContent.trim();
        if (text === "Cancel" || text === "×") {
          event.preventDefault();
          closeInviteModal();
        }
        if (text === "Send Invitation") {
          event.preventDefault();
          sendInvite();
        }
      });
    }

    if (els.editModal) {
      els.editModal.addEventListener("click", function (event) {
        if (event.target === els.editModal) closeEditModal();
        const button = event.target.closest("button");
        if (!button) return;
        const text = button.textContent.trim();
        if (text === "Cancel" || text === "×") {
          event.preventDefault();
          closeEditModal();
        }
        if (text === "Save Changes") {
          event.preventDefault();
          saveAdminEdit();
        }
      });
    }

    if (els.tableBody) {
      els.tableBody.addEventListener("click", function (event) {
        const button = event.target.closest("[data-action]");
        if (!button) return;
        const admin = state.admins.find((item) => item.id === button.dataset.id);
        if (!admin) return;

        if (button.dataset.action === "edit") openEditModal(admin);
        if (button.dataset.action === "status") toggleStatus(admin);
        if (button.dataset.action === "sessions") killSessions(admin.id);
      });
    }

    if (els.invitesCard) {
      els.invitesCard.addEventListener("click", function (event) {
        const button = event.target.closest("[data-action]");
        if (!button) return;
        if (button.dataset.action === "resend-invite") resendInvite(button.dataset.id);
        if (button.dataset.action === "revoke-invite") revokeInvite(button.dataset.id);
        if (button.dataset.action === "new-invite") openInviteModal();
      });
    }
  }

  async function loadData() {
    try {
      const [adminsResp, rolesResp] = await Promise.all([
        fetch("/api/admin/admins"),
        fetch("/api/admin/roles"),
      ]);

      state.admins = adminsResp.ok ? await adminsResp.json() : [];
      state.roles = rolesResp.ok ? await rolesResp.json() : fallbackRoles;

      if (!Array.isArray(state.admins)) state.admins = [];
      if (!Array.isArray(state.roles) || !state.roles.length) state.roles = fallbackRoles;
      if (!state.inviteForm.role && state.roles.length) state.inviteForm.role = state.roles[0].name;

      try {
        const invitesResp = await fetch("/api/admin/admins/invitations");
        state.pendingInvites = invitesResp.ok ? await invitesResp.json() : [];
        if (!Array.isArray(state.pendingInvites)) state.pendingInvites = [];
      } catch (_) {
        state.pendingInvites = [];
      }

      computeStats();
      applyFilters();
      render();
    } catch (error) {
      console.error("[AdminDirectory] loadData error:", error);
      captureException(error);
      showToast("Unable to load admin directory", "error");
      render();
    }
  }

  function computeStats() {
    state.stats.total = state.admins.length;
    state.stats.active = state.admins.filter((admin) => admin.status === "active").length;
    state.stats.suspended = state.admins.filter((admin) => admin.status === "suspended").length;
    state.stats.noMfa = state.admins.filter((admin) => !admin.totp_enabled).length;
    state.stats.pendingInvites = state.pendingInvites.length;
    state.stats.activeSessions = state.admins.reduce(function (sum, admin) {
      return sum + (admin.session_count || (admin.status === "active" ? 1 : 0));
    }, 0);
  }

  function applyFilters() {
    let result = state.admins.slice();

    if (state.searchQuery) {
      const query = state.searchQuery.toLowerCase();
      result = result.filter(function (admin) {
        return (
          String(admin.email || "").toLowerCase().includes(query) ||
          String(admin.first_name || "").toLowerCase().includes(query) ||
          String(admin.last_name || "").toLowerCase().includes(query) ||
          String(admin.id || "").toLowerCase().includes(query) ||
          (admin.roles || []).some((role) => String(role).toLowerCase().includes(query))
        );
      });
    }

    if (state.filterRole) {
      result = result.filter((admin) => (admin.roles || []).includes(state.filterRole));
    }

    if (state.filterStatus) {
      result = result.filter((admin) => admin.status === state.filterStatus);
    }

    state.filteredAdmins = result;
  }

  function render() {
    renderStats();
    renderFilters();
    renderTabs();
    renderAdmins();
    renderInvites();
    renderInviteModal();
  }

  function renderStats() {
    setText('[x-text="stats.total"]', state.stats.total);
    setText('[x-text="stats.active"]', state.stats.active);
    setText('[x-text="stats.suspended"]', state.stats.suspended);
    setText('[x-text="stats.activeSessions"]', state.stats.activeSessions);
    setText('[x-text="stats.pendingInvites"]', state.stats.pendingInvites);
    setText('[x-text="stats.noMfa"]', state.stats.noMfa);
    setText('[x-text="filteredAdmins.length"]', state.filteredAdmins.length);
    setText('[x-text="pendingInvites.length"]', state.pendingInvites.length);

    const coverage = state.stats.total > 0
      ? Math.round(((state.stats.total - state.stats.noMfa) / state.stats.total) * 100) + "%"
      : "---";
    const coverageEl = document.querySelector('[x-text^="stats.total > 0"]');
    if (coverageEl) coverageEl.textContent = coverage;

    document.querySelectorAll('[x-show="stats.noMfa > 0"]').forEach((el) => {
      el.hidden = state.stats.noMfa <= 0;
    });
    document.querySelectorAll('[x-show="stats.noMfa === 0"]').forEach((el) => {
      el.hidden = state.stats.noMfa !== 0;
    });
    document.querySelectorAll('[x-show="pendingInvites.length > 0"]').forEach((el) => {
      el.hidden = state.pendingInvites.length === 0;
    });

    const mfaIcon = document.querySelector(".security-kpi-grid .security-kpi:nth-child(2) .security-kpi-icon");
    if (mfaIcon) {
      mfaIcon.classList.toggle("security-kpi-icon--danger", state.stats.noMfa > 0);
      mfaIcon.classList.toggle("security-kpi-icon--ok", state.stats.noMfa === 0);
    }

    const inviteIcon = document.querySelector(".security-kpi-grid .security-kpi:nth-child(4) .security-kpi-icon");
    if (inviteIcon) {
      inviteIcon.classList.toggle("security-kpi-icon--warn", state.stats.pendingInvites > 0);
      inviteIcon.classList.toggle("security-kpi-icon--neutral", state.stats.pendingInvites === 0);
    }
  }

  function renderFilters() {
    const roleOptions = '<option value="">All Roles</option>' + state.roles.map(function (role) {
      return `<option value="${escapeAttr(role.name)}">${escapeHtml(role.name)}</option>`;
    }).join("");

    if (els.roleFilter) {
      els.roleFilter.innerHTML = roleOptions;
      els.roleFilter.value = state.filterRole;
    }

    if (els.statusFilter) {
      els.statusFilter.value = state.filterStatus;
    }
  }

  function renderTabs() {
    if (els.adminsPanel) els.adminsPanel.hidden = state.activeTab !== "admins";
    if (els.invitesPanel) els.invitesPanel.hidden = state.activeTab !== "invites";

    els.tabButtons.forEach(function (button, index) {
      button.classList.toggle("active", state.activeTab === (index === 0 ? "admins" : "invites"));
    });
  }

  function renderAdmins() {
    if (!els.tableBody) return;

    if (!state.filteredAdmins.length) {
      els.tableBody.innerHTML = `
        <tr>
          <td colspan="8" style="text-align:center;padding:48px;color:var(--admin-text-muted);font-size:13px;">
            No admins match your search criteria.
          </td>
        </tr>`;
      return;
    }

    els.tableBody.innerHTML = state.filteredAdmins.map(function (admin) {
      const roles = Array.isArray(admin.roles) ? admin.roles : [];
      const displayName = admin.first_name
        ? `${admin.first_name} ${admin.last_name || ""}`.trim()
        : admin.email;
      const nextAction = admin.status === "active" ? "Suspend" : "Activate";
      const statusClass = admin.status === "active" ? "admin-badge--success" : "admin-badge--danger";

      return `
        <tr>
          <td>
            <div class="admin-user-inline">
              <div class="admin-avatar-sm" style="background:${escapeAttr(getAvatarColor(admin.email))}">
                ${escapeHtml(getInitials(admin.email))}
              </div>
              <div>
                <div class="admin-user-inline-name">${escapeHtml(displayName)}</div>
                <div class="admin-user-inline-email">${escapeHtml(admin.email || "")}</div>
              </div>
            </div>
          </td>
          <td>
            <div style="display:flex;gap:4px;flex-wrap:wrap">
              ${roles.map((role) => `<span class="admin-badge ${getRoleBadgeClass(role)}">${escapeHtml(role)}</span>`).join("")}
            </div>
          </td>
          <td>
            <span class="admin-2fa-badge ${admin.totp_enabled ? "admin-2fa-badge--on" : "admin-2fa-badge--off"}">
              ${admin.totp_enabled ? "On" : "Off"}
            </span>
          </td>
          <td>
            <span class="admin-badge ${statusClass}">
              <span class="admin-badge-dot"></span><span>${escapeHtml(admin.status || "unknown")}</span>
            </span>
          </td>
          <td style="font-size:12px;color:var(--admin-text-muted)">${escapeHtml(formatDate(admin.last_active))}</td>
          <td style="font-size:12px;color:var(--admin-text-muted);font-family:monospace">${escapeHtml(admin.last_ip || "---")}</td>
          <td style="font-size:12px;color:var(--admin-text-muted)">${escapeHtml(formatDate(admin.created_at))}</td>
          <td style="text-align:right">
            <div style="display:flex;gap:6px;justify-content:flex-end">
              <button class="admin-btn admin-btn--secondary admin-btn--sm" data-action="edit" data-id="${escapeAttr(admin.id)}" title="Edit roles">Edit</button>
              <button class="admin-btn admin-btn--secondary admin-btn--sm" data-action="status" data-id="${escapeAttr(admin.id)}"
                style="color:${admin.status === "active" ? "var(--admin-warning)" : "var(--admin-success)"}">${nextAction}</button>
              <button class="admin-btn admin-btn--secondary admin-btn--sm" data-action="sessions" data-id="${escapeAttr(admin.id)}"
                title="Kill Sessions" style="color:var(--admin-danger)">Kill</button>
            </div>
          </td>
        </tr>`;
    }).join("");
  }

  function renderInvites() {
    if (!els.invitesCard) return;

    const rows = state.pendingInvites.length
      ? state.pendingInvites.map(function (invite) {
        return `
          <div class="invite-row">
            <div>
              <div style="font-size:13px;font-weight:500;color:var(--admin-text-primary);">${escapeHtml(invite.email || "")}</div>
              <div style="font-size:11px;color:var(--admin-text-muted)">
                Role: <strong>${escapeHtml(invite.role || "")}</strong> &middot; Sent ${escapeHtml(formatDate(invite.created_at))}
              </div>
            </div>
            <div style="display:flex;gap:8px">
              <button class="admin-btn admin-btn--secondary admin-btn--sm" data-action="resend-invite" data-id="${escapeAttr(invite.id)}">Resend</button>
              <button class="admin-btn admin-btn--secondary admin-btn--sm" data-action="revoke-invite" data-id="${escapeAttr(invite.id)}"
                style="color:var(--admin-danger)">Revoke</button>
            </div>
          </div>`;
      }).join("")
      : `<div style="text-align:center;padding:48px;color:var(--admin-text-muted);font-size:13px;">No pending invitations.</div>`;

    els.invitesCard.innerHTML = `
      <div style="padding:16px 20px;border-bottom:1px solid var(--admin-border);display:flex;align-items:center;justify-content:space-between;">
        <h3 style="font-size:14px;font-weight:600;margin:0">Pending Admin Invitations</h3>
        <button class="admin-btn admin-btn--primary admin-btn--sm" data-action="new-invite">New Invite</button>
      </div>
      ${rows}`;
  }

  function renderInviteModal() {
    if (els.inviteRole) {
      els.inviteRole.innerHTML = state.roles.map(function (role) {
        return `<option value="${escapeAttr(role.name)}">${escapeHtml(role.name)}</option>`;
      }).join("");
      els.inviteRole.value = state.inviteForm.role || (state.roles[0] && state.roles[0].name) || "";
    }
  }

  function openInviteModal() {
    if (!els.inviteModal) return;
    if (els.inviteEmail) els.inviteEmail.value = "";
    renderInviteModal();
    els.inviteModal.classList.add("is-open");
  }

  function closeInviteModal() {
    if (els.inviteModal) els.inviteModal.classList.remove("is-open");
  }

  async function sendInvite() {
    const email = els.inviteEmail ? els.inviteEmail.value.trim() : "";
    const role = els.inviteRole ? els.inviteRole.value : "";
    if (!email) {
      showToast("Email address is required", "error");
      return;
    }

    try {
      const resp = await fetch("/api/admin/admins/invite", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, role, message: "" }),
      });

      if (resp.ok) {
        showToast("Invitation sent to " + email, "success");
        closeInviteModal();
        await loadData();
      } else {
        const err = await resp.json().catch(() => ({}));
        showToast("Failed: " + (err.error || "Unknown error"), "error");
      }
    } catch (error) {
      console.error("Error sending invite", error);
      captureException(error);
      showToast("Connection error - please try again", "error");
    }
  }

  async function resendInvite(inviteId) {
    try {
      const resp = await fetch(`/api/admin/admins/invitations/${encodeURIComponent(inviteId)}/resend`, {
        method: "POST",
      });
      showToast(resp.ok ? "Invitation resent" : "Failed to resend invitation", resp.ok ? "success" : "error");
    } catch (error) {
      console.error("Error resending invite", error);
      captureException(error);
      showToast("Connection error", "error");
    }
  }

  async function revokeInvite(inviteId) {
    if (!await confirmAction({
      title: "Revoke invitation",
      message: "The invite link will no longer work.",
      confirmText: "Revoke",
      type: "danger",
    })) return;

    try {
      const resp = await fetch(`/api/admin/admins/invitations/${encodeURIComponent(inviteId)}`, {
        method: "DELETE",
      });
      if (resp.ok) {
        state.pendingInvites = state.pendingInvites.filter((invite) => invite.id !== inviteId);
        computeStats();
        render();
        showToast("Invitation revoked", "success");
      } else {
        showToast("Failed to revoke invitation", "error");
      }
    } catch (error) {
      console.error("Error revoking invite", error);
      captureException(error);
      showToast("Connection error", "error");
    }
  }

  function openEditModal(admin) {
    state.editTarget = admin;
    state.editRoles = Array.isArray(admin.roles) ? admin.roles.slice() : [];
    renderEditModal();
    if (els.editModal) els.editModal.classList.add("is-open");
  }

  function closeEditModal() {
    if (els.editModal) els.editModal.classList.remove("is-open");
    state.editTarget = null;
    state.editRoles = [];
  }

  function renderEditModal() {
    if (!els.editModal || !state.editTarget) return;
    const body = els.editModal.querySelector(".admin-modal-body");
    if (!body) return;

    body.hidden = false;
    body.innerHTML = `
      <div style="display:flex;align-items:center;gap:12px;padding:12px 16px;background:var(--admin-bg);border-radius:var(--admin-radius-sm);margin-bottom:20px;">
        <div class="admin-avatar-sm" style="background:${escapeAttr(getAvatarColor(state.editTarget.email || ""))}">
          ${escapeHtml(getInitials(state.editTarget.email || ""))}
        </div>
        <div>
          <div style="font-size:14px;font-weight:600">${escapeHtml(state.editTarget.email || "")}</div>
          <div style="font-size:12px;color:var(--admin-text-muted)">ID: ${escapeHtml(String(state.editTarget.id || "").substring(0, 8))}...</div>
        </div>
      </div>
      <div class="admin-form-group">
        <label class="admin-form-label">Assigned Roles</label>
        <div style="display:flex;flex-wrap:wrap;gap:8px">
          ${state.roles.map(function (role) {
            const checked = state.editRoles.includes(role.name);
            return `
              <label class="admin-role-choice" data-role="${escapeAttr(role.name)}"
                style="display:flex;align-items:center;gap:6px;padding:6px 12px;border:1px solid var(--admin-border);border-radius:var(--admin-radius-sm);cursor:pointer;font-size:13px;transition:all 0.15s;${checked ? "background:var(--admin-accent-bg);border-color:var(--admin-accent);color:var(--admin-accent);font-weight:600;" : "color:var(--admin-text-secondary);"}">
                <input type="checkbox" ${checked ? "checked" : ""} style="display:none">
                <span>${escapeHtml(role.name)}</span>
              </label>`;
          }).join("")}
        </div>
      </div>
      <div style="margin-top:20px;padding-top:16px;border-top:1px solid var(--admin-border);">
        <h4 style="font-size:13px;font-weight:700;margin:0 0 12px">Security Actions</h4>
        <div style="display:flex;flex-direction:column;gap:8px">
          <button class="admin-btn admin-btn--secondary" data-edit-action="password-reset" style="justify-content:flex-start;width:100%">
            Force Password Reset on Next Login
          </button>
          <button class="admin-btn admin-btn--secondary" data-edit-action="sessions" style="justify-content:flex-start;width:100%;color:var(--admin-warning)">
            Revoke All Active Sessions
          </button>
        </div>
      </div>`;

    body.querySelectorAll(".admin-role-choice").forEach(function (label) {
      label.addEventListener("click", function () {
        const roleName = label.dataset.role;
        if (state.editRoles.includes(roleName)) {
          state.editRoles = state.editRoles.filter((role) => role !== roleName);
        } else {
          state.editRoles.push(roleName);
        }
        renderEditModal();
      });
    });

    body.querySelectorAll("[data-edit-action]").forEach(function (button) {
      button.addEventListener("click", function () {
        if (!state.editTarget) return;
        if (button.dataset.editAction === "password-reset") forcePasswordReset(state.editTarget.id);
        if (button.dataset.editAction === "sessions") killSessions(state.editTarget.id);
      });
    });
  }

  async function saveAdminEdit() {
    if (!state.editTarget) return;

    try {
      const resp = await fetch(`/api/admin/users/${encodeURIComponent(state.editTarget.id)}/roles`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ roles: state.editRoles }),
      });

      if (resp.ok) {
        showToast("Admin roles updated", "success");
        closeEditModal();
        await loadData();
      } else {
        const err = await resp.json().catch(() => ({}));
        showToast("Failed: " + (err.error || "Unknown error"), "error");
      }
    } catch (error) {
      console.error("Error saving admin edit", error);
      captureException(error);
      showToast("Connection error", "error");
    }
  }

  async function toggleStatus(admin) {
    const newStatus = admin.status === "active" ? "suspended" : "active";
    const action = newStatus === "suspended" ? "SUSPEND" : "ACTIVATE";

    if (!await confirmAction({
      title: `${action} admin`,
      message: `${admin.email}${newStatus === "suspended" ? " - all active sessions will be revoked immediately." : " - access will be restored."}`,
      confirmText: action,
      type: newStatus === "suspended" ? "danger" : "success",
    })) return;

    try {
      const resp = await fetch(`/api/admin/users/${encodeURIComponent(admin.id)}/status`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ status: newStatus }),
      });

      if (resp.ok) {
        showToast(`Admin ${action.toLowerCase()}d: ${admin.email}`, "success");
        await loadData();
      } else {
        showToast(`Failed to ${action.toLowerCase()} admin`, "error");
      }
    } catch (error) {
      console.error("Error toggling admin status", error);
      captureException(error);
      showToast("Connection error", "error");
    }
  }

  async function killSessions(adminId) {
    if (!await confirmAction({
      title: "Terminate all sessions",
      message: "This admin will be immediately logged out of all devices.",
      confirmText: "Terminate",
      type: "danger",
    })) return;

    try {
      const resp = await fetch(`/api/admin/users/${encodeURIComponent(adminId)}/sessions`, {
        method: "DELETE",
      });

      if (resp.ok) {
        showToast("All sessions terminated", "success");
        await loadData();
      } else {
        showToast("Failed to terminate sessions", "error");
      }
    } catch (error) {
      console.error("Error killing sessions", error);
      captureException(error);
      showToast("Connection error", "error");
    }
  }

  async function forcePasswordReset(adminId) {
    if (!await confirmAction({
      title: "Force password reset",
      message: "This admin will be required to reset their password on next login.",
      confirmText: "Force Reset",
      type: "warning",
    })) return;

    try {
      const resp = await fetch(`/api/admin/users/${encodeURIComponent(adminId)}/force-password-reset`, {
        method: "POST",
      });

      showToast(resp.ok ? "Password reset forced on next login" : "Failed to force password reset", resp.ok ? "success" : "error");
    } catch (error) {
      console.error("Error forcing password reset", error);
      captureException(error);
      showToast("Connection error", "error");
    }
  }

  function confirmAction(options) {
    if (typeof window.pooolConfirm === "function") return window.pooolConfirm(options);
    return Promise.resolve(window.confirm(options.message || options.title || "Confirm action"));
  }

  function getInitials(email) {
    if (!email) return "??";
    const parts = email.split("@")[0].split(/[._-]/);
    if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase();
    return email.substring(0, 2).toUpperCase();
  }

  function getAvatarColor(email) {
    let hash = 0;
    for (let i = 0; i < String(email || "").length; i += 1) {
      hash = email.charCodeAt(i) + ((hash << 5) - hash);
    }
    const colors = ["#6366F1", "#EC4899", "#F59E0B", "#10B981", "#3B82F6", "#8B5CF6", "#EF4444", "#06B6D4"];
    return colors[Math.abs(hash) % colors.length];
  }

  function getRoleBadgeClass(role) {
    switch (role) {
      case "super_admin":
      case "admin":
        return "admin-badge--danger";
      case "compliance":
      case "compliance_officer":
        return "admin-badge--warning";
      case "finance":
      case "finance_admin":
        return "admin-badge--info";
      case "support":
      case "support_agent":
        return "admin-badge--success";
      case "auditor_read_only":
      case "developer":
      default:
        return "admin-badge--neutral";
    }
  }

  function formatDate(isoString) {
    if (!isoString) return "Never";
    const date = new Date(isoString);
    if (Number.isNaN(date.getTime())) return "Never";

    const now = new Date();
    const diffHours = Math.floor((now - date) / (1000 * 60 * 60));
    if (diffHours < 1) return "Just now";
    if (diffHours < 24) return diffHours + "h ago";
    if (diffHours < 48) return "Yesterday";

    return date.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: date.getFullYear() !== now.getFullYear() ? "numeric" : undefined,
    });
  }

  function showToast(message, type) {
    if (!els.toast) return;
    els.toast.textContent = message;
    els.toast.classList.toggle("is-visible", true);
    els.toast.style.background = type === "error" ? "var(--admin-danger)" : "var(--admin-success)";
    els.toast.style.color = "#fff";

    window.clearTimeout(els.toast._hideTimer);
    els.toast._hideTimer = window.setTimeout(function () {
      els.toast.classList.remove("is-visible");
    }, 3500);
  }

  function setText(selector, value) {
    document.querySelectorAll(selector).forEach((el) => {
      el.textContent = String(value);
    });
  }

  function escapeHtml(value) {
    return String(value == null ? "" : value)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }

  function escapeAttr(value) {
    return escapeHtml(value);
  }

  function captureException(error) {
    if (typeof window.Sentry !== "undefined") window.Sentry.captureException(error);
  }
})();
