(function () {
  "use strict";

  const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
  const MAX_XP_ADJUSTMENT = 10000;

  const state = {
    lastFocused: null,
    submitting: false,
  };

  function $(id) {
    return document.getElementById(id);
  }

  async function parseJsonResponse(res) {
    try {
      return await res.json();
    } catch (_err) {
      return {};
    }
  }

  function setLeaderboardState(mode, message) {
    $("leaderboard-loading").hidden = mode !== "loading";
    $("leaderboard-error").hidden = mode !== "error";
    $("leaderboard-table").hidden = mode !== "table";
    $("leaderboard-empty").hidden = mode !== "empty";

    if (mode === "error") {
      $("leaderboard-error-message").textContent = message || "Unable to load leaderboard.";
    }
  }

  function formatNumber(value) {
    const n = Number(value);
    return Number.isFinite(n) ? n.toLocaleString() : "0";
  }

  function appendCell(row, text, options = {}) {
    const td = document.createElement("td");
    if (options.alignRight) td.style.textAlign = "right";
    if (options.bold) td.style.fontWeight = "600";
    if (text !== undefined) td.textContent = text;
    row.appendChild(td);
    return td;
  }

  function renderLeaderboard(entries) {
    const tbody = $("leaderboard-body");
    tbody.replaceChildren();

    entries.forEach((entry, index) => {
      const row = document.createElement("tr");

      const rankCell = appendCell(row);
      const rank = document.createElement("div");
      rank.style.fontWeight = "500";
      rank.style.color = "var(--text-tertiary)";
      rank.textContent = `#${index + 1}`;
      rankCell.appendChild(rank);

      const userCell = appendCell(row);
      const userWrap = document.createElement("div");
      userWrap.style.display = "flex";
      userWrap.style.alignItems = "center";
      userWrap.style.gap = "8px";

      const userId = document.createElement("div");
      userId.style.fontFamily = "monospace";
      userId.style.fontSize = "13px";
      userId.style.color = "var(--brand-blue)";
      userId.textContent = entry.user_id || "";
      userWrap.appendChild(userId);

      const userLink = document.createElement("a");
      userLink.href = `/admin/community/user-detail.html?id=${encodeURIComponent(entry.user_id || "")}`;
      userLink.textContent = "Open";
      userLink.setAttribute("aria-label", `Open community user detail for ${entry.user_id || "user"}`);
      userLink.style.color = "var(--text-tertiary)";
      userWrap.appendChild(userLink);
      userCell.appendChild(userWrap);

      const levelCell = appendCell(row);
      const level = document.createElement("div");
      level.style.fontWeight = "500";
      level.style.fontFamily = "var(--font-heading)";
      level.textContent = `Lvl ${entry.level || 1}`;
      const levelName = document.createElement("div");
      levelName.style.fontSize = "13px";
      levelName.style.color = "var(--text-tertiary)";
      levelName.textContent = entry.level_name || "Seedling";
      levelCell.append(level, levelName);

      appendCell(row, formatNumber(entry.xp_total), { alignRight: true, bold: true });
      appendCell(row, `${formatNumber(entry.login_streak)} streak`, { alignRight: true });

      const actionCell = appendCell(row);
      actionCell.style.textAlign = "right";
      const action = document.createElement("button");
      action.type = "button";
      action.className = "admin-btn admin-btn--sm admin-btn--outline";
      action.textContent = "Manage XP";
      action.addEventListener("click", () => openModal(entry.user_id || ""));
      actionCell.appendChild(action);

      tbody.appendChild(row);
    });
  }

  async function loadLeaderboard() {
    setLeaderboardState("loading");
    try {
      const res = await fetch("/api/admin/community/leaderboard?limit=100", {
        credentials: "same-origin",
      });
      const data = await parseJsonResponse(res);
      if (!res.ok) {
        throw new Error(data.error || data.message || "Unable to load leaderboard.");
      }

      const entries = Array.isArray(data.leaderboard) ? data.leaderboard : [];
      renderLeaderboard(entries);
      setLeaderboardState(entries.length ? "table" : "empty");
    } catch (err) {
      console.error("Failed to load admin community leaderboard", err);
      setLeaderboardState("error", err.message || "Unable to load leaderboard.");
    }
  }

  function setModalStatus(message, kind) {
    const status = $("xp-modal-status");
    if (!message) {
      status.hidden = true;
      status.textContent = "";
      status.className = "admin-alert";
      return;
    }
    status.hidden = false;
    status.textContent = message;
    status.className = `admin-alert admin-alert--${kind || "error"}`;
  }

  function setSubmitting(isSubmitting) {
    state.submitting = isSubmitting;
    $("submit-xp-btn").disabled = isSubmitting;
    $("submit-xp-btn-label").textContent = isSubmitting ? "Processing..." : "Submit XP Adjustment";
  }

  function focusableModalElements() {
    return Array.from(
      $("xp-modal").querySelectorAll(
        'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      ),
    );
  }

  function openModal(userId) {
    state.lastFocused = document.activeElement;
    $("xp-user-id").value = userId || "";
    $("xp-amount").value = "50";
    $("xp-action").value = "admin_grant";
    $("xp-description").value = "Manual XP adjustment by admin";
    setModalStatus("");
    $("xp-modal-overlay").classList.add("active");
    window.requestAnimationFrame(() => {
      ($("xp-user-id").value ? $("xp-amount") : $("xp-user-id")).focus();
    });
  }

  function closeModal() {
    if (state.submitting) return;
    $("xp-modal-overlay").classList.remove("active");
    setModalStatus("");
    if (state.lastFocused && typeof state.lastFocused.focus === "function") {
      state.lastFocused.focus();
    }
  }

  function getAdjustmentPayload() {
    const userId = $("xp-user-id").value.trim();
    const amount = Number($("xp-amount").value);
    const action = $("xp-action").value;
    const description = $("xp-description").value.trim();

    if (!UUID_RE.test(userId)) {
      throw new Error("Enter a valid user UUID.");
    }
    if (!Number.isInteger(amount) || amount < 1 || amount > MAX_XP_ADJUSTMENT) {
      throw new Error(`XP amount must be a whole number from 1 to ${MAX_XP_ADJUSTMENT}.`);
    }
    if (!["admin_grant", "admin_revoke"].includes(action)) {
      throw new Error("Choose a valid XP action.");
    }
    if (!description || description.length > 200) {
      throw new Error("Description is required and must be 200 characters or fewer.");
    }

    return {
      userId,
      payload: {
        amount: action === "admin_grant" ? amount : -amount,
        reason_label: action,
        description,
      },
    };
  }

  async function submitXpAdjustment() {
    if (state.submitting) return;

    let request;
    try {
      request = getAdjustmentPayload();
    } catch (err) {
      setModalStatus(err.message, "error");
      return;
    }

    setSubmitting(true);
    setModalStatus("");

    try {
      const res = await fetch(`/api/admin/community/users/${request.userId}/xp`, {
        method: "POST",
        credentials: "same-origin",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request.payload),
      });
      const data = await parseJsonResponse(res);
      if (!res.ok) {
        throw new Error(data.error || data.message || "Failed to adjust XP.");
      }

      setModalStatus(`XP adjusted. New total: ${formatNumber(data.new_xp)}.`, "success");
      await loadLeaderboard();
      window.setTimeout(closeModal, 650);
    } catch (err) {
      console.error("Failed to adjust XP", err);
      setModalStatus(err.message || "Failed to adjust XP.", "error");
    } finally {
      setSubmitting(false);
    }
  }

  function handleModalKeydown(event) {
    if (!$("xp-modal-overlay").classList.contains("active")) return;
    if (event.key === "Escape") {
      event.preventDefault();
      closeModal();
      return;
    }
    if (event.key !== "Tab") return;

    const focusable = focusableModalElements();
    if (!focusable.length) return;
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

  function bindEvents() {
    $("refresh-leaderboard-btn").addEventListener("click", loadLeaderboard);
    $("retry-leaderboard-btn").addEventListener("click", loadLeaderboard);
    $("open-xp-modal-btn").addEventListener("click", () => openModal(""));
    $("close-xp-modal-btn").addEventListener("click", closeModal);
    $("cancel-xp-modal-btn").addEventListener("click", closeModal);
    $("submit-xp-btn").addEventListener("click", submitXpAdjustment);
    $("xp-modal-overlay").addEventListener("click", (event) => {
      if (event.target === $("xp-modal-overlay")) closeModal();
    });
    document.addEventListener("keydown", handleModalKeydown);
  }

  document.addEventListener("DOMContentLoaded", () => {
    bindEvents();
    loadLeaderboard();
  });
})();
