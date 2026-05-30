(function () {
  "use strict";

  const state = {
    comments: [],
    loading: false,
    searchQuery: "",
    statusFilter: "all",
    errorMessage: "",
  };

  const els = {};

  function setText(el, value) {
    if (el) el.textContent = value == null ? "" : String(value);
  }

  function setHidden(el, hidden) {
    if (el) el.hidden = Boolean(hidden);
  }

function showError(message) {
    state.errorMessage = message || "An error occurred.";
    setText(els.errorText, message);
    setHidden(els.error, false);
    setText(els.status, message);
  }

  function clearError() {
    state.errorMessage = "";
    setText(els.errorText, "");
    setHidden(els.error, true);
  }

  async function parseError(response, fallback) {
    try {
      const data = await response.json();
      return data.message || data.error || fallback;
    } catch (_error) {
      return fallback;
    }
  }

  function getCsrfToken() {
    const value = `; ${document.cookie || ""}`;
    const parts = value.split("; csrf_token=");
    if (parts.length !== 2) return "";
    return decodeURIComponent(parts.pop().split(";").shift() || "");
  }

  function csrfHeaders(headers) {
    const token = getCsrfToken();
    return token ? { ...headers, "X-CSRF-Token": token } : headers;
  }

  function formatDateTime(value, options) {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "-";
    return date.toLocaleString("en-US", options);
  }

  function filteredComments() {
    const query = state.searchQuery.trim().toLowerCase();
    return state.comments.filter((comment) => {
      const matchesSearch =
        !query ||
        (comment.author_name || "").toLowerCase().includes(query) ||
        (comment.content || "").toLowerCase().includes(query);
      const matchesStatus =
        state.statusFilter === "all" ||
        (state.statusFilter === "hidden" && comment.is_hidden) ||
        (state.statusFilter === "visible" && !comment.is_hidden);

      return matchesSearch && matchesStatus;
    });
  }

  function makeCell() {
    return document.createElement("td");
  }

  function makeButton(label, className, title) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = className;
    button.title = title || label;
    button.textContent = label;
    return button;
  }

  function renderAuthorCell(comment) {
    const cell = makeCell();
    const wrapper = document.createElement("div");
    wrapper.style.cssText = "display: flex; align-items: center; gap: 12px;";

    const avatar = document.createElement("div");
    avatar.style.cssText = "width: 32px; height: 32px; border-radius: 50%; background: #f3f4f6; display: flex; align-items: center; justify-content: center; color: var(--text-secondary); font-weight: 600; font-size: 12px; border: 1px solid #e5e7eb;";
    avatar.textContent = comment.author_name ? comment.author_name.substring(0, 2).toUpperCase() : "??";

    const details = document.createElement("div");
    const name = document.createElement("div");
    name.style.cssText = "font-weight: 500; font-family: var(--font-heading); color: var(--text-primary);";
    name.textContent = comment.author_name || "Unknown";

    const userId = document.createElement("div");
    userId.style.cssText = "font-size: 11px; color: var(--text-tertiary); font-family: monospace;";
    userId.textContent = comment.user_id || "";

    details.append(name, userId);
    wrapper.append(avatar, details);
    cell.appendChild(wrapper);
    return cell;
  }

  function renderContentCell(comment) {
    const cell = makeCell();
    const content = document.createElement("div");
    const text = comment.content || "";
    content.style.cssText = "max-width: 320px; white-space: normal; font-size: 13px; line-height: 1.5; color: var(--text-secondary);";
    content.textContent = text.length > 150 ? `${text.substring(0, 150)}...` : text;
    cell.appendChild(content);
    return cell;
  }

  function renderDateCell(comment) {
    const cell = makeCell();
    const date = document.createElement("div");
    date.style.cssText = "font-size: 13px; font-weight: 500; color: var(--text-primary);";
    date.textContent = formatDateTime(comment.created_at, { month: "short", day: "numeric", year: "numeric" });

    const time = document.createElement("div");
    time.style.cssText = "font-size: 11px; color: var(--text-tertiary); margin-top: 2px;";
    time.textContent = formatDateTime(comment.created_at, { hour: "2-digit", minute: "2-digit" });

    cell.append(date, time);
    return cell;
  }

  function renderHelpfulCell(comment) {
    const cell = makeCell();
    const badge = document.createElement("span");
    badge.className = "admin-badge";
    badge.style.cssText = "background:#f3f4f6; color:#4b5563; align-items: center; gap: 4px;";
    badge.textContent = String(comment.helpful_count || 0);
    cell.appendChild(badge);
    return cell;
  }

  function renderStatusCell(comment) {
    const cell = makeCell();
    const badge = document.createElement("span");
    badge.className = "admin-badge";
    badge.style.cssText = comment.is_hidden
      ? "background: #fef2f2; color: #991b1b; border: 1px solid #fee2e2;"
      : "background: #dcfce7; color: #166534; border: 1px solid #d1fae5;";
    badge.textContent = comment.is_hidden ? "Hidden" : "Visible";
    cell.appendChild(badge);
    return cell;
  }

  function renderActionsCell(comment) {
    const cell = makeCell();
    cell.style.textAlign = "right";

    const actions = document.createElement("div");
    actions.style.cssText = "display: flex; gap: 8px; justify-content: flex-end;";

    const context = document.createElement("a");
    context.href = `/community/post/${encodeURIComponent(comment.post_id || "")}`;
    context.target = "_blank";
    context.rel = "noopener noreferrer";
    context.className = "admin-btn admin-btn--secondary admin-btn--sm";
    context.title = "View Context";
    context.textContent = "Context";
    actions.appendChild(context);

    if (!comment.is_hidden) {
      const hide = makeButton("Hide", "admin-btn admin-btn--secondary admin-btn--sm", "Hide Comment");
      hide.style.cssText = "color: var(--error-color); border-color: var(--error-color);";
      hide.addEventListener("click", () => hideComment(comment));
      actions.appendChild(hide);
    }

    const deleteButton = makeButton("", "admin-btn admin-btn--icon", "Delete Permanently");
    deleteButton.style.color = "var(--error-color)";
    deleteButton.setAttribute("aria-label", "Delete comment permanently");
    deleteButton.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>';
    deleteButton.addEventListener("click", () => deleteComment(comment));
    actions.appendChild(deleteButton);

    cell.appendChild(actions);
    return cell;
  }

  function renderTable() {
    const comments = filteredComments();
    els.body.replaceChildren();

    comments.forEach((comment) => {
      const row = document.createElement("tr");
      if (comment.is_hidden) {
        row.style.opacity = "0.7";
        row.style.background = "var(--bg-color)";
      }

      row.append(
        renderAuthorCell(comment),
        renderContentCell(comment),
        renderDateCell(comment),
        renderHelpfulCell(comment),
        renderStatusCell(comment),
        renderActionsCell(comment)
      );
      els.body.appendChild(row);
    });

    setText(els.filteredCount, comments.length);
    setText(els.totalCount, state.comments.length);
    setHidden(els.loading, !state.loading);
    setHidden(els.table, state.loading || comments.length === 0);
    setHidden(els.filterEmpty, state.loading || Boolean(state.errorMessage) || state.comments.length === 0 || comments.length > 0);
    setHidden(els.empty, state.loading || Boolean(state.errorMessage) || state.comments.length !== 0);
  }

  function setLoading(isLoading) {
    state.loading = isLoading;
    els.refresh.disabled = isLoading;
    els.refresh.querySelector("svg")?.classList.toggle("spinning", isLoading);
    renderTable();
  }

  async function loadComments() {
    setLoading(true);
    clearError();

    try {
      const response = await fetch("/api/admin/community/comments?limit=200", {
        credentials: "same-origin",
      });

      if (!response.ok) {
        throw new Error(await parseError(response, "Failed to load comments."));
      }

      const data = await response.json();
      state.comments = Array.isArray(data) ? data : [];
      setText(els.status, `Loaded ${state.comments.length} comments.`);
    } catch (error) {
      showError(error.message || "Failed to load comments.");
    } finally {
      setLoading(false);
    }
  }

  async function hideComment(comment) {
    if (!window.confirm("Are you sure you want to hide this comment from users?")) return;

    try {
      const response = await fetch(`/api/admin/community/comments/${encodeURIComponent(comment.id)}/hide`, {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify({ reason: "Admin hide" }),
      });

      if (!response.ok) {
        throw new Error(await parseError(response, "Failed to hide comment."));
      }

      comment.is_hidden = true;
      clearError();
      setText(els.status, "Comment hidden.");
      renderTable();
    } catch (error) {
      showError(error.message || "Failed to hide comment.");
    }
  }

  async function deleteComment(comment) {
    if (!window.confirm("Are you absolutely sure you want to permanently delete this comment?")) return;

    try {
      const response = await fetch(`/api/admin/community/comments/${encodeURIComponent(comment.id)}`, {
        method: "DELETE",
        headers: csrfHeaders({}),
        credentials: "same-origin",
      });

      if (!response.ok) {
        throw new Error(await parseError(response, "Failed to delete comment."));
      }

      state.comments = state.comments.filter((item) => item.id !== comment.id);
      clearError();
      setText(els.status, "Comment deleted.");
      renderTable();
    } catch (error) {
      showError(error.message || "Failed to delete comment.");
    }
  }

  function init() {
    els.refresh = document.getElementById("comments-refresh-btn");
    els.search = document.getElementById("comments-search");
    els.statusFilter = document.getElementById("comments-status-filter");
    els.filteredCount = document.getElementById("comments-filtered-count");
    els.totalCount = document.getElementById("comments-total-count");
    els.loading = document.getElementById("comments-loading");
    els.table = document.getElementById("comments-table");
    els.body = document.getElementById("comments-table-body");
    els.filterEmpty = document.getElementById("comments-filter-empty");
    els.empty = document.getElementById("comments-empty");
    els.clearFilters = document.getElementById("comments-clear-filters");
    els.error = document.getElementById("comments-error");
    els.errorText = document.getElementById("comments-error-text");
    els.errorRetry = document.getElementById("comments-error-retry");
    els.status = document.getElementById("comments-status");

    if (!els.refresh || !els.body) return;

    els.refresh.addEventListener("click", loadComments);
    els.errorRetry.addEventListener("click", loadComments);
    els.search.addEventListener("input", (event) => {
      state.searchQuery = event.target.value;
      renderTable();
    });
    els.statusFilter.addEventListener("change", (event) => {
      state.statusFilter = event.target.value;
      renderTable();
    });
    els.clearFilters.addEventListener("click", () => {
      state.searchQuery = "";
      state.statusFilter = "all";
      els.search.value = "";
      els.statusFilter.value = "all";
      renderTable();
    });

    loadComments();
  }

  document.addEventListener("DOMContentLoaded", init);
})();
