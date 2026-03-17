/**
 * Admin Settings & RBAC JS
 * Wired to backend APIs:
 *   GET  /api/admin/settings          — load settings + admin list
 *   POST /api/admin/settings          — save platform settings
 *   POST /api/admin/settings/admins   — add admin
 *   DELETE /api/admin/settings/admins/:id — remove admin
 *   GET  /api/admin/settings/roles    — list available roles
 *   POST /api/admin/settings/maintenance — toggle maintenance mode
 *   POST /api/admin/notifications/broadcast — send broadcast
 */

document.addEventListener("alpine:init", () => {
  Alpine.data("adminSettings", () => ({
    activeTab: "general",
    saving: false,
    broadcasting: false,
    toast: { show: false, message: "", type: "success" },

    settings: {
      platform_name: "POOOL Finance",
      support_email: "support@poool.finance",
      enable_registrations: true,
      require_kyc: true,
      platform_fee_percent: 2.5,
      withdrawal_fee_cents: 5.0,
      referral_commission_percent: 1.0,
      min_withdrawal_cents: 10.0,
      maintenance_mode: false,
      resend_api_key: "",
    },

    admins: [],
    adminPage: 1,
    adminPageSize: 10,
    availableRoles: [],

    get paginatedAdmins() {
      const start = (this.adminPage - 1) * this.adminPageSize;
      return this.admins.slice(start, start + this.adminPageSize);
    },

    newAdmin: { email: "", role: "admin", id: null },
    isEditingAdmin: false,

    broadcast: { title: "", message: "", type: "system" },

    // ─── KYC Provider Configuration ───────────────────
    kycProvider: {
      name: "manual",
      supports_redirect: false,
      loading: true,
      testing: false,
      testResult: null,
    },

    async init() {
      await Promise.all([
        this.loadSettings(),
        this.loadRoles(),
        this.loadKycProvider(),
      ]);
    },

    showToast(message, type = "success") {
      this.toast = { show: true, message, type };
      setTimeout(() => {
        this.toast.show = false;
      }, 3000);
    },

    // ─── Admin Management ─────────────────────────────
    editAdmin(admin) {
      this.isEditingAdmin = true;
      this.newAdmin = {
        id: admin.id,
        email: admin.email,
        role: admin.role,
      };
      // Scroll to the form or just highlight it
      const form = document.querySelector(
        '.admin-input[x-model="newAdmin.email"]',
      );
      if (form) form.focus();
    },

    cancelEditAdmin() {
      this.isEditingAdmin = false;
      this.newAdmin = { email: "", role: "admin", id: null };
    },

    // ─── Maintenance Utils ────────────────────────────
    async clearCache() {
      if (!confirm("Are you sure you want to clear the system cache?")) return;
      try {
        const resp = await fetch("/api/admin/maintenance/clear-cache", {
          method: "POST",
        });
        if (resp.ok) {
          const data = await resp.json();
          this.showToast(data.message || "Cache cleared");
        } else {
          this.showToast("Failed to clear cache", "error");
        }
      } catch (e) {
        this.showToast("Network error", "error");
      }
    },

    async runLogRotation() {
      if (!confirm("Trigger log rotation now?")) return;
      try {
        const resp = await fetch("/api/admin/maintenance/rotate-logs", {
          method: "POST",
        });
        if (resp.ok) {
          const data = await resp.json();
          this.showToast(data.message || "Log rotation initiated");
        } else {
          this.showToast("Failed to rotate logs", "error");
        }
      } catch (e) {
        this.showToast("Network error", "error");
      }
    },

    // ─── Load Settings from DB ─────────────────────────
    async loadSettings() {
      try {
        const resp = await fetch("/api/admin/settings");
        if (resp.ok) {
          const data = await resp.json();
          if (data.settings) {
            // Merge DB values over defaults
            for (const [k, v] of Object.entries(data.settings)) {
              this.settings[k] = v;
            }
          }
          if (data.admins && data.admins.length) {
            this.admins = data.admins;
          }
        }
      } catch (e) {
        console.error('Failed to load admin settings:', e);
        if (window.Sentry) Sentry.captureException(e);
      }
    },

    // ─── Load Available Roles ──────────────────────────
    async loadRoles() {
      try {
        const resp = await fetch("/api/admin/settings/roles");
        if (resp.ok) {
          const data = await resp.json();
          this.availableRoles = data.roles || [];
          if (this.availableRoles.length && !this.newAdmin.role) {
            this.newAdmin.role = this.availableRoles[0].name;
          }
        } else {
          console.error('Admin roles API error:', resp.status);
        }
      } catch (e) {
        console.error('Failed to load admin roles:', e);
        if (window.Sentry) Sentry.captureException(e);
      }
    },

    // ─── Save Platform Settings ────────────────────────
    async saveSettings() {
      this.saving = true;
      try {
        const resp = await fetch("/api/admin/settings", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(this.settings),
        });
        if (resp.ok) {
          this.showToast("Settings saved successfully");
        } else {
          const err = await resp.json().catch(() => ({}));
          this.showToast(err.error || "Failed to save settings", "error");
        }
      } catch (e) {
        this.showToast("Network error — settings not saved", "error");
      } finally {
        this.saving = false;
      }
    },

    // ─── Toggle Maintenance Mode ───────────────────────
    async toggleMaintenance() {
      const newState = !this.settings.maintenance_mode;
      try {
        const resp = await fetch("/api/admin/settings/maintenance", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: newState }),
        });
        if (resp.ok) {
          this.settings.maintenance_mode = newState;
          this.showToast(
            `Maintenance mode ${newState ? "enabled" : "disabled"}`,
          );
        } else {
          this.showToast("Failed to toggle maintenance mode", "error");
        }
      } catch (e) {
        this.showToast("Network error", "error");
      }
    },

    // ─── Add or Update Admin User ─────────────────────
    async addAdmin() {
      if (!this.newAdmin.email.trim()) {
        this.showToast("Please enter an email address", "error");
        return;
      }

      if (this.isEditingAdmin) {
        // UPDATE
        try {
          const resp = await fetch(
            `/api/admin/settings/admins/${this.newAdmin.id}`,
            {
              method: "PATCH",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ role: this.newAdmin.role }),
            },
          );
          if (resp.ok) {
            this.showToast(`Updated role for ${this.newAdmin.email}`);
            this.cancelEditAdmin();
            await this.loadSettings();
          } else {
            const err = await resp.json().catch(() => ({}));
            this.showToast(err.error || "Failed to update admin", "error");
          }
        } catch (e) {
          this.showToast("Network error", "error");
        }
      } else {
        // ADD
        try {
          const resp = await fetch("/api/admin/settings/admins", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(this.newAdmin),
          });
          if (resp.ok) {
            this.showToast(
              `Added ${this.newAdmin.email} as ${this.newAdmin.role}`,
            );
            this.newAdmin.email = "";
            await this.loadSettings(); // Refresh admin list
          } else {
            const err = await resp.json().catch(() => ({}));
            this.showToast(err.error || "Failed to add admin", "error");
          }
        } catch (e) {
          this.showToast("Network error", "error");
        }
      }
    },

    // ─── Remove Admin User ────────────────────────────
    async removeAdmin(userId) {
      if (!confirm("Remove admin role from this user?")) return;
      try {
        const resp = await fetch(`/api/admin/settings/admins/${userId}`, {
          method: "DELETE",
        });
        if (resp.ok) {
          this.admins = this.admins.filter((a) => a.id !== userId);
          this.showToast("Admin role removed");
        } else {
          const err = await resp.json().catch(() => ({}));
          this.showToast(err.error || "Failed to remove admin", "error");
        }
      } catch (e) {
        this.showToast("Network error", "error");
      }
    },

    // ─── Send Broadcast Notification ──────────────────
    async sendBroadcast() {
      if (!this.broadcast.title.trim()) {
        this.showToast("Notification title is required", "error");
        return;
      }
      this.broadcasting = true;
      try {
        const resp = await fetch("/api/admin/notifications/broadcast", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(this.broadcast),
        });
        if (resp.ok) {
          const data = await resp.json();
          const count = data.count || 0;
          this.showToast(`Broadcast sent to ${count} users`);
          this.broadcast = { title: "", message: "", type: "system" };
        } else {
          const err = await resp.json().catch(() => ({}));
          this.showToast(err.error || "Failed to send broadcast", "error");
        }
      } catch (e) {
        this.showToast("Network error", "error");
      } finally {
        this.broadcasting = false;
      }
    },

    // ─── KYC Provider Detection ───────────────────────
    async loadKycProvider() {
      this.kycProvider.loading = true;
      try {
        const resp = await fetch("/api/kyc/provider");
        if (resp.ok) {
          const data = await resp.json();
          this.kycProvider.name = data.provider || "manual";
          this.kycProvider.supports_redirect = data.supports_redirect || false;
        }
      } catch (e) {
      } finally {
        this.kycProvider.loading = false;
      }
    },

    /** Readable label for the active KYC provider. */
    getKycProviderLabel() {
      const labels = {
        didit: "Didit.me",
        sumsub: "Sumsub",
        manual: "Manual Review",
      };
      return labels[this.kycProvider.name] || this.kycProvider.name;
    },

    /** Status class for the provider badge. */
    getKycStatusClass() {
      if (this.kycProvider.name === "manual") return "kyc-status--manual";
      return "kyc-status--active";
    },

    /** Test the KYC provider connection. */
    async testKycProvider() {
      this.kycProvider.testing = true;
      this.kycProvider.testResult = null;
      try {
        // Try to hit the provider info endpoint to verify configuration
        const resp = await fetch("/api/kyc/provider");
        if (resp.ok) {
          const data = await resp.json();
          if (data.provider && data.provider !== "manual") {
            this.kycProvider.testResult = {
              success: true,
              message: `${this.getKycProviderLabel()} is configured and responding.`,
            };
          } else {
            this.kycProvider.testResult = {
              success: false,
              message:
                "No external KYC provider configured. Using manual review.",
            };
          }
        } else {
          this.kycProvider.testResult = {
            success: false,
            message: "Failed to reach KYC provider endpoint.",
          };
        }
      } catch (e) {
        this.kycProvider.testResult = {
          success: false,
          message: "Network error testing KYC provider.",
        };
      } finally {
        this.kycProvider.testing = false;
      }
    },

    // ─── Legal Document Versioning ───────────────────────
    legalStats: null,
    savingLegal: false,
    legalForm: {
      legal_terms_version: "1.0",
      legal_privacy_version: "1.0",
      legal_last_updated: new Date().toISOString().split("T")[0],
    },

    async loadLegalStatus() {
      try {
        const resp = await fetch("/api/admin/legal/version");
        if (resp.ok) {
          const data = await resp.json();
          this.legalStats = data;
          // Pre-populate form with current values
          if (data.settings) {
            this.legalForm.legal_terms_version =
              data.settings.legal_terms_version || "1.0";
            this.legalForm.legal_privacy_version =
              data.settings.legal_privacy_version || "1.0";
            this.legalForm.legal_last_updated =
              data.settings.legal_last_updated ||
              new Date().toISOString().split("T")[0];
          }
        }
      } catch (e) {
        console.error('Failed to load legal status:', e);
        if (window.Sentry) Sentry.captureException(e);
      }
    },

    async saveLegalVersion() {
      this.savingLegal = true;
      try {
        const resp = await fetch("/api/admin/legal/version", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(this.legalForm),
        });
        if (resp.ok) {
          const data = await resp.json();
          this.showToast(
            "Legal versions updated — users will be prompted to re-accept",
          );
          await this.loadLegalStatus(); // Refresh stats
        } else {
          const err = await resp.json().catch(() => ({}));
          this.showToast(
            err.error || "Failed to update legal versions",
            "error",
          );
        }
      } catch (e) {
        this.showToast("Network error — legal versions not saved", "error");
      } finally {
        this.savingLegal = false;
      }
    },
  }));
});
