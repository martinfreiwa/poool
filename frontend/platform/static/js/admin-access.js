/**
 * Admin Access Control JS — Manages Admins, Roles, and Granular Permissions.
 * Used by both admins.html and roles.html
 */

document.addEventListener("alpine:init", () => {
  Alpine.data("adminAccess", () => ({
    admins: [],
    filteredAdmins: [],
    roles: [],
    availablePermissions: [],
    searchQuery: "",
    showInviteModal: false,
    showEditModal: false,
    editingAdmin: null,
    editForm: { role: "", status: "" },
    inviteForm: {
      email: "",
      role: "support",
    },

    async init() {
      await this.loadData();
      this.$watch("searchQuery", () => this.applyFilters());
    },

    async loadData() {
      try {
        const [adminsResp, rolesResp, permsResp] = await Promise.all([
          fetch("/api/admin/admins"),
          fetch("/api/admin/roles"),
          fetch("/api/admin/permissions"),
        ]);

        if (adminsResp.ok) this.admins = await adminsResp.json();
        if (rolesResp.ok) this.roles = await rolesResp.json();
        if (permsResp.ok) this.availablePermissions = await permsResp.json();

        this.applyFilters();
      } catch (e) {
        if (window.Sentry) Sentry.captureException(e);
      }
    },

    applyFilters() {
      if (!this.searchQuery) {
        this.filteredAdmins = this.admins;
        return;
      }
      const q = this.searchQuery.toLowerCase();
      this.filteredAdmins = this.admins.filter(
        (a) =>
          (a.email || "").toLowerCase().includes(q) ||
          (a.id || "").toLowerCase().includes(q) ||
          (a.name || "").toLowerCase().includes(q) ||
          (a.roles || []).some((r) => r.toLowerCase().includes(q)),
      );
    },

    // ─── Invite ────────────────────────────────────────
    async sendInvite() {
      if (!this.inviteForm.email) return alert("Email is required");

      try {
        const resp = await fetch("/api/admin/admins/invite", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(this.inviteForm),
        });

        if (resp.ok) {
          this.showToast("Invitation sent successfully");
          this.showInviteModal = false;
          this.inviteForm.email = "";
          await this.loadData();
        } else {
          const err = await resp.json();
          alert("Failed to send invite: " + (err.error || "Unknown error"));
        }
      } catch (e) {
        alert("Connection error");
      }
    },

    // ─── Edit Admin ────────────────────────────────────
    editAdmin(admin) {
      this.editingAdmin = admin;
      this.editForm.role = (admin.roles && admin.roles[0]) || "admin";
      this.editForm.status = admin.status || "active";
      this.showEditModal = true;
    },

    async saveAdminEdit() {
      if (!this.editingAdmin) return;

      try {
        // Update role
        const resp = await fetch(
          `/api/admin/settings/admins/${this.editingAdmin.id}`,
          {
            method: "PATCH",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ role: this.editForm.role }),
          },
        );

        if (resp.ok) {
          this.showToast("Admin role updated successfully");
          this.showEditModal = false;
          this.editingAdmin = null;
          await this.loadData();
        } else {
          const err = await resp.json();
          alert("Failed: " + (err.error || "Unknown error"));
        }
      } catch (e) {
        alert("Connection error");
      }
    },

    // ─── Remove Admin ──────────────────────────────────
    async removeAdmin(admin) {
      if (
        !await pooolConfirm({
          title: 'Remove admin privileges',
          message: `Remove admin privileges from ${admin.email}? They will lose all admin access immediately.`,
          confirmText: 'Remove',
          type: 'danger',
        })
      )
        return;

      try {
        const resp = await fetch(`/api/admin/settings/admins/${admin.id}`, {
          method: "DELETE",
        });

        if (resp.ok) {
          this.showToast("Admin privileges removed");
          await this.loadData();
        } else {
          alert("Failed to remove admin");
        }
      } catch (e) {
        alert("Connection error");
      }
    },

    // ─── Kill Sessions ─────────────────────────────────
    async killSessions(adminId) {
      if (
        !await pooolConfirm({
          title: 'Terminate all sessions',
          message: 'All sessions for this admin will be terminated and they will be logged out immediately.',
          confirmText: 'Terminate',
          type: 'danger',
        })
      )
        return;

      try {
        const resp = await fetch(`/api/admin/users/${adminId}/sessions`, {
          method: "DELETE",
        });

        if (resp.ok) {
          this.showToast("All sessions terminated");
          await this.loadData();
        } else {
          alert("Failed to terminate sessions");
        }
      } catch (e) {
        alert("Connection error");
      }
    },

    // ─── RBAC Matrix ───────────────────────────────────
    togglePermission(roleId, permission) {
      const role = this.roles.find((r) => r.id === roleId);
      if (!role) return;

      if (role.permissions.includes(permission)) {
        role.permissions = role.permissions.filter((p) => p !== permission);
      } else {
        role.permissions.push(permission);
      }
    },

    async saveMatrix() {
      try {
        const resp = await fetch("/api/admin/roles/permissions", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ roles: this.roles }),
        });

        if (resp.ok) {
          this.showToast("Permissions updated successfully");
        } else {
          const err = await resp.json();
          alert("Failed to save: " + (err.error || "Unknown error"));
        }
      } catch (e) {
        alert("Connection error");
      }
    },

    async resetMatrix() {
      if (await pooolConfirm({ title: 'Revert changes', message: 'Revert all unsaved permission changes?', confirmText: 'Revert', type: 'warning' })) {
        this.loadData();
      }
    },

    // ─── Helpers ───────────────────────────────────────
    getInitials(str) {
      if (!str) return "??";
      const parts = str.split(/[@\s.]+/);
      if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase();
      return str.substring(0, 2).toUpperCase();
    },

    getAvatarColor(email) {
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
    },

    formatDate(isoString) {
      if (!isoString) return "Never";
      const d = new Date(isoString);
      return d.toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    },

    getRoleColor(role) {
      const map = {
        super_admin: "#EF4444",
        admin: "#6366F1",
        compliance: "#F59E0B",
        finance: "#10B981",
        support: "#3B82F6",
      };
      return map[role] || "#8B5CF6";
    },

    getRoleLabel(role) {
      const map = {
        super_admin: "Super Admin",
        admin: "Admin",
        compliance: "Compliance",
        finance: "Finance",
        support: "Support",
      };
      return map[role] || role;
    },

    getPermDesc(perm) {
      const descs = {
        "users.view": "View user profiles and data",
        "users.edit": "Edit user details and status",
        "users.delete": "Permanently delete user accounts",
        "users.balance.adjust": "Adjust wallet balances",
        "pii.view": "Access personally identifiable information",
        "treasury.read": "View treasury and wallet balances",
        "treasury.write": "Modify treasury operations",
        "deposits.view": "View deposit records",
        "deposits.manage": "Confirm/reject deposits",
        "financials.payout.draft": "Draft dividend payouts",
        "financials.payout.approve": "Approve and execute payouts",
        "assets.view": "View asset listings",
        "assets.edit": "Edit asset details",
        "assets.publish": "Publish/unpublish from marketplace",
        "assets.freeze": "Freeze secondary trading",
        "submissions.review": "Review developer submissions",
        "submissions.approve": "Approve/reject submissions",
        "kyc.read": "View KYC verification status",
        "kyc.write": "Update KYC verification",
        "kyc.override": "Override KYC decisions",
        "aml.flag": "Flag suspicious activity",
        "sanctions.check": "Run sanctions screenings",
        "orders.view": "View order history",
        "orders.approve": "Approve pending orders",
        "orders.reject": "Reject orders",
        "orders.refund": "Process refunds",
        "support.view": "View support tickets",
        "support.reply": "Reply to tickets",
        "support.escalate": "Escalate tickets",
        "support.close": "Close resolved tickets",
        "notifications.send": "Send individual notifications",
        "notifications.broadcast": "Broadcast to all users",
        "emails.templates": "Manage email templates",
        "emails.campaigns": "Create and send campaigns",
        "admins.manage": "Invite/remove admin users",
        "roles.edit": "Modify role permissions",
        "settings.view": "View platform settings",
        "settings.edit": "Modify platform settings",
        "reports.view": "View analytics reports",
        "reports.export": "Export reports to CSV/JSON",
        "audit.view": "View audit trail",
        "system.health": "View system health status",
        all: "⚡ Full unrestricted access",
      };
      return descs[perm] || "";
    },

    showToast(msg) {
      const t = document.createElement("div");
      t.style.cssText =
        "position:fixed;bottom:24px;right:24px;z-index:9999;padding:12px 20px;border-radius:8px;font-size:13px;font-weight:500;background:var(--admin-success);color:#fff;box-shadow:0 4px 20px rgba(0,0,0,0.2);animation:admin-fadeIn 0.25s ease;";
      t.textContent = msg;
      document.body.appendChild(t);
      setTimeout(() => t.remove(), 3000);
    },
  }));
});
