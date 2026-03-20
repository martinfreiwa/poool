document.addEventListener("alpine:init", () => {
  Alpine.data("emailApp", () => ({
    activeTab: "overview",
    templateSearch: "",
    isLoading: false,
    errorState: false,

    // Data
    templates: [],
    templatesAll: [], // Used for campaign dropdown
    logs: [],
    templatePage: 1,
    templatePageSize: 10,
    templateTotal: 0,
    logPage: 1,
    logPageSize: 15,
    logTotal: 0,
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
    editorInstance: null,
    editingTemplate: false,
    currentTemplate: {
      id: null,
      name: "",
      subject: "",
      description: "",
      html_template: "",
    },

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

    // Debounce
    _searchTimeout: null,

    async init() {
      // Setup debounced search watcher
      this.$watch('templateSearch', () => {
        clearTimeout(this._searchTimeout);
        this._searchTimeout = setTimeout(() => {
          this.templatePage = 1;
          this.loadTemplates();
        }, 300);
      });
      // Setup tab watcher to load data if needed
      this.$watch('activeTab', (tabs) => {
        if (tabs === 'templates' && this.templates.length === 0) {
           this.loadTemplates();
        }
        if (tabs === 'logs' && this.logs.length === 0) {
           this.loadLogs();
        }
        if (tabs === 'campaigns' && this.templatesAll.length === 0) {
           this.loadAllTemplates();
        }
      });
      
      await this.loadStats();
      await this.loadLogs();
    },

    async loadStats() {
      this.isLoading = true;
      this.errorState = false;
      try {
        const resp = await fetch("/api/admin/emails");
        if (!resp.ok) throw new Error("Failed to load email stats");
        const data = await resp.json();
        if (data.stats) {
          this.stats = { ...this.stats, ...data.stats };
        }
      } catch (err) {
        this.errorState = true;
        this.showToast("Failed to connect to email server.", "error");
      } finally {
        this.isLoading = false;
      }
    },

    async loadLogs() {
      this.isLoading = true;
      try {
        const resp = await fetch(`/api/admin/emails/logs?page=${this.logPage}&limit=${this.logPageSize}`);
        if (!resp.ok) throw new Error("Failed to load generic logs");
        const data = await resp.json();
        this.logs = data.items || [];
        this.logTotal = data.total || 0;
      } catch (err) {
        this.showToast("Failed to load logs.", "error");
      } finally {
        this.isLoading = false;
      }
    },

    async loadTemplates() {
      this.isLoading = true;
      try {
        const q = this.templateSearch ? `&search=${encodeURIComponent(this.templateSearch)}` : '';
        const resp = await fetch(`/api/admin/emails/templates?page=${this.templatePage}&limit=${this.templatePageSize}${q}`);
        if (!resp.ok) throw new Error("Failed to load templates");
        const data = await resp.json();
        this.templates = data.items || [];
        this.templateTotal = data.total || 0;
      } catch (err) {
        this.showToast("Failed to load templates.", "error");
      } finally {
        this.isLoading = false;
      }
    },

    async loadAllTemplates() {
      try {
        const resp = await fetch(`/api/admin/emails/templates/all`);
        if (!resp.ok) throw new Error("Failed to load templates for campaigns");
        const data = await resp.json();
        this.templatesAll = data.items || [];
      } catch (err) {
        this.showToast("Failed to load all templates.", "error");
      }
    },

    get paginatedTemplates() {
      return this.templates;
    },

    get paginatedLogs() {
      return this.logs;
    },
    
    prevTemplatePage() {
      if (this.templatePage > 1) {
         this.templatePage--;
         this.loadTemplates();
      }
    },
    
    nextTemplatePage() {
      if (this.templatePage * this.templatePageSize < this.templateTotal) {
         this.templatePage++;
         this.loadTemplates();
      }
    },

    prevLogPage() {
      if (this.logPage > 1) {
         this.logPage--;
         this.loadLogs();
      }
    },
    
    nextLogPage() {
      if (this.logPage * this.logPageSize < this.logTotal) {
         this.logPage++;
         this.loadLogs();
      }
    },

    startNewTemplate() {
      this.currentTemplate = {
        id: null,
        name: "",
        subject: "",
        description: "",
        html_template: "<h1>Welcome {{first_name}}</h1>\n<p>Start editing here...</p>",
      };
      this.editingTemplate = true;
      this.initEditor();
    },

    editTemplate(t) {
      this.currentTemplate = {
        id: t.id,
        name: t.name,
        subject: t.subject,
        description: t.description || "",
        html_template: t.html_template || "<h1>" + t.subject + "</h1>",
      };
      this.editingTemplate = true;
      this.initEditor();
    },

    initEditor() {
      // Delay initialization so DOM completes Alpine state transition
      setTimeout(() => {
        const el = document.getElementById('html_editor');
        if (!el) return;
        // Clean previous CodeMirror wrapper
        const nextSilbling = el.nextSibling;
        if (nextSilbling && nextSilbling.classList && nextSilbling.classList.contains("CodeMirror")) {
            nextSilbling.remove();
        }
        
        if (window.CodeMirror) {
            this.editorInstance = window.CodeMirror.fromTextArea(el, {
              mode: "htmlmixed",
              lineNumbers: true,
              theme: "default"
            });
            this.editorInstance.on("change", () => {
              this.currentTemplate.html_template = this.editorInstance.getValue();
            });
        }
      }, 50);
    },

    cancelEdit() {
      if (this.editorInstance) {
          this.editorInstance.toTextArea();
          this.editorInstance = null;
      }
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
        const url = isNew
          ? "/api/admin/emails/templates"
          : `/api/admin/emails/templates/${this.currentTemplate.id}`;

        const csrfTokenMeta = document.querySelector('meta[name="csrf-token"]');
        const headers = { "Content-Type": "application/json" };
        if (csrfTokenMeta) headers["X-CSRF-Token"] = csrfTokenMeta.getAttribute("content");

        const resp = await fetch(url, {
          method,
          headers,
          body: JSON.stringify(this.currentTemplate),
        });

        if (!resp.ok) {
          const err = await resp.json();
          throw new Error(err.error || "Failed to save template");
        }

        await this.loadTemplates();
        if (this.editorInstance) {
            this.editorInstance.toTextArea();
            this.editorInstance = null;
        }
        this.editingTemplate = false;
        this.showToast("Template saved successfully!");
      } catch (err) {
        this.showToast(err.message, "error");
      }
    },

    async sendTestTemplate() {
      if (!this.currentTemplate.id) {
          this.showToast("Save the template first before sending a test.", "error");
          return;
      }
      try {
        const csrfTokenMeta = document.querySelector('meta[name="csrf-token"]');
        const headers = { "Content-Type": "application/json" };
        if (csrfTokenMeta) headers["X-CSRF-Token"] = csrfTokenMeta.getAttribute("content");

        const resp = await fetch("/api/admin/emails/test", {
          method: "POST",
          headers,
          body: JSON.stringify({ templateId: this.currentTemplate.id }),
        });
        if (!resp.ok) throw new Error("Failed to send test email");
        this.showToast("Test email queued successfully!");
      } catch (err) {
        this.showToast(err.message, "error");
      }
    },

    async sendCampaign() {
      if (!this.campaign.templateId) return;

      const confirmed = confirm("WARNING: You are about to send an email campaign to a large number of users. Are you absolutely SURE you want to proceed?");
      if (!confirmed) return;

      this.sending = true;
      try {
        const csrfTokenMeta = document.querySelector('meta[name="csrf-token"]');
        const headers = { "Content-Type": "application/json" };
        if (csrfTokenMeta) headers["X-CSRF-Token"] = csrfTokenMeta.getAttribute("content");

        const resp = await fetch("/api/admin/emails/campaigns", {
          method: "POST",
          headers,
          body: JSON.stringify(this.campaign),
        });
        if (!resp.ok) {
          throw new Error("Failed to start campaign.");
        }
        const data = await resp.json();
        this.showToast(`Campaign started! ${data.target_count} users queued.`);

        // Switch to logs to watch sending
        this.activeTab = "logs";
        this.campaign.templateId = "";
        this.logPage = 1;

        // Reload data shortly to show logs
        setTimeout(() => this.loadLogs(), 2000);
      } catch (err) {
        this.showToast(err.message, "error");
      } finally {
        this.sending = true; // Stay functionally true until tab switches
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
