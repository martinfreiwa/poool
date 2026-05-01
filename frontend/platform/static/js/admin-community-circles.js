(function () {
  "use strict";

  const PAGE_SIZE = 50;

  const state = {
    circles: [],
    total: 0,
    totalMembers: 0,
    totalXp: 0,
    offset: 0,
    loading: false,
    error: "",
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

  function formatNumber(value) {
    return Number(value || 0).toLocaleString();
  }

  function formatDate(value) {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? "Unknown" : date.toLocaleDateString();
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
    badge.className = "admin-badge";
    badge.style.background = circle.is_public ? "#dcfce7" : "#f3f4f6";
    badge.style.color = circle.is_public ? "#166534" : "#4b5563";
    badge.textContent = circle.is_public ? "Public" : "Private";
    cell.appendChild(badge);
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

  function renderRows() {
    if (!els.tbody) return;
    els.tbody.replaceChildren();

    state.circles.forEach((circle) => {
      const row = document.createElement("tr");
      row.appendChild(createCircleCell(circle));
      row.appendChild(createLevelCell(circle));
      appendCell(row, `${circle.member_count || 0} / ${circle.max_members || 0}`);
      appendCell(row, formatNumber(circle.total_xp));
      appendCell(row, formatDate(circle.created_at));
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
    els.refresh?.addEventListener("click", loadCircles);
    els.errorRetry?.addEventListener("click", loadCircles);
    els.filterForm?.addEventListener("submit", (event) => {
      event.preventDefault();
      state.offset = 0;
      loadCircles();
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
    document.addEventListener("keydown", (event) => {
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
  });
})();
