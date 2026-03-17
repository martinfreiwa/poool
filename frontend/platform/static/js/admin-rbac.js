/**
 * Admin RBAC Matrix JS — Granular Permission Management with SoD Detection.
 * Implements Zero Trust Architecture permission controls.
 */

document.addEventListener("alpine:init", () => {
  Alpine.data("rbacMatrix", () => ({
    roles: [],
    originalRoles: [], // For reset
    isUsingFallback: false, // True when using hardcoded demo data (API failed)
    selectedRole: null,
    sodConflicts: [],
    saving: false,
    showCreateRoleModal: false,

    // Toast
    toast: { show: false, message: "", type: "success" },

    // New role form
    newRoleForm: {
      name: "",
      description: "",
      cloneFrom: "",
    },

    // ── Permission Categories (Source of Truth) ──
    permissionCategories: [
      {
        name: "User Management",
        icon: "👥",
        permissions: [
          {
            key: "users.view",
            description: "View user profiles and directory",
          },
          { key: "users.edit", description: "Edit user profiles and settings" },
          {
            key: "users.delete",
            description: "Delete or deactivate user accounts",
          },
          {
            key: "users.suspend",
            description: "Suspend/activate user accounts",
          },
          {
            key: "pii.view",
            description: "View unmasked PII (full email, phone, tax ID)",
          },
          {
            key: "users.impersonate",
            description: "Impersonate a user session (super_admin only)",
          },
        ],
      },
      {
        name: "KYC & Compliance",
        icon: "🛡️",
        permissions: [
          {
            key: "kyc.read",
            description: "View KYC records and verification status",
          },
          {
            key: "kyc.write",
            description: "Process KYC reviews (approve/reject)",
          },
          {
            key: "kyc.override",
            description: "Override KYC status outside normal flow",
          },
          {
            key: "aml.read",
            description: "View AML/PEP/sanctions check results",
          },
          {
            key: "aml.escalate",
            description: "Escalate cases for enhanced due diligence",
          },
        ],
      },
      {
        name: "Financial Operations",
        icon: "💰",
        permissions: [
          {
            key: "treasury.read",
            description: "View ledger, balances, and transaction history",
          },
          {
            key: "treasury.write",
            description: "Create manual wallet adjustments",
          },
          {
            key: "deposits.confirm",
            description: "Manually confirm pending deposits",
          },
          {
            key: "withdrawals.approve",
            description: "Approve pending withdrawal requests",
          },
          {
            key: "financials.payout.draft",
            description: "Draft dividend payout distributions",
          },
          {
            key: "financials.payout.approve",
            description: "Execute/approve dividend payouts (Four-Eyes)",
          },
          {
            key: "invoices.manage",
            description: "Issue, void, and reissue invoices",
          },
          {
            key: "fees.configure",
            description: "Modify platform fee percentages",
          },
        ],
      },
      {
        name: "Asset Management",
        icon: "🏠",
        permissions: [
          {
            key: "assets.view",
            description: "View all asset details and financials",
          },
          {
            key: "assets.edit",
            description: "Edit asset properties and status",
          },
          {
            key: "assets.publish",
            description: "Publish/unpublish assets on marketplace",
          },
          {
            key: "assets.feature",
            description: "Toggle featured status for assets",
          },
          {
            key: "submissions.review",
            description: "Review developer asset submissions",
          },
          {
            key: "submissions.approve",
            description: "Approve/reject submissions for listing",
          },
          {
            key: "assets.refund",
            description: "Force-refund an asset (reverses all investments)",
          },
        ],
      },
      {
        name: "Orders & Investments",
        icon: "📦",
        permissions: [
          {
            key: "orders.view",
            description: "View all orders and order items",
          },
          {
            key: "orders.cancel",
            description: "Cancel orders and process refunds",
          },
          {
            key: "investments.view",
            description: "View investment cap tables",
          },
          {
            key: "investments.adjust",
            description: "Manual investment adjustments",
          },
        ],
      },
      {
        name: "Support",
        icon: "💬",
        permissions: [
          { key: "support.read", description: "View support tickets" },
          {
            key: "support.write",
            description: "Reply to and manage support tickets",
          },
          {
            key: "support.assign",
            description: "Assign tickets to other admins",
          },
          {
            key: "support.escalate",
            description: "Escalate tickets to higher priority",
          },
        ],
      },
      {
        name: "Rewards & Referrals",
        icon: "⭐",
        permissions: [
          {
            key: "rewards.view",
            description: "View rewards balances and tiers",
          },
          {
            key: "rewards.adjust",
            description: "Manual credit/debit rewards balances",
          },
          {
            key: "tiers.configure",
            description: "Edit tier thresholds and cashback rates",
          },
          {
            key: "referrals.manage",
            description: "Qualify/flag referrals, manage program",
          },
        ],
      },
      {
        name: "Notifications & Email",
        icon: "📧",
        permissions: [
          {
            key: "notifications.send",
            description: "Send individual notifications",
          },
          {
            key: "notifications.broadcast",
            description: "Broadcast notifications to all users",
          },
          { key: "email.templates", description: "Edit email templates" },
          {
            key: "email.campaigns",
            description: "Create and send marketing campaigns",
          },
        ],
      },
      {
        name: "System Administration",
        icon: "⚙️",
        permissions: [
          {
            key: "admins.manage",
            description: "Invite, edit, and suspend admin accounts",
          },
          {
            key: "roles.view",
            description: "View role definitions and permission matrix",
          },
          {
            key: "roles.edit",
            description: "Modify role permissions (dangerous)",
          },
          { key: "settings.view", description: "View platform configuration" },
          {
            key: "settings.edit",
            description: "Modify platform configuration",
          },
          { key: "audit.read", description: "Read audit logs for compliance" },
          {
            key: "system.health",
            description: "View system health, jobs, and webhooks",
          },
          {
            key: "reports.generate",
            description: "Generate and export compliance reports",
          },
          {
            key: "all",
            description: "Wildcard — grants all current & future permissions",
          },
        ],
      },
    ],

    // ── SoD Conflict Rules ──
    // Pairs of permissions that should NOT be held by the same role
    sodRules: [
      {
        perm1: "financials.payout.draft",
        perm2: "financials.payout.approve",
        message:
          "Cannot both Draft AND Approve financial payouts (Four-Eyes Principle violation)",
      },
      {
        perm1: "roles.edit",
        perm2: "audit.read",
        message:
          "Editing roles and reading audit logs in the same role creates a conflict of interest",
        severity: "warning", // Not a hard block
      },
      {
        perm1: "users.delete",
        perm2: "audit.read",
        message:
          "Deleting users and reading audit logs in the same non-super role creates oversight gaps",
        severity: "warning",
      },
    ],

    async init() {
      await this.loadData();
    },

    async loadData() {
      try {
        const rolesResp = await fetch("/api/admin/roles");

        if (rolesResp.ok) {
          this.roles = await rolesResp.json();
          this.isUsingFallback = false;
        } else {
          // Fallback default roles for UI demonstration — READ ONLY
          this.isUsingFallback = true;
          this.roles = [
            {
              id: "1",
              name: "super_admin",
              description: "Full system access — godmode",
              admin_count: 1,
              permissions: ["all"],
            },
            {
              id: "2",
              name: "compliance_officer",
              description: "KYC/AML review and compliance operations",
              admin_count: 0,
              permissions: [
                "users.view",
                "pii.view",
                "kyc.read",
                "kyc.write",
                "kyc.override",
                "aml.read",
                "aml.escalate",
                "treasury.read",
                "audit.read",
              ],
            },
            {
              id: "3",
              name: "support_agent",
              description: "Support tickets and basic user lookup",
              admin_count: 0,
              permissions: [
                "users.view",
                "support.read",
                "support.write",
                "support.assign",
                "notifications.send",
              ],
            },
            {
              id: "4",
              name: "finance_admin",
              description: "Treasury read, draft payouts, manage invoices",
              admin_count: 0,
              permissions: [
                "treasury.read",
                "treasury.write",
                "deposits.confirm",
                "financials.payout.draft",
                "invoices.manage",
                "orders.view",
                "investments.view",
              ],
            },
            {
              id: "5",
              name: "auditor_read_only",
              description: "View-only access for external regulators/auditors",
              admin_count: 0,
              permissions: [
                "users.view",
                "kyc.read",
                "aml.read",
                "treasury.read",
                "orders.view",
                "investments.view",
                "audit.read",
                "reports.generate",
                "rewards.view",
                "assets.view",
              ],
            },
          ];
          this.showToast("⚠️ Using demo roles — API unavailable. Changes cannot be saved.", "error");
        }

        // Deep clone for reset
        this.originalRoles = JSON.parse(JSON.stringify(this.roles));
        this.checkSodConflicts();
      } catch (e) {
        console.error("Error loading rbac data", e);
        if (typeof Sentry !== 'undefined') Sentry.captureException(e);
        this.isUsingFallback = true;
        this.showToast("⚠️ Failed to load roles. Changes cannot be saved.", "error");
      }
    },

    // ── Permissions ──
    hasPermission(role, permKey) {
      if (role.name === "super_admin") return true;
      return (
        (role.permissions || []).includes(permKey) ||
        (role.permissions || []).includes("all")
      );
    },

    togglePermission(roleId, permKey) {
      const role = this.roles.find((r) => r.id === roleId);
      if (!role || role.name === "super_admin") return;

      if (role.permissions.includes(permKey)) {
        role.permissions = role.permissions.filter((p) => p !== permKey);
      } else {
        role.permissions.push(permKey);
      }

      this.checkSodConflicts();
    },

    // ── SoD Conflict Detection ──
    checkSodConflicts() {
      this.sodConflicts = [];

      for (const role of this.roles) {
        if (role.name === "super_admin") continue; // Super admin is exempt

        for (const rule of this.sodRules) {
          const hasPerm1 = role.permissions.includes(rule.perm1);
          const hasPerm2 = role.permissions.includes(rule.perm2);

          if (hasPerm1 && hasPerm2) {
            this.sodConflicts.push({
              role: role.name,
              message: rule.message,
              severity: rule.severity || "error",
            });
          }
        }
      }
    },

    // ── Save / Reset ──
    async saveMatrix() {
      // Block save when using fallback data
      if (this.isUsingFallback) {
        this.showToast("Cannot save — using demo data. Roles API is unavailable.", "error");
        return;
      }

      // Check for hard SoD violations
      const hardConflicts = this.sodConflicts.filter(
        (c) => c.severity !== "warning",
      );
      if (hardConflicts.length > 0) {
        if (
          !confirm(
            "⚠️ There are Segregation of Duties violations:\n\n" +
            hardConflicts
              .map((c) => "• " + c.role + ": " + c.message)
              .join("\n") +
            "\n\nSave anyway?",
          )
        ) {
          return;
        }
      }

      this.saving = true;

      try {
        const resp = await fetch("/api/admin/roles/permissions", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ roles: this.roles }),
        });

        if (resp.ok) {
          this.originalRoles = JSON.parse(JSON.stringify(this.roles));
          this.showToast("Permission matrix saved successfully", "success");
        } else {
          const err = await resp.json().catch(() => ({}));
          this.showToast(
            "Failed to save: " + (err.error || "Server error"),
            "error",
          );
        }
      } catch (e) {
        this.showToast("Connection error — please try again", "error");
      } finally {
        this.saving = false;
      }
    },

    resetMatrix() {
      if (
        !confirm(
          "Revert all unsaved permission changes to the last saved state?",
        )
      )
        return;
      this.roles = JSON.parse(JSON.stringify(this.originalRoles));
      this.checkSodConflicts();
      this.showToast("Changes reverted", "success");
    },

    // ── Create Role ──
    async createRole() {
      if (!this.newRoleForm.name) return;

      // Validate name format
      if (!/^[a-z][a-z0-9_]*$/.test(this.newRoleForm.name)) {
        this.showToast(
          "Role name must be snake_case (lowercase letters, numbers, underscores)",
          "error",
        );
        return;
      }

      // Check uniqueness
      if (this.roles.some((r) => r.name === this.newRoleForm.name)) {
        this.showToast("A role with this name already exists", "error");
        return;
      }

      // Clone permissions if specified
      let permissions = [];
      if (this.newRoleForm.cloneFrom) {
        const sourceRole = this.roles.find(
          (r) => r.id === this.newRoleForm.cloneFrom,
        );
        if (sourceRole) {
          permissions = [...sourceRole.permissions];
        }
      }

      try {
        const resp = await fetch("/api/admin/roles", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            name: this.newRoleForm.name,
            description: this.newRoleForm.description,
            permissions: permissions,
          }),
        });

        if (resp.ok) {
          this.showToast(
            'Role "' + this.newRoleForm.name + '" created',
            "success",
          );
          this.showCreateRoleModal = false;
          this.newRoleForm = { name: "", description: "", cloneFrom: "" };
          await this.loadData();
        } else {
          // Fallback: add locally for UI demonstration
          this.roles.push({
            id: "new_" + Date.now(),
            name: this.newRoleForm.name,
            description: this.newRoleForm.description,
            admin_count: 0,
            permissions: permissions,
          });
          this.originalRoles = JSON.parse(JSON.stringify(this.roles));
          this.showToast(
            'Role "' + this.newRoleForm.name + '" created (local)',
            "success",
          );
          this.showCreateRoleModal = false;
          this.newRoleForm = { name: "", description: "", cloneFrom: "" };
          this.checkSodConflicts();
        }
      } catch (e) {
        // Fallback: add locally
        this.roles.push({
          id: "new_" + Date.now(),
          name: this.newRoleForm.name,
          description: this.newRoleForm.description,
          admin_count: 0,
          permissions: permissions,
        });
        this.originalRoles = JSON.parse(JSON.stringify(this.roles));
        this.showToast(
          'Role "' + this.newRoleForm.name + '" created (local)',
          "success",
        );
        this.showCreateRoleModal = false;
        this.newRoleForm = { name: "", description: "", cloneFrom: "" };
        this.checkSodConflicts();
      }
    },

    showToast(message, type = "success") {
      this.toast = { show: true, message, type };
      setTimeout(() => {
        this.toast.show = false;
      }, 3500);
    },
  }));
});
