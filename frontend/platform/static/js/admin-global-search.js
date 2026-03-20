/**
 * Admin Global Search — Unified search via server-side /api/admin/search endpoint.
 * Searches all relevant entities (Users, Assets, Orders, Deposits, Tickets) in a single
 * indexed DB query instead of downloading all data from 4 endpoints per keystroke.
 * Loaded on every admin page via the #admin-global-search input.
 */
(function () {
  "use strict";

  let searchTimeout = null;
  let resultsEl = null;
  let isOpen = false;
  let currentRequest = null; // AbortController for in-flight requests

  document.addEventListener("DOMContentLoaded", () => {
    const input = document.getElementById("admin-global-search");
    if (!input) return;

    // Create results dropdown
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

    // Listen for typed input
    input.addEventListener("input", () => {
      clearTimeout(searchTimeout);
      const q = input.value.trim();
      if (q.length < 2) {
        closeResults();
        return;
      }
      searchTimeout = setTimeout(() => runSearch(q), 300);
    });

    // Keyboard navigation
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

    // Close on outside click
    document.addEventListener("click", (e) => {
      if (!input.contains(e.target) && !resultsEl.contains(e.target))
        closeResults();
    });

    // Keyboard shortcut: Cmd+K or Ctrl+K to focus search
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

    // Cancel any in-flight request
    if (currentRequest) currentRequest.abort();
    currentRequest = new AbortController();

    try {
      const resp = await fetch(
        `/api/admin/search?q=${encodeURIComponent(query)}&limit=15`,
        { signal: currentRequest.signal }
      );

      if (!resp.ok) {
        renderError("Search failed — server error");
        return;
      }

      const data = await resp.json();
      const results = data.results || [];
      renderResults(results, query);
    } catch (e) {
      // Don't show error for aborted requests (user typed again)
      if (e.name === "AbortError") return;
      renderError("Search unavailable");
    } finally {
      currentRequest = null;
    }
  }

  function renderResults(results, query) {
    if (results.length === 0) {
      resultsEl.innerHTML = `
                <div style="padding:24px;text-align:center;color:var(--admin-text-muted);font-size:13px;">
                    <div style="font-size:20px;margin-bottom:8px;">🔍</div>
                    No results for "<strong>${esc(query)}</strong>"
                </div>`;
      openResults();
      return;
    }

    const grouped = {};
    results.forEach((r) => {
      if (!grouped[r.type]) grouped[r.type] = [];
      grouped[r.type].push(r);
    });

    const typeLabels = {
      user: "Users",
      asset: "Assets",
      order: "Orders",
      deposit: "Deposits",
      ticket: "Support Tickets",
    };

    let html = "";
    for (const [type, items] of Object.entries(grouped)) {
      html += `<div style="padding:6px 16px 2px;font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:0.5px;color:var(--admin-text-muted);">${typeLabels[type] || type}</div>`;
      items.forEach((item) => {
        html += `
                    <a href="${esc(item.url)}" class="admin-search-result-item" tabindex="0"
                       style="display:flex;align-items:center;gap:10px;padding:10px 16px;text-decoration:none;color:var(--admin-text-primary);transition:background 0.15s;"
                       onmouseover="this.style.background='var(--admin-bg-hover)'" onmouseout="this.style.background='transparent'"
                       onfocus="this.style.background='var(--admin-bg-hover)'" onblur="this.style.background='transparent'">
                        <span style="font-size:16px;width:24px;text-align:center;">${item.icon}</span>
                        <div style="flex:1;min-width:0;">
                            <div style="font-size:13px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${esc(item.title)}</div>
                            <div style="font-size:11px;color:var(--admin-text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">${esc(item.subtitle)}</div>
                        </div>
                        ${item.badge ? `<span class="admin-badge admin-badge--${badgeClass(item.badge)}" style="font-size:10px;">${esc(item.badge)}</span>` : ""}
                    </a>`;
      });
    }

    html += `
            <div style="padding:8px 16px;border-top:1px solid var(--admin-border);text-align:center;">
                <span style="font-size:11px;color:var(--admin-text-muted);">${results.length} result${results.length !== 1 ? "s" : ""} · <kbd style="padding:1px 4px;border:1px solid var(--admin-border);border-radius:3px;font-size:10px;">⌘K</kbd> to search</span>
            </div>`;

    resultsEl.innerHTML = html;

    // Arrow key navigation between results
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

    openResults();
  }

  function renderError(message) {
    resultsEl.innerHTML = `
            <div style="padding:24px;text-align:center;color:var(--admin-text-muted);font-size:13px;">
                <div style="font-size:20px;margin-bottom:8px;">⚠️</div>
                ${esc(message)}
            </div>`;
    openResults();
  }

  function showLoading() {
    resultsEl.innerHTML = `
            <div style="padding:24px;text-align:center;color:var(--admin-text-muted);font-size:13px;">
                <div style="width:18px;height:18px;border:2px solid var(--admin-border);border-top-color:var(--admin-accent);border-radius:50%;animation:spin 0.8s linear infinite;margin:0 auto 8px;"></div>
                Searching…
            </div>`;
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

  function badgeClass(status) {
    const s = (status || "").toLowerCase();
    if (["approved", "completed", "paid", "active", "funding_open", "resolved"].includes(s))
      return "success";
    if (["pending", "processing", "in_review", "submitted", "in_progress", "open"].includes(s))
      return "warning";
    if (["rejected", "failed", "cancelled", "expired", "suspended", "closed"].includes(s))
      return "danger";
    return "neutral";
  }

  function esc(s) {
    if (typeof s !== "string") return s || "";
    const d = document.createElement("div");
    d.textContent = s;
    return d.innerHTML;
  }
})();
