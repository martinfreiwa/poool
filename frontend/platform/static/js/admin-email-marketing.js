document.addEventListener("alpine:init", () => {
  Alpine.data("emailApp", () => ({
    activeTab: "overview",
    templateSearch: "",

    // Data
    templates: [],
    logs: [],
    templatePage: 1,
    templatePageSize: 10,
    logPage: 1,
    logPageSize: 15,
    stats: {
      deliveryRate: "--",
      deliveryTrend: 0,
      openRate: "--",
      clickRate: "--",
      bounceRate: "--",
      bouncesTotal: 0,
      totalSent: 0,
    },

    // Editor State
    editingTemplate: false,
    currentTemplate: {
      id: null,
      name: "",
      subject: "",
      description: "",
      html_template: "",
    },
    // Live CodeMirror instance (created when editor opens, torn down on
    // cancel/save). Plain textarea is no good for editing HTML email
    // markup — no syntax colouring, no line numbers.
    _cm: null,

    // Campaign State
    sending: false,
    campaign: {
      audience: "all",
      templateId: "",
    },

    // Toast
    toast: {
      show: false,
      message: "",
      type: "success",
    },

    async init() {
      // Bring up CodeMirror whenever the editor opens, and tear it down
      // when it closes, so a fresh instance is bound to each template.
      this.$watch("editingTemplate", (opened) => {
        if (opened) {
          this.$nextTick(() => this._mountCodeMirror());
        } else {
          this._destroyCodeMirror();
        }
      });
      await this.loadData();
    },

    _mountCodeMirror() {
      if (this._cm || typeof CodeMirror === "undefined") return;
      const el = document.getElementById("html_editor");
      if (!el) return;
      this._cm = CodeMirror.fromTextArea(el, {
        mode: "htmlmixed",
        lineNumbers: true,
        lineWrapping: true,
        theme: "default",
        indentUnit: 2,
        tabSize: 2,
      });
      this._cm.setSize("100%", 480);
      this._cm.on("change", (cm) => {
        // Keep Alpine state in sync — saveTemplate reads from
        // currentTemplate.html_template, not from the DOM.
        this.currentTemplate.html_template = cm.getValue();
      });
    },

    _destroyCodeMirror() {
      if (!this._cm) return;
      try {
        this._cm.toTextArea();
      } catch (_) {
        // Already detached — ignore.
      }
      this._cm = null;
    },

    async loadData() {
      try {
        const resp = await fetch("/api/admin/emails");
        if (!resp.ok) throw new Error("Failed to load email data");
        const data = await resp.json();

        this.templates = data.templates || [];
        // Update stats block with server data if available
        if (data.stats) {
          this.stats = { ...this.stats, ...data.stats };
        }
        if (data.logs) {
          this.logs = data.logs;
        }
      } catch (err) {
        this.showToast("Failed to connect to email server.", "error");
      }
    },

    get filteredTemplates() {
      let res = this.templates;
      if (this.templateSearch) {
        const s = this.templateSearch.toLowerCase();
        res = res.filter(
          (t) =>
            (t.name && t.name.toLowerCase().includes(s)) ||
            (t.subject && t.subject.toLowerCase().includes(s)),
        );
      }
      return res;
    },

    get paginatedTemplates() {
      const start = (this.templatePage - 1) * this.templatePageSize;
      return this.filteredTemplates.slice(start, start + this.templatePageSize);
    },

    get paginatedLogs() {
      const start = (this.logPage - 1) * this.logPageSize;
      return this.logs.slice(start, start + this.logPageSize);
    },

    startNewTemplate() {
      this.currentTemplate = {
        id: null,
        name: "",
        subject: "",
        description: "",
        html_template:
          "<h1>Welcome {{first_name}}</h1>\n<p>Start editing here...</p>",
      };
      this.editingTemplate = true;
    },

    async editTemplate(t) {
      // The list endpoint omits `html_template` to keep payload small —
      // fetch the full record from the detail endpoint.
      try {
        const resp = await fetch(`/api/admin/emails/templates/${t.id}`);
        if (!resp.ok) throw new Error("Failed to load template");
        const data = await resp.json();
        this.currentTemplate = {
          id: data.id,
          name: data.name,
          subject: data.subject,
          description: data.description || "",
          html_template: data.html_template || "",
        };
        this.editingTemplate = true;
      } catch (err) {
        this.showToast("Failed to load template HTML.", "error");
      }
    },

    async deleteTemplate(t) {
      if (
        !confirm(
          `Delete template "${t.name}"? This cannot be undone. Historical delivery logs will be preserved.`,
        )
      ) {
        return;
      }
      try {
        const resp = await fetch(`/api/admin/emails/templates/${t.id}`, {
          method: "DELETE",
        });
        if (!resp.ok) {
          const err = await resp.json().catch(() => ({}));
          throw new Error(err.error || "Failed to delete template");
        }
        await this.loadData();
        this.showToast(`Template "${t.name}" deleted.`);
      } catch (err) {
        this.showToast(err.message, "error");
      }
    },

    cancelEdit() {
      this.editingTemplate = false;
    },

    async saveTemplate() {
      if (
        !this.currentTemplate.name ||
        !this.currentTemplate.subject ||
        !this.currentTemplate.html_template
      ) {
        this.showToast(
          "Name, Subject, and HTML Content are required.",
          "error",
        );
        return;
      }

      try {
        const isNew = !this.currentTemplate.id;
        const method = isNew ? "POST" : "PUT";
        // If it's new, we post to /api/admin/emails/templates
        // If it's updating, we put to /api/admin/emails/templates/:id
        const url = isNew
          ? "/api/admin/emails/templates"
          : `/api/admin/emails/templates/${this.currentTemplate.id}`;

        const resp = await fetch(url, {
          method,
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(this.currentTemplate),
        });

        if (!resp.ok) {
          const err = await resp.json();
          throw new Error(err.error || "Failed to save template");
        }

        await this.loadData();
        this.editingTemplate = false;
        this.showToast("Template saved successfully!");
      } catch (err) {
        this.showToast(err.message, "error");
      }
    },

    async sendCampaign() {
      if (!this.campaign.templateId) return;
      this.sending = true;
      try {
        const resp = await fetch("/api/admin/emails/campaigns", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(this.campaign),
        });
        if (!resp.ok) {
          // Surface the real backend error (rate limit / unknown segment /
          // missing template / missing permission) instead of a generic
          // "Failed to start campaign" string.
          const err = await resp.json().catch(() => ({}));
          throw new Error(err.error || `Campaign send failed (${resp.status})`);
        }
        const data = await resp.json();
        this.showToast(`Campaign started! ${data.target_count} users queued.`);

        // Switch to logs to watch sending
        this.activeTab = "logs";
        this.campaign.templateId = "";

        // Reload data shortly to show logs
        setTimeout(() => this.loadData(), 2000);
      } catch (err) {
        this.showToast(err.message, "error");
      } finally {
        this.sending = false;
      }
    },

    showToast(message, type = "success") {
      this.toast.message = message;
      this.toast.type = type;
      this.toast.show = true;
      setTimeout(() => {
        this.toast.show = false;
      }, 3000);
    },
  }));
});
