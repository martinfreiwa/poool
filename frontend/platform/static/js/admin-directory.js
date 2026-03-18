/**
 * Admin Directory JS — Complete Identity Lifecycle Management.
 * Manages admin list, invitations, role editing, session control, security posture.
 */

document.addEventListener("alpine:init", () => {
  Alpine.data("adminDirectory", () => ({
    // State
    admins: [],
    filteredAdmins: [],
    roles: [],
    pendingInvites: [],
    searchQuery: "",
    filterRole: "",
    filterStatus: "",
    activeTab: "admins",

    // Stats
    stats: {
      total: 0,
      active: 0,
      suspended: 0,
      noMfa: 0,
      activeSessions: 0,
      pendingInvites: 0,
    },

    // Modals
    showInviteModal: false,
    showEditModal: false,
    editTarget: null,
    editForm: { roles: [] },
    inviteForm: {
      email: "",
      role: "",
      message: "",
    },

    // Toast
    toast: { show: false, message: "", type: "success" },

    async init() {
      await this.loadData();
      this.$watch("searchQuery", () => this.applyFilters());
      this.$watch("filterRole", () => this.applyFilters());
      this.$watch("filterStatus", () => this.applyFilters());
    },

    async loadData() {
      try {
        const [adminsResp, rolesResp] = await Promise.all([
          fetch("/api/admin/admins"),
          fetch("/api/admin/roles"),
        ]);

        if (adminsResp.ok) {
          this.admins = await adminsResp.json();
        } else {
          this.admins = [];
        }

        if (rolesResp.ok) {
          this.roles = await rolesResp.json();
        } else {
          // Fallback default roles
          this.roles = [
            { id: "1", name: "super_admin", permissions: ["all"] },
            {
              id: "2",
              name: "compliance_officer",
              permissions: [
                "kyc.read",
                "kyc.write",
                "kyc.override",
                "users.view",
                "treasury.read",
              ],
            },
            {
              id: "3",
              name: "support_agent",
              permissions: ["users.view", "support.read", "support.write"],
            },
            {
              id: "4",
              name: "finance_admin",
              permissions: [
                "treasury.read",
                "treasury.write",
                "financials.payout.draft",
              ],
            },
            {
              id: "5",
              name: "auditor_read_only",
              permissions: [
                "users.view",
                "treasury.read",
                "kyc.read",
                "audit.read",
              ],
            },
          ];
        }

        // Set default invite role to first available role
        if (this.roles.length && !this.inviteForm.role) {
          this.inviteForm.role = this.roles[0].name;
        }

        // Try loading pending invites
        try {
          const invResp = await fetch("/api/admin/admins/invitations");
          if (invResp.ok) {
            this.pendingInvites = await invResp.json();
          }
        } catch (e) {
          /* Silent fail — endpoint might not exist yet */
        }

        this.computeStats();
        this.applyFilters();
      } catch (e) {
        console.error("[AdminDirectory] loadData error:", e);
        if (typeof Sentry !== 'undefined') Sentry.captureException(e);
      }
    },

    computeStats() {
      this.stats.total = this.admins.length;
      this.stats.active = this.admins.filter(
        (a) => a.status === "active",
      ).length;
      this.stats.suspended = this.admins.filter(
        (a) => a.status === "suspended",
      ).length;
      this.stats.noMfa = this.admins.filter((a) => !a.totp_enabled).length;
      this.stats.pendingInvites = this.pendingInvites.length;

      // Count active sessions (sum of session counts per admin; fallback to counting active admins)
      this.stats.activeSessions = this.admins.reduce(
        (sum, a) => sum + (a.session_count || (a.status === "active" ? 1 : 0)),
        0,
      );
    },

    applyFilters() {
      let result = this.admins;

      if (this.searchQuery) {
        const q = this.searchQuery.toLowerCase();
        result = result.filter(
          (a) =>
            a.email.toLowerCase().includes(q) ||
            (a.first_name || "").toLowerCase().includes(q) ||
            (a.last_name || "").toLowerCase().includes(q) ||
            a.id.toLowerCase().includes(q) ||
            (a.roles || []).some((r) => r.toLowerCase().includes(q)),
        );
      }

      if (this.filterRole) {
        result = result.filter((a) =>
          (a.roles || []).includes(this.filterRole),
        );
      }

      if (this.filterStatus) {
        result = result.filter((a) => a.status === this.filterStatus);
      }

      this.filteredAdmins = result;
    },

    // ── Invite ──
    async sendInvite() {
      if (!this.inviteForm.email) return;

      try {
        const resp = await fetch("/api/admin/admins/invite", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(this.inviteForm),
        });

        if (resp.ok) {
          this.showToast("Invitation sent to " + this.inviteForm.email, "success");
          this.showInviteModal = false;
          this.inviteForm = { email: "", role: this.roles.length ? this.roles[0].name : "", message: "" };
          await this.loadData();
        } else {
          const err = await resp.json().catch(() => ({}));
          this.showToast("Failed: " + (err.error || "Unknown error"), "error");
        }
      } catch (e) {
        console.error("Error sending invite", e);
        if (typeof Sentry !== 'undefined') Sentry.captureException(e);
        this.showToast("Connection error — please try again", "error");
      }
    },

    async resendInvite(inviteId) {
      try {
        const resp = await fetch(
          `/api/admin/admins/invitations/${inviteId}/resend`,
          { method: "POST" },
        );
        if (resp.ok) {
          this.showToast("Invitation resent", "success");
        } else {
          this.showToast("Failed to resend invitation", "error");
        }
      } catch (e) {
        console.error("Error resending invite", e);
        if (typeof Sentry !== 'undefined') Sentry.captureException(e);
        this.showToast("Connection error", "error");
      }
    },

    async revokeInvite(inviteId) {
      if (
        !await pooolConfirm({ title: 'Revoke invitation', message: 'The invite link will no longer work.', confirmText: 'Revoke', type: 'danger' })
      )
        return;

      try {
        const resp = await fetch(`/api/admin/admins/invitations/${inviteId}`, {
          method: "DELETE",
        });
        if (resp.ok) {
          this.pendingInvites = this.pendingInvites.filter(
            (i) => i.id !== inviteId,
          );
          this.stats.pendingInvites = this.pendingInvites.length;
          this.showToast("Invitation revoked", "success");
        } else {
          this.showToast("Failed to revoke invitation", "error");
        }
      } catch (e) {
        console.error("Error revoking invite", e);
        if (typeof Sentry !== 'undefined') Sentry.captureException(e);
        this.showToast("Connection error", "error");
      }
    },

    // ── Edit Admin ──
    editAdmin(admin) {
      this.editTarget = admin;
      this.editForm.roles = [...(admin.roles || [])];
      this.showEditModal = true;
    },

    toggleEditRole(roleName) {
      if (this.editForm.roles.includes(roleName)) {
        this.editForm.roles = this.editForm.roles.filter((r) => r !== roleName);
      } else {
        this.editForm.roles.push(roleName);
      }
    },

    async saveAdminEdit() {
      if (!this.editTarget) return;

      try {
        const resp = await fetch(
          `/api/admin/users/${this.editTarget.id}/roles`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ roles: this.editForm.roles }),
          },
        );

        if (resp.ok) {
          this.showToast("Admin roles updated", "success");
          this.showEditModal = false;
          await this.loadData();
        } else {
          const err = await resp.json().catch(() => ({}));
          this.showToast("Failed: " + (err.error || "Unknown error"), "error");
        }
      } catch (e) {
        console.error("Error saving admin edit", e);
        if (typeof Sentry !== 'undefined') Sentry.captureException(e);
        this.showToast("Connection error", "error");
      }
    },

    // ── Status Toggle ──
    async toggleStatus(admin) {
      const newStatus = admin.status === "active" ? "suspended" : "active";
      const action = newStatus === "suspended" ? "SUSPEND" : "ACTIVATE";

      if (
        !await pooolConfirm({
          title: `${action} admin`,
          message: `${admin.email}${newStatus === 'suspended' ? ' — all active sessions will be revoked immediately.' : ' — access will be restored.'}`,
          confirmText: action,
          type: newStatus === 'suspended' ? 'danger' : 'success',
        })
      )
        return;

      try {
        const resp = await fetch(`/api/admin/users/${admin.id}/status`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ status: newStatus }),
        });

        if (resp.ok) {
          this.showToast(
            `Admin ${action.toLowerCase()}d: ${admin.email}`,
            "success",
          );
          await this.loadData();
        } else {
          this.showToast(`Failed to ${action.toLowerCase()} admin`, "error");
        }
      } catch (e) {
        console.error("Error toggling admin status", e);
        if (typeof Sentry !== 'undefined') Sentry.captureException(e);
        this.showToast("Connection error", "error");
      }
    },

    // ── Session Kill ──
    async killSessions(adminId) {
      if (
        !await pooolConfirm({ title: 'Terminate all sessions', message: 'This admin will be immediately logged out of all devices.', confirmText: 'Terminate', type: 'danger' })
      )
        return;

      try {
        const resp = await fetch(`/api/admin/users/${adminId}/sessions`, {
          method: "DELETE",
        });

        if (resp.ok) {
          this.showToast("All sessions terminated", "success");
          await this.loadData();
        } else {
          this.showToast("Failed to terminate sessions", "error");
        }
      } catch (e) {
        console.error("Error killing sessions", e);
        if (typeof Sentry !== 'undefined') Sentry.captureException(e);
        this.showToast("Connection error", "error");
      }
    },

    // ── Force Password Reset ──
    async forcePasswordReset(adminId) {
      if (!await pooolConfirm({ title: 'Force password reset', message: 'This admin will be required to reset their password on next login.', confirmText: 'Force Reset', type: 'warning' }))
        return;

      try {
        const resp = await fetch(
          `/api/admin/users/${adminId}/force-password-reset`,
          { method: "POST" },
        );

        if (resp.ok) {
          this.showToast("Password reset forced on next login", "success");
        } else {
          this.showToast("Failed to force password reset", "error");
        }
      } catch (e) {
        console.error("Error forcing password reset", e);
        if (typeof Sentry !== 'undefined') Sentry.captureException(e);
        this.showToast("Connection error", "error");
      }
    },

    // ── Helpers ──
    getInitials(email) {
      if (!email) return "??";
      const parts = email.split("@")[0].split(/[._-]/);
      if (parts.length >= 2) {
        return (parts[0][0] + parts[1][0]).toUpperCase();
      }
      return email.substring(0, 2).toUpperCase();
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

    getRoleBadgeClass(role) {
      switch (role) {
        case "super_admin":
          return "admin-badge--danger";
        case "admin":
          return "admin-badge--danger";
        case "compliance":
        case "compliance_officer":
          return "admin-badge--warning";
        case "finance":
        case "finance_admin":
          return "admin-badge--info";
        case "auditor_read_only":
          return "admin-badge--neutral";
        case "support":
        case "support_agent":
          return "admin-badge--success";
        case "developer":
          return "admin-badge--neutral";
        default:
          return "admin-badge--neutral";
      }
    },

    formatDate(isoString) {
      if (!isoString) return "Never";
      const d = new Date(isoString);
      const now = new Date();
      const diffMs = now - d;
      const diffHours = Math.floor(diffMs / (1000 * 60 * 60));

      if (diffHours < 1) return "Just now";
      if (diffHours < 24) return diffHours + "h ago";
      if (diffHours < 48) return "Yesterday";
      return d.toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        year: d.getFullYear() !== now.getFullYear() ? "numeric" : undefined,
      });
    },

    showToast(message, type = "success") {
      this.toast = { show: true, message, type };
      setTimeout(() => {
        this.toast.show = false;
      }, 3500);
    },
  }));
});
