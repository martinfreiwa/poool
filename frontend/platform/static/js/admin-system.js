/**
 * Admin System Health — Enhanced Ops Dashboard
 * §15: Background Jobs, Webhook Logs, Active Sessions, Password Resets, DB Stats.
 */

document.addEventListener("alpine:init", () => {
  Alpine.data("systemOps", () => ({
    activeTab: "overview",
    loading: true,
    toast: { show: false, message: "", type: "success" },

    // Overview
    overallStatus: "checking",
    apiLatency: null,
    system: null,

    // Background Jobs
    jobs: [],
    jobFilter: "",
    jobStatusFilter: "",
    isDemoJobs: false,

    // Webhook Logs
    webhooks: [],
    isDemoWebhooks: false,
    webhookFilter: "",

    // Active Sessions
    sessions: [],
    sessionSearch: "",

    // Password Resets
    resetTokens: [],
    serverCosts: {
      storage_monthly_usd: 0,
      database_monthly_usd: 0,
      compute_monthly_usd: 0,
      total_monthly_usd: 0
    },

    async init() {
      await this.loadOverview();
      setInterval(() => this.loadOverview(), 30000);
    },

    // ── Tab loaders ──
    async switchTab(tab) {
      this.activeTab = tab;
      if (tab === "overview") await this.loadOverview();
      else if (tab === "jobs") await this.loadJobs();
      else if (tab === "webhooks") await this.loadWebhooks();
      else if (tab === "sessions") await this.loadSessions();
      else if (tab === "resets") await this.loadResets();
    },

    // ── Overview ──
    async loadOverview() {
      this.loading = true;
      const start = Date.now();
      try {
        const resp = await fetch("/api/admin/system");
        this.apiLatency = Date.now() - start;
        if (resp.ok) {
          this.system = await resp.json();
          if (this.system.costs) {
            this.serverCosts = this.system.costs;
          }
          this.overallStatus = "up";
        } else {
          this.overallStatus = "degraded";
        }
      } catch (e) {
        this.overallStatus = "down";
      } finally {
        this.loading = false;
      }
    },

    get dbSize() {
      return this.system?.database?.size || "—";
    },
    get totalRecords() {
      if (!this.system?.database?.tables) return "—";
      const total = this.system.database.tables.reduce(
        (s, t) => s + (t.row_count || 0),
        0,
      );
      return total.toLocaleString();
    },
    get tableCount() {
      return this.system?.database?.tables?.length || "—";
    },
    get tables() {
      return this.system?.database?.tables || [];
    },
    get maxRows() {
      if (!this.tables.length) return 1;
      return Math.max(...this.tables.map((t) => t.row_count || 0), 1);
    },
    get envVars() {
      return this.system?.environment || {};
    },
    get recentErrors() {
      return this.system?.recent_errors || [];
    },

    // ── Background Jobs §15.1 ──
    async loadJobs() {
      try {
        const resp = await fetch("/api/admin/system/jobs");
        if (resp.ok) this.jobs = await resp.json();
        else this.jobs = [];
      } catch (e) {
        this.jobs = [];
      }
    },

    get filteredJobs() {
      let result = this.jobs;
      if (this.jobFilter) {
        const q = this.jobFilter.toLowerCase();
        result = result.filter(
          (j) =>
            j.name.toLowerCase().includes(q) ||
            (j.payload || "").toLowerCase().includes(q),
        );
      }
      if (this.jobStatusFilter) {
        result = result.filter((j) => j.status === this.jobStatusFilter);
      }
      return result;
    },

    async retryJob(jobId) {
      try {
        const resp = await fetch(`/api/admin/system/jobs/${jobId}/retry`, {
          method: "POST",
        });
        if (resp.ok) {
          this.showToast("Job queued for retry");
          await this.loadJobs();
        } else {
          this.showToast("Failed to retry job", "error");
        }
      } catch (e) {
        this.showToast("Connection error", "error");
      }
    },

    async cancelJob(jobId) {
      if (!await pooolConfirm({ title: 'Cancel job', message: 'This cannot be undone.', confirmText: 'Cancel Job', type: 'danger' })) return;
      try {
        const resp = await fetch(`/api/admin/system/jobs/${jobId}`, {
          method: "DELETE",
        });
        if (resp.ok) {
          this.showToast("Job cancelled");
          await this.loadJobs();
        } else {
          this.showToast("Failed to cancel job", "error");
        }
      } catch (e) {
        this.showToast("Connection error", "error");
      }
    },

    getJobStatusClass(status) {
      switch (status) {
        case "completed":
          return "admin-badge--success";
        case "processing":
          return "admin-badge--info";
        case "pending":
          return "admin-badge--warning";
        case "failed":
          return "admin-badge--danger";
        default:
          return "admin-badge--neutral";
      }
    },

    // ── Webhook Logs §15.2 ──
    async loadWebhooks() {
      try {
        const resp = await fetch("/api/admin/system/webhooks");
        if (resp.ok) this.webhooks = await resp.json();
        else this.webhooks = [];
      } catch (e) {
        this.webhooks = [];
      }
    },

    get filteredWebhooks() {
      if (!this.webhookFilter) return this.webhooks;
      const q = this.webhookFilter.toLowerCase();
      return this.webhooks.filter(
        (w) =>
          w.provider.toLowerCase().includes(q) ||
          w.endpoint.toLowerCase().includes(q) ||
          String(w.http_status).includes(q),
      );
    },

    async replayWebhook(webhookId) {
      if (!await pooolConfirm({ title: 'Replay webhook', message: 'The payload will be re-processed by the system.', confirmText: 'Replay', type: 'warning' }))
        return;
      try {
        const resp = await fetch(
          `/api/admin/system/webhooks/${webhookId}/replay`,
          { method: "POST" },
        );
        if (resp.ok) {
          this.showToast("Webhook replayed successfully");
          await this.loadWebhooks();
        } else {
          this.showToast("Failed to replay webhook", "error");
        }
      } catch (e) {
        this.showToast("Connection error", "error");
      }
    },

    // ── Active Sessions §15.4 ──
    async loadSessions() {
      try {
        const resp = await fetch("/api/admin/system/sessions");
        if (resp.ok) {
          const data = await resp.json();
          this.sessions = (data.sessions || []).map(s => ({
            ...s,
            user_email: s.email || s.user_email || '',
          }));
        } else {
          this.sessions = [];
        }
      } catch (e) {
        this.sessions = [];
      }
    },

    get filteredSessions() {
      if (!this.sessionSearch) return this.sessions;
      const q = this.sessionSearch.toLowerCase();
      return this.sessions.filter(
        (s) =>
          (s.user_email || "").toLowerCase().includes(q) ||
          (s.ip_address || "").includes(q) ||
          (s.user_agent || "").toLowerCase().includes(q),
      );
    },

    async revokeSession(sessionId) {
      if (
        !await pooolConfirm({ title: 'Revoke session', message: 'The user will be logged out of this device immediately.', confirmText: 'Revoke', type: 'danger' })
      )
        return;
      try {
        const resp = await fetch(`/api/admin/system/sessions/${sessionId}`, {
          method: "DELETE",
        });
        if (resp.ok) {
          this.sessions = this.sessions.filter((s) => s.id !== sessionId);
          this.showToast("Session revoked");
        } else {
          this.showToast("Failed to revoke session", "error");
        }
      } catch (e) {
        this.showToast("Connection error", "error");
      }
    },

    async bulkRevokeSessions(pattern) {
      const toRevoke = this.sessions.filter(
        (s) => s.ip_address && s.ip_address.startsWith(pattern),
      );
      if (
        !await pooolConfirm({
          title: 'Bulk revoke sessions',
          message: `Revoke all ${toRevoke.length} sessions from IP pattern "${pattern}*"?`,
          confirmText: `Revoke ${toRevoke.length} Sessions`,
          type: 'danger',
        })
      )
        return;
      try {
        const resp = await fetch("/api/admin/system/sessions/bulk-revoke", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ ip_pattern: pattern }),
        });
        if (resp.ok) {
          this.showToast(`${toRevoke.length} sessions revoked`);
          await this.loadSessions();
        }
      } catch (e) {
        this.showToast("Connection error", "error");
      }
    },

    // ── Password Resets §15.5 ──
    async loadResets() {
      try {
        const resp = await fetch("/api/admin/system/password-resets");
        if (resp.ok) {
          const data = await resp.json();
          this.resetTokens = (data.resets || []).map(r => ({
            ...r,
            user_email: r.email || r.user_email || '',
            used: r.action === 'password.reset_complete',
          }));
        } else {
          this.resetTokens = [];
        }
      } catch (e) {
        this.resetTokens = [];
      }
    },

    // ── Maintenance Actions ──
    async clearCache() {
      try {
        const resp = await fetch("/api/admin/maintenance/clear-cache", {
          method: "POST",
        });
        this.showToast(
          resp.ok ? "Cache cleared successfully" : "Failed to clear cache",
          resp.ok ? "success" : "error",
        );
      } catch (e) {
        this.showToast("Connection error", "error");
      }
    },

    async rotateLogs() {
      try {
        const resp = await fetch("/api/admin/maintenance/rotate-logs", {
          method: "POST",
        });
        this.showToast(
          resp.ok ? "Logs rotated successfully" : "Failed to rotate logs",
          resp.ok ? "success" : "error",
        );
      } catch (e) {
        this.showToast("Connection error", "error");
      }
    },

    async toggleMaintenance() {
      const currentlyEnabled = this.system?.maintenance_mode === true || this.system?.maintenance_mode === 'true';
      const newEnabled = !currentlyEnabled;
      if (
        !await pooolConfirm({
          title: newEnabled ? 'Enable maintenance mode' : 'Disable maintenance mode',
          message: newEnabled
            ? 'All users will see a maintenance page until this is disabled.'
            : 'The platform will be accessible to all users again.',
          confirmText: newEnabled ? 'Enable' : 'Disable',
          type: newEnabled ? 'danger' : 'success',
        })
      )
        return;
      try {
        const resp = await fetch("/api/admin/settings/maintenance", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: newEnabled }),
        });
        if (resp.ok) {
          if (this.system) this.system.maintenance_mode = newEnabled;
          this.showToast(newEnabled ? "Maintenance mode enabled" : "Maintenance mode disabled");
        } else {
          this.showToast("Failed to toggle maintenance mode", "error");
        }
      } catch (e) {
        this.showToast("Connection error", "error");
      }
    },

    // ── Demo Data Generators (when APIs unavailable) ──
    generateDemoJobs() {
      return [
        {
          id: "1",
          name: "deposit.expire_check",
          status: "completed",
          attempts: 1,
          payload: '{"batch_size":100}',
          run_at: new Date(Date.now() - 300000).toISOString(),
          created_at: new Date(Date.now() - 600000).toISOString(),
        },
        {
          id: "2",
          name: "dividend.calculate",
          status: "processing",
          attempts: 1,
          payload: '{"asset_id":"abc-123"}',
          run_at: new Date().toISOString(),
          created_at: new Date(Date.now() - 120000).toISOString(),
        },
        {
          id: "3",
          name: "kyc.webhook_sync",
          status: "pending",
          attempts: 0,
          payload: '{"provider":"didit"}',
          run_at: new Date(Date.now() + 60000).toISOString(),
          created_at: new Date().toISOString(),
        },
        {
          id: "4",
          name: "email.campaign_send",
          status: "failed",
          attempts: 3,
          payload: '{"campaign_id":"promo-march","error":"SMTP timeout"}',
          run_at: new Date(Date.now() - 3600000).toISOString(),
          created_at: new Date(Date.now() - 7200000).toISOString(),
        },
        {
          id: "5",
          name: "report.monthly_summary",
          status: "completed",
          attempts: 1,
          payload: '{"month":"2026-02"}',
          run_at: new Date(Date.now() - 86400000).toISOString(),
          created_at: new Date(Date.now() - 86400000).toISOString(),
        },
      ];
    },
    generateDemoWebhooks() {
      return [
        {
          id: "1",
          provider: "Stripe",
          endpoint: "/api/webhooks/payments",
          http_status: 200,
          payload: '{"type":"payment_intent.succeeded","id":"pi_xxx"}',
          processed: true,
          created_at: new Date(Date.now() - 120000).toISOString(),
        },
        {
          id: "2",
          provider: "Didit.me",
          endpoint: "/api/webhooks/kyc",
          http_status: 200,
          payload: '{"event":"verification.completed","user_id":"abc"}',
          processed: true,
          created_at: new Date(Date.now() - 300000).toISOString(),
        },
        {
          id: "3",
          provider: "Xendit",
          endpoint: "/api/webhooks/payments",
          http_status: 500,
          payload: '{"type":"fva.paid","amount":15000000}',
          processed: false,
          created_at: new Date(Date.now() - 600000).toISOString(),
        },
        {
          id: "4",
          provider: "Stripe",
          endpoint: "/api/webhooks/payments",
          http_status: 200,
          payload: '{"type":"charge.refunded","id":"ch_yyy"}',
          processed: true,
          created_at: new Date(Date.now() - 900000).toISOString(),
        },
      ];
    },
    generateDemoSessions() {
      return [
        {
          id: "1",
          user_id: "u1",
          user_email: "admin@poool.finance",
          ip_address: "103.22.45.67",
          user_agent: "Chrome/120 macOS",
          remember_me: false,
          expires_at: new Date(Date.now() + 3600000).toISOString(),
          created_at: new Date(Date.now() - 1800000).toISOString(),
        },
        {
          id: "2",
          user_id: "u2",
          user_email: "investor@example.com",
          ip_address: "185.120.33.12",
          user_agent: "Safari/17 iOS",
          remember_me: true,
          expires_at: new Date(Date.now() + 86400000).toISOString(),
          created_at: new Date(Date.now() - 7200000).toISOString(),
        },
        {
          id: "3",
          user_id: "u3",
          user_email: "dev@company.co",
          ip_address: "103.22.45.68",
          user_agent: "Firefox/121 Linux",
          remember_me: false,
          expires_at: new Date(Date.now() + 1800000).toISOString(),
          created_at: new Date(Date.now() - 900000).toISOString(),
        },
      ];
    },
    generateDemoResets() {
      return [
        {
          id: "1",
          user_email: "user@example.com",
          ip_address: "185.120.33.12",
          used: false,
          expires_at: new Date(Date.now() + 3600000).toISOString(),
          created_at: new Date(Date.now() - 300000).toISOString(),
        },
        {
          id: "2",
          user_email: "user@example.com",
          ip_address: "185.120.33.12",
          used: false,
          expires_at: new Date(Date.now() + 3000000).toISOString(),
          created_at: new Date(Date.now() - 120000).toISOString(),
        },
        {
          id: "3",
          user_email: "other@test.com",
          ip_address: "92.44.12.8",
          used: true,
          expires_at: new Date(Date.now() - 1800000).toISOString(),
          created_at: new Date(Date.now() - 7200000).toISOString(),
        },
      ];
    },

    // ── Helpers ──
    formatDate(iso) {
      if (!iso) return "—";
      const d = new Date(iso);
      const now = new Date();
      const diffMs = now - d;
      if (diffMs < 0) {
        // Future
        const mins = Math.ceil(Math.abs(diffMs) / 60000);
        if (mins < 60) return "in " + mins + "m";
        return "in " + Math.ceil(mins / 60) + "h";
      }
      const mins = Math.floor(diffMs / 60000);
      if (mins < 1) return "just now";
      if (mins < 60) return mins + "m ago";
      const hrs = Math.floor(mins / 60);
      if (hrs < 24) return hrs + "h ago";
      return d.toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    },

    truncate(str, len = 60) {
      if (!str) return "—";
      return str.length > len ? str.substring(0, len) + "…" : str;
    },

    showToast(message, type = "success") {
      this.toast = { show: true, message, type };
      setTimeout(() => {
        this.toast.show = false;
      }, 3500);
    },
  }));
});
