/**
 * Admin Global Search - scoped server-side search for admin pages.
 */
(function () {
  "use strict";

  let searchTimeout = null;
  let resultsEl = null;
  let isOpen = false;

  document.addEventListener("DOMContentLoaded", () => {
    const input = document.getElementById("admin-global-search");
    if (!input) return;

    resultsEl = document.createElement("div");
    resultsEl.className = "admin-global-search-results";
    resultsEl.style.cssText = `
      position:absolute;top:100%;left:0;right:0;z-index:1001;
      background:var(--admin-bg-card);border:1px solid var(--admin-border);
      border-radius:0 0 var(--admin-radius-lg) var(--admin-radius-lg);
      box-shadow:0 12px 40px rgba(0,0,0,0.15);max-height:420px;overflow-y:auto;
      display:none;
    `;
    input.parentElement.style.position = "relative";
    input.parentElement.appendChild(resultsEl);

    input.addEventListener("input", () => {
      clearTimeout(searchTimeout);
      const q = input.value.trim();
      if (q.length < 2) {
        closeResults();
        return;
      }
      searchTimeout = setTimeout(() => runSearch(q), 300);
    });

    input.addEventListener("keydown", (e) => {
      if (e.key === "Escape") {
        closeResults();
        input.blur();
      }
      if (e.key === "ArrowDown" && isOpen) {
        e.preventDefault();
        const first = resultsEl.querySelector(".admin-search-result-item");
        if (first) first.focus();
      }
    });

    document.addEventListener("click", (e) => {
      if (!input.contains(e.target) && !resultsEl.contains(e.target)) closeResults();
    });

    document.addEventListener("keydown", (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        input.focus();
        input.select();
      }
    });
  });

  async function runSearch(query) {
    showLoading();
    try {
      const response = await fetch(`/api/admin/search?q=${encodeURIComponent(query)}`);
      if (!response.ok) {
        renderMessage("Search is unavailable. Try again shortly.");
        return;
      }
      const payload = await response.json();
      renderResults(extractResults(payload).slice(0, 15), query);
    } catch {
      renderMessage("Search is unavailable. Check your connection and retry.");
    }
  }

  function extractResults(payload) {
    if (Array.isArray(payload)) return payload;
    if (Array.isArray(payload.results)) return payload.results;

    const results = [];
    ["users", "assets", "orders", "deposits"].forEach((type) => {
      if (!Array.isArray(payload[type])) return;
      payload[type].forEach((item) => {
        results.push({
          type: item.type || type.slice(0, -1),
          icon: item.icon,
          title: item.title,
          subtitle: item.subtitle,
          url: item.url,
          badge: item.badge,
        });
      });
    });
    return results;
  }

  function renderResults(results, query) {
    resultsEl.replaceChildren();
    if (results.length === 0) {
      const empty = createEl("div", "", "");
      empty.style.cssText = "padding:24px;text-align:center;color:var(--admin-text-muted);font-size:13px;";
      empty.appendChild(createEl("div", "", "No results for "));
      empty.appendChild(createEl("strong", "", query));
      resultsEl.appendChild(empty);
      openResults();
      return;
    }

    const grouped = {};
    results.forEach((result) => {
      const type = safeText(result.type || "result");
      if (!grouped[type]) grouped[type] = [];
      grouped[type].push(result);
    });

    const labels = { user: "Users", asset: "Assets", order: "Orders", deposit: "Deposits" };
    Object.entries(grouped).forEach(([type, items]) => {
      const label = createEl("div", "", labels[type] || type);
      label.style.cssText = "padding:6px 16px 2px;font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:0.5px;color:var(--admin-text-muted);";
      resultsEl.appendChild(label);
      items.forEach((item) => resultsEl.appendChild(createResultItem(item)));
    });

    const footer = createEl("div", "", `${results.length} result${results.length === 1 ? "" : "s"} - Cmd/Ctrl+K to search`);
    footer.style.cssText = "padding:8px 16px;border-top:1px solid var(--admin-border);text-align:center;font-size:11px;color:var(--admin-text-muted);";
    resultsEl.appendChild(footer);
    bindResultKeyboard();
    openResults();
  }

  function createResultItem(item) {
    const link = createEl("a", "admin-search-result-item");
    link.href = safeAdminUrl(item.url);
    link.tabIndex = 0;
    link.style.cssText = "display:flex;align-items:center;gap:10px;padding:10px 16px;text-decoration:none;color:var(--admin-text-primary);transition:background 0.15s;";

    const icon = createEl("span", "", safeText(item.icon || iconForType(item.type)));
    icon.style.cssText = "font-size:16px;width:24px;text-align:center;";

    const body = createEl("div");
    body.style.cssText = "flex:1;min-width:0;";
    const title = createEl("div", "", item.title || "Untitled");
    title.style.cssText = "font-size:13px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;";
    const subtitle = createEl("div", "", item.subtitle || "");
    subtitle.style.cssText = "font-size:11px;color:var(--admin-text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;";
    body.append(title, subtitle);

    link.append(icon, body);
    if (item.badge) {
      const badge = createEl("span", `admin-badge admin-badge--${badgeClass(item.badge)}`, item.badge);
      badge.style.fontSize = "10px";
      link.appendChild(badge);
    }

    ["mouseover", "focus"].forEach((eventName) => {
      link.addEventListener(eventName, () => {
        link.style.background = "var(--admin-bg-hover)";
      });
    });
    ["mouseout", "blur"].forEach((eventName) => {
      link.addEventListener(eventName, () => {
        link.style.background = "transparent";
      });
    });
    return link;
  }

  function bindResultKeyboard() {
    const items = resultsEl.querySelectorAll(".admin-search-result-item");
    items.forEach((el, i) => {
      el.addEventListener("keydown", (e) => {
        if (e.key === "ArrowDown" && i < items.length - 1) {
          e.preventDefault();
          items[i + 1].focus();
        }
        if (e.key === "ArrowUp") {
          e.preventDefault();
          if (i > 0) items[i - 1].focus();
          else document.getElementById("admin-global-search").focus();
        }
        if (e.key === "Escape") {
          closeResults();
          document.getElementById("admin-global-search").focus();
        }
      });
    });
  }

  function showLoading() {
    renderMessage("Searching...");
  }

  function renderMessage(message) {
    resultsEl.replaceChildren();
    const el = createEl("div", "", message);
    el.style.cssText = "padding:24px;text-align:center;color:var(--admin-text-muted);font-size:13px;";
    resultsEl.appendChild(el);
    openResults();
  }

  function openResults() {
    resultsEl.style.display = "block";
    isOpen = true;
  }

  function closeResults() {
    resultsEl.style.display = "none";
    isOpen = false;
  }

  function safeAdminUrl(url) {
    const value = safeText(url);
    return value.startsWith("/admin/") ? value : "/admin/";
  }

  function iconForType(type) {
    return { user: "U", asset: "A", order: "O", deposit: "$" }[safeText(type)] || "*";
  }

  function badgeClass(status) {
    const s = safeText(status).toLowerCase();
    if (["approved", "completed", "paid", "active", "funding_open"].includes(s)) return "success";
    if (["pending", "processing", "in_review", "submitted"].includes(s)) return "warning";
    if (["rejected", "failed", "cancelled", "expired", "suspended"].includes(s)) return "danger";
    return "neutral";
  }

  function createEl(tag, className, text) {
    const el = document.createElement(tag);
    if (className) el.className = className;
    if (text !== undefined) el.textContent = safeText(text);
    return el;
  }

  function safeText(value) {
    if (value === null || value === undefined) return "";
    return String(value);
  }
})();
