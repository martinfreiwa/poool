(function () {
  "use strict";

  const PAGE_SIZE = 50;
  const ALERT_PAGE_SIZE = 50;
  const ALERT_ACTIONS = {
    acknowledge: {
      title: "Acknowledge alert",
      message: "Mark this Circle ops alert as acknowledged while it remains visible to operators.",
      button: "Acknowledge",
    },
    resolve: {
      title: "Resolve alert",
      message: "Mark this Circle ops alert as resolved after the operational condition has been handled.",
      button: "Resolve",
    },
    assign: {
      title: "Assign alert",
      message: "Assign this Circle ops alert to a platform operator for follow-up.",
      button: "Assign",
    },
    escalate: {
      title: "Escalate alert",
      message: "Increase this Circle ops alert's escalation level for priority triage.",
      button: "Escalate",
    },
    snooze: {
      title: "Snooze alert",
      message: "Temporarily lower this active alert's triage priority without resolving it.",
      button: "Snooze",
    },
    unsnooze: {
      title: "Unsnooze alert",
      message: "Return this alert to the active triage queue immediately.",
      button: "Unsnooze",
    },
    mark_on_call_notified: {
      title: "Mark on-call notified",
      message: "Record that an on-call operator has been manually notified.",
      button: "Mark Notified",
    },
    set_workflow_state: {
      title: "Set workflow state",
      message: "Update the human SLA workflow state for this active alert.",
      button: "Update Workflow",
    },
  };

  const state = {
    circles: [],
    total: 0,
    totalMembers: 0,
    totalXp: 0,
    offset: 0,
    loading: false,
    error: "",
    alerts: [],
    alertTotal: 0,
    alertSummary: {
      open_count: 0,
      critical_active_count: 0,
      acknowledged_count: 0,
      failed_worker_active_count: 0,
      escalated_active_count: 0,
      snoozed_active_count: 0,
      blocked_workflow_count: 0,
    },
    alertsLoading: false,
    alertError: "",
    pendingAlertAction: null,
    pendingDelete: null,
    lastFocused: null,
  };

  const els = {};

  function bindElements() {
    els.refresh = document.getElementById("circles-refresh-btn");
    els.status = document.getElementById("circles-status");
    els.error = document.getElementById("circles-error");
    els.errorText = document.getElementById("circles-error-text");
    els.errorRetry = document.getElementById("circles-error-retry");
    els.totalCount = document.getElementById("circles-total-count");
    els.avgMembers = document.getElementById("circles-avg-members");
    els.totalXp = document.getElementById("circles-total-xp");
    els.filterForm = document.getElementById("circles-filter-form");
    els.search = document.getElementById("circles-search");
    els.visibility = document.getElementById("circles-visibility");
    els.prev = document.getElementById("circles-prev-btn");
    els.next = document.getElementById("circles-next-btn");
    els.pageLabel = document.getElementById("circles-page-label");
    els.loading = document.getElementById("circles-loading");
    els.table = document.getElementById("circles-table");
    els.tbody = document.getElementById("circles-table-body");
    els.empty = document.getElementById("circles-empty");
    els.alertStatus = document.getElementById("circle-alerts-status");
    els.alertError = document.getElementById("circle-alerts-error");
    els.alertErrorText = document.getElementById("circle-alerts-error-text");
    els.alertFilterForm = document.getElementById("circle-alerts-filter-form");
    els.alertStatusFilter = document.getElementById("circle-alerts-status-filter");
    els.alertSeverityFilter = document.getElementById("circle-alerts-severity-filter");
    els.alertTypeFilter = document.getElementById("circle-alerts-type-filter");
    els.alertRefresh = document.getElementById("circle-alerts-refresh-btn");
    els.alertLoading = document.getElementById("circle-alerts-loading");
    els.alertTable = document.getElementById("circle-alerts-table");
    els.alertTbody = document.getElementById("circle-alerts-table-body");
    els.alertEmpty = document.getElementById("circle-alerts-empty");
    els.alertOpenCount = document.getElementById("circle-alerts-open-count");
    els.alertCriticalCount = document.getElementById("circle-alerts-critical-count");
    els.alertAcknowledgedCount = document.getElementById("circle-alerts-acknowledged-count");
    els.alertFailedWorkerCount = document.getElementById("circle-alerts-failed-worker-count");
    els.alertEscalatedCount = document.getElementById("circle-alerts-escalated-count");
    els.alertSnoozedCount = document.getElementById("circle-alerts-snoozed-count");
    els.alertBlockedWorkflowCount = document.getElementById("circle-alerts-blocked-workflow-count");
    els.alertActionModal = document.getElementById("circle-alert-action-modal");
    els.alertActionPanel = els.alertActionModal?.querySelector(".ds-modal");
    els.alertActionTitle = document.getElementById("circle-alert-action-title");
    els.alertActionId = document.getElementById("circle-alert-action-id");
    els.alertActionType = document.getElementById("circle-alert-action-type");
    els.alertActionMessage = document.getElementById("circle-alert-action-message");
    els.alertActionNote = document.getElementById("circle-alert-action-note");
    els.alertActionAssigneeGroup = document.getElementById("circle-alert-action-assignee-group");
    els.alertActionAssignee = document.getElementById("circle-alert-action-assignee");
    els.alertActionSnoozeGroup = document.getElementById("circle-alert-action-snooze-group");
    els.alertActionSnoozeMinutes = document.getElementById(
      "circle-alert-action-snooze-minutes"
    );
    els.alertActionWorkflowGroup = document.getElementById("circle-alert-action-workflow-group");
    els.alertActionWorkflowState = document.getElementById("circle-alert-action-workflow-state");
    els.alertActionError = document.getElementById("circle-alert-action-error");
    els.alertActionConfirm = document.getElementById("circle-alert-action-confirm");
    els.alertActionCancel = document.querySelectorAll("[data-circle-alert-action-cancel]");
    els.deleteModal = document.getElementById("circle-delete-modal");
    els.deletePanel = els.deleteModal?.querySelector(".ds-modal");
    els.deleteMessage = document.getElementById("circle-delete-message");
    els.deleteError = document.getElementById("circle-delete-error");
    els.deleteConfirm = document.getElementById("circle-delete-confirm");
    els.deleteCancel = document.querySelectorAll("[data-circle-delete-cancel]");
  }

  function setStatus(message) {
    if (els.status) els.status.textContent = message;
  }

  function showError(message) {
    state.error = message;
    if (!els.error || !els.errorText) return;
    els.errorText.textContent = message;
    els.error.hidden = false;
    setStatus(message);
  }

  function clearError() {
    state.error = "";
    if (!els.error || !els.errorText) return;
    els.error.hidden = true;
    els.errorText.textContent = "";
  }

  function setAlertStatus(message) {
    if (els.alertStatus) els.alertStatus.textContent = message;
  }

  function showAlertError(message) {
    state.alertError = message;
    if (!els.alertError || !els.alertErrorText) return;
    els.alertErrorText.textContent = message;
    els.alertError.hidden = false;
    setAlertStatus(message);
  }

  function clearAlertError() {
    state.alertError = "";
    if (!els.alertError || !els.alertErrorText) return;
    els.alertError.hidden = true;
    els.alertErrorText.textContent = "";
  }

  function formatNumber(value) {
    return Number(value || 0).toLocaleString();
  }

  function formatDate(value) {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? "Unknown" : date.toLocaleDateString();
  }

  function formatDateTime(value) {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? "Unknown" : date.toLocaleString();
  }

  function isFutureDate(value) {
    const date = new Date(value);
    return !Number.isNaN(date.getTime()) && date.getTime() > Date.now();
  }

  function formatShortId(value) {
    const text = String(value || "");
    return text.length > 12 ? `${text.slice(0, 8)}...${text.slice(-4)}` : text;
  }

  function formatLabel(value) {
    return String(value || "")
      .split("_")
      .filter(Boolean)
      .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
      .join(" ");
  }

  function formatActionPastTense(action) {
    const labels = {
      acknowledge: "acknowledged",
      resolve: "resolved",
      assign: "assigned",
      escalate: "escalated",
      snooze: "snoozed",
      unsnooze: "unsnoozed",
      mark_on_call_notified: "marked as on-call notified",
      set_workflow_state: "workflow updated",
    };
    return labels[action] || "updated";
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

  function readFilters() {
    const search = els.search?.value.trim() || "";
    const visibility = els.visibility?.value || "all";
    return { search, visibility };
  }

  function buildListUrl() {
    const { search, visibility } = readFilters();
    const params = new URLSearchParams({
      limit: String(PAGE_SIZE),
      offset: String(state.offset),
      visibility,
    });
    if (search) params.set("search", search);
    return `/api/admin/community/circles?${params.toString()}`;
  }

  function readAlertFilters() {
    return {
      status: els.alertStatusFilter?.value || "active",
      severity: els.alertSeverityFilter?.value || "all",
      alertType: els.alertTypeFilter?.value || "all",
    };
  }

  function buildAlertsUrl() {
    const filters = readAlertFilters();
    const params = new URLSearchParams({
      limit: String(ALERT_PAGE_SIZE),
      offset: "0",
      status: filters.status,
      severity: filters.severity,
      alert_type: filters.alertType,
    });
    return `/api/admin/community/ops-alerts?${params.toString()}`;
  }

  async function parseError(response) {
    try {
      const data = await response.json();
      return data.error || data.message || `Request failed with ${response.status}`;
    } catch (_err) {
      return `Request failed with ${response.status}`;
    }
  }

  function setLoading(isLoading) {
    state.loading = isLoading;
    if (els.loading) els.loading.hidden = !isLoading;
    if (els.refresh) els.refresh.disabled = isLoading;
    if (els.table) els.table.hidden = isLoading || state.circles.length === 0;
    if (els.empty) els.empty.hidden = isLoading || Boolean(state.error) || state.circles.length > 0;
  }

  async function loadCircles() {
    setLoading(true);
    clearError();
    setStatus("Loading circles...");

    try {
      const response = await fetch(buildListUrl(), { credentials: "same-origin" });
      if (!response.ok) {
        throw new Error(await parseError(response));
      }
      const data = await response.json();
      state.circles = Array.isArray(data.circles) ? data.circles : [];
      state.total = Number(data.total ?? state.circles.length);
      state.totalMembers = Number(data.total_members ?? 0);
      state.totalXp = Number(data.total_xp ?? 0);
      render();
      setStatus(`Loaded ${state.circles.length} circles.`);
    } catch (err) {
      state.circles = [];
      state.total = 0;
      state.totalMembers = 0;
      state.totalXp = 0;
      render();
      showError(err.message || "Unable to load circles.");
    } finally {
      setLoading(false);
    }
  }

  function setAlertsLoading(isLoading) {
    state.alertsLoading = isLoading;
    if (els.alertLoading) els.alertLoading.hidden = !isLoading;
    if (els.alertRefresh) els.alertRefresh.disabled = isLoading;
    if (els.alertTable) els.alertTable.hidden = isLoading || state.alerts.length === 0;
    if (els.alertEmpty) {
      els.alertEmpty.hidden = isLoading || Boolean(state.alertError) || state.alerts.length > 0;
    }
  }

  async function loadCircleOpsAlerts() {
    setAlertsLoading(true);
    clearAlertError();
    setAlertStatus("Loading Circle ops alerts...");

    try {
      const response = await fetch(buildAlertsUrl(), { credentials: "same-origin" });
      if (!response.ok) {
        throw new Error(await parseError(response));
      }
      const data = await response.json();
      state.alerts = Array.isArray(data.alerts) ? data.alerts : [];
      state.alertTotal = Number(data.total ?? state.alerts.length);
      state.alertSummary = {
        open_count: Number(data.summary?.open_count ?? 0),
        critical_active_count: Number(data.summary?.critical_active_count ?? 0),
        acknowledged_count: Number(data.summary?.acknowledged_count ?? 0),
        failed_worker_active_count: Number(data.summary?.failed_worker_active_count ?? 0),
        escalated_active_count: Number(data.summary?.escalated_active_count ?? 0),
        snoozed_active_count: Number(data.summary?.snoozed_active_count ?? 0),
        blocked_workflow_count: Number(data.summary?.blocked_workflow_count ?? 0),
      };
      renderAlerts();
      setAlertStatus(`Loaded ${state.alerts.length} Circle ops alerts.`);
    } catch (err) {
      state.alerts = [];
      state.alertTotal = 0;
      state.alertSummary = {
        open_count: 0,
        critical_active_count: 0,
        acknowledged_count: 0,
        failed_worker_active_count: 0,
        escalated_active_count: 0,
        snoozed_active_count: 0,
        blocked_workflow_count: 0,
      };
      renderAlerts();
      showAlertError(err.message || "Unable to load Circle ops alerts.");
    } finally {
      setAlertsLoading(false);
    }
  }

  function renderStats() {
    const avg = state.total === 0 ? "0" : (state.totalMembers / state.total).toFixed(1);

    if (els.totalCount) els.totalCount.textContent = formatNumber(state.total);
    if (els.avgMembers) els.avgMembers.textContent = avg;
    if (els.totalXp) els.totalXp.textContent = formatNumber(state.totalXp);
  }

  function renderPagination() {
    const start = state.total === 0 ? 0 : state.offset + 1;
    const end = Math.min(state.offset + state.circles.length, state.total);
    if (els.pageLabel) els.pageLabel.textContent = `${start}-${end} of ${state.total}`;
    if (els.prev) els.prev.disabled = state.loading || state.offset === 0;
    if (els.next) els.next.disabled = state.loading || state.offset + PAGE_SIZE >= state.total;
  }

  function appendCell(row, text) {
    const cell = document.createElement("td");
    cell.textContent = text;
    row.appendChild(cell);
    return cell;
  }

  function createCircleCell(circle) {
    const cell = document.createElement("td");
    const wrap = document.createElement("div");
    wrap.style.display = "flex";
    wrap.style.alignItems = "center";
    wrap.style.gap = "10px";

    const avatar = document.createElement("div");
    avatar.style.fontSize = "24px";
    avatar.textContent = circle.avatar_emoji || "O";

    const meta = document.createElement("div");
    const name = document.createElement("div");
    name.style.fontWeight = "500";
    name.style.fontFamily = "var(--font-heading)";
    name.style.color = "var(--text-primary)";
    name.textContent = circle.name || "Untitled circle";

    const id = document.createElement("div");
    id.style.fontSize = "13px";
    id.style.color = "var(--text-tertiary)";
    id.textContent = circle.id || "";

    meta.append(name, id);
    wrap.append(avatar, meta);
    cell.appendChild(wrap);
    return cell;
  }

  function createLevelCell(circle) {
    const cell = document.createElement("td");
    const level = document.createElement("div");
    level.style.fontWeight = "500";
    level.style.fontFamily = "var(--font-heading)";
    level.textContent = `Lvl ${circle.level || 1}`;

    const label = document.createElement("div");
    label.style.fontSize = "13px";
    label.style.color = "var(--text-tertiary)";
    label.textContent = circle.level_name || "";

    cell.append(level, label);
    return cell;
  }

  function createVisibilityCell(circle) {
    const cell = document.createElement("td");
    const badge = document.createElement("span");
    const visibility = circle.visibility || (circle.is_public ? "public" : "private");
    badge.className = "admin-badge";
    badge.style.background = visibility === "public" ? "#dcfce7" : visibility === "hidden" ? "#fef3c7" : "#f3f4f6";
    badge.style.color = visibility === "public" ? "#166534" : visibility === "hidden" ? "#92400e" : "#4b5563";
    badge.textContent = visibility.charAt(0).toUpperCase() + visibility.slice(1);
    cell.appendChild(badge);
    return cell;
  }

  function createTypeCell(circle) {
    const labels = {
      social: "Social",
      asset: "Asset",
      topic: "Topic",
      expert: "Expert",
      private_investor: "Investor Club",
      official: "Official",
    };
    const cell = document.createElement("td");
    const type = circle.circle_type || "social";
    const primary = document.createElement("div");
    primary.textContent = labels[type] || type;
    primary.style.fontWeight = "500";

    const flags = [];
    if (circle.join_policy === "holder_only" || circle.token_gate_asset_id) flags.push("Holder-only");
    if (circle.kyc_required || circle.join_policy === "kyc_required") flags.push("KYC");
    if (circle.is_official) flags.push("Official");
    if (circle.private_investor_club) flags.push("Investor club");

    const secondary = document.createElement("div");
    secondary.style.fontSize = "13px";
    secondary.style.color = "var(--text-tertiary)";
    secondary.textContent = flags.join(" · ");
    cell.append(primary, secondary);
    return cell;
  }

  function createActionsCell(circle) {
    const cell = document.createElement("td");
    cell.style.textAlign = "right";

    const actions = document.createElement("div");
    actions.style.display = "flex";
    actions.style.gap = "8px";
    actions.style.justifyContent = "flex-end";

    const view = document.createElement("a");
    view.href = `/admin/community/circle-detail.html?id=${encodeURIComponent(circle.id)}`;
    view.className = "admin-btn admin-btn--secondary admin-btn--sm";
    view.textContent = "View";

    const disband = document.createElement("button");
    disband.type = "button";
    disband.className = "admin-btn admin-btn--icon";
    disband.style.color = "var(--error-color)";
    disband.setAttribute("aria-label", `Disband circle: ${circle.name || circle.id}`);
    disband.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>';
    disband.addEventListener("click", () => openDeleteModal(circle, disband));

    actions.append(view, disband);
    cell.appendChild(actions);
    return cell;
  }

  function createBadge(text, tone) {
    const badge = document.createElement("span");
    badge.className = "admin-badge";
    const styles = {
      critical: ["#fef2f2", "#991b1b"],
      warning: ["#fef3c7", "#92400e"],
      info: ["#eff6ff", "#1d4ed8"],
      open: ["#fef2f2", "#991b1b"],
      acknowledged: ["#fef3c7", "#92400e"],
      resolved: ["#dcfce7", "#166534"],
      default: ["#f3f4f6", "#4b5563"],
    };
    const [background, color] = styles[tone] || styles.default;
    badge.style.background = background;
    badge.style.color = color;
    badge.textContent = text;
    return badge;
  }

  function createAlertCell(alert) {
    const cell = document.createElement("td");
    const title = document.createElement("div");
    title.style.fontWeight = "500";
    title.style.fontFamily = "var(--font-heading)";
    title.style.color = "var(--text-primary)";
    title.textContent = alert.summary || formatLabel(alert.alert_type) || "Circle ops alert";

    const meta = document.createElement("div");
    meta.style.fontSize = "13px";
    meta.style.color = "var(--text-tertiary)";
    const metaParts = [formatLabel(alert.alert_type)];
    if (Number(alert.escalation_level || 0) > 0) {
      metaParts.push(`Escalation L${alert.escalation_level}`);
    }
    if (alert.assigned_to_user_id) {
      metaParts.push(`Assigned ${formatShortId(alert.assigned_to_user_id)}`);
    }
    if (alert.workflow_state) {
      metaParts.push(`Workflow ${formatLabel(alert.workflow_state)}`);
    }
    if (isFutureDate(alert.snoozed_until)) {
      metaParts.push(`Snoozed until ${formatDateTime(alert.snoozed_until)}`);
    }
    if (alert.on_call_notified_at) {
      metaParts.push(`On-call noted ${formatDateTime(alert.on_call_notified_at)}`);
    }
    meta.textContent = metaParts.filter(Boolean).join(" · ");
    cell.append(title, meta);
    return cell;
  }

  function createAlertCircleCell(alert) {
    const cell = document.createElement("td");
    if (alert.circle_id) {
      const link = document.createElement("a");
      link.href = `/admin/community/circle-detail.html?id=${encodeURIComponent(alert.circle_id)}`;
      link.textContent = alert.circle_name || alert.circle_slug || alert.circle_id;
      link.style.fontWeight = "500";
      cell.appendChild(link);

      const slug = document.createElement("div");
      slug.style.fontSize = "13px";
      slug.style.color = "var(--text-tertiary)";
      slug.textContent = alert.circle_slug || alert.circle_id;
      cell.appendChild(slug);
      return cell;
    }

    const label = document.createElement("div");
    label.style.fontWeight = "500";
    label.textContent = "Platform-wide";
    const helper = document.createElement("div");
    helper.style.fontSize = "13px";
    helper.style.color = "var(--text-tertiary)";
    helper.textContent = "No Circle scope";
    cell.append(label, helper);
    return cell;
  }

  function createAlertActionsCell(alert) {
    const cell = document.createElement("td");
    cell.style.textAlign = "right";

    if (alert.status === "resolved") {
      cell.textContent = "Resolved";
      return cell;
    }

    const actions = document.createElement("div");
    actions.style.display = "flex";
    actions.style.gap = "8px";
    actions.style.justifyContent = "flex-end";
    actions.style.flexWrap = "wrap";

    const appendActionButton = (action, label) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "admin-btn admin-btn--secondary admin-btn--sm";
      button.dataset.alertAction = action;
      button.setAttribute("data-alert-action", action);
      button.textContent = label;
      button.addEventListener("click", () => openAlertActionModal(alert, action, button));
      actions.appendChild(button);
      return button;
    };

    if (alert.status === "open") {
      appendActionButton("acknowledge", "Acknowledge");
    }

    appendActionButton("resolve", "Resolve");
    appendActionButton("assign", "Assign");
    appendActionButton("escalate", "Escalate");
    appendActionButton("set_workflow_state", "Workflow");
    appendActionButton(isFutureDate(alert.snoozed_until) ? "unsnooze" : "snooze", isFutureDate(alert.snoozed_until) ? "Unsnooze" : "Snooze");
    if (!alert.on_call_notified_at) {
      appendActionButton("mark_on_call_notified", "On-call");
    }

    cell.appendChild(actions);
    return cell;
  }

  function renderAlertSummary() {
    if (els.alertOpenCount) {
      els.alertOpenCount.textContent = formatNumber(state.alertSummary.open_count);
    }
    if (els.alertCriticalCount) {
      els.alertCriticalCount.textContent = formatNumber(state.alertSummary.critical_active_count);
    }
    if (els.alertAcknowledgedCount) {
      els.alertAcknowledgedCount.textContent = formatNumber(state.alertSummary.acknowledged_count);
    }
    if (els.alertFailedWorkerCount) {
      els.alertFailedWorkerCount.textContent = formatNumber(
        state.alertSummary.failed_worker_active_count
      );
    }
    if (els.alertEscalatedCount) {
      els.alertEscalatedCount.textContent = formatNumber(
        state.alertSummary.escalated_active_count
      );
    }
    if (els.alertSnoozedCount) {
      els.alertSnoozedCount.textContent = formatNumber(state.alertSummary.snoozed_active_count);
    }
    if (els.alertBlockedWorkflowCount) {
      els.alertBlockedWorkflowCount.textContent = formatNumber(
        state.alertSummary.blocked_workflow_count
      );
    }
  }

  function renderAlertRows() {
    if (!els.alertTbody) return;
    els.alertTbody.replaceChildren();

    state.alerts.forEach((alert) => {
      const row = document.createElement("tr");
      row.dataset.alertId = alert.id || "";
      row.appendChild(createAlertCell(alert));
      row.appendChild(createAlertCircleCell(alert));

      const severity = document.createElement("td");
      severity.appendChild(createBadge(formatLabel(alert.severity), alert.severity));
      row.appendChild(severity);

      const status = document.createElement("td");
      status.appendChild(createBadge(formatLabel(alert.status), alert.status));
      row.appendChild(status);

      const workflow = document.createElement("td");
      workflow.appendChild(createBadge(formatLabel(alert.workflow_state || "triage"), "info"));
      row.appendChild(workflow);

      appendCell(row, formatDateTime(alert.created_at));
      row.appendChild(createAlertActionsCell(alert));
      els.alertTbody.appendChild(row);
    });
  }

  function renderAlerts() {
    renderAlertSummary();
    renderAlertRows();
    if (els.alertTable) {
      els.alertTable.hidden = state.alertsLoading || state.alerts.length === 0;
    }
    if (els.alertEmpty) {
      els.alertEmpty.hidden =
        state.alertsLoading || Boolean(state.alertError) || state.alerts.length > 0;
    }
  }

  function renderRows() {
    if (!els.tbody) return;
    els.tbody.replaceChildren();

    state.circles.forEach((circle) => {
      const row = document.createElement("tr");
      row.appendChild(createCircleCell(circle));
      row.appendChild(createLevelCell(circle));
      appendCell(row, `${circle.member_count || 0}`);
      appendCell(row, formatNumber(circle.total_xp));
      appendCell(row, formatDate(circle.created_at));
      row.appendChild(createTypeCell(circle));
      row.appendChild(createVisibilityCell(circle));
      row.appendChild(createActionsCell(circle));
      els.tbody.appendChild(row);
    });
  }

  function render() {
    renderStats();
    renderPagination();
    renderRows();
    if (els.table) els.table.hidden = state.loading || state.circles.length === 0;
    if (els.empty) els.empty.hidden = state.loading || Boolean(state.error) || state.circles.length > 0;
  }

  function openAlertActionModal(alert, action, trigger) {
    const config = ALERT_ACTIONS[action] || ALERT_ACTIONS.resolve;
    state.pendingAlertAction = { alert, action };
    state.lastFocused = trigger || document.activeElement;
    if (els.alertActionId) els.alertActionId.value = alert.id || "";
    if (els.alertActionType) els.alertActionType.value = action;
    if (els.alertActionTitle) els.alertActionTitle.textContent = config.title;
    if (els.alertActionMessage) {
      const scope = alert.circle_name || alert.circle_slug || "platform-wide operations";
      els.alertActionMessage.textContent = `${config.message} Scope: ${scope}.`;
    }
    if (els.alertActionNote) els.alertActionNote.value = "";
    if (els.alertActionAssignee) {
      els.alertActionAssignee.value = alert.assigned_to_user_id || "";
      els.alertActionAssignee.disabled = action !== "assign";
    }
    if (els.alertActionSnoozeMinutes) {
      els.alertActionSnoozeMinutes.value = "60";
      els.alertActionSnoozeMinutes.disabled = action !== "snooze";
    }
    if (els.alertActionWorkflowState) {
      els.alertActionWorkflowState.value = alert.workflow_state || "triage";
      els.alertActionWorkflowState.disabled = action !== "set_workflow_state";
    }
    if (els.alertActionAssigneeGroup) {
      els.alertActionAssigneeGroup.hidden = action !== "assign";
    }
    if (els.alertActionSnoozeGroup) {
      els.alertActionSnoozeGroup.hidden = action !== "snooze";
    }
    if (els.alertActionWorkflowGroup) {
      els.alertActionWorkflowGroup.hidden = action !== "set_workflow_state";
    }
    if (els.alertActionError) els.alertActionError.textContent = "";
    if (els.alertActionConfirm) {
      els.alertActionConfirm.disabled = false;
      els.alertActionConfirm.textContent = config.button;
    }
    if (els.alertActionModal) els.alertActionModal.classList.add("active");
    setTimeout(() => {
      if (action === "assign") {
        els.alertActionAssignee?.focus();
      } else if (action === "snooze") {
        els.alertActionSnoozeMinutes?.focus();
      } else if (action === "set_workflow_state") {
        els.alertActionWorkflowState?.focus();
      } else {
        els.alertActionNote?.focus();
      }
    }, 0);
  }

  function closeAlertActionModal() {
    state.pendingAlertAction = null;
    if (els.alertActionModal) els.alertActionModal.classList.remove("active");
    if (els.alertActionError) els.alertActionError.textContent = "";
    if (els.alertActionAssigneeGroup) els.alertActionAssigneeGroup.hidden = true;
    if (els.alertActionSnoozeGroup) els.alertActionSnoozeGroup.hidden = true;
    if (els.alertActionWorkflowGroup) els.alertActionWorkflowGroup.hidden = true;
    if (els.alertActionAssignee) els.alertActionAssignee.disabled = true;
    if (els.alertActionSnoozeMinutes) els.alertActionSnoozeMinutes.disabled = true;
    if (els.alertActionWorkflowState) els.alertActionWorkflowState.disabled = true;
    if (state.lastFocused && typeof state.lastFocused.focus === "function") {
      state.lastFocused.focus();
    }
    state.lastFocused = null;
  }

  async function confirmAlertAction() {
    const pending = state.pendingAlertAction;
    if (!pending?.alert?.id || !els.alertActionConfirm) return;

    const note = els.alertActionNote?.value.trim() || "";
    const body = {
      action: pending.action,
      note,
    };
    if (pending.action === "assign") {
      body.assigned_to_user_id = els.alertActionAssignee?.value.trim() || "";
    }
    if (pending.action === "snooze") {
      body.snooze_minutes = Number(els.alertActionSnoozeMinutes?.value || 0);
    }
    if (pending.action === "set_workflow_state") {
      body.workflow_state = els.alertActionWorkflowState?.value || "triage";
    }
    els.alertActionConfirm.disabled = true;
    if (els.alertActionError) els.alertActionError.textContent = "";
    setAlertStatus(`${formatLabel(pending.action)} Circle ops alert...`);

    try {
      const response = await fetch(
        `/api/admin/community/ops-alerts/${encodeURIComponent(pending.alert.id)}/action`,
        {
          method: "POST",
          credentials: "same-origin",
          headers: {
            "Content-Type": "application/json",
            "X-CSRF-Token": getCsrfToken(),
          },
          body: JSON.stringify(body),
        }
      );
      if (!response.ok) {
        throw new Error(await parseError(response));
      }
      closeAlertActionModal();
      setAlertStatus(`Circle ops alert ${formatActionPastTense(pending.action)}.`);
      await loadCircleOpsAlerts();
    } catch (err) {
      if (els.alertActionError) {
        els.alertActionError.textContent = err.message || "Unable to update Circle ops alert.";
      }
      setAlertStatus("Unable to update Circle ops alert.");
    } finally {
      els.alertActionConfirm.disabled = false;
    }
  }

  function openDeleteModal(circle, trigger) {
    state.pendingDelete = circle;
    state.lastFocused = trigger || document.activeElement;
    if (els.deleteMessage) {
      els.deleteMessage.textContent = `Disband "${circle.name}" and unlink its members? This action cannot be undone.`;
    }
    if (els.deleteError) {
      els.deleteError.textContent = "";
    }
    if (els.deleteModal) els.deleteModal.classList.add("active");
    setTimeout(() => els.deletePanel?.focus(), 0);
  }

  function closeDeleteModal() {
    state.pendingDelete = null;
    if (els.deleteModal) els.deleteModal.classList.remove("active");
    if (state.lastFocused && typeof state.lastFocused.focus === "function") {
      state.lastFocused.focus();
    }
    state.lastFocused = null;
  }

  async function confirmDelete() {
    const circle = state.pendingDelete;
    if (!circle || !els.deleteConfirm) return;

    els.deleteConfirm.disabled = true;
    if (els.deleteError) els.deleteError.textContent = "";
    setStatus(`Disbanding ${circle.name}...`);

    try {
      const response = await fetch(`/api/admin/community/circles/${encodeURIComponent(circle.id)}`, {
        method: "DELETE",
        credentials: "same-origin",
      });
      if (!response.ok) {
        throw new Error(await parseError(response));
      }
      closeDeleteModal();
      setStatus(`Circle ${circle.name} disbanded.`);
      await loadCircles();
    } catch (err) {
      if (els.deleteError) {
        els.deleteError.textContent = err.message || "Unable to disband circle.";
      }
      setStatus("Unable to disband circle.");
    } finally {
      els.deleteConfirm.disabled = false;
    }
  }

  function bindEvents() {
    els.refresh?.addEventListener("click", () => {
      loadCircles();
      loadCircleOpsAlerts();
    });
    els.errorRetry?.addEventListener("click", loadCircles);
    els.filterForm?.addEventListener("submit", (event) => {
      event.preventDefault();
      state.offset = 0;
      loadCircles();
    });
    els.alertRefresh?.addEventListener("click", loadCircleOpsAlerts);
    els.alertFilterForm?.addEventListener("submit", (event) => {
      event.preventDefault();
      loadCircleOpsAlerts();
    });
    els.prev?.addEventListener("click", () => {
      state.offset = Math.max(0, state.offset - PAGE_SIZE);
      loadCircles();
    });
    els.next?.addEventListener("click", () => {
      if (state.offset + PAGE_SIZE < state.total) {
        state.offset += PAGE_SIZE;
        loadCircles();
      }
    });
    els.deleteConfirm?.addEventListener("click", confirmDelete);
    els.deleteCancel?.forEach((button) => button.addEventListener("click", closeDeleteModal));
    els.alertActionConfirm?.addEventListener("click", confirmAlertAction);
    els.alertActionCancel?.forEach((button) =>
      button.addEventListener("click", closeAlertActionModal)
    );
    document.addEventListener("keydown", (event) => {
      if (els.alertActionModal?.classList.contains("active")) {
        if (event.key === "Escape") {
          closeAlertActionModal();
          return;
        }
        if (event.key === "Tab") {
          const focusable = els.alertActionModal.querySelectorAll(
            'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
          );
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
        return;
      }
      if (!els.deleteModal || !els.deleteModal.classList.contains("active")) return;
      if (event.key === "Escape") {
        closeDeleteModal();
        return;
      }
      if (event.key === "Tab") {
        const focusable = els.deleteModal.querySelectorAll(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        );
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
    });
  }

  document.addEventListener("DOMContentLoaded", () => {
    bindElements();
    bindEvents();
    loadCircles();
    loadCircleOpsAlerts();
  });
})();
