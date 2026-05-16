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

function registerAdminSettingsComponent() {
  if (!window.Alpine || window.__pooolAdminSettingsRegistered) return;
  window.__pooolAdminSettingsRegistered = true;
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
      // Deposit bank wire (managed in Deposits tab)
      deposit_bank_name: "",
      deposit_account_holder: "",
      deposit_iban: "",
      deposit_bic: "",
      deposit_bank_address: "",
      deposit_reference_prefix: "POOOL",
      deposit_processing_hours: 24,
      deposit_min_amount_cents: 5000,
      deposit_max_amount_cents: 10000000,
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
      if (!await pooolConfirm({ title: 'Clear system cache', message: 'This will purge all cached data. Are you sure?', confirmText: 'Clear Cache', type: 'warning' })) return;
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
      if (!await pooolConfirm({ title: 'Trigger log rotation', message: 'This will rotate logs immediately. Continue?', confirmText: 'Rotate Now', type: 'warning' })) return;
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
      if (!await pooolConfirm({ title: 'Remove admin role', message: 'This user will lose all admin access.', confirmText: 'Remove', type: 'danger' })) return;
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

  const root = document.querySelector('[x-data="adminSettings"]');
  if (root && !root._x_dataStack && typeof Alpine.initTree === "function") {
    Alpine.initTree(root);
  }
}

document.addEventListener("alpine:init", registerAdminSettingsComponent);
registerAdminSettingsComponent();

(function () {
  "use strict";

  const state = {
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
      // Deposit bank wire (managed in Deposits tab)
      deposit_bank_name: "",
      deposit_account_holder: "",
      deposit_iban: "",
      deposit_bic: "",
      deposit_bank_address: "",
      deposit_reference_prefix: "POOOL",
      deposit_processing_hours: 24,
      deposit_min_amount_cents: 5000,
      deposit_max_amount_cents: 10000000,
    },
    broadcast: { title: "", message: "", type: "system" },
    legalStats: null,
    savingLegal: false,
    legalForm: {
      legal_terms_version: "1.0",
      legal_privacy_version: "1.0",
      legal_last_updated: new Date().toISOString().split("T")[0],
    },
  };

  document.addEventListener("DOMContentLoaded", initVanillaAdminSettings);

  async function initVanillaAdminSettings() {
    const root = document.querySelector('[x-data="adminSettings"]');
    if (!root) return;

    attachEvents(root);
    render();
    await Promise.all([loadSettings(), loadKycProvider()]);
    render();
  }

  // Match both `x-model="..."` and the Alpine-modifier form `x-model.number="..."`.
  // CSS selectors can't escape `.` inside attribute names portably, so iterate manually.
  function findModelInputs(root) {
    const inputs = [];
    root.querySelectorAll("input, select, textarea").forEach((el) => {
      for (const attr of el.attributes) {
        if (attr.name === "x-model" || attr.name.startsWith("x-model.")) {
          inputs.push({ el, attrName: attr.name, path: attr.value });
          break;
        }
      }
    });
    return inputs;
  }

  function attachEvents(root) {
    findModelInputs(root).forEach(({ el, path }) => {
      el.addEventListener("input", () => setPath(path, getInputValue(el)));
      el.addEventListener("change", () => {
        setPath(path, getInputValue(el));
        render();
      });
    });

    root.querySelectorAll(".admin-tabs .admin-tab").forEach((button) => {
      button.addEventListener("click", async () => {
        const action = button.getAttribute("@click") || "";
        const match = action.match(/activeTab\s*=\s*'([^']+)'/);
        if (!match) return;
        state.activeTab = match[1];
        render();
        if (state.activeTab === "legal") {
          await loadLegalStatus();
          render();
        }
      });
    });

    root.querySelectorAll("button").forEach((button) => {
      button.addEventListener("click", async (event) => {
        const action = button.getAttribute("@click") || "";
        if (!action) return;
        if (action.includes("activeTab")) return;

        event.preventDefault();
        if (action.includes("saveSettings")) await saveSettings();
        if (action.includes("saveLegalVersion")) await saveLegalVersion();
        if (action.includes("sendBroadcast")) await sendBroadcast();
        if (action.includes("toggleMaintenance")) await toggleMaintenance();
        if (action.includes("clearCache")) await clearCache();
        if (action.includes("runLogRotation")) await runLogRotation();
      });
    });
  }

  function getInputValue(input) {
    if (input.type === "checkbox") return input.checked;
    if (input.type === "number") return input.value === "" ? "" : Number(input.value);
    return input.value;
  }

  function setInputValue(input, value) {
    if (input.type === "checkbox") {
      input.checked = Boolean(value);
    } else {
      input.value = value ?? "";
    }
  }

  function getPath(path) {
    return String(path || "").split(".").reduce((current, key) => current?.[key], state);
  }

  function setPath(path, value) {
    const keys = String(path || "").split(".");
    const finalKey = keys.pop();
    const target = keys.reduce((current, key) => current?.[key], state);
    if (target && finalKey) target[finalKey] = value;
  }

  function render() {
    findModelInputs(document).forEach(({ el, path }) => {
      setInputValue(el, getPath(path));
    });

    document.querySelectorAll(".admin-tabs .admin-tab").forEach((button) => {
      const action = button.getAttribute("@click") || "";
      const match = action.match(/activeTab\s*=\s*'([^']+)'/);
      button.classList.toggle("active", Boolean(match && match[1] === state.activeTab));
    });

    document.querySelectorAll("[x-show]").forEach((element) => {
      const expr = element.getAttribute("x-show") || "";
      if (expr.startsWith("activeTab ===")) {
        const match = expr.match(/'([^']+)'/);
        element.style.display = match && match[1] === state.activeTab ? "" : "none";
      } else if (expr === "toast.show") {
        element.style.display = state.toast.show ? "flex" : "none";
        element.textContent = state.toast.message;
        element.style.background = state.toast.type === "success" ? "var(--admin-success)" : "var(--admin-danger)";
        element.style.color = "#fff";
      } else if (expr === "settings.maintenance_mode") {
        element.style.display = state.settings.maintenance_mode ? "" : "none";
      } else if (expr === "legalStats") {
        element.style.display = state.legalStats ? "" : "none";
      }
    });

    setButton('button[x-text*="Save General Settings"]', state.saving, "Saving...", "Save General Settings");
    setButton('button[x-text*="Save Financial Settings"]', state.saving, "Saving...", "Save Financial Settings");
    setButton('button[x-text*="Save Integrations"]', state.saving, "Saving...", "Save Integrations");
    setButton('button[x-text*="Send to All Users"]', state.broadcasting, "Sending...", "Send to All Users");
    setButton('button[x-text*="Save Legal Versions"]', state.savingLegal, "Saving...", "Save Legal Versions");

    const maintenanceButton = document.querySelector('button[x-text*="Enable Maintenance"]');
    if (maintenanceButton) {
      maintenanceButton.textContent = state.settings.maintenance_mode ? "Disable Maintenance" : "Enable Maintenance";
      maintenanceButton.classList.toggle("admin-btn--danger", Boolean(state.settings.maintenance_mode));
      maintenanceButton.classList.toggle("admin-btn--secondary", !state.settings.maintenance_mode);
    }

    const stats = state.legalStats?.stats || {};
    setText('[x-text*="total_consents"]', stats.total_consents ?? "-");
    setText('[x-text*="accepted_current_version"]', stats.accepted_current_version ?? "-");
    setText('[x-text*="pending_reacceptance"]', stats.pending_reacceptance ?? "-");
  }

  function setButton(selector, busy, busyText, idleText) {
    const button = document.querySelector(selector);
    if (!button) return;
    button.disabled = Boolean(busy);
    button.textContent = busy ? busyText : idleText;
  }

  function setText(selector, value) {
    document.querySelectorAll(selector).forEach((element) => {
      element.textContent = String(value);
    });
  }

  function showToast(message, type = "success") {
    state.toast = { show: true, message, type };
    render();
    window.clearTimeout(showToast.timeoutId);
    showToast.timeoutId = window.setTimeout(() => {
      state.toast.show = false;
      render();
    }, 3000);
  }

  async function loadSettings() {
    try {
      const resp = await fetch("/api/admin/settings");
      if (!resp.ok) return;
      const data = await resp.json();
      if (data.settings) {
        Object.assign(state.settings, data.settings);
      }
    } catch (error) {
      console.error("Failed to load admin settings:", error);
      if (window.Sentry) window.Sentry.captureException(error);
    }
  }

  async function saveSettings() {
    state.saving = true;
    render();
    try {
      const resp = await fetch("/api/admin/settings", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(state.settings),
      });
      if (resp.ok) {
        showToast("Settings saved successfully");
      } else {
        const err = await resp.json().catch(() => ({}));
        showToast(err.error || "Failed to save settings", "error");
      }
    } catch (_) {
      showToast("Network error - settings not saved", "error");
    } finally {
      state.saving = false;
      render();
    }
  }

  async function toggleMaintenance() {
    const newState = !state.settings.maintenance_mode;
    try {
      const resp = await fetch("/api/admin/settings/maintenance", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled: newState }),
      });
      if (resp.ok) {
        state.settings.maintenance_mode = newState;
        showToast(`Maintenance mode ${newState ? "enabled" : "disabled"}`);
      } else {
        showToast("Failed to toggle maintenance mode", "error");
      }
    } catch (_) {
      showToast("Network error", "error");
    } finally {
      render();
    }
  }

  async function clearCache() {
    if (window.pooolConfirm && !await window.pooolConfirm({ title: "Clear system cache", message: "This will purge all cached data. Are you sure?", confirmText: "Clear Cache", type: "warning" })) return;
    try {
      const resp = await fetch("/api/admin/maintenance/clear-cache", { method: "POST" });
      if (resp.ok) {
        const data = await resp.json();
        showToast(data.message || "Cache cleared");
      } else {
        showToast("Failed to clear cache", "error");
      }
    } catch (_) {
      showToast("Network error", "error");
    }
  }

  async function runLogRotation() {
    if (window.pooolConfirm && !await window.pooolConfirm({ title: "Trigger log rotation", message: "This will rotate logs immediately. Continue?", confirmText: "Rotate Now", type: "warning" })) return;
    try {
      const resp = await fetch("/api/admin/maintenance/rotate-logs", { method: "POST" });
      if (resp.ok) {
        const data = await resp.json();
        showToast(data.message || "Log rotation initiated");
      } else {
        showToast("Failed to rotate logs", "error");
      }
    } catch (_) {
      showToast("Network error", "error");
    }
  }

  async function sendBroadcast() {
    if (!state.broadcast.title.trim()) {
      showToast("Notification title is required", "error");
      return;
    }
    state.broadcasting = true;
    render();
    try {
      const resp = await fetch("/api/admin/notifications/broadcast", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(state.broadcast),
      });
      if (resp.ok) {
        const data = await resp.json();
        showToast(`Broadcast sent to ${data.count || 0} users`);
        state.broadcast = { title: "", message: "", type: "system" };
      } else {
        const err = await resp.json().catch(() => ({}));
        showToast(err.error || "Failed to send broadcast", "error");
      }
    } catch (_) {
      showToast("Network error", "error");
    } finally {
      state.broadcasting = false;
      render();
    }
  }

  async function loadKycProvider() {
    try {
      await fetch("/api/kyc/provider");
    } catch (_) {
      // KYC provider status is informational on this page.
    }
  }

  async function loadLegalStatus() {
    try {
      const resp = await fetch("/api/admin/legal/version");
      if (!resp.ok) return;
      const data = await resp.json();
      state.legalStats = data;
      if (data.settings) {
        state.legalForm.legal_terms_version = data.settings.legal_terms_version || "1.0";
        state.legalForm.legal_privacy_version = data.settings.legal_privacy_version || "1.0";
        state.legalForm.legal_last_updated = data.settings.legal_last_updated || new Date().toISOString().split("T")[0];
      }
    } catch (error) {
      console.error("Failed to load legal status:", error);
      if (window.Sentry) window.Sentry.captureException(error);
    }
  }

  async function saveLegalVersion() {
    state.savingLegal = true;
    render();
    try {
      const resp = await fetch("/api/admin/legal/version", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(state.legalForm),
      });
      if (resp.ok) {
        showToast("Legal versions updated - users will be prompted to re-accept");
        await loadLegalStatus();
      } else {
        const err = await resp.json().catch(() => ({}));
        showToast(err.error || "Failed to update legal versions", "error");
      }
    } catch (_) {
      showToast("Network error - legal versions not saved", "error");
    } finally {
      state.savingLegal = false;
      render();
    }
  }
})();
