/**
 * Admin RBAC Matrix JS - CSP-safe role and permission management.
 */
(function () {
  "use strict";

  const permissionCategories = [
    {
      name: "User Management",
      icon: "People",
      permissions: [
        { key: "users.view", description: "View user profiles and directory" },
        { key: "users.edit", description: "Edit user profiles and settings" },
        { key: "users.delete", description: "Delete or deactivate user accounts" },
        { key: "users.suspend", description: "Suspend/activate user accounts" },
        { key: "pii.view", description: "View unmasked PII (full email, phone, tax ID)" },
        { key: "users.impersonate", description: "Impersonate a user session (super_admin only)" },
      ],
    },
    {
      name: "KYC & Compliance",
      icon: "Shield",
      permissions: [
        { key: "kyc.read", description: "View KYC records and verification status" },
        { key: "kyc.write", description: "Process KYC reviews (approve/reject)" },
        { key: "kyc.override", description: "Override KYC status outside normal flow" },
        { key: "aml.read", description: "View AML/PEP/sanctions check results" },
        { key: "aml.escalate", description: "Escalate cases for enhanced due diligence" },
      ],
    },
    {
      name: "Financial Operations",
      icon: "Finance",
      permissions: [
        { key: "treasury.read", description: "View ledger, balances, and transaction history" },
        { key: "treasury.write", description: "Create manual wallet adjustments" },
        { key: "deposits.confirm", description: "Manually confirm pending deposits" },
        { key: "withdrawals.approve", description: "Approve pending withdrawal requests" },
        { key: "financials.payout.draft", description: "Draft dividend payout distributions" },
        { key: "financials.payout.approve", description: "Execute/approve dividend payouts (Four-Eyes)" },
        { key: "invoices.manage", description: "Issue, void, and reissue invoices" },
        { key: "fees.configure", description: "Modify platform fee percentages" },
      ],
    },
    {
      name: "Asset Management",
      icon: "Assets",
      permissions: [
        { key: "assets.view", description: "View all asset details and financials" },
        { key: "assets.edit", description: "Edit asset properties and status" },
        { key: "assets.publish", description: "Publish/unpublish assets on marketplace" },
        { key: "assets.feature", description: "Toggle featured status for assets" },
        { key: "submissions.review", description: "Review developer asset submissions" },
        { key: "submissions.approve", description: "Approve/reject submissions for listing" },
        { key: "assets.refund", description: "Force-refund an asset (reverses all investments)" },
      ],
    },
    {
      name: "Orders & Investments",
      icon: "Orders",
      permissions: [
        { key: "orders.view", description: "View all orders and order items" },
        { key: "orders.cancel", description: "Cancel orders and process refunds" },
        { key: "investments.view", description: "View investment cap tables" },
        { key: "investments.adjust", description: "Manual investment adjustments" },
      ],
    },
    {
      name: "Support",
      icon: "Support",
      permissions: [
        { key: "support.read", description: "View support tickets" },
        { key: "support.write", description: "Reply to and manage support tickets" },
        { key: "support.assign", description: "Assign tickets to other admins" },
        { key: "support.escalate", description: "Escalate tickets to higher priority" },
      ],
    },
    {
      name: "Rewards & Referrals",
      icon: "Rewards",
      permissions: [
        { key: "rewards.view", description: "View rewards balances and tiers" },
        { key: "rewards.adjust", description: "Manual credit/debit rewards balances" },
        { key: "tiers.configure", description: "Edit tier thresholds and cashback rates" },
        { key: "referrals.manage", description: "Qualify/flag referrals, manage program" },
      ],
    },
    {
      name: "Notifications & Email",
      icon: "Email",
      permissions: [
        { key: "notifications.send", description: "Send individual notifications" },
        { key: "notifications.broadcast", description: "Broadcast notifications to all users" },
        { key: "email.templates", description: "Edit email templates" },
        { key: "email.campaigns", description: "Create and send marketing campaigns" },
      ],
    },
    {
      name: "Content",
      icon: "Content",
      permissions: [
        { key: "blog.view", description: "View Sanity blog articles, authors, categories, and status" },
        { key: "blog.edit", description: "Create and edit blog drafts in POOOL Admin" },
        { key: "blog.publish", description: "Publish and unpublish Sanity blog articles" },
        { key: "blog.archive", description: "Archive and restore blog articles" },
        { key: "blog.import", description: "Import legacy POOOL database blog articles into Sanity" },
        { key: "blog.manage", description: "Full backwards-compatible Blog CMS access" },
      ],
    },
    {
      name: "System Administration",
      icon: "System",
      permissions: [
        { key: "admins.manage", description: "Invite, edit, and suspend admin accounts" },
        { key: "roles.view", description: "View role definitions and permission matrix" },
        { key: "roles.edit", description: "Modify role permissions (dangerous)" },
        { key: "settings.view", description: "View platform configuration" },
        { key: "settings.edit", description: "Modify platform configuration" },
        { key: "audit.read", description: "Read audit logs for compliance" },
        { key: "system.health", description: "View system health, jobs, and webhooks" },
        { key: "reports.generate", description: "Generate and export compliance reports" },
        { key: "all", description: "Wildcard - grants all current and future permissions" },
      ],
    },
  ];

  const sodRules = [
    {
      perm1: "financials.payout.draft",
      perm2: "financials.payout.approve",
      message: "Cannot both Draft AND Approve financial payouts (Four-Eyes Principle violation)",
    },
    {
      perm1: "roles.edit",
      perm2: "audit.read",
      message: "Editing roles and reading audit logs in the same role creates a conflict of interest",
      severity: "warning",
    },
    {
      perm1: "users.delete",
      perm2: "audit.read",
      message: "Deleting users and reading audit logs in the same non-super role creates oversight gaps",
      severity: "warning",
    },
  ];

  const state = {
    roles: [],
    originalRoles: [],
    isUsingFallback: false,
    sodConflicts: [],
    saving: false,
  };

  const els = {};

  document.addEventListener("DOMContentLoaded", init);

  async function init() {
    els.root = document.getElementById("rbac-matrix");
    if (!els.root) return;

    els.toast = document.getElementById("rbac-toast");
    els.roleCards = document.getElementById("rbac-role-cards");
    els.sodWarnings = document.getElementById("rbac-sod-warnings");
    els.table = document.querySelector(".perm-matrix");
    els.theadRow = document.querySelector(".perm-matrix thead tr");
    els.tbody = document.querySelector(".perm-matrix tbody");
    els.saveButton = document.getElementById("rbac-save");
    els.resetButton = document.getElementById("rbac-reset");
    els.modal = document.getElementById("rbac-create-modal");
    els.roleName = document.getElementById("rbac-role-name");
    els.roleDescription = document.getElementById("rbac-role-description");
    els.roleClone = document.getElementById("rbac-role-clone");

    attachEvents();
    await loadData();
  }

  function attachEvents() {
    if (els.saveButton) els.saveButton.addEventListener("click", saveMatrix);
    if (els.resetButton) els.resetButton.addEventListener("click", resetMatrix);

    if (els.roleCards) {
      els.roleCards.addEventListener("click", function (event) {
        if (event.target.closest("[data-action='create-role']")) openCreateRoleModal();
      });
    }

    if (els.tbody) {
      els.tbody.addEventListener("change", function (event) {
        const checkbox = event.target.closest("[data-role-id][data-permission]");
        if (!checkbox) return;
        togglePermission(checkbox.dataset.roleId, checkbox.dataset.permission);
      });
    }

    if (els.modal) {
      els.modal.addEventListener("click", function (event) {
        if (event.target === els.modal) closeCreateRoleModal();
        const button = event.target.closest("button");
        if (!button) return;
        const text = button.textContent.trim();
        if (text === "Cancel" || text === "×") {
          event.preventDefault();
          closeCreateRoleModal();
        }
        if (text === "Create Role") {
          event.preventDefault();
          createRole();
        }
      });
    }
  }

  async function loadData() {
    try {
      const rolesResp = await fetch("/api/admin/roles");
      if (rolesResp.ok) {
        state.roles = await rolesResp.json();
        state.isUsingFallback = false;
      } else {
        state.roles = [];
        state.isUsingFallback = true;
        showToast(`Failed to load roles (${rolesResp.status}). Contact support if this persists.`, "error");
      }
    } catch (error) {
      console.error("Error loading rbac data", error);
      captureException(error);
      state.roles = [];
      state.isUsingFallback = true;
      showToast("Network error loading roles. Check your connection and reload.", "error");
    }

    normalizeRoles();
    state.originalRoles = deepClone(state.roles);
    checkSodConflicts();
    render();
  }

  function normalizeRoles() {
    if (!Array.isArray(state.roles)) state.roles = [];
    state.roles = state.roles.map(function (role) {
      return {
        id: String(role.id || role.name || Date.now()),
        name: role.name || "unnamed_role",
        description: role.description || "",
        admin_count: role.admin_count || 0,
        permissions: Array.isArray(role.permissions) ? role.permissions.slice() : [],
      };
    });
  }

  function render() {
    renderRoleCards();
    renderSodWarnings();
    renderMatrix();
    renderCreateRoleModal();
    renderSaveButton();
  }

  function renderRoleCards() {
    if (!els.roleCards) return;

    els.roleCards.innerHTML = state.roles.map(function (role) {
      return `
        <div class="role-card">
          <div class="role-card-name">${escapeHtml(role.name)}</div>
          <div class="role-card-desc">${escapeHtml(role.description || "No description")}</div>
          <div class="role-card-perms">${escapeHtml(String((role.permissions || []).length))} permissions</div>
        </div>`;
    }).join("") + `
      <div class="role-card" data-action="create-role" style="border-style:dashed;display:flex;align-items:center;justify-content:center;min-height:90px;">
        <div style="text-align:center;color:var(--admin-text-muted)">
          <div style="font-size:20px;margin-bottom:4px">+</div>
          <div style="font-size:12px;font-weight:600">Create Role</div>
        </div>
      </div>`;
  }

  function renderSodWarnings() {
    if (!els.sodWarnings) return;

    els.sodWarnings.innerHTML = state.sodConflicts.map(function (conflict) {
      return `
        <div class="sod-warn">
          <strong>${escapeHtml(conflict.role)}</strong>:
          <span>${escapeHtml(conflict.message)}</span>
        </div>`;
    }).join("");
  }

  function renderMatrix() {
    if (!els.theadRow || !els.tbody) return;

    els.theadRow.innerHTML = `<th>Permission</th>` + state.roles.map(function (role) {
      return `<th style="font-size:11px">${escapeHtml(role.name)}</th>`;
    }).join("");

    els.tbody.innerHTML = permissionCategories.map(function (category) {
      const categoryRow = `
        <tr class="perm-matrix-cat">
          <td colspan="${state.roles.length + 1}">${escapeHtml(category.icon + " " + category.name)}</td>
        </tr>`;

      const permissionRows = category.permissions.map(function (permission) {
        return `
          <tr>
            <td>
              <code style="font-size:12px;padding:2px 6px;background:var(--admin-code-bg);border-radius:3px;">${escapeHtml(permission.key)}</code>
              <div class="perm-desc">${escapeHtml(permission.description)}</div>
            </td>
            ${state.roles.map(function (role) {
              const disabled = role.name === "super_admin";
              return `
                <td>
                  <input type="checkbox" class="perm-check"
                    data-role-id="${escapeAttr(role.id)}"
                    data-permission="${escapeAttr(permission.key)}"
                    ${hasPermission(role, permission.key) ? "checked" : ""}
                    ${disabled ? "disabled" : ""}>
                </td>`;
            }).join("")}
          </tr>`;
      }).join("");

      return categoryRow + permissionRows;
    }).join("");
  }

  function renderCreateRoleModal() {
    if (!els.roleClone) return;

    els.roleClone.innerHTML = `<option value="">Start with no permissions</option>` + state.roles.map(function (role) {
      return `<option value="${escapeAttr(role.id)}">${escapeHtml(role.name)}</option>`;
    }).join("");
  }

  function renderSaveButton() {
    if (!els.saveButton) return;
    els.saveButton.disabled = state.saving;
    els.saveButton.textContent = state.saving ? "Saving..." : "Save Permissions";
  }

  function hasPermission(role, permissionKey) {
    if (role.name === "super_admin") return true;
    return (role.permissions || []).includes(permissionKey) || (role.permissions || []).includes("all");
  }

  function togglePermission(roleId, permissionKey) {
    const role = state.roles.find((item) => item.id === roleId);
    if (!role || role.name === "super_admin") return;

    if (!Array.isArray(role.permissions)) role.permissions = [];
    if (role.permissions.includes(permissionKey)) {
      role.permissions = role.permissions.filter((permission) => permission !== permissionKey);
    } else {
      role.permissions.push(permissionKey);
    }

    checkSodConflicts();
    renderSodWarnings();
  }

  function checkSodConflicts() {
    state.sodConflicts = [];

    state.roles.forEach(function (role) {
      if (role.name === "super_admin") return;

      sodRules.forEach(function (rule) {
        const permissions = role.permissions || [];
        if (permissions.includes(rule.perm1) && permissions.includes(rule.perm2)) {
          state.sodConflicts.push({
            role: role.name,
            message: rule.message,
            severity: rule.severity || "error",
          });
        }
      });
    });
  }

  async function saveMatrix() {
    if (state.isUsingFallback) {
      showToast("Cannot save - using demo data. Roles API is unavailable.", "error");
      return;
    }

    const hardConflicts = state.sodConflicts.filter((conflict) => conflict.severity !== "warning");
    if (hardConflicts.length > 0) {
      const message = hardConflicts.map((conflict) => `${conflict.role}: ${conflict.message}`).join("\n") + "\n\nSave anyway?";
      if (!await confirmAction({
        title: "Segregation of Duties Violations",
        message,
        confirmText: "Save Anyway",
        type: "danger",
      })) return;
    }

    state.saving = true;
    renderSaveButton();

    try {
      const resp = await fetch("/api/admin/roles/permissions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ roles: state.roles }),
      });

      if (resp.ok) {
        state.originalRoles = deepClone(state.roles);
        showToast("Permission matrix saved successfully", "success");
      } else {
        const err = await resp.json().catch(() => ({}));
        showToast("Failed to save: " + (err.error || "Server error"), "error");
      }
    } catch (error) {
      console.error("Error saving rbac matrix", error);
      captureException(error);
      showToast("Connection error - please try again", "error");
    } finally {
      state.saving = false;
      renderSaveButton();
    }
  }

  async function resetMatrix() {
    if (!await confirmAction({
      title: "Revert permission changes",
      message: "Revert all unsaved permission changes to the last saved state?",
      confirmText: "Revert",
      type: "warning",
    })) return;

    state.roles = deepClone(state.originalRoles);
    checkSodConflicts();
    render();
    showToast("Changes reverted", "success");
  }

  function openCreateRoleModal() {
    if (!els.modal) return;
    if (els.roleName) els.roleName.value = "";
    if (els.roleDescription) els.roleDescription.value = "";
    if (els.roleClone) els.roleClone.value = "";
    els.modal.classList.add("is-open");
  }

  function closeCreateRoleModal() {
    if (els.modal) els.modal.classList.remove("is-open");
  }

  async function createRole() {
    const name = els.roleName ? els.roleName.value.trim() : "";
    const description = els.roleDescription ? els.roleDescription.value.trim() : "";
    const cloneFrom = els.roleClone ? els.roleClone.value : "";

    if (!name) return;
    if (!/^[a-z][a-z0-9_]*$/.test(name)) {
      showToast("Role name must be snake_case (lowercase letters, numbers, underscores)", "error");
      return;
    }
    if (state.roles.some((role) => role.name === name)) {
      showToast("A role with this name already exists", "error");
      return;
    }

    const sourceRole = cloneFrom ? state.roles.find((role) => role.id === cloneFrom) : null;
    const permissions = sourceRole ? sourceRole.permissions.slice() : [];

    try {
      const resp = await fetch("/api/admin/roles", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, description, permissions }),
      });

      if (resp.ok) {
        showToast(`Role "${name}" created`, "success");
        closeCreateRoleModal();
        await loadData();
      } else {
        const err = await resp.json().catch(() => ({}));
        showToast("Failed to create role: " + (err.error || "Server error"), "error");
      }
    } catch (error) {
      console.error("Error creating role", error);
      captureException(error);
      showToast("Connection error - please try again", "error");
    }
  }

  function showToast(message, type) {
    if (!els.toast) return;
    els.toast.textContent = message;
    els.toast.style.background = type === "error" ? "var(--admin-danger)" : "var(--admin-success)";
    els.toast.style.color = "#fff";
    els.toast.classList.add("is-visible");

    window.clearTimeout(els.toast._hideTimer);
    els.toast._hideTimer = window.setTimeout(function () {
      els.toast.classList.remove("is-visible");
    }, 3500);
  }

  function confirmAction(options) {
    if (typeof window.pooolConfirm === "function") return window.pooolConfirm(options);
    return Promise.resolve(window.confirm(options.message || options.title || "Confirm action"));
  }

  function deepClone(value) {
    return JSON.parse(JSON.stringify(value));
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
