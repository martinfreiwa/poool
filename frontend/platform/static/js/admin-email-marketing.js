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
    audienceCount: null,
    audienceCountLoading: false,

    // Logs filter state — server-side via /api/admin/emails/logs
    logFilters: {
      status: "all",
      search: "",
      days: 7,
    },

    // Workflows tab state
    workflows: [],
    workflowSearch: "",
    workflowCategoryFilter: "all",
    workflowStatusFilter: "all", // all | enabled | disabled | optional | mandatory

    // Preview modal
    preview: {
      show: false,
      sending: false,
      event_type: null,
      template_id: null,
      subject: "",
      html: "",
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
      // Re-fetch recipient count whenever the audience changes (campaign tab).
      this.$watch("campaign.audience", () => this.refreshAudienceCount());
      await this.loadData();
      this.refreshAudienceCount();
      this.loadWorkflows();
    },

    async loadWorkflows() {
      try {
        const r = await fetch("/api/admin/emails/workflows");
        if (!r.ok) throw new Error("Failed to load workflows");
        const d = await r.json();
        this.workflows = d.workflows || [];
      } catch (_) {
        // Non-fatal — workflows tab will show "no workflows".
      }
    },

    get workflowCategories() {
      return [...new Set(this.workflows.map((w) => w.category))].sort();
    },

    get filteredWorkflows() {
      let list = this.workflows;
      if (this.workflowCategoryFilter && this.workflowCategoryFilter !== "all") {
        list = list.filter((w) => w.category === this.workflowCategoryFilter);
      }
      // Status filter: enabled vs disabled (admin toggle) and optional vs
      // mandatory (system classification). Mandatory is always enabled.
      if (this.workflowStatusFilter === "enabled") {
        list = list.filter((w) => w.enabled);
      } else if (this.workflowStatusFilter === "disabled") {
        list = list.filter((w) => !w.enabled);
      } else if (this.workflowStatusFilter === "optional") {
        list = list.filter((w) => !w.mandatory);
      } else if (this.workflowStatusFilter === "mandatory") {
        list = list.filter((w) => w.mandatory);
      }
      if (this.workflowSearch) {
        const s = this.workflowSearch.toLowerCase();
        list = list.filter(
          (w) =>
            w.event_type.toLowerCase().includes(s) ||
            (w.subject && w.subject.toLowerCase().includes(s)) ||
            (w.summary && w.summary.toLowerCase().includes(s)),
        );
      }
      return list;
    },

    async toggleWorkflow(workflow, enabled) {
      if (workflow.mandatory) return; // UI prevents this but defense-in-depth.
      try {
        const r = await fetch(
          `/api/admin/emails/workflow-settings/${encodeURIComponent(workflow.event_type)}`,
          {
            method: "PUT",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ enabled }),
          },
        );
        if (!r.ok) {
          const err = await r.json().catch(() => ({}));
          throw new Error(err.error || `Toggle failed (${r.status})`);
        }
        // Optimistically update the local row so the row reflects state
        // without a full reload.
        workflow.enabled = enabled;
        this.showToast(
          `${workflow.event_type} ${enabled ? "enabled" : "disabled"}`,
        );
      } catch (err) {
        // Re-fetch to roll back the visual state.
        await this.loadWorkflows();
        this.showToast(err.message, "error");
      }
    },

    async openWorkflowPreview(workflow) {
      this.preview = {
        show: true,
        sending: false,
        event_type: workflow.event_type,
        template_id: null,
        subject: "Loading…",
        html: "",
      };
      try {
        const r = await fetch("/api/admin/emails/preview", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ event_type: workflow.event_type }),
        });
        if (!r.ok) {
          const err = await r.json().catch(() => ({}));
          throw new Error(err.error || "Preview failed");
        }
        const d = await r.json();
        this.preview.subject = d.subject || workflow.subject || "(no subject)";
        this.preview.html = d.html || "";
        this.$nextTick(() => this._writePreviewIframe());
      } catch (err) {
        this.preview.subject = "Preview failed";
        this.preview.html = `<pre style="color:#B42318;padding:16px;">${err.message}</pre>`;
        this.$nextTick(() => this._writePreviewIframe());
      }
    },

    async openTemplatePreview(t) {
      this.preview = {
        show: true,
        sending: false,
        event_type: null,
        template_id: t.id,
        subject: "Loading…",
        html: "",
      };
      try {
        const r = await fetch("/api/admin/emails/preview", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ template_id: t.id }),
        });
        if (!r.ok) {
          const err = await r.json().catch(() => ({}));
          throw new Error(err.error || "Preview failed");
        }
        const d = await r.json();
        this.preview.subject = d.subject || t.subject || "(no subject)";
        this.preview.html = d.html || "";
        this.$nextTick(() => this._writePreviewIframe());
      } catch (err) {
        this.preview.subject = "Preview failed";
        this.preview.html = `<pre style="color:#B42318;padding:16px;">${err.message}</pre>`;
        this.$nextTick(() => this._writePreviewIframe());
      }
    },

    _writePreviewIframe(retries = 6) {
      // Modal mount + iframe-doc readiness are both async. Poll briefly
      // (≤ ~300ms total) so the first preview after opening the modal
      // doesn't land before the iframe DOM is alive.
      const frame = this.$refs.previewFrame;
      const doc = frame?.contentDocument || frame?.contentWindow?.document;
      if (!frame || !doc) {
        if (retries > 0) {
          setTimeout(() => this._writePreviewIframe(retries - 1), 50);
        }
        return;
      }
      doc.open();
      doc.write(
        `<!doctype html><html><head><meta charset="utf-8"></head><body style="margin:0;padding:24px;background:#fff;">${this.preview.html}</body></html>`,
      );
      doc.close();
    },

    closePreview() {
      this.preview.show = false;
    },

    async sendTestEvent(workflow) {
      this.preview.event_type = workflow.event_type;
      this.preview.template_id = null;
      await this._doTestSend({ event_type: workflow.event_type });
    },

    async sendTestFromPreview() {
      const body = this.preview.event_type
        ? { event_type: this.preview.event_type }
        : { template_id: this.preview.template_id };
      await this._doTestSend(body);
    },

    async _doTestSend(body) {
      this.preview.sending = true;
      try {
        const r = await fetch("/api/admin/emails/test-send", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
        });
        if (!r.ok) {
          const err = await r.json().catch(() => ({}));
          throw new Error(err.error || `Test send failed (${r.status})`);
        }
        const d = await r.json();
        this.showToast(`Test sent to ${d.to}`);
      } catch (err) {
        this.showToast(err.message, "error");
      } finally {
        this.preview.sending = false;
      }
    },

    async refreshAudienceCount() {
      this.audienceCountLoading = true;
      this.audienceCount = null;
      try {
        const seg = this.campaign.audience;
        const r = await fetch(`/api/admin/emails/audiences/${seg}/count`);
        if (!r.ok) throw new Error("Failed to load audience count");
        const d = await r.json();
        this.audienceCount = d.count;
      } catch (_) {
        this.audienceCount = null;
      } finally {
        this.audienceCountLoading = false;
      }
    },

    async refreshLogs() {
      const p = new URLSearchParams();
      if (this.logFilters.status && this.logFilters.status !== "all") {
        p.set("status", this.logFilters.status);
      }
      if (this.logFilters.search) p.set("search", this.logFilters.search);
      p.set("days", String(this.logFilters.days || 7));
      try {
        const r = await fetch(`/api/admin/emails/logs?${p.toString()}`);
        if (!r.ok) throw new Error("Failed to load logs");
        const d = await r.json();
        this.logs = d.logs || [];
        this.logPage = 1;
      } catch (err) {
        this.showToast("Failed to load filtered logs.", "error");
      }
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
