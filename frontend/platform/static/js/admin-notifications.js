/**
 * Admin Notifications Center JS
 */
let allNotifs = [];
let filteredNotifs = [];
let currentPage = 1;
let isLoading = false;
let loadError = "";
let isBroadcasting = false;
const PAGE_SIZE = 15;
let sortField = "created_at";
let sortOrder = "desc";

document.addEventListener("DOMContentLoaded", () => {
  loadNotifications();
  document
    .getElementById("notif-search")
    ?.addEventListener("input", debounce(applyFilters, 200));
  document
    .getElementById("filter-type")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("filter-read")
    ?.addEventListener("change", applyFilters);
  document
    .getElementById("broadcast-send-btn")
    ?.addEventListener("click", sendBroadcast);

  setupSorting();
  setupPagination();
  updateSortState();
});

async function loadNotifications() {
  isLoading = true;
  loadError = "";
  setStatsUnavailable();
  renderTable();

  try {
    const response = await fetch("/api/admin/notifications");
    const payload = await parseJsonResponse(response);
    if (!response.ok) {
      throw new Error(payload.error || `Notifications API returned ${response.status}`);
    }
    allNotifs = Array.isArray(payload.notifications) ? payload.notifications : [];
    isLoading = false;
    updateStats();
    applyFilters();
  } catch (error) {
    console.error("Notifications fetch failed:", error);
    if (window.Sentry) window.Sentry.captureException(error);
    allNotifs = [];
    filteredNotifs = [];
    isLoading = false;
    loadError = error.message || "Failed to load notifications.";
    setStatsUnavailable();
    updateCountLabel(0);
    renderTable();
  }
}

function setupSorting() {
  document.querySelectorAll(".admin-table th[data-sort]").forEach((th) => {
    const button = th.querySelector("button");
    if (!button) return;
    button.addEventListener("click", () => {
      const field = th.dataset.sort;
      if (sortField === field) {
        sortOrder = sortOrder === "asc" ? "desc" : "asc";
      } else {
        sortField = field;
        sortOrder = "asc";
      }
      updateSortState();
      applyFilters();
    });
  });
}

function updateSortState() {
  document.querySelectorAll(".admin-table th[data-sort]").forEach((th) => {
    if (th.dataset.sort === sortField) {
      th.setAttribute("aria-sort", sortOrder === "asc" ? "ascending" : "descending");
    } else {
      th.setAttribute("aria-sort", "none");
    }
  });
}

function setupPagination() {
  document.getElementById("prev-page")?.addEventListener("click", () => {
    if (currentPage > 1) {
      currentPage--;
      renderTable();
    }
  });
  document.getElementById("next-page")?.addEventListener("click", () => {
    const totalPages = Math.ceil(filteredNotifs.length / PAGE_SIZE);
    if (currentPage < totalPages) {
      currentPage++;
      renderTable();
    }
  });
}

function applyFilters() {
  if (isLoading || loadError) {
    renderTable();
    return;
  }

  const search = (
    document.getElementById("notif-search")?.value || ""
  ).toLowerCase();
  const type = document.getElementById("filter-type")?.value || "";
  const read = document.getElementById("filter-read")?.value;

  let result = allNotifs.slice();
  if (type) result = result.filter((n) => n.type === type);
  if (read === "true") result = result.filter((n) => n.is_read);
  else if (read === "false") result = result.filter((n) => !n.is_read);
  if (search) {
    result = result.filter((n) =>
      `${n.title || ""} ${n.message || ""} ${n.user_email || ""} ${n.user_name || ""}`
        .toLowerCase()
        .includes(search),
    );
  }

  result.sort((a, b) => {
    const valA = sortValue(a[sortField]);
    const valB = sortValue(b[sortField]);
    if (valA < valB) return sortOrder === "asc" ? -1 : 1;
    if (valA > valB) return sortOrder === "asc" ? 1 : -1;
    return 0;
  });

  filteredNotifs = result;
  currentPage = 1;
  updateCountLabel(filteredNotifs.length);
  renderTable();
}

function sortValue(value) {
  if (value === null || value === undefined) return "";
  if (typeof value === "boolean") return value ? 1 : 0;
  return String(value).toLowerCase();
}

function updateStats() {
  const total = allNotifs.length;
  const unread = allNotifs.filter((n) => !n.is_read).length;
  const today = allNotifs.filter((n) => {
    const d = new Date(n.created_at);
    const t = new Date();
    return d.toDateString() === t.toDateString();
  }).length;
  setText("stat-total", total);
  setText("stat-unread", unread);
  setText(
    "stat-read-rate",
    total > 0 ? `${Math.round(((total - unread) / total) * 100)}%` : "—",
  );
  setText("stat-today", today);
}

function setStatsUnavailable() {
  ["stat-total", "stat-unread", "stat-read-rate", "stat-today"].forEach((id) =>
    setText(id, "—"),
  );
}

function renderTable() {
  const tbody = document.getElementById("notif-table-body");
  if (!tbody) return;
  tbody.replaceChildren();

  if (isLoading) {
    tbody.appendChild(stateRow("Loading notifications…"));
    updatePagination(1, 0);
    return;
  }

  if (loadError) {
    const row = stateRow(loadError, { isError: true, retry: true });
    tbody.appendChild(row);
    updatePagination(1, 0);
    return;
  }

  const totalPages = Math.max(1, Math.ceil(filteredNotifs.length / PAGE_SIZE));
  currentPage = Math.min(currentPage, totalPages);
  const start = (currentPage - 1) * PAGE_SIZE;
  const slice = filteredNotifs.slice(start, start + PAGE_SIZE);
  updatePagination(totalPages, filteredNotifs.length);

  if (!slice.length) {
    tbody.appendChild(stateRow("No notifications found."));
    return;
  }

  slice.forEach((notification) => tbody.appendChild(notificationRow(notification)));
}

function stateRow(message, options = {}) {
  const tr = document.createElement("tr");
  const td = document.createElement("td");
  td.colSpan = 5;
  td.style.textAlign = "center";
  td.style.padding = "40px";
  td.style.color = options.isError ? "var(--admin-danger)" : "var(--admin-text-muted)";
  if (options.isError) td.setAttribute("role", "alert");

  const messageEl = document.createElement("div");
  messageEl.textContent = message;
  td.appendChild(messageEl);

  if (options.retry) {
    const retry = document.createElement("button");
    retry.type = "button";
    retry.className = "admin-btn admin-btn--secondary admin-btn--sm";
    retry.style.marginTop = "12px";
    retry.textContent = "Retry";
    retry.addEventListener("click", loadNotifications);
    td.appendChild(retry);
  }

  tr.appendChild(td);
  return tr;
}

function notificationRow(notification) {
  const tr = document.createElement("tr");
  if (!notification.is_read) tr.style.background = "var(--admin-accent-bg)";

  const userCell = document.createElement("td");
  const userInline = document.createElement("div");
  userInline.className = "admin-user-inline";
  const userText = document.createElement("div");
  const name = document.createElement("div");
  name.className = "admin-user-inline-name";
  name.textContent = notification.user_name || "";
  const email = document.createElement("div");
  email.className = "admin-user-inline-email";
  email.textContent = notification.user_email || "";
  userText.append(name, email);
  userInline.appendChild(userText);
  userCell.appendChild(userInline);

  const typeCell = document.createElement("td");
  typeCell.appendChild(typeBadge(notification.type));

  const titleCell = document.createElement("td");
  const title = document.createElement("div");
  title.style.fontWeight = notification.is_read ? "400" : "600";
  title.style.color = "var(--admin-text-primary)";
  title.style.marginBottom = "2px";
  title.textContent = notification.title || "";
  const message = document.createElement("div");
  message.style.fontSize = "11px";
  message.style.color = "var(--admin-text-muted)";
  message.style.maxWidth = "300px";
  message.style.overflow = "hidden";
  message.style.textOverflow = "ellipsis";
  message.style.whiteSpace = "nowrap";
  message.textContent = notification.message || "";
  titleCell.append(title, message);

  const readCell = document.createElement("td");
  const readBadge = document.createElement("span");
  readBadge.className = notification.is_read
    ? "admin-badge admin-badge--neutral"
    : "admin-badge admin-badge--warning";
  readBadge.textContent = notification.is_read ? "Read" : "Unread";
  readCell.appendChild(readBadge);

  const dateCell = document.createElement("td");
  dateCell.style.fontSize = "12px";
  dateCell.style.color = "var(--admin-text-muted)";
  dateCell.style.whiteSpace = "nowrap";
  dateCell.textContent = fmtDate(notification.created_at);

  tr.append(userCell, typeCell, titleCell, readCell, dateCell);
  return tr;
}

function updatePagination(totalPages, totalCount) {
  const info = document.getElementById("pagination-info");
  if (info) info.textContent = `Page ${currentPage} of ${totalPages} (${totalCount} total)`;
  const prevBtn = document.getElementById("prev-page");
  const nextBtn = document.getElementById("next-page");
  if (prevBtn) prevBtn.disabled = currentPage <= 1 || isLoading || !!loadError;
  if (nextBtn) nextBtn.disabled = currentPage >= totalPages || isLoading || !!loadError;
}

function updateCountLabel(count) {
  setText("notif-count-label", `${count} notifications`);
}

async function sendBroadcast() {
  if (isBroadcasting) return;

  const type = document.getElementById("broadcast-type")?.value || "system";
  const titleInput = document.getElementById("broadcast-title");
  const messageInput = document.getElementById("broadcast-message");
  const title = titleInput?.value.trim() || "";
  const message = messageInput?.value.trim() || "";

  if (!title) {
    setBroadcastStatus("Title is required.", "error");
    titleInput?.focus();
    return;
  }
  if (!message) {
    setBroadcastStatus("Message is required.", "error");
    messageInput?.focus();
    return;
  }

  setBroadcastBusy(true);
  setBroadcastStatus("Sending broadcast…", "status");
  try {
    const response = await fetch("/api/admin/notifications/broadcast", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ type, title, message }),
    });
    const payload = await parseJsonResponse(response);
    if (!response.ok) {
      throw new Error(payload.error || "Failed to send broadcast.");
    }

    titleInput.value = "";
    messageInput.value = "";
    setBroadcastStatus(
      `Broadcast sent to ${payload.count || 0} recipients.`,
      "success",
    );
    await loadNotifications();
  } catch (error) {
    setBroadcastStatus(error.message || "Network error sending broadcast.", "error");
  } finally {
    setBroadcastBusy(false);
  }
}

function setBroadcastBusy(isBusy) {
  isBroadcasting = isBusy;
  const button = document.getElementById("broadcast-send-btn");
  if (!button) return;
  button.disabled = isBusy;
  button.setAttribute("aria-busy", isBusy ? "true" : "false");
  button.dataset.originalText = button.dataset.originalText || button.textContent.trim();
  const textNode = Array.from(button.childNodes).find((node) => node.nodeType === Node.TEXT_NODE);
  if (textNode) textNode.textContent = isBusy ? " Sending…" : " Send to All";
}

function setBroadcastStatus(message, type) {
  const el = document.getElementById("broadcast-status");
  if (!el) return;
  el.textContent = message;
  el.setAttribute("role", type === "error" ? "alert" : "status");
  el.style.color = type === "error"
    ? "var(--admin-danger)"
    : type === "success"
      ? "var(--admin-success)"
      : "var(--admin-text-muted)";
}

function typeBadge(type) {
  const badgeMap = {
    system: ["admin-badge--neutral", "System"],
    kyc: ["admin-badge--warning", "KYC"],
    investment: ["admin-badge--info", "Investment"],
    payout: ["admin-badge--success", "Payout"],
    promo: ["admin-badge--info", "Promo"],
  };
  const [className, label] = badgeMap[type] || ["admin-badge--neutral", type || "Unknown"];
  const badge = document.createElement("span");
  badge.className = `admin-badge ${className}`;
  badge.textContent = label;
  return badge;
}

function setText(id, value) {
  const el = document.getElementById(id);
  if (el) el.textContent = String(value);
}

async function parseJsonResponse(response) {
  try {
    return await response.json();
  } catch (_error) {
    return {};
  }
}

function fmtDate(iso) {
  if (!iso) return "—";
  return new Date(iso).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function debounce(fn, ms) {
  let t;
  return function (...args) {
    clearTimeout(t);
    t = setTimeout(() => fn.apply(this, args), ms);
  };
}
