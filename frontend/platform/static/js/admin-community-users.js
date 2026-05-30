(function () {
  "use strict";

  const usersTable = document.getElementById("users-table");
  const refreshButton = document.getElementById("refresh-users-btn");
  const statusRegion = document.getElementById("community-users-status");
  const banDialog = document.getElementById("community-user-ban-dialog");
  const banForm = document.getElementById("community-user-ban-form");
  const banTitle = document.getElementById("community-user-ban-title");
  const banDescription = document.getElementById("community-user-ban-description");
  const banReasonField = document.getElementById("community-user-ban-reason-field");
  const banReasonInput = document.getElementById("community-user-ban-reason");
  const banDialogStatus = document.getElementById("community-user-ban-dialog-status");
  const banCancelButton = document.getElementById("community-user-ban-cancel");
  const banSubmitButton = document.getElementById("community-user-ban-submit");

  let activeModerationTarget = null;

  document.addEventListener("DOMContentLoaded", () => {
    refreshButton?.addEventListener("click", () => loadUsers());
    banCancelButton?.addEventListener("click", () => banDialog?.close());
    banForm?.addEventListener("submit", submitBanForm);
    banDialog?.addEventListener("close", resetBanDialog);
    loadUsers();
  });

  async function loadUsers() {
    setStatus("Loading users.");
    setRefreshBusy(true);
    renderMessageRow("Loading users...", false);

    try {
      const response = await fetch("/api/admin/community/users", { credentials: "same-origin" });
      if (!response.ok) {
        renderMessageRow(await getErrorMessage(response, "Failed to load users."), true);
        setStatus("Failed to load community users.", true);
        return;
      }

      const users = await response.json();
      if (!Array.isArray(users) || users.length === 0) {
        renderMessageRow("No community users found.", false);
        setStatus("No community users found.");
        return;
      }

      renderUsers(users);
      setStatus(`${users.length} community ${users.length === 1 ? "user" : "users"} loaded.`);
    } catch (error) {
      console.error(error);
      renderMessageRow("Network error.", true);
      setStatus("Network error while loading community users.", true);
    } finally {
      setRefreshBusy(false);
    }
  }

  function renderUsers(users) {
    const fragment = document.createDocumentFragment();
    users.forEach((user) => fragment.appendChild(buildUserRow(user)));
    usersTable.replaceChildren(fragment);
  }

  function buildUserRow(user) {
    const row = document.createElement("tr");
    const userId = String(user.user_id || "");
    const displayName = String(user.display_name || "Unknown");
    const isBanned = Boolean(user.is_community_banned);

    const userCell = document.createElement("td");
    const profileWrap = document.createElement("div");
    profileWrap.style.cssText = "display:flex;align-items:center;gap:12px;";
    profileWrap.appendChild(buildAvatar(user.avatar_url, displayName));

    const identity = document.createElement("div");
    const detailLink = document.createElement("a");
    detailLink.href = `/admin/community/user-detail.html?id=${encodeURIComponent(userId)}`;
    detailLink.style.cssText = "font-weight:500;color:#101828;text-decoration:none;";
    detailLink.textContent = displayName;
    identity.appendChild(detailLink);

    const shortId = document.createElement("div");
    shortId.style.cssText = "font-size:11px;color:#667085;font-family:monospace;";
    shortId.textContent = `${userId.substring(0, 8)}...`;
    identity.appendChild(shortId);

    profileWrap.appendChild(identity);
    userCell.appendChild(profileWrap);
    row.appendChild(userCell);

    row.appendChild(buildTextCell(String(toInteger(user.post_count))));
    row.appendChild(buildWarningsCell(toInteger(user.warning_count)));
    row.appendChild(buildStatusCell(isBanned));
    row.appendChild(buildDateCell(user.created_at));
    row.appendChild(buildActionsCell(userId, displayName, isBanned));

    return row;
  }

  function buildAvatar(avatarUrl, displayName) {
    const safeAvatarUrl = toSafeImageUrl(avatarUrl);
    if (safeAvatarUrl) {
      const image = document.createElement("img");
      image.src = safeAvatarUrl;
      image.alt = `${displayName} avatar`;
      image.loading = "lazy";
      image.style.cssText = "width:32px;height:32px;border-radius:50%;object-fit:cover;";
      return image;
    }

    const fallback = document.createElement("div");
    fallback.setAttribute("aria-hidden", "true");
    fallback.style.cssText = "width:32px;height:32px;border-radius:50%;background:#f2f4f7;display:flex;align-items:center;justify-content:center;font-weight:600;color:#475467;font-size:12px;";
    fallback.textContent = displayName.charAt(0).toUpperCase() || "?";
    return fallback;
  }

  function buildWarningsCell(count) {
    const cell = document.createElement("td");
    if (count > 0) {
      const badge = document.createElement("span");
      badge.className = "admin-badge";
      badge.style.cssText = "background:#FFFAEB;color:#B54708;";
      badge.textContent = `${count} ${count === 1 ? "Warning" : "Warnings"}`;
      cell.appendChild(badge);
    } else {
      const text = document.createElement("span");
      text.style.color = "#667085";
      text.textContent = "0 Warnings";
      cell.appendChild(text);
    }
    return cell;
  }

  function buildStatusCell(isBanned) {
    const cell = document.createElement("td");
    const badge = document.createElement("span");
    badge.className = "admin-badge";
    badge.style.cssText = isBanned ? "background:#FEF3F2;color:#B42318;" : "background:#ECFDF3;color:#027A48;";
    badge.textContent = isBanned ? "Banned" : "Active";
    cell.appendChild(badge);
    return cell;
  }

  function buildDateCell(value) {
    const cell = document.createElement("td");
    const text = document.createElement("span");
    text.style.cssText = "font-size:13px;color:#667085;";
    const date = new Date(value);
    text.textContent = Number.isNaN(date.getTime()) ? "Unknown" : date.toLocaleDateString();
    cell.appendChild(text);
    return cell;
  }

  function buildActionsCell(userId, displayName, isBanned) {
    const cell = document.createElement("td");
    cell.style.textAlign = "right";

    const actions = document.createElement("div");
    actions.style.cssText = "display:flex;gap:4px;justify-content:flex-end;";

    const viewLink = document.createElement("a");
    viewLink.href = `/admin/community/user-detail.html?id=${encodeURIComponent(userId)}`;
    viewLink.className = "admin-btn admin-btn--ghost admin-btn--sm";
    viewLink.textContent = "View";
    actions.appendChild(viewLink);

    const moderationButton = document.createElement("button");
    moderationButton.type = "button";
    moderationButton.className = isBanned
      ? "admin-btn admin-btn--secondary admin-btn--sm"
      : "admin-btn admin-btn--ghost admin-btn--sm";
    if (!isBanned) moderationButton.style.color = "#D92D20";
    moderationButton.textContent = isBanned ? "Unban" : "Ban";
    moderationButton.addEventListener("click", () => openBanDialog(userId, displayName, !isBanned));
    actions.appendChild(moderationButton);

    cell.appendChild(actions);
    return cell;
  }

  function buildTextCell(value) {
    const cell = document.createElement("td");
    const text = document.createElement("span");
    text.style.fontWeight = "500";
    text.textContent = value;
    cell.appendChild(text);
    return cell;
  }

  function renderMessageRow(message, isError) {
    const row = document.createElement("tr");
    const cell = document.createElement("td");
    cell.colSpan = 6;
    cell.style.cssText = `text-align:center;padding:40px;color:${isError ? "#D92D20" : "var(--admin-text-muted)"};`;
    cell.textContent = message;
    row.appendChild(cell);
    usersTable.replaceChildren(row);
  }

  function openBanDialog(userId, displayName, shouldBan) {
    activeModerationTarget = { userId, displayName, shouldBan };
    banTitle.textContent = shouldBan ? "Ban community user" : "Unban community user";
    banDescription.textContent = shouldBan
      ? `Ban ${displayName} from community posting, commenting, and reactions.`
      : `Restore community access for ${displayName}.`;
    banReasonField.hidden = !shouldBan;
    banReasonInput.required = shouldBan;
    banReasonInput.value = "";
    banDialogStatus.textContent = "";
    banSubmitButton.textContent = shouldBan ? "Ban user" : "Unban user";
    if (typeof banDialog.showModal === "function") {
      banDialog.showModal();
    } else {
      banDialog.setAttribute("open", "open");
    }
    window.setTimeout(() => (shouldBan ? banReasonInput : banSubmitButton).focus(), 0);
  }

  async function submitBanForm(event) {
    event.preventDefault();
    if (!activeModerationTarget) return;

    const reason = banReasonInput.value.trim();
    if (activeModerationTarget.shouldBan && !reason) {
      banDialogStatus.textContent = "Ban reason is required.";
      banReasonInput.focus();
      return;
    }
    if (reason.length > 1000) {
      banDialogStatus.textContent = "Reason must be 1000 characters or fewer.";
      banReasonInput.focus();
      return;
    }

    setBanBusy(true);
    banDialogStatus.textContent = "";
    try {
      const response = await fetch(`/api/admin/community/users/${encodeURIComponent(activeModerationTarget.userId)}/ban`, {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify({
          reason: activeModerationTarget.shouldBan ? reason : null,
          is_banned: activeModerationTarget.shouldBan
        })
      });

      if (!response.ok) {
        banDialogStatus.textContent = await getErrorMessage(response, "Failed to update user ban status.");
        setBanBusy(false);
        return;
      }

      const successMessage = `${activeModerationTarget.displayName} ${activeModerationTarget.shouldBan ? "banned" : "unbanned"}.`;
      setStatus(successMessage);
      banDialog.close();
      await loadUsers();
      setStatus(successMessage);
    } catch (error) {
      console.error(error);
      banDialogStatus.textContent = "Error connecting to server.";
      setBanBusy(false);
    }
  }

  function resetBanDialog() {
    activeModerationTarget = null;
    banForm.reset();
    setBanBusy(false);
    banDialogStatus.textContent = "";
  }

  function setRefreshBusy(isBusy) {
    if (!refreshButton) return;
    refreshButton.disabled = isBusy;
    refreshButton.setAttribute("aria-busy", String(isBusy));
  }

  function setBanBusy(isBusy) {
    banSubmitButton.disabled = isBusy;
    banCancelButton.disabled = isBusy;
    banSubmitButton.setAttribute("aria-busy", String(isBusy));
  }

  function setStatus(message, isError) {
    if (!statusRegion) return;
    statusRegion.textContent = message || "";
    statusRegion.style.color = isError ? "#D92D20" : "var(--admin-text-muted)";
  }

  function csrfHeaders(headers) {
    const token = getCsrfToken();
    return token ? { ...headers, "X-CSRF-Token": token } : headers;
  }

  function getCsrfToken() {
    if (typeof window.getCsrfToken === "function") return window.getCsrfToken() || "";
    const value = `; ${document.cookie}`;
    const parts = value.split("; csrf_token=");
    return parts.length === 2 ? decodeURIComponent(parts.pop().split(";").shift() || "") : "";
  }

  function toSafeImageUrl(value) {
    if (!value) return "";
    try {
      const url = new URL(String(value), window.location.origin);
      if (url.protocol !== "http:" && url.protocol !== "https:") return "";
      return url.href;
    } catch (_error) {
      return "";
    }
  }

  function toInteger(value) {
    const number = Number(value);
    return Number.isFinite(number) ? Math.trunc(number) : 0;
  }

  async function getErrorMessage(response, fallback) {
    try {
      const data = await response.json();
      return data.error || data.message || fallback;
    } catch (_error) {
      return fallback;
    }
  }
})();
