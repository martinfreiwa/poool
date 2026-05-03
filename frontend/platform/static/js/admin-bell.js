/**
 * Admin notifications bell — drop-down panel with grouped alerts.
 * Wires the existing .admin-notification-btn in the topbar to a fetch-backed
 * panel that groups by type, sorts unread first, and links into the relevant
 * admin section.
 */
(function () {
  const TYPE_META = {
    kyc: { label: "KYC", icon: "🛂", href: "/admin/kyc.html" },
    kyc_pending: { label: "KYC", icon: "🛂", href: "/admin/kyc.html" },
    deposit: { label: "Deposit", icon: "💵", href: "/admin/deposits.html" },
    withdrawal: { label: "Withdrawal", icon: "💸", href: "/admin/deposits.html" },
    fraud: { label: "Fraud", icon: "🚩", href: "/admin/users.html?view=flagged" },
    flag: { label: "Flag", icon: "🚩", href: "/admin/users.html" },
    order: { label: "Order", icon: "📑", href: "/admin/orders.html" },
    settlement: { label: "Settlement", icon: "📑", href: "/admin/pending-settlements.html" },
    support: { label: "Support", icon: "💬", href: "/admin/support-ticket.html" },
    system: { label: "System", icon: "⚙️", href: "/admin/audit-logs.html" },
  };

  function meta(type) {
    return (
      TYPE_META[type] || {
        label: (type || "info").toUpperCase(),
        icon: "•",
        href: "/admin/audit-logs.html",
      }
    );
  }

  function timeAgo(iso) {
    if (!iso) return "";
    const t = new Date(iso.replace(" ", "T")).getTime();
    if (!t) return "";
    const diff = Date.now() - t;
    const m = Math.floor(diff / 60000);
    if (m < 1) return "just now";
    if (m < 60) return `${m}m`;
    const h = Math.floor(m / 60);
    if (h < 24) return `${h}h`;
    const d = Math.floor(h / 24);
    return `${d}d`;
  }

  function escapeHtml(s) {
    const d = document.createElement("div");
    d.textContent = String(s ?? "");
    return d.innerHTML;
  }

  let panelEl = null;
  let lastNotifs = [];

  function ensurePanel() {
    if (panelEl) return panelEl;
    panelEl = document.createElement("div");
    panelEl.className = "admin-bell-panel";
    panelEl.hidden = true;
    panelEl.setAttribute("role", "dialog");
    panelEl.setAttribute("aria-label", "Notifications");
    panelEl.innerHTML = `
      <div class="admin-bell-panel__header">
        <strong>Notifications</strong>
        <div class="admin-bell-panel__header-actions">
          <button type="button" data-bell-action="mark-all-read" class="admin-btn admin-btn--secondary admin-btn--sm">Mark all read</button>
          <a href="/admin/notifications.html" class="admin-btn admin-btn--secondary admin-btn--sm">All →</a>
        </div>
      </div>
      <div class="admin-bell-panel__filter" role="tablist">
        <button data-bell-tab="all" aria-selected="true">All</button>
        <button data-bell-tab="unread">Unread</button>
        <button data-bell-tab="kyc">KYC</button>
        <button data-bell-tab="deposit">Deposits</button>
        <button data-bell-tab="fraud">Risk</button>
      </div>
      <div class="admin-bell-panel__body" id="admin-bell-list"></div>
      <div class="admin-bell-panel__footer">
        <span id="admin-bell-meta"></span>
      </div>
    `;
    document.body.appendChild(panelEl);

    panelEl.addEventListener("click", (e) => {
      const tab = e.target.closest("[data-bell-tab]");
      if (tab) {
        panelEl
          .querySelectorAll("[data-bell-tab]")
          .forEach((t) =>
            t.setAttribute(
              "aria-selected",
              t === tab ? "true" : "false",
            ),
          );
        renderList(tab.dataset.bellTab);
        return;
      }
      const action = e.target.closest("[data-bell-action]")?.dataset.bellAction;
      if (action === "mark-all-read") markAllRead();
    });

    document.addEventListener("click", (e) => {
      if (panelEl.hidden) return;
      if (
        !e.target.closest(".admin-bell-panel") &&
        !e.target.closest(".admin-notification-btn")
      ) {
        panelEl.hidden = true;
      }
    });
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && !panelEl.hidden) panelEl.hidden = true;
    });

    return panelEl;
  }

  function groupSort(notifs, tab) {
    let list = notifs.slice();
    if (tab === "unread") list = list.filter((n) => !n.is_read);
    else if (tab && tab !== "all")
      list = list.filter((n) => (n.type || "").toLowerCase().includes(tab));
    // unread first, then newest
    list.sort((a, b) => {
      if (a.is_read !== b.is_read) return a.is_read ? 1 : -1;
      const at = new Date((a.created_at || "").replace(" ", "T")).getTime() || 0;
      const bt = new Date((b.created_at || "").replace(" ", "T")).getTime() || 0;
      return bt - at;
    });
    return list;
  }

  function renderList(tab) {
    const list = document.getElementById("admin-bell-list");
    if (!list) return;
    const items = groupSort(lastNotifs, tab || "all");
    if (items.length === 0) {
      list.innerHTML = `<div class="admin-bell-empty">No notifications.</div>`;
    } else {
      list.innerHTML = items
        .slice(0, 50)
        .map((n) => {
          const m = meta(n.type);
          const unread = !n.is_read;
          return `
            <a href="${escapeHtml(m.href)}" class="admin-bell-item${unread ? " is-unread" : ""}" data-notif-id="${escapeHtml(n.id)}">
              <span class="admin-bell-item__icon" aria-hidden="true">${m.icon}</span>
              <span class="admin-bell-item__body">
                <span class="admin-bell-item__title">${escapeHtml(n.title || m.label)}</span>
                <span class="admin-bell-item__msg">${escapeHtml(n.message || n.user_name || n.user_email || "")}</span>
              </span>
              <span class="admin-bell-item__meta">
                <span class="admin-bell-tag admin-bell-tag--${escapeHtml((n.type || "info").split("_")[0])}">${escapeHtml(m.label)}</span>
                <span class="admin-bell-item__time">${escapeHtml(timeAgo(n.created_at))}</span>
              </span>
            </a>`;
        })
        .join("");
    }
    const meta_el = document.getElementById("admin-bell-meta");
    if (meta_el) {
      const unread = lastNotifs.filter((n) => !n.is_read).length;
      meta_el.textContent = `${lastNotifs.length} total · ${unread} unread`;
    }
  }

  async function fetchNotifs() {
    try {
      const r = await fetch("/api/admin/notifications");
      if (!r.ok) return;
      const data = await r.json();
      lastNotifs = data.notifications || [];
      const unread = lastNotifs.filter((n) => !n.is_read).length;
      updateBadge(unread);
      renderList(currentTab());
    } catch (e) {
      console.error("bell fetch failed", e);
    }
  }

  function currentTab() {
    return (
      panelEl?.querySelector('[data-bell-tab][aria-selected="true"]')
        ?.dataset.bellTab || "all"
    );
  }

  function updateBadge(unread) {
    document
      .querySelectorAll(".admin-topbar .admin-notification-badge")
      .forEach((b) => {
        if (unread > 0) {
          b.style.display = "";
          b.textContent = unread > 99 ? "99+" : String(unread);
        } else {
          b.style.display = "none";
        }
      });
  }

  async function markAllRead() {
    try {
      await fetch("/api/admin/notifications/read-all", { method: "POST" });
    } catch {}
    lastNotifs = lastNotifs.map((n) => ({ ...n, is_read: true }));
    updateBadge(0);
    renderList(currentTab());
  }

  function positionPanel(btn) {
    const r = btn.getBoundingClientRect();
    panelEl.style.top = `${r.bottom + 8}px`;
    panelEl.style.right = `${Math.max(8, window.innerWidth - r.right)}px`;
  }

  function init() {
    const btn = document.querySelector(".admin-topbar .admin-notification-btn");
    if (!btn) return;
    ensurePanel();
    // Document-level capture handler intercepts the click before the legacy
    // navigate-away listener in admin-theme.js (which is bound to the target
    // and would otherwise fire first during target-phase).
    document.addEventListener(
      "click",
      (e) => {
        const hit = e.target.closest(".admin-topbar .admin-notification-btn");
        if (!hit) return;
        e.preventDefault();
        e.stopPropagation();
        e.stopImmediatePropagation();
        if (panelEl.hidden) {
          positionPanel(btn);
          panelEl.hidden = false;
          renderList(currentTab());
        } else {
          panelEl.hidden = true;
        }
      },
      true,
    );
    btn.dataset.bellWired = "true";
    fetchNotifs();
    // Refresh every 60s
    setInterval(fetchNotifs, 60000);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
