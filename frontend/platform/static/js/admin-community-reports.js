(function () {
  "use strict";

  const ACTIONS = {
    hide_post: {
      title: "Hide Reported Post",
      description: "This will hide the post from the community feed and resolve the report.",
      buttonText: "Hide Post",
      color: "#D92D20",
    },
    warn_user: {
      title: "Record Warning",
      description: "This will increase the author's warning count and resolve the report.",
      buttonText: "Record Warning",
      color: "#d97706",
    },
    ban_user: {
      title: "Ban User",
      description: "This will ban the author from community participation and resolve the report.",
      buttonText: "Ban User",
      color: "#b42318",
    },
    dismiss_report: {
      title: "Dismiss Report",
      description: "This will mark the report as dismissed without changing the post or author.",
      buttonText: "Dismiss Report",
      color: "#027A48",
    },
  };

  const state = {
    activeTrigger: null,
    isSubmitting: false,
  };

  function byId(id) {
    return document.getElementById(id);
  }

  function clearChildren(node) {
    while (node.firstChild) node.removeChild(node.firstChild);
  }

  function appendText(parent, text, className, style) {
    const el = document.createElement("div");
    if (className) el.className = className;
    if (style) el.setAttribute("style", style);
    el.textContent = text || "";
    parent.appendChild(el);
    return el;
  }

  function renderTableMessage(message, options = {}) {
    const tbody = byId("reports-table");
    if (!tbody) return;
    clearChildren(tbody);

    const row = document.createElement("tr");
    const cell = document.createElement("td");
    cell.colSpan = 5;
    cell.style.textAlign = "center";
    cell.style.padding = "40px";
    cell.style.color = options.error ? "#D92D20" : "var(--admin-text-muted)";
    cell.textContent = message;
    row.appendChild(cell);
    tbody.appendChild(row);
  }

  function setBadgeCount(count) {
    const badge = byId("com-reports-badge");
    if (!badge) return;
    if (count > 0) {
      badge.textContent = String(count);
      badge.style.display = "";
    } else {
      badge.textContent = "";
      badge.style.display = "none";
    }
  }

  function formatId(value) {
    if (!value || typeof value !== "string") return "unknown";
    return value.length > 8 ? `${value.substring(0, 8)}...` : value;
  }

  function formatDate(value) {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "Unknown";
    return date.toLocaleString();
  }

  function truncate(value, maxLength) {
    const text = typeof value === "string" ? value : "";
    return text.length > maxLength ? `${text.substring(0, maxLength)}...` : text;
  }

  function getCsrfToken() {
    if (typeof window.getCsrfToken === "function") {
      return window.getCsrfToken() || "";
    }
    const value = `; ${document.cookie}`;
    const parts = value.split("; csrf_token=");
    if (parts.length === 2) return parts.pop().split(";").shift() || "";
    return "";
  }

  function createActionButton(reportId, actionType, label, style) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = actionType === "dismiss_report"
      ? "admin-btn admin-btn--ghost admin-btn--sm"
      : "admin-btn admin-btn--secondary admin-btn--sm";
    if (style) button.setAttribute("style", style);
    button.dataset.reportId = reportId;
    button.dataset.actionType = actionType;
    button.setAttribute("aria-label", `${label} report`);
    button.textContent = label;
    button.addEventListener("click", () => openActionModal(reportId, actionType, button));
    return button;
  }

  function renderReportRow(report) {
    const row = document.createElement("tr");
    row.dataset.reportId = report.id || "";

    const reporterCell = document.createElement("td");
    appendText(reporterCell, report.reporter_name || "Unknown", null, "font-weight: 500; color: #101828;");
    appendText(
      reporterCell,
      formatId(report.reporter_id),
      null,
      "font-size: 12px; color: #667085; font-family: monospace;"
    );
    row.appendChild(reporterCell);

    const reasonCell = document.createElement("td");
    const reason = document.createElement("span");
    reason.className = "admin-badge";
    reason.setAttribute("style", "background:#FEF3F2; color:#B42318; border: 1px solid #FECDCA;");
    reason.textContent = report.reason || "Unspecified";
    reasonCell.appendChild(reason);
    row.appendChild(reasonCell);

    const contentCell = document.createElement("td");
    const authorLine = document.createElement("div");
    authorLine.setAttribute("style", "font-size: 12px; color: #667085; margin-bottom: 4px;");
    authorLine.appendChild(document.createTextNode("Author: "));
    const author = document.createElement("span");
    author.setAttribute("style", "font-weight: 500; color: #344054;");
    author.textContent = report.post_author_name || "Unknown";
    authorLine.appendChild(author);
    contentCell.appendChild(authorLine);
    appendText(
      contentCell,
      truncate(report.post_content, 100),
      null,
      "font-size: 14px; max-width: 300px; white-space: normal; line-height: 1.4;"
    );
    row.appendChild(contentCell);

    const dateCell = document.createElement("td");
    const dateText = document.createElement("span");
    dateText.setAttribute("style", "font-size: 13px; color: #667085;");
    dateText.textContent = formatDate(report.created_at);
    dateCell.appendChild(dateText);
    row.appendChild(dateCell);

    const actionsCell = document.createElement("td");
    actionsCell.style.textAlign = "right";
    const actions = document.createElement("div");
    actions.setAttribute("style", "display: flex; gap: 8px; justify-content: flex-end;");
    actions.appendChild(createActionButton(report.id, "hide_post", "Hide Post"));
    actions.appendChild(createActionButton(report.id, "warn_user", "Warn User", "color:#d97706; border-color:#d97706;"));
    actions.appendChild(createActionButton(report.id, "ban_user", "Ban User", "color:#b42318; border-color:#b42318;"));
    actions.appendChild(createActionButton(report.id, "dismiss_report", "Dismiss"));
    actionsCell.appendChild(actions);
    row.appendChild(actionsCell);

    return row;
  }

  async function loadReports() {
    const tbody = byId("reports-table");
    if (!tbody) return;

    renderTableMessage("Loading reports...");
    try {
      const response = await fetch("/api/admin/community/reports");
      if (!response.ok) {
        renderTableMessage("Failed to load reports.", { error: true });
        return;
      }

      const reports = await response.json();
      clearChildren(tbody);

      if (!Array.isArray(reports) || reports.length === 0) {
        renderTableMessage("No pending reports. The queue is empty.");
        setBadgeCount(0);
        return;
      }

      reports.forEach((report) => tbody.appendChild(renderReportRow(report)));
      setBadgeCount(reports.length);
    } catch (error) {
      console.error("Failed to load reports", error);
      renderTableMessage("Error connecting to server.", { error: true });
    }
  }

  function setModalError(message) {
    const error = byId("modal-error");
    if (!error) return;
    error.textContent = message || "";
    error.style.display = message ? "block" : "none";
  }

  function focusableElements() {
    const modal = byId("action-modal");
    if (!modal) return [];
    return Array.from(
      modal.querySelectorAll(
        'button:not([disabled]), textarea:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])'
      )
    ).filter((el) => el.offsetParent !== null || el === document.activeElement);
  }

  function trapFocus(event) {
    if (event.key !== "Tab") return;
    const focusable = focusableElements();
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  function handleModalKeydown(event) {
    if (event.key === "Escape") {
      event.preventDefault();
      closeActionModal();
      return;
    }
    trapFocus(event);
  }

  function openActionModal(reportId, actionType, trigger) {
    const config = ACTIONS[actionType] || ACTIONS.dismiss_report;
    state.activeTrigger = trigger || document.activeElement;

    byId("modal-report-id").value = reportId;
    byId("modal-action-type").value = actionType;
    byId("modal-notes").value = "";
    byId("modal-title").textContent = config.title;
    byId("modal-desc").textContent = config.description;
    setModalError("");

    const confirm = byId("modal-confirm-btn");
    confirm.disabled = false;
    confirm.style.background = config.color;
    confirm.textContent = config.buttonText;

    const modal = byId("action-modal");
    modal.classList.add("active");
    modal.setAttribute("aria-hidden", "false");
    modal.addEventListener("keydown", handleModalKeydown);
    window.setTimeout(() => byId("modal-notes").focus(), 0);
  }

  function closeActionModal() {
    const modal = byId("action-modal");
    if (!modal) return;
    modal.classList.remove("active");
    modal.setAttribute("aria-hidden", "true");
    modal.removeEventListener("keydown", handleModalKeydown);
    setModalError("");

    if (state.activeTrigger && typeof state.activeTrigger.focus === "function") {
      state.activeTrigger.focus();
    }
    state.activeTrigger = null;
  }

  async function confirmAction() {
    if (state.isSubmitting) return;

    const reportId = byId("modal-report-id").value;
    const actionType = byId("modal-action-type").value;
    const notes = byId("modal-notes").value.trim();

    if (!notes) {
      setModalError("Admin notes are required.");
      byId("modal-notes").focus();
      return;
    }

    const confirm = byId("modal-confirm-btn");
    const originalText = confirm.textContent;
    state.isSubmitting = true;
    confirm.disabled = true;
    confirm.textContent = "Processing...";
    setModalError("");

    try {
      const response = await fetch(`/api/admin/community/reports/${encodeURIComponent(reportId)}/action`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-CSRF-Token": getCsrfToken(),
        },
        body: JSON.stringify({
          action: actionType,
          admin_notes: notes,
        }),
      });

      if (response.ok) {
        closeActionModal();
        await loadReports();
        return;
      }

      let message = "Failed to process report action.";
      try {
        const body = await response.json();
        if (body && body.error) message = body.error;
      } catch (_error) {
        const text = await response.text();
        if (text) message = text;
      }
      setModalError(message);
    } catch (error) {
      console.error("Report action failed", error);
      setModalError("Network error. Please retry.");
    } finally {
      state.isSubmitting = false;
      confirm.disabled = false;
      confirm.textContent = originalText;
    }
  }

  document.addEventListener("DOMContentLoaded", () => {
    byId("reports-refresh-btn")?.addEventListener("click", loadReports);
    byId("modal-confirm-btn")?.addEventListener("click", confirmAction);
    document.querySelectorAll("[data-reports-close-modal]").forEach((el) => {
      el.addEventListener("click", closeActionModal);
    });
    loadReports();
  });
})();
